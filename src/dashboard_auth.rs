//! Direct-client dashboard authentication. Forwarded headers are never trusted.
use axum::{
    extract::{ConnectInfo, Form, Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

const TTL: Duration = Duration::from_secs(8 * 3600);
const WINDOW: Duration = Duration::from_secs(60);
const LIMIT: usize = 256;
#[derive(Clone)]
pub struct DashboardAuth {
    credentials: [u8; 32],
    pub tls: bool,
    records: Arc<Mutex<Records>>,
}
#[derive(Default)]
struct Records {
    sessions: HashMap<String, Session>,
    attempts: HashMap<IpAddr, (Instant, u32)>,
}
struct Session {
    expires: Instant,
    csrf: String,
    authenticated: bool,
}

pub fn is_allowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V6(v) => v
            .to_ipv4_mapped()
            .map(|v| is_allowed_ip(v.into()))
            .unwrap_or(v.is_loopback()),
        IpAddr::V4(v) => v.is_loopback() || v.octets()[0] == 10,
    }
}
fn normalized(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v) => v.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        _ => ip,
    }
}
fn digest(user: &str, password: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update((user.len() as u64).to_be_bytes());
    h.update(user);
    h.update(password);
    h.finalize().into()
}
fn token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
impl DashboardAuth {
    pub fn from_env() -> Result<Option<Self>, String> {
        let user = std::env::var("DASHBOARD_ADMIN_USER").unwrap_or_default();
        let password = std::env::var("DASHBOARD_ADMIN_PASSWORD").unwrap_or_default();
        if user.is_empty() && password.is_empty() {
            return Ok(None);
        }
        if user.trim().is_empty() || password.len() < 16 {
            return Err(
                "Dashboard requires an administrator name and password of at least 16 bytes".into(),
            );
        }
        let cert = std::env::var("DASHBOARD_TLS_CERT").unwrap_or_default();
        let key = std::env::var("DASHBOARD_TLS_KEY").unwrap_or_default();
        if cert.is_empty() != key.is_empty() {
            return Err("Dashboard TLS requires both certificate and key".into());
        }
        Ok(Some(Self::new(&user, &password, !cert.is_empty())))
    }
    fn new(user: &str, password: &str, tls: bool) -> Self {
        Self {
            credentials: digest(user, password),
            tls,
            records: Arc::new(Mutex::new(Records::default())),
        }
    }
    fn cookie(&self, id: &str, age: u64) -> String {
        format!(
            "dashboard_session={id}; Path=/; HttpOnly; SameSite=Strict; Max-Age={age}{}",
            if self.tls { "; Secure" } else { "" }
        )
    }
}
fn cookie(req: &Request) -> Option<String> {
    req.headers()
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|s| {
            s.trim()
                .strip_prefix("dashboard_session=")
                .map(str::to_owned)
        })
}
fn prune(r: &mut Records) {
    let now = Instant::now();
    r.sessions.retain(|_, s| s.expires > now);
    r.attempts
        .retain(|_, (t, _)| now.duration_since(*t) < WINDOW);
}

