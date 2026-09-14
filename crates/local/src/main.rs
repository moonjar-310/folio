use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, put},
};
use folio_core::{
    ApiError, LIST_LIMIT, MAX_REQUEST_BYTES, Note, NoteSummary, SaveNote, summarize, valid_id,
    validate_note,
};
use rusqlite::{Connection, params};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
}

struct Store {
    vault: PathBuf,
    db: Connection,
}

impl Store {
    fn open(root: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let vault = root.join("vault");
        std::fs::create_dir_all(&vault)?;
        let db = Connection::open(root.join("index.sqlite"))?;
        db.execute_batch(include_str!("../../../migrations/0001_notes.sql"))?;
        Ok(Self { vault, db })
    }

    fn list(&self) -> Result<Vec<NoteSummary>, String> {
        let mut query = self.db.prepare("SELECT id, title, preview, updated_at FROM notes ORDER BY updated_at DESC, id LIMIT ?1").map_err(|e| e.to_string())?;
        query
            .query_map([LIST_LIMIT], |row| {
                Ok(NoteSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    preview: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    fn read(&self, id: &str) -> Result<Option<Note>, String> {
        if !valid_id(id) {
            return Err("Invalid note ID".into());
        }
        match std::fs::read_to_string(self.vault.join(format!("{id}.md"))) {
            Ok(markdown) => Ok(Some(Note {
                id: id.into(),
                markdown,
            })),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn save(&mut self, id: String, input: SaveNote) -> Result<NoteSummary, String> {
        validate_note(&id, &input.markdown).map_err(str::to_string)?;
        let note = Note {
            id,
            markdown: input.markdown,
        };
        let updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64;
        let summary = summarize(&note, updated);
        let mut staged = tempfile::NamedTempFile::new_in(&self.vault).map_err(|e| e.to_string())?;
        staged
            .write_all(note.markdown.as_bytes())
            .map_err(|e| e.to_string())?;
        staged.as_file().sync_all().map_err(|e| e.to_string())?;
        staged
            .persist(self.vault.join(format!("{}.md", note.id)))
            .map_err(|e| e.to_string())?;
        self.db.execute(
            "INSERT INTO notes (id,title,preview,updated_at) VALUES (?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET title=excluded.title,preview=excluded.preview,updated_at=excluded.updated_at",
            params![summary.id, summary.title, summary.preview, summary.updated_at],
        ).map_err(|_| "Markdown saved; index update failed. Retry Save to repair the index.".to_string())?;
        Ok(summary)
    }
}

struct Error(StatusCode, String);
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.0, Json(ApiError { message: self.1 })).into_response()
    }
}
fn bad(message: &str) -> Error {
    Error(StatusCode::BAD_REQUEST, message.into())
}
fn storage_error(message: String) -> Error {
    eprintln!("Storage error: {message}");
    Error(
        StatusCode::SERVICE_UNAVAILABLE,
        if message.starts_with("Markdown saved;") {
            message
        } else {
            "Storage unavailable; your draft has not been cleared.".into()
        },
    )
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<NoteSummary>>, Error> {
    tokio::task::spawn_blocking(move || state.store.lock().map_err(|e| e.to_string())?.list())
        .await
        .map_err(|e| storage_error(e.to_string()))?
        .map(Json)
        .map_err(storage_error)
}
async fn read(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<Note>, Error> {
    if !valid_id(&id) {
        return Err(bad("Invalid note ID"));
    }
    tokio::task::spawn_blocking(move || state.store.lock().map_err(|e| e.to_string())?.read(&id))
        .await
        .map_err(|e| storage_error(e.to_string()))?
        .map_err(storage_error)?
        .map(Json)
        .ok_or(Error(StatusCode::NOT_FOUND, "Note not found".into()))
}
async fn save(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(input): Json<SaveNote>,
) -> Result<Json<NoteSummary>, Error> {
    // JSON-only writes, no permissive CORS; reject cross-origin browser writes.
    if let Some(origin) = headers.get("origin") {
        let expected = format!(
            "http://{}",
            headers
                .get("host")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
        );
        let origin = origin.to_str().unwrap_or("");
        // The optional Trunk dev server proxies requests from its loopback port.
        if origin != expected
            && !["http://127.0.0.1:8080", "http://localhost:8080"].contains(&origin)
        {
            return Err(Error(
                StatusCode::FORBIDDEN,
                "Cross-origin write rejected".into(),
            ));
        }
    }
    validate_note(&id, &input.markdown).map_err(bad)?;
    tokio::task::spawn_blocking(move || {
        state
            .store
            .lock()
            .map_err(|e| e.to_string())?
            .save(id, input)
    })
    .await
    .map_err(|e| storage_error(e.to_string()))?
    .map(Json)
    .map_err(storage_error)
}

fn app(root: PathBuf, assets: PathBuf) -> Result<Router, Box<dyn std::error::Error>> {
    let state = AppState {
        store: Arc::new(Mutex::new(Store::open(root)?)),
    };
    let api = Router::new()
        .route(
            "/health",
            get(|| async { Json(serde_json::json!({"status":"ok","runtime":"local"})) }),
        )
        .route("/notes", get(list))
        .route("/notes/{id}", get(read).merge(put(save)))
        .fallback(|| async { Error(StatusCode::NOT_FOUND, "API route not found".into()) });
    Ok(Router::new()
        .nest("/api", api)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                response
                    .headers_mut()
                    .insert("x-content-type-options", "nosniff".parse().unwrap());
                response
            },
        ))
        .fallback_service(
            ServeDir::new(&assets).fallback(ServeFile::new(assets.join("index.html"))),
        )
        .with_state(state))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("FOLIO_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or(".local/folio-app".into());
    let assets = std::env::var_os("FOLIO_ASSETS_DIR")
        .map(PathBuf::from)
        .unwrap_or("dist/web".into());
    let port = std::env::var("FOLIO_PORT")
        .unwrap_or("8788".into())
        .parse::<u16>()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!("Folio local: http://127.0.0.1:{port}");
    axum::serve(listener, app(root, assets)?).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn saves_reads_and_reopens_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        let router = app(dir.path().into(), assets.clone()).unwrap();
        for text in ["# Hello\nFirst draft", "# Updated\n- [ ] Keep this"] {
            let response = router
                .clone()
                .oneshot(
                    Request::put("/api/notes/test-1")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::to_vec(&SaveNote {
                                markdown: text.into(),
                            })
                            .unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        let reopened = app(dir.path().into(), assets).unwrap();
        let response = reopened
            .clone()
            .oneshot(
                Request::get("/api/notes/test-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let note: Note =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(note.markdown, "# Updated\n- [ ] Keep this");
        let response = reopened
            .oneshot(Request::get("/api/notes").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let notes: Vec<NoteSummary> =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Updated");
    }

    #[tokio::test]
    async fn rejects_cross_origin_and_unknown_api_routes() {
        let dir = tempfile::tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(
            assets.join("index.html"),
            "<!doctype html><title>Folio</title>",
        )
        .unwrap();
        let router = app(dir.path().into(), dir.path().join("assets")).unwrap();
        let response = router
            .clone()
            .oneshot(
                Request::put("/api/notes/test")
                    .header("content-type", "application/json")
                    .header("host", "127.0.0.1:8788")
                    .header("origin", "https://other.example")
                    .body(Body::from(r##"{"markdown":"# No"}"##))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let response = router
            .clone()
            .oneshot(Request::get("/api/missing").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = router
            .oneshot(Request::get("/notes").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("content-type").unwrap(), "text/html");
        assert!(!dir.path().join("vault/test.md").exists());
    }

    #[test]
    fn index_failure_keeps_canonical_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path().into()).unwrap();
        store.db.execute("DROP TABLE notes", []).unwrap();
        let error = store
            .save(
                "safe".into(),
                SaveNote {
                    markdown: "# Keep me".into(),
                },
            )
            .unwrap_err();
        assert!(error.starts_with("Markdown saved;"));
        assert_eq!(store.read("safe").unwrap().unwrap().markdown, "# Keep me");
    }
}
