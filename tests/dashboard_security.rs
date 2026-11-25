use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
    Router,
};
use bibliotecaria::{
    cache::GuildCache,
    dashboard::{create_router_with_auth, DashboardState},
    dashboard_auth::DashboardAuth,
};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    cookie: &str,
    body: &str,
    forged: bool,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("cookie", cookie)
        .header("content-type", "application/x-www-form-urlencoded");
    if forged {
        builder = builder
            .header("x-forwarded-for", "127.0.0.1")
            .header("x-forwarded-proto", "https");
    }
    let mut request = builder.body(Body::from(body.to_owned())).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo("127.0.0.1:1234".parse::<SocketAddr>().unwrap()));
    app.clone().oneshot(request).await.unwrap()
}
fn cookie(response: &axum::response::Response) -> String {
    response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}
#[tokio::test]
async fn enabled_production_router_enforces_auth_network_and_session_without_external_io() {
    // This integration-test executable owns its environment and has only this test.
    for name in [
        "DASHBOARD_ADMIN_USER",
        "DASHBOARD_ADMIN_PASSWORD",
        "DASHBOARD_TLS_CERT",
        "DASHBOARD_TLS_KEY",
    ] {
        std::env::remove_var(name);
    }
    assert!(DashboardAuth::from_env().unwrap().is_none());
    std::env::set_var("DASHBOARD_ADMIN_USER", "admin");
    assert!(DashboardAuth::from_env().is_err());
    std::env::set_var("DASHBOARD_ADMIN_PASSWORD", "test-password-only");
    let auth = DashboardAuth::from_env().unwrap().unwrap();
    std::env::set_var("DASHBOARD_TLS_CERT", "/unused.pem");
    assert!(DashboardAuth::from_env().is_err());
    for name in [
        "DASHBOARD_ADMIN_USER",
        "DASHBOARD_ADMIN_PASSWORD",
        "DASHBOARD_TLS_CERT",
    ] {
        std::env::remove_var(name);
    }
    let state = DashboardState {
        pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .unwrap(),
        http: Arc::new(serenity::http::Http::new("unused")),
        guild_cache: Arc::new(GuildCache::new()),
        start_time: chrono::Utc::now(),
    };
    let disabled = create_router_with_auth(state.clone(), None);
    assert_eq!(
        call(&disabled, "GET", "/login", "", "", false)
            .await
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let app = create_router_with_auth(state, Some(auth));
    for (method, path) in [
        ("GET", "/api/stats"),
        ("GET", "/api/guilds"),
        ("GET", "/api/config/123"),
        ("POST", "/api/config/123"),
        ("POST", "/api/message"),
        ("POST", "/api/role"),
        ("GET", "/api/messages/123"),
    ] {
        assert_eq!(
            call(&app, method, path, "", "", false).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(&app, method, path, "", "", true).await.status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        call(&app, "GET", "/health", "", "", false).await.status(),
        StatusCode::OK
    );
    let page = call(&app, "GET", "/login", "", "", false).await;
    let prelogin = cookie(&page);
    let text =
        String::from_utf8(to_bytes(page.into_body(), 16384).await.unwrap().to_vec()).unwrap();
    let csrf = text
        .split("name=csrf value='")
        .nth(1)
        .unwrap()
        .split('\'')
        .next()
        .unwrap();
    let login = call(
        &app,
        "POST",
        "/login",
        &prelogin,
        &format!("username=admin&password=test-password-only&csrf={csrf}"),
        false,
    )
    .await;
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    let session = cookie(&login);
    // The production private handler rejects invalid JSON before any Discord/DB effect.
    assert_eq!(
        call(&app, "POST", "/api/message", &session, "", false)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let home = call(&app, "GET", "/", &session, "", false).await;
    assert_eq!(home.status(), StatusCode::OK);
    let text =
        String::from_utf8(to_bytes(home.into_body(), 16384).await.unwrap().to_vec()).unwrap();
    assert!(text.contains("Authenticated"));
    let csrf = text
        .split("name=csrf value='")
        .nth(1)
        .unwrap()
        .split('\'')
        .next()
        .unwrap();
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/message")
        .header("cookie", &session)
        .header("x-csrf-token", csrf)
        .header("content-type", "application/json")
        .body(Body::from("not json"))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo("127.0.0.1:1".parse::<SocketAddr>().unwrap()));
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
}