pub fn protect(private: Router, auth: Option<DashboardAuth>) -> Router {
    let private = private.layer(axum::middleware::from_fn_with_state(
        auth.clone(),
        session_gate,
    ));
    Router::new()
        .route("/", get(index))
        .route(
            "/login",
            get(login_page)
                .post(login)
                .layer(axum::extract::DefaultBodyLimit::max(8192)),
        )
        .route(
            "/logout",
            post(logout).layer(axum::extract::DefaultBodyLimit::max(8192)),
        )
        .with_state(auth.clone())
        .merge(private)
        .layer(axum::middleware::from_fn_with_state(auth, network_gate))
}
async fn network_gate(
    State(auth): State<Option<DashboardAuth>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(auth) = auth else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Some(ConnectInfo(peer)) = req.extensions().get::<ConnectInfo<SocketAddr>>() else {
        return StatusCode::FORBIDDEN.into_response();
    };
    // Plain HTTP is loopback-only. There is deliberately no proxy trust switch.
    if !is_allowed_ip(peer.ip()) || (!auth.tls && !normalized(peer.ip()).is_loopback()) {
        return StatusCode::FORBIDDEN.into_response();
    }
    // Proxies are unsupported: reject forwarding evidence, even on loopback.
    if [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-proto",
        "x-real-ip",
    ]
    .iter()
    .any(|h| req.headers().contains_key(*h))
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    headers.insert(header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'".parse().unwrap());
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    response
}
async fn session_gate(
    State(auth): State<Option<DashboardAuth>>,
    req: Request,
    next: Next,
) -> Response {
    let Some(auth) = auth else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let allowed = {
        let mut records = auth.records.lock().unwrap();
        prune(&mut records);
        cookie(&req)
            .and_then(|id| records.sessions.get(&id))
            .map(|s| {
                s.authenticated
                    && (matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS)
                        || req
                            .headers()
                            .get("x-csrf-token")
                            .and_then(|v| v.to_str().ok())
                            == Some(s.csrf.as_str()))
            })
    };
    match allowed {
        Some(true) => next.run(req).await,
        Some(false) => StatusCode::FORBIDDEN.into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}
async fn login_page(State(auth): State<Option<DashboardAuth>>, req: Request) -> Response {
    let auth = auth.unwrap();
    let id = token();
    let csrf = token();
    let mut r = auth.records.lock().unwrap();
    prune(&mut r);
    if cookie(&req)
        .and_then(|id| r.sessions.get(&id))
        .is_some_and(|s| s.authenticated)
    {
        return (StatusCode::SEE_OTHER, [(header::LOCATION, "/")]).into_response();
    }
    if r.sessions.len() >= LIMIT {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    r.sessions.insert(
        id.clone(),
        Session {
            expires: Instant::now() + Duration::from_secs(600),
            csrf: csrf.clone(),
            authenticated: false,
        },
    );
    let html = format!("<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content='width=device-width,initial-scale=1'><title>Administrator login</title><body><h1>Bot administrator</h1><form method=post action=/login><input type=hidden name=csrf value='{csrf}'><label>Username <input name=username autocomplete=username required></label><label>Password <input type=password name=password autocomplete=current-password required></label><button>Log in</button></form></body></html>");
    ([(header::SET_COOKIE, auth.cookie(&id, 600))], Html(html)).into_response()
}
#[derive(serde::Deserialize)]
struct Login {
    username: String,
    password: String,
    csrf: String,
}
async fn login(
    State(auth): State<Option<DashboardAuth>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Form(form): Form<Login>,
) -> Response {
    let auth = auth.unwrap();
    let mut r = auth.records.lock().unwrap();
    prune(&mut r);
    let ip = normalized(peer.ip());
    if !r.attempts.contains_key(&ip) && r.attempts.len() >= LIMIT {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let attempt = r.attempts.entry(ip).or_insert((Instant::now(), 0));
    attempt.1 += 1;
    if attempt.1 > 10 {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let id = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split(';')
                .find_map(|s| s.trim().strip_prefix("dashboard_session="))
        });
    let valid_csrf = id
        .and_then(|id| r.sessions.get(id))
        .map(|s| !s.authenticated && s.csrf == form.csrf)
        .unwrap_or(false);
    if !valid_csrf {
        return StatusCode::FORBIDDEN.into_response();
    }
    if !bool::from(
        auth.credentials
            .ct_eq(&digest(&form.username, &form.password)),
    ) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    r.sessions.remove(id.unwrap());
    let id = token();
    r.sessions.insert(
        id.clone(),
        Session {
            expires: Instant::now() + TTL,
            csrf: token(),
            authenticated: true,
        },
    );
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, "/".to_owned()),
            (header::SET_COOKIE, auth.cookie(&id, TTL.as_secs())),
        ],
    )
        .into_response()
}
async fn index(State(auth): State<Option<DashboardAuth>>, req: Request) -> Response {
    let auth = auth.unwrap();
    let mut r = auth.records.lock().unwrap();
    prune(&mut r);
    if let Some(s) = cookie(&req)
        .and_then(|id| r.sessions.get(&id))
        .filter(|s| s.authenticated)
    {
        return Html(format!("<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content='width=device-width,initial-scale=1'><title>Bot administrator</title><body><h1>Authenticated</h1><p>Administrator API available. Send X-CSRF-Token on mutations.</p><code>{}</code><form method=post action=/logout><input type=hidden name=csrf value='{}'><button>Log out</button></form></body></html>", s.csrf, s.csrf)).into_response();
    }
    (StatusCode::SEE_OTHER, [(header::LOCATION, "/login")]).into_response()
}
#[derive(serde::Deserialize)]
struct Logout {
    csrf: String,
}
async fn logout(
    State(auth): State<Option<DashboardAuth>>,
    headers: axum::http::HeaderMap,
    Form(form): Form<Logout>,
) -> Response {
    let auth = auth.unwrap();
    let mut r = auth.records.lock().unwrap();
    prune(&mut r);
    let id = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| {
            v.split(';')
                .find_map(|s| s.trim().strip_prefix("dashboard_session="))
        });
    if !id
        .and_then(|id| r.sessions.get(id))
        .map(|s| s.authenticated && s.csrf == form.csrf)
        .unwrap_or(false)
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    r.sessions.remove(id.unwrap());
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, "/login".to_owned()),
            (header::SET_COOKIE, auth.cookie("", 0)),
        ],
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;
    fn app(auth: DashboardAuth) -> Router {
        protect(
            Router::new().route(
                "/private",
                get(|| async { "secret" }).post(|| async { "changed" }),
            ),
            Some(auth),
        )
    }
    async fn call(
        app: &Router,
        peer: &str,
        method: &str,
        path: &str,
        cookie: &str,
        body: &str,
        csrf: &str,
    ) -> Response {
        let mut req = Request::builder()
            .method(method)
            .uri(path)
            .header("cookie", cookie)
            .header("content-type", "application/x-www-form-urlencoded")
            .header("x-csrf-token", csrf)
            .body(Body::from(body.to_owned()))
            .unwrap();
        req.extensions_mut()
            .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
        app.clone().oneshot(req).await.unwrap()
    }
    fn session_cookie(r: &Response) -> String {
        r.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
    }
    async fn text(r: Response) -> String {
        String::from_utf8(to_bytes(r.into_body(), 16384).await.unwrap().to_vec()).unwrap()
    }
    fn csrf(html: &str) -> String {
        html.split("name=csrf value='")
            .nth(1)
            .unwrap()
            .split("'")
            .next()
            .unwrap()
            .to_owned()
    }
    #[tokio::test]
    async fn api_body_limit_and_authenticated_login_are_preserved() {
        let auth = DashboardAuth::new("admin", "test-password-only", true);
        auth.records.lock().unwrap().sessions.insert(
            "active".into(),
            Session {
                expires: Instant::now() + TTL,
                csrf: "csrf".into(),
                authenticated: true,
            },
        );
        let private = Router::new().route(
            "/api/message",
            post(
                |axum::Json(value): axum::Json<serde_json::Value>| async move {
                    assert_eq!(value["content"].as_str().unwrap().len(), 2000);
                    StatusCode::OK
                },
            ),
        );
        let app = protect(private, Some(auth.clone()));
        let body = format!("{{\"content\":\"{}\"}}", "\\u0061".repeat(2000));
        let mut req = Request::builder()
            .method("POST")
            .uri("/api/message")
            .header("content-type", "application/json")
            .header("cookie", "dashboard_session=active")
            .header("x-csrf-token", "csrf")
            .body(Body::from(body))
            .unwrap();
        req.extensions_mut()
            .insert(ConnectInfo("10.0.0.1:1".parse::<SocketAddr>().unwrap()));
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            StatusCode::OK
        );
        let response = call(
            &app,
            "10.0.0.1:1",
            "GET",
            "/login",
            "dashboard_session=active",
            "",
            "",
        )
        .await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert!(!response.headers().contains_key(header::SET_COOKIE));
        assert_eq!(auth.records.lock().unwrap().sessions.len(), 1);
        for path in ["/login", "/logout"] {
            assert_eq!(
                call(&app, "10.0.0.1:1", "POST", path, "", &"a".repeat(9000), "")
                    .await
                    .status(),
                StatusCode::PAYLOAD_TOO_LARGE
            );
        }
    }

    #[tokio::test]
    async fn exact_throttle_window_and_pruning() {
        let auth = DashboardAuth::new("admin", "test-password-only", true);
        let app = app(auth.clone());
        for attempt in 1..=11 {
            let status = call(
                &app,
                "10.0.0.1:1",
                "POST",
                "/login",
                "",
                "username=a&password=b&csrf=c",
                "",
            )
            .await
            .status();
            assert_eq!(
                status,
                if attempt <= 10 {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::TOO_MANY_REQUESTS
                }
            );
        }
        {
            let mut records = auth.records.lock().unwrap();
            records
                .attempts
                .get_mut(&"10.0.0.1".parse().unwrap())
                .unwrap()
                .0 = Instant::now() - WINDOW;
            records.sessions.insert(
                "stale".into(),
                Session {
                    expires: Instant::now() - Duration::from_secs(1),
                    csrf: token(),
                    authenticated: false,
                },
            );
        }
        assert_eq!(
            call(
                &app,
                "10.0.0.1:1",
                "POST",
                "/login",
                "",
                "username=a&password=b&csrf=c",
                ""
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        let records = auth.records.lock().unwrap();
        assert!(!records.sessions.contains_key("stale"));
        assert_eq!(
            records.attempts[&"10.0.0.1".parse::<IpAddr>().unwrap()].1,
            1
        );
    }

    #[test]
    fn ip_policy() {
        for ip in [
            "10.0.0.1",
            "127.0.0.2",
            "::1",
            "::ffff:10.0.0.1",
            "::ffff:127.0.0.1",
        ] {
            assert!(is_allowed_ip(ip.parse().unwrap()));
        }
        for ip in [
            "192.168.1.1",
            "11.0.0.1",
            "::ffff:192.168.1.1",
            "2001:db8::1",
        ] {
            assert!(!is_allowed_ip(ip.parse().unwrap()));
        }
    }
    #[tokio::test]
    async fn login_csrf_logout_expiry_and_bounds() {
        let auth = DashboardAuth::new("admin", "test-password-only", true);
        let app = app(auth.clone());
        let peer = "10.0.0.1:1";
        assert_eq!(
            call(&app, peer, "GET", "/private", "", "", "")
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let page = call(&app, peer, "GET", "/login", "", "", "").await;
        let set = page.headers()[header::SET_COOKIE].to_str().unwrap();
        for flag in ["HttpOnly", "SameSite=Strict", "Secure"] {
            assert!(set.contains(flag));
        }
        let cookie = session_cookie(&page);
        let nonce = csrf(&text(page).await);
        assert_eq!(
            call(
                &app,
                peer,
                "POST",
                "/login",
                &cookie,
                "username=admin&password=test-password-only&csrf=wrong",
                ""
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(
                &app,
                peer,
                "POST",
                "/login",
                &cookie,
                &format!("username=admin&password=wrong&csrf={nonce}"),
                ""
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
        let login = call(
            &app,
            peer,
            "POST",
            "/login",
            &cookie,
            &format!("username=admin&password=test-password-only&csrf={nonce}"),
            "",
        )
        .await;
        assert_eq!(login.status(), StatusCode::SEE_OTHER);
        let active = session_cookie(&login);
        assert_ne!(active, cookie);
        let remaining = auth.records.lock().unwrap().sessions
            [active.strip_prefix("dashboard_session=").unwrap()]
        .expires
        .duration_since(Instant::now());
        assert!(
            remaining <= Duration::from_secs(8 * 3600)
                && remaining > Duration::from_secs(8 * 3600 - 5)
        );
        assert_eq!(
            call(
                &app,
                peer,
                "POST",
                "/login",
                &cookie,
                &format!("username=admin&password=test-password-only&csrf={nonce}"),
                ""
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );

        assert_eq!(
            call(&app, peer, "GET", "/private", &active, "", "")
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            call(&app, peer, "POST", "/private", &active, "", "")
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        let nonce = csrf(&text(call(&app, peer, "GET", "/", &active, "", "").await).await);
        assert_eq!(
            call(&app, peer, "POST", "/private", &active, "", &nonce)
                .await
                .status(),
            StatusCode::OK
        );
        assert_eq!(
            call(&app, peer, "POST", "/logout", &active, "csrf=wrong", "")
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(
                &app,
                peer,
                "POST",
                "/logout",
                &active,
                &format!("csrf={nonce}"),
                ""
            )
            .await
            .status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            call(&app, peer, "GET", "/private", &active, "", "")
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        {
            let mut r = auth.records.lock().unwrap();
            r.sessions.insert(
                "expired".into(),
                Session {
                    expires: Instant::now() - Duration::from_secs(1),
                    csrf: token(),
                    authenticated: true,
                },
            );
        }
        assert_eq!(
            call(
                &app,
                peer,
                "GET",
                "/private",
                "dashboard_session=expired",
                "",
                ""
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
        for _ in 0..20 {
            let _ = call(
                &app,
                peer,
                "POST",
                "/login",
                "",
                "username=a&password=b&csrf=c",
                "",
            )
            .await;
        }
        assert_eq!(
            call(
                &app,
                peer,
                "POST",
                "/login",
                "",
                "username=a&password=b&csrf=c",
                ""
            )
            .await
            .status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        for _ in 0..LIMIT + 5 {
            let _ = call(&app, peer, "GET", "/login", "", "", "").await;
        }
        assert!(auth.records.lock().unwrap().sessions.len() <= LIMIT);
    }
    #[tokio::test]
    async fn transport_peer_and_forwarding_boundaries() {
        for tls in [false, true] {
            let app = app(DashboardAuth::new("admin", "test-password-only", tls));
            for (peer, allowed) in [
                ("127.0.0.1:1", true),
                ("[::ffff:127.0.0.1]:1", true),
                ("10.0.0.1:1", tls),
                ("192.168.1.1:1", false),
            ] {
                let expected = if allowed {
                    StatusCode::OK
                } else {
                    StatusCode::FORBIDDEN
                };
                assert_eq!(
                    call(&app, peer, "GET", "/login", "", "", "").await.status(),
                    expected
                );
                for header in [
                    "forwarded",
                    "x-forwarded-for",
                    "x-forwarded-proto",
                    "x-real-ip",
                ] {
                    let mut req = Request::builder()
                        .uri("/login")
                        .header(header, "127.0.0.1")
                        .body(Body::empty())
                        .unwrap();
                    req.extensions_mut()
                        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
                    assert_eq!(
                        app.clone().oneshot(req).await.unwrap().status(),
                        StatusCode::FORBIDDEN
                    );
                }
            }
            assert_eq!(
                app.oneshot(
                    Request::builder()
                        .uri("/login")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
                StatusCode::FORBIDDEN
            );
        }
        let disabled = protect(Router::new(), None);
        assert_eq!(
            call(&disabled, "127.0.0.1:1", "GET", "/login", "", "", "")
                .await
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
