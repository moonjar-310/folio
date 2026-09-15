#![cfg(target_arch = "wasm32")]
use folio_adapter::RuntimeStore;
use folio_application::Application;
use folio_core::MAX_REQUEST_BYTES;
use futures_util::StreamExt;
use worker::*;
fn error(status: u16, message: &str) -> Result<Response> {
    Ok(Response::from_json(&serde_json::json!({"message":message}))?.with_status(status))
}
async fn handle(mut req: Request, env: Env) -> Result<Response> {
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
        .find_map(|c| c.trim().strip_prefix("folio_session=").map(String::from))
        .unwrap_or_default();
    let mut input = folio_application::Request {
        method,
        path: req.path(),
        query: url.query().unwrap_or("").into(),
        body: Vec::new(),
        session,
        csrf: req.headers().get("x-csrf-token")?.unwrap_or_default(),
        origin: req.headers().get("origin")?,
        expected_origin: url.origin().ascii_serialization(),
        client: req
            .headers()
            .get("cf-connecting-ip")?
            .unwrap_or("local-emulator".into()),
        setup_token: req.headers().get("x-setup-token")?.unwrap_or_default(),
        secure: url.scheme() == "https",
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
    let mut service = Application {
        store,
        now: Date::now().as_millis(),
        setup_secret: env.secret("FOLIO_SETUP_TOKEN").ok().map(|s| s.to_string()),
        runtime: "cloudflare",
    };
    let result = service.handle(input).await;
    let mut response = Response::from_json(&result.body)?.with_status(result.status);
    if let Some(cookie) = result.cookie {
        response.headers_mut().set("Set-Cookie", &cookie)?;
    }
    Ok(response)
}
#[event(fetch)]
pub async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let mut response = match handle(req, env).await {
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
