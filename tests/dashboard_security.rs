use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use bibliotecaria::{
    cache::GuildCache,
    dashboard::{create_router_with_auth, DashboardState},
};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;

#[tokio::test]
async fn anonymous_and_forged_headers_cannot_reach_admin_routes() {
    let state = DashboardState {
        pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
            .unwrap(),
        http: Arc::new(serenity::http::Http::new("unused")),
        guild_cache: Arc::new(GuildCache::new()),
        start_time: chrono::Utc::now(),
    };
    let app = create_router_with_auth(state, None);
    for peer in ["127.0.0.1:1234", "192.168.1.1:1234"] {
        let mut request = Request::builder()
            .uri("/api/config/123")
            .header("x-forwarded-for", "127.0.0.1")
            .header("x-forwarded-proto", "https")
            .body(Body::empty())
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        let response = app.clone().oneshot(request).await.unwrap();
        assert!(
            matches!(
                response.status(),
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::SERVICE_UNAVAILABLE
            ),
            "unexpected {}",
            response.status()
        );
    }
}
