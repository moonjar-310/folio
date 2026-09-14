use folio_core::{
    ApiError, LIST_LIMIT, MAX_REQUEST_BYTES, Note, SaveNote, object_key, summarize, valid_id,
    validate_note,
};
use worker::*;

fn error(status: u16, message: &str) -> Result<Response> {
    Ok(Response::from_json(&ApiError {
        message: message.into(),
    })?
    .with_status(status))
}

async fn handle(mut req: Request, env: Env) -> Result<Response> {
    let path = req.path();
    if path == "/api/health" && req.method() == Method::Get {
        return Response::from_json(&serde_json::json!({"status":"ok","runtime":"cloudflare"}));
    }
    // Bootstrap is local-development only until session authentication is implemented.
    // Missing/production configuration must never expose private notes anonymously.
    if env
        .var("APP_ENV")
        .map(|v| v.to_string())
        .unwrap_or_default()
        != "development"
    {
        return error(
            503,
            "Note API is disabled until authentication is configured",
        );
    }
    if path == "/api/notes" && req.method() == Method::Get {
        let db = env.d1("DB")?;
        let rows = db.prepare("SELECT id,title,preview,updated_at FROM notes ORDER BY updated_at DESC,id LIMIT ?1")
            .bind(&[(LIST_LIMIT as u32).into()])?.all().await?;
        return Response::from_json(&rows.results::<folio_core::NoteSummary>()?);
    }
    let Some(id) = path.strip_prefix("/api/notes/") else {
        return error(404, "API route not found");
    };
    if !valid_id(id) {
        return error(400, "Invalid note ID");
    }
    let key = object_key(id).map_err(|e| Error::RustError(e.into()))?;
    match req.method() {
        Method::Get => {
            let Some(object) = env.bucket("NOTES")?.get(&key).execute().await? else {
                return error(404, "Note not found");
            };
            let markdown = object
                .body()
                .ok_or_else(|| Error::RustError("Missing object body".into()))?
                .text()
                .await?;
            Response::from_json(&Note {
                id: id.into(),
                markdown,
            })
        }
        Method::Put => {
            let url = req.url()?;
            if let Some(origin) = req.headers().get("origin")?
                && origin != url.origin().ascii_serialization()
            {
                return error(403, "Cross-origin write rejected");
            }
            if req
                .headers()
                .get("content-type")?
                .is_none_or(|v| v.split(';').next().unwrap_or("").trim() != "application/json")
            {
                return error(415, "Expected application/json");
            }
            if req
                .headers()
                .get("content-length")?
                .and_then(|v| v.parse::<usize>().ok())
                .is_some_and(|n| n > MAX_REQUEST_BYTES)
            {
                return error(413, "Request too large");
            }
            // Bound even chunked bodies before deserializing, not just Content-Length.
            let mut stream = req.stream()?;
            use futures_util::StreamExt;
            let mut bytes = Vec::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                if bytes.len() + chunk.len() > MAX_REQUEST_BYTES {
                    return error(413, "Request too large");
                }
                bytes.extend_from_slice(&chunk);
            }
            let input: SaveNote = match serde_json::from_slice(&bytes) {
                Ok(input) => input,
                Err(_) => return error(400, "Invalid note JSON"),
            };
            if let Err(message) = validate_note(id, &input.markdown) {
                return error(400, message);
            }
            let note = Note {
                id: id.into(),
                markdown: input.markdown,
            };
            let summary = summarize(&note, Date::now().as_millis());
            let db = env.d1("DB")?;
            let statement = db.prepare("INSERT INTO notes (id,title,preview,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET title=excluded.title,preview=excluded.preview,updated_at=excluded.updated_at")
                .bind(&[summary.id.clone().into(), summary.title.clone().into(), summary.preview.clone().into(), (summary.updated_at as f64).into()])?;
            env.bucket("NOTES")?
                .put(&key, note.markdown)
                .execute()
                .await?;
            let index = statement.run().await;
            if !index.is_ok_and(|result| result.success()) {
                return error(
                    503,
                    "Markdown saved; index update failed. Retry Save to repair the index.",
                );
            }
            Response::from_json(&summary)
        }
        _ => error(405, "Method not allowed"),
    }
}

#[event(fetch)]
pub async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let mut response = match handle(req, env).await {
        Ok(response) => response,
        Err(e) => {
            console_error!("API failure: {e}");
            error(503, "Storage unavailable; your draft has not been cleared.")?
        }
    };
    response.headers_mut().set("Cache-Control", "no-store")?;
    response
        .headers_mut()
        .set("X-Content-Type-Options", "nosniff")?;
    Ok(response)
}
