#![cfg(target_arch = "wasm32")]
use folio_adapter::RuntimeStore;
use folio_application::Application;
use folio_application::auth::AuthMethod;
use folio_core::MAX_REQUEST_BYTES;
use futures_util::StreamExt;
use worker::*;
fn error(status: u16, message: &str) -> Result<Response> {
    Ok(Response::from_json(&serde_json::json!({"message":message}))?.with_status(status))
}
async fn handle(mut req: Request, env: Env, _ctx: &Context) -> Result<Response> {
    let auth_method = match env.var("FOLIO_AUTH_METHOD")?.to_string().as_str() {
        "password" => AuthMethod::Password,
        "cloudflare_access" => AuthMethod::CloudflareAccess,
        _ => return error(503, "Authentication is not configured."),
    };
    let identity = if auth_method == AuthMethod::CloudflareAccess {
        serde_json::from_str(&env.var("FOLIO_INTERNAL_IDENTITY")?.to_string())?
    } else {
        None
    };
    if req.path() == "/api/health" && req.method() == Method::Get {
        return Response::from_json(&serde_json::json!({"status":"ok","runtime":"cloudflare"}));
    }
    let method = req.method().to_string();
    if method != "GET"
        && req
            .headers()
            .get("content-type")?
            .unwrap_or_default()
            .split(';')
            .next()
            != Some("application/json")
    {
        return error(415, "Expected application/json");
    }
    if req
        .headers()
        .get("content-length")?
        .and_then(|s| s.parse::<usize>().ok())
        .is_some_and(|n| n > MAX_REQUEST_BYTES)
    {
        return error(413, "Request too large");
    }
    let url = req.url()?;
    let session = req
        .headers()
        .get("cookie")?
        .unwrap_or_default()
        .split(';')
        .find_map(|c| {
            c.trim()
                .strip_prefix(if url.scheme() == "https" {
                    "__Host-folio_refresh="
                } else {
                    "folio_refresh="
                })
                .map(String::from)
        })
        .unwrap_or_default();
    let mut input = folio_application::Request {
        method,
        path: req.path(),
        query: url.query().unwrap_or("").into(),
        body: Vec::new(),
        session,
        access_token: req
            .headers()
            .get("authorization")?
            .unwrap_or_default()
            .strip_prefix("Bearer ")
            .unwrap_or("")
            .into(),
        csrf: req.headers().get("x-csrf-token")?.unwrap_or_default(),
        origin: req.headers().get("origin")?,
        expected_origin: url.origin().ascii_serialization(),
        client: req
            .headers()
            .get("cf-connecting-ip")?
            .unwrap_or("local-emulator".into()),
        setup_token: req.headers().get("x-setup-token")?.unwrap_or_default(),
        secure: url.scheme() == "https",
        identity,
    };
    if input.method != "GET" {
        let mut stream = req.stream()?;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if input.body.len() + chunk.len() > MAX_REQUEST_BYTES {
                return error(413, "Request too large");
            }
            input.body.extend_from_slice(&chunk);
        }
    }
    let store = RuntimeStore::new(&env).map_err(Error::RustError)?;
    let timings = store.timings();
    let mut service = Application {
        store,
        now: Date::now().as_millis(),
        jwt_secret: env.secret("FOLIO_JWT_SECRET")?.to_string(),
        setup_secret: env.secret("FOLIO_SETUP_TOKEN").ok().map(|s| s.to_string()),
        runtime: "cloudflare",
        auth_method,
    };
    let result = service.handle(input).await;
    let mut response = Response::from_json(&result.body)?.with_status(result.status);
    response
        .headers_mut()
        .set("Server-Timing", &timings.server_timing())?;
    if let Some(cookie) = result.cookie {
        response.headers_mut().set("Set-Cookie", &cookie)?;
    }
    Ok(response)
}
#[event(fetch)]
pub async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let mut response = match handle(req, env, &ctx).await {
        Ok(r) => r,
        Err(e) => {
            console_error!("{e}");
            error(503, "Storage unavailable. Your draft is preserved.")?
        }
    };
    response.headers_mut().set("Cache-Control", "no-store")?;
    response
        .headers_mut()
        .set("X-Content-Type-Options", "nosniff")?;
    Ok(response)
}
