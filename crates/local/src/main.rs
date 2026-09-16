use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
};
use folio_adapter::RuntimeStore;
use folio_application::{Application, Request};
use folio_core::MAX_REQUEST_BYTES;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::services::{ServeDir, ServeFile};
#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<RuntimeStore>>,
    origin: String,
    jwt_secret: String,
}
fn header(headers: &HeaderMap, key: &str) -> String {
    headers
        .get(key)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .into()
}
async fn api(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if method != Method::GET
        && header(&headers, "content-type").split(';').next() != Some("application/json")
    {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Json(serde_json::json!({"message":"Expected application/json"})),
        )
            .into_response();
    }
    let token = header(&headers, "cookie")
        .split(';')
        .find_map(|c| c.trim().strip_prefix("folio_refresh=").map(String::from))
        .unwrap_or_default();
    let origin = headers.get("origin").map(|_| header(&headers, "origin"));
    let mut expected = state.origin.clone();
    // Explicit development proxy; local server still binds loopback only.
    if origin
        .as_ref()
        .is_some_and(|o| ["http://localhost:8080", "http://127.0.0.1:8080"].contains(&o.as_str()))
    {
        expected = origin.clone().unwrap();
    }
    if header(&headers, "host")
        == state
            .origin
            .trim_start_matches("http://")
            .replace("127.0.0.1", "localhost")
    {
        expected = state.origin.replace("127.0.0.1", "localhost");
    }
    let request = Request {
        method: method.to_string(),
        path: uri.path().into(),
        query: uri.query().unwrap_or("").into(),
        body: body.to_vec(),
        session: token,
        access_token: header(&headers, "authorization")
            .strip_prefix("Bearer ")
            .unwrap_or("")
            .into(),
        csrf: header(&headers, "x-csrf-token"),
        origin,
        expected_origin: expected,
        client: "loopback".into(),
        setup_token: header(&headers, "x-setup-token"),
        secure: false,
        identity: None,
    };
    let result = tokio::task::spawn_blocking(move || {
        let mut store = state.store.lock().map_err(|_| "Storage lock unavailable")?;
        let mut service = Application {
            store: &mut *store,
            now: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "Invalid clock")?
                .as_millis() as u64,
            setup_secret: None,
            jwt_secret: state.jwt_secret.clone(),
            runtime: "local",
            auth_method: folio_application::auth::AuthMethod::Password,
        };
        Ok::<_, &str>(futures::executor::block_on(service.handle(request)))
    })
    .await;
    let mut response = match result {
        Ok(Ok(result)) => {
            let mut response = (
                StatusCode::from_u16(result.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(result.body),
            )
                .into_response();
            if let Some(cookie) = result.cookie {
                response
                    .headers_mut()
                    .insert("set-cookie", cookie.parse().unwrap());
            }
            response
        }
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"message":"Storage unavailable. Your draft is preserved."})),
        )
            .into_response(),
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("x-content-type-options", "nosniff".parse().unwrap());
    response
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("FOLIO_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or(".local/folio-app".into());
    std::fs::create_dir_all(&root)?;
    let key_path = root.join("jwt-secret");
    let jwt_secret = match std::env::var("FOLIO_JWT_SECRET") {
        Ok(key) => key,
        Err(_) => match std::fs::read_to_string(&key_path) {
            Ok(key) => key,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                use std::io::Write;
                let key = format!(
                    "{}{}{}",
                    folio_application::uid(),
                    folio_application::uid(),
                    folio_application::uid()
                );
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                options.open(&key_path)?.write_all(key.as_bytes())?;
                key
            }
            Err(e) => return Err(e.into()),
        },
    };
    if jwt_secret.len() < 64 {
        return Err("FOLIO_JWT_SECRET must contain at least 64 characters".into());
    }
    let command = std::env::args().nth(1);
    if matches!(
        command.as_deref(),
        Some("--migrate-storage" | "--rebuild-index")
    ) {
        let mut app = folio_application::Application {
            store: RuntimeStore::open(root)?,
            now: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64,
            setup_secret: None,
            jwt_secret: jwt_secret.clone(),
            runtime: "local",
            auth_method: folio_application::auth::AuthMethod::Password,
        };
        app.maintain_offline(command.as_deref() == Some("--migrate-storage"))
            .await
            .map_err(|e| std::io::Error::other(e.message))?;
        println!("Folio storage maintenance complete.");
        return Ok(());
    }
    let assets = std::env::var_os("FOLIO_ASSETS_DIR")
        .map(PathBuf::from)
        .unwrap_or("dist/web".into());
    let port = std::env::var("FOLIO_PORT")
        .unwrap_or("8788".into())
        .parse::<u16>()?;
    let state = AppState {
        store: Arc::new(Mutex::new(RuntimeStore::open(root)?)),
        origin: format!("http://127.0.0.1:{port}"),
        jwt_secret,
    };
    let app = Router::new()
        .route("/api", any(api))
        .route("/api/{*path}", any(api))
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .fallback_service(
            ServeDir::new(&assets).fallback(ServeFile::new(assets.join("index.html"))),
        )
        .with_state(state);
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Folio local: http://127.0.0.1:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
