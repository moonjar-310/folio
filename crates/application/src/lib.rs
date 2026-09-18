//! One application service for the native and Cloudflare HTTP entrypoints.
use folio_core::{
    storage::{Statement, Store},
    *,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
pub mod auth;
#[cfg(test)]
mod auth_tests;
mod folders;
mod notes;
mod password;
mod planning;
mod recovery;
#[cfg(test)]
mod tests;
mod tokens;
mod vault;
pub type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug)]
pub struct Failure {
    pub status: u16,
    pub message: String,
}
impl From<String> for Failure {
    fn from(e: String) -> Self {
        if e == "conflict" {
            return fail(
                409,
                "This note changed elsewhere. Reopen it or save your draft as a new note.",
            );
        }
        eprintln!("Storage failure: {e}");
        fail(503, "Storage unavailable. Your draft has not been cleared.")
    }
}
pub fn fail(status: u16, message: &str) -> Failure {
    Failure {
        status,
        message: message.into(),
    }
}
pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
pub fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn statement(sql: &str, params: Vec<Value>) -> Statement {
    Statement::new(sql, params)
}
pub fn folder_paths(path: &str) -> Vec<String> {
    let mut current = String::new();
    path.split('/')
        .map(|part| {
            if !current.is_empty() {
                current.push('/');
            }
            current.push_str(part);
            current.clone()
        })
        .collect()
}
pub fn string<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn boolean(v: &Value, key: &str) -> bool {
    v[key]
        .as_bool()
        .unwrap_or_else(|| v[key].as_i64() == Some(1))
}
pub fn normalize(mut rows: Vec<Value>, keys: &[&str]) -> Vec<Value> {
    for row in &mut rows {
        for key in keys {
            let value = boolean(row, key);
            row[*key] = json!(value);
        }
    }
    rows
}
#[derive(Clone, Default)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: String,
    pub body: Vec<u8>,
    pub session: String,
    pub access_token: String,
    pub csrf: String,
    pub origin: Option<String>,
    pub expected_origin: String,
    pub client: String,
    pub setup_token: String,
    pub secure: bool,
    /// Set only by the trusted runtime authentication adapter, never request JSON.
    pub identity: Option<auth::VerifiedIdentity>,
}
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: Value,
    pub cookie: Option<String>,
}
impl Response {
    fn ok(body: Value) -> Self {
        Self {
            status: 200,
            body,
            cookie: None,
        }
    }
}
pub struct Application<S> {
    pub store: S,
    pub now: u64,
    pub setup_secret: Option<String>,
    pub jwt_secret: String,
    pub runtime: &'static str,
    pub auth_method: auth::AuthMethod,
}
impl<S: Store> Application<S> {
    pub async fn handle(&mut self, req: Request) -> Response {
        match self.route(req).await {
            Ok(response) => response,
            Err(e) => Response {
                status: e.status,
                body: json!({"message":e.message}),
                cookie: None,
            },
        }
    }
    async fn route(&mut self, req: Request) -> Result<Response> {
        if req.path == "/api/health" && req.method == "GET" {
            return Ok(Response::ok(json!({"status":"ok","runtime":self.runtime})));
        }
        if req.body.len() > MAX_REQUEST_BYTES {
            return Err(fail(413, "Request too large"));
        }
        if !["GET", "POST", "PUT", "DELETE"].contains(&req.method.as_str()) {
            return Err(fail(405, "Method not allowed"));
        }
        if req.method != "GET"
            && req
                .origin
                .as_ref()
                .is_some_and(|o| o != &req.expected_origin)
        {
            return Err(fail(403, "Cross-origin write rejected"));
        }
        let input: Value = if req.body.is_empty() {
            json!({})
        } else {
            serde_json::from_slice(&req.body).map_err(|_| fail(400, "Invalid JSON"))?
        };
        if req.path.starts_with("/api/auth/") {
            return self.auth(&req, input).await;
        }
        self.authorize(&req).await?;
        let query: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(req.query.as_bytes())
                .into_owned()
                .collect();
        let body = match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/api/workspace") => {
                let entries = self.all_entries().await?;
                let notes = self
                    .browse_notes_from_entries(&std::collections::HashMap::new(), &entries)
                    .await?;
                let mut task_query = std::collections::HashMap::new();
                if let Some(today) = query.get("today") {
                    if !valid_date(today) {
                        return Err(fail(400, "A valid current date is required"));
                    }
                    task_query.insert("from".into(), today.clone());
                    task_query.insert("to".into(), today.clone());
                }
                let tasks = self.list_tasks(&task_query).await?;
                let goals = self
                    .store
                    .query(statement(
                        "SELECT * FROM goals ORDER BY position,created_at LIMIT 200",
                        vec![],
                    ))
                    .await?;
                let folders = self.vault_folders_from_entries(&entries).await?;
                json!({"notes":notes,"tasks":tasks,"goals":goals,"folders":folders})
            }
            ("GET", "/api/notes") => self.list_notes(&query).await?,
            ("GET", "/api/folders") => self.vault_folders().await?,
            ("POST", "/api/storage/rebuild") => self.rebuild(&input).await?,
            ("POST", "/api/storage/migrate") => self.migrate(&input).await?,
            ("GET", "/api/folders/rename") => self.pending_folder_rename().await?,
            ("POST" | "PUT" | "DELETE", "/api/folders") => {
                self.mutate_folder(&req.method, input).await?
            }
            ("GET", "/api/tasks") => self.list_tasks(&query).await?,
            ("POST", "/api/tasks") => self.save_task(None, input).await?,
            ("GET", "/api/goals") => json!(
                self.store
                    .query(statement(
                        "SELECT * FROM goals ORDER BY position,created_at LIMIT 200",
                        vec![]
                    ))
                    .await?
            ),
            ("POST", "/api/goals") => self.save_goal(None, input).await?,
            _ => {
                if let Some(id) = req.path.strip_prefix("/api/notes/") {
                    if !paths::valid_note_id(id) {
                        return Err(fail(400, "Invalid note ID"));
                    }
                    match req.method.as_str() {
                        "GET" => self.read_note(id).await?,
                        "PUT" => {
                            self.save_note(
                                id,
                                serde_json::from_value(input)
                                    .map_err(|_| fail(400, "Invalid note fields"))?,
                            )
                            .await?
                        }
                        "DELETE" => self.delete_note(id, input).await?,
                        _ => return Err(fail(405, "Method not allowed")),
                    }
                } else if let Some(id) = req.path.strip_prefix("/api/tasks/") {
                    if !valid_id(id) {
                        return Err(fail(400, "Invalid task ID"));
                    }
                    match req.method.as_str() {
                        "PUT" => self.save_task(Some(id), input).await?,
                        "DELETE" => self.delete_task(id).await?,
                        _ => return Err(fail(405, "Method not allowed")),
                    }
                } else if let Some(id) = req.path.strip_prefix("/api/goals/") {
                    if !valid_id(id) {
                        return Err(fail(400, "Invalid goal ID"));
                    }
                    if req.method != "PUT" {
                        return Err(fail(405, "Method not allowed"));
                    }
                    self.save_goal(Some(id), input).await?
                } else {
                    return Err(fail(404, "API route not found"));
                }
            }
        };
        Ok(Response::ok(body))
    }
}
