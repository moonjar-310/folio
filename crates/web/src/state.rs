use crate::folder_tree::FolderListing;
use folio_core::*;
use gloo_net::http::Request;
use leptos::{prelude::*, task::spawn_local};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use wasm_bindgen::JsCast;
#[derive(Clone, Copy)]
pub struct AppState {
    pub ready: RwSignal<bool>,
    pub authenticated: RwSignal<bool>,
    pub setup: RwSignal<bool>,
    pub csrf: RwSignal<String>,
    pub access_token: RwSignal<String>,
    pub token_deadline: RwSignal<f64>,
    pub refresh_lock: StoredValue<std::sync::Arc<futures::lock::Mutex<()>>>,
    pub username: RwSignal<String>,
    pub runtime: RwSignal<String>,
    pub auth_method: RwSignal<String>,
    pub reauthenticate: RwSignal<bool>,
    pub notes: RwSignal<Vec<NoteSummary>>,
    pub notes_epoch: RwSignal<u64>,
    pub tasks: RwSignal<Vec<Task>>,
    pub goals: RwSignal<Vec<Goal>>,
    pub folders: RwSignal<Vec<String>>,
    pub expanded: RwSignal<BTreeSet<String>>,
    pub folder_files: RwSignal<BTreeMap<String, FolderListing>>,
    pub folder_epoch: RwSignal<u64>,
    pub folder_rename: RwSignal<Option<(String, String)>>,
    pub active: RwSignal<Note>,
    pub dirty: RwSignal<bool>,
    pub busy: RwSignal<bool>,
    pub pending_tasks: RwSignal<BTreeSet<String>>,
    pub pending_goals: RwSignal<BTreeSet<String>>,
    pub task_version: RwSignal<u64>,
    pub quick_saving: RwSignal<bool>,
    pub loading: RwSignal<bool>,
    pub maintaining: RwSignal<bool>,
    pub message: RwSignal<String>,
    pub error: RwSignal<String>,
    pub page: RwSignal<String>,
    pub folder: RwSignal<String>,
    pub drawer: RwSignal<bool>,
    pub search_open: RwSignal<bool>,
    pub search: RwSignal<String>,
    pub results: RwSignal<Vec<NoteSummary>>,
    pub search_epoch: RwSignal<u64>,
    pub task_query: RwSignal<String>,
    pub task_epoch: RwSignal<u64>,
    pub quick: RwSignal<String>,
    pub next_notes: RwSignal<Option<u64>>,
    pub next_tasks: RwSignal<Option<u64>>,
    pub preview: RwSignal<bool>,
    pub dark: RwSignal<bool>,
    pub epoch: RwSignal<u64>,
    pub date: RwSignal<String>,
    pub weekly: RwSignal<bool>,
    pub task_group: RwSignal<String>,
}
pub fn today() -> String {
    date_string(&js_sys::Date::new_0())
}
fn date_string(d: &js_sys::Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}
pub fn shift_date(date: &str, delta: i32) -> String {
    let d = js_sys::Date::new(&format!("{date}T12:00:00").into());
    d.set_time(d.get_time() + f64::from(delta) * 86_400_000.0);
    date_string(&d)
}
pub fn week(date: &str) -> Vec<String> {
    let d = js_sys::Date::new(&format!("{date}T12:00:00").into());
    let monday = shift_date(date, -((d.get_day() as i32 + 6) % 7));
    (0..7).map(|i| shift_date(&monday, i)).collect()
}
pub fn nice_date(date: &str) -> String {
    js_sys::Date::new(&format!("{date}T12:00:00").into())
        .to_date_string()
        .as_string()
        .unwrap_or_else(|| date.into())
}
pub fn set_route(page: &str, id: Option<&str>, replace: bool) {
    let path = if page == "Home" {
        "/".to_string()
    } else if let Some(id) = id {
        format!("/notes?note={id}")
    } else {
        format!("/{}", page.to_lowercase())
    };
    if let Some(w) = web_sys::window()
        && let Ok(history) = w.history()
    {
        if replace {
            let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));
        } else {
            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));
        }
    }
}
pub fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
pub fn new_note(folder: String) -> Note {
    Note {
        id: uuid::Uuid::new_v4().to_string(),
        markdown: String::new(),
        folder,
        revision: String::new(),
    }
}
impl AppState {
    pub fn new() -> Self {
        let dark = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
            .and_then(|e| e.get_attribute("data-theme"))
            .as_deref()
            == Some("dark");
        Self {
            ready: RwSignal::new(false),
            authenticated: RwSignal::new(false),
            setup: RwSignal::new(false),
            csrf: RwSignal::new(String::new()),
            access_token: RwSignal::new(String::new()),
            token_deadline: RwSignal::new(0.0),
            refresh_lock: StoredValue::new(std::sync::Arc::new(futures::lock::Mutex::new(()))),
            username: RwSignal::new(String::new()),
            runtime: RwSignal::new(String::new()),
            auth_method: RwSignal::new("unknown".into()),
            reauthenticate: RwSignal::new(false),
            notes: RwSignal::new(vec![]),
            notes_epoch: RwSignal::new(0),
            tasks: RwSignal::new(vec![]),
            goals: RwSignal::new(vec![]),
            folders: RwSignal::new(vec![]),
            expanded: RwSignal::new(
                storage()
                    .and_then(|s| s.get_item("folio-expanded-folders").ok().flatten())
                    .and_then(|v| serde_json::from_str(&v).ok())
                    .unwrap_or_default(),
            ),
            folder_files: RwSignal::new(BTreeMap::new()),
            folder_epoch: RwSignal::new(0),
            folder_rename: RwSignal::new(None),
            active: RwSignal::new(new_note(default_folder())),
            dirty: RwSignal::new(false),
            busy: RwSignal::new(false),
            pending_tasks: RwSignal::new(BTreeSet::new()),
            pending_goals: RwSignal::new(BTreeSet::new()),
            task_version: RwSignal::new(0),
            quick_saving: RwSignal::new(false),
            loading: RwSignal::new(false),
            maintaining: RwSignal::new(false),
            message: RwSignal::new("Ready when you are".into()),
            error: RwSignal::new(String::new()),
            page: RwSignal::new("Home".into()),
            folder: RwSignal::new(String::new()),
            drawer: RwSignal::new(false),
            search_open: RwSignal::new(false),
            search: RwSignal::new(String::new()),
            results: RwSignal::new(vec![]),
            search_epoch: RwSignal::new(0),
            task_query: RwSignal::new(String::new()),
            task_epoch: RwSignal::new(0),
            quick: RwSignal::new(
                storage()
                    .and_then(|s| s.get_item("folio-quick-draft").ok().flatten())
                    .unwrap_or_default(),
            ),
            next_notes: RwSignal::new(None),
            next_tasks: RwSignal::new(None),
            preview: RwSignal::new(false),
            dark: RwSignal::new(dark),
            epoch: RwSignal::new(0),
            date: RwSignal::new(today()),
            weekly: RwSignal::new(true),
            task_group: RwSignal::new("Today".into()),
        }
    }
    async fn refresh_tokens(self, previous: &str) -> Result<(), String> {
        let lock = self.refresh_lock.get_value();
        let _guard = lock.lock().await;
        if self.access_token.get_untracked() != previous
            && self.token_deadline.get_untracked() > js_sys::Date::now()
        {
            return Ok(());
        }
        let data = self.raw_api("POST", "/api/auth/refresh", json!({})).await?;
        self.accept_auth(&data);
        Ok(())
    }
    pub async fn api(self, method: &str, path: &str, body: Value) -> Result<Value, String> {
        if (!path.starts_with("/api/auth/") || path == "/api/auth/logout")
            && self.token_deadline.get_untracked() <= js_sys::Date::now()
        {
            self.refresh_tokens(&self.access_token.get_untracked())
                .await?;
        }
        self.raw_api(method, path, body).await
    }
    async fn raw_api(self, method: &str, path: &str, body: Value) -> Result<Value, String> {
        self.send_api(method, path, body, false).await
    }
    async fn send_api(
        self,
        method: &str,
        path: &str,
        body: Value,
        retried: bool,
    ) -> Result<Value, String> {
        let previous = self.access_token.get_untracked();
        let builder = match method {
            "POST" => Request::post(path),
            "PUT" => Request::put(path),
            "DELETE" => Request::delete(path),
            _ => Request::get(path),
        }
        .header("x-csrf-token", &self.csrf.get_untracked())
        .header(
            "authorization",
            &format!("Bearer {}", self.access_token.get_untracked()),
        );
        let response = if method == "GET" {
            builder.send().await
        } else {
            builder.json(&body).map_err(|e| e.to_string())?.send().await
        }
        .map_err(|e| {
            if self.auth_method.get_untracked() == "cloudflare_access" {
                self.reauthenticate.set(true);
            }
            format!("Could not connect. Your draft is preserved. {e}")
        })?;
        let status = response.status();
        if !response
            .headers()
            .get("content-type")
            .unwrap_or_default()
            .contains("application/json")
        {
            self.reauthenticate.set(true);
            return Err("The server did not return app data. Reopen sign-in if your session expired; your draft is preserved.".into());
        }
        let data = response
            .json::<Value>()
            .await
            .map_err(|_| "Could not read the server response".to_string())?;
        if let Some(method) = data["method"].as_str() {
            self.auth_method.set(method.into());
        }
        if status == 401
            && !retried
            && (!path.starts_with("/api/auth/") || path == "/api/auth/logout")
        {
            Box::pin(self.refresh_tokens(&previous)).await?;
            return Box::pin(self.send_api(method, path, body, true)).await;
        }
        if status == 401 {
            if self.auth_method.get_untracked() == "cloudflare_access" {
                self.reauthenticate.set(true);
            } else {
                self.authenticated.set(false);
            }
        }
        if !response.ok() {
            return Err(data["message"].as_str().unwrap_or("Request failed").into());
        }
        Ok(data)
    }
    pub async fn start(self) {
        let refreshed = self.raw_api("POST", "/api/auth/refresh", json!({})).await;
        if let Ok(ref auth) = refreshed {
            self.accept_auth(auth);
        }
        let status = match refreshed {
            Ok(auth) => Ok(auth),
            Err(_) => self.api("GET", "/api/auth/status", Value::Null).await,
        };
        match status {
            Ok(mut data) => {
                if data["method"] == "cloudflare_access" && data["authenticated"] == false {
                    match self.api("POST", "/api/auth/login", json!({})).await {
                        Ok(auth) => data = auth,
                        Err(e) => {
                            self.error.set(e);
                            self.ready.set(true);
                            return;
                        }
                    }
                }
                self.setup
                    .set(data["setup_required"].as_bool().unwrap_or(false));
                self.runtime
                    .set(data["runtime"].as_str().unwrap_or("").into());
                self.accept_auth(&data);
                if self.authenticated.get_untracked() {
                    self.load_workspace().await;
                }
            }
            Err(e) => self.error.set(e),
        }
        self.ready.set(true);
    }
    pub fn accept_auth(self, data: &Value) {
        self.access_token
            .set(data["access_token"].as_str().unwrap_or("").into());
        self.token_deadline.set(
            js_sys::Date::now() + data["expires_in"].as_f64().unwrap_or(0.0) * 1000.0 - 10_000.0,
        );
        self.auth_method
            .set(data["method"].as_str().unwrap_or("password").into());
        self.reauthenticate.set(false);
        self.authenticated
            .set(data["authenticated"].as_bool().unwrap_or(false));
        self.csrf.set(data["csrf"].as_str().unwrap_or("").into());
        self.username
            .set(data["username"].as_str().unwrap_or("").into());
        self.runtime
            .set(data["runtime"].as_str().unwrap_or("").into());
    }
    pub async fn maintain_storage(self, migrate: bool) {
        if self.maintaining.get_untracked() || !self.save().await {
            return;
        }
        self.maintaining.set(true);
        self.loading.set(true);
        let endpoint = if migrate {
            "/api/storage/migrate"
        } else {
            "/api/storage/rebuild"
        };
        let mut input = json!({});
        let mut warnings = Vec::new();
        loop {
            match self.api("POST", endpoint, input).await {
                Ok(result) => {
                    if let Some(items) = result["warnings"].as_array() {
                        warnings.extend(items.iter().map(|w| {
                            format!(
                                "{}: {}",
                                w["path"].as_str().unwrap_or(""),
                                w["message"].as_str().unwrap_or("")
                            )
                        }));
                    }
                    if result["done"] == true {
                        break;
                    }
                    input = result;
                    self.message.set(
                        if migrate {
                            "Moving existing files…"
                        } else {
                            "Rebuilding search and planner lists…"
                        }
                        .into(),
                    );
                }
                Err(error) => {
                    self.error.set(error);
                    self.loading.set(false);
                    self.maintaining.set(false);
                    return;
                }
            }
        }
        self.folder_epoch.update(|n| *n += 1);
        self.folder_files.set(BTreeMap::new());
        self.loading.set(false);
        self.load_workspace().await;
        self.maintaining.set(false);
        if !warnings.is_empty() {
            self.error.set(format!(
                "Rebuild finished with skipped records: {}",
                warnings.join("; ")
            ));
        }
        self.message.set(
            if migrate {
                "Existing files migrated. Original copies kept in .folio/legacy."
            } else {
                "Search and planner lists rebuilt from files."
            }
            .into(),
        );
    }
    pub async fn load_workspace(self) {
        self.loading.set(true);
        match self
            .api(
                "GET",
                &format!("/api/workspace?today={}", today()),
                Value::Null,
            )
            .await
        {
            Ok(v) => {
                self.notes
                    .set(serde_json::from_value(v["notes"]["items"].clone()).unwrap_or_default());
                self.next_notes.set(v["notes"]["next_offset"].as_u64());
                self.tasks
                    .set(serde_json::from_value(v["tasks"]["items"].clone()).unwrap_or_default());
                self.task_query
                    .set(format!("group=today&today={}", today()));
                self.next_tasks.set(v["tasks"]["next_offset"].as_u64());
                self.goals
                    .set(serde_json::from_value(v["goals"].clone()).unwrap_or_default());
                self.folders.set(
                    v["folders"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v["path"].as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                );
                self.error.set(String::new());
                if self.dirty.get_untracked() {
                    self.page.set("Notes".into());
                } else if let Some(draft) = storage()
                    .and_then(|s| s.get_item("folio-draft").ok().flatten())
                    .and_then(|s| serde_json::from_str::<Note>(&s).ok())
                {
                    self.active.set(draft);
                    self.dirty.set(true);
                    self.page.set("Notes".into());
                    self.message
                        .set("Recovered an unsaved draft. Review it, then Save.".into());
                } else if let Some(path) =
                    web_sys::window().and_then(|w| w.location().pathname().ok())
                {
                    let page = match path.trim_matches('/') {
                        "notes" => "Notes",
                        "todo" => "Todo",
                        "planner" => "Planner",
                        "settings" => "Settings",
                        _ => "Home",
                    };
                    self.page.set(page.into());
                    if page == "Notes"
                        && let Some(id) = web_sys::window()
                            .and_then(|w| w.location().search().ok())
                            .and_then(|q| q.strip_prefix("?note=").map(String::from))
                            .filter(|id| folio_core::paths::valid_note_id(id))
                    {
                        let expanded = self.expanded.get_untracked();
                        self.open_note(id).await;
                        self.expanded.set(expanded);
                        self.persist_expanded();
                    }
                }
            }
            Err(e) => self.error.set(e),
        }
        self.loading.set(false);
    }
    pub fn edit(self, markdown: String) {
        self.active.update(|n| n.markdown = markdown);
        self.mark_dirty();
    }
    pub fn mark_dirty(self) {
        self.dirty.set(true);
        self.message.set("Unsaved changes".into());
        self.epoch.update(|n| *n += 1);
        self.backup_draft();
        let epoch = self.epoch.get_untracked();
        set_timeout(
            move || {
                if self.epoch.get_untracked() == epoch && self.dirty.get_untracked() {
                    spawn_local(async move {
                        self.save().await;
                    });
                }
            },
            Duration::from_secs(3),
        );
    }
    pub fn backup_draft(self) {
        if let Some(s) = storage() {
            match s.set_item(
                "folio-draft",
                &serde_json::to_string(&self.active.get_untracked()).unwrap_or_default(),
            ) {
                Ok(()) => {}
                Err(_) => self.error.set(
                    "Browser draft backup is unavailable. Keep this tab open until saved.".into(),
                ),
            }
        }
    }
    /// Never leave an unsaved editor if persistent recovery cannot be confirmed.
    pub fn reopen_sign_in(self) {
        if self.dirty.get_untracked() {
            let encoded = serde_json::to_string(&self.active.get_untracked()).unwrap_or_default();
            let stored = storage().is_some_and(|s| {
                s.set_item("folio-draft", &encoded).is_ok()
                    && s.get_item("folio-draft").ok().flatten().as_deref() == Some(encoded.as_str())
            });
            if !stored {
                self.error.set("Cannot preserve this draft in browser storage. Copy your text before reopening sign-in.".into());
                return;
            }
        }
        if let Some(w) = web_sys::window() {
            let _ = w.location().reload();
        }
    }
    pub async fn save(self) -> bool {
        if !self.dirty.get_untracked() {
            return true;
        }
        if self.busy.get_untracked() {
            return false;
        }
        let note = self.active.get_untracked();
        if note.markdown.len() > MAX_MARKDOWN_BYTES {
            self.error
                .set("Note exceeds 128 KiB. Your draft is preserved.".into());
            return false;
        }
        self.busy.set(true);
        self.message.set("Saving…".into());
        let result = self
            .api(
                "PUT",
                &format!("/api/notes/{}", note.id),
                json!(SaveNote {
                    markdown: note.markdown.clone(),
                    folder: note.folder.clone(),
                    revision: if note.revision.is_empty() {
                        None
                    } else {
                        Some(note.revision.clone())
                    }
                }),
            )
            .await;
        self.busy.set(false);
        match result {
            Ok(v) => {
                self.merge_note_tasks(&note.id, &v);
                if let Ok(summary) = serde_json::from_value::<NoteSummary>(v) {
                    self.merge_tree_note(&summary);
                    self.active
                        .update(|n| n.revision = summary.revision.clone());
                    self.notes.update(|items| {
                        items.retain(|n| n.id != summary.id);
                        if (!summary.archived && self.folder.get_untracked().is_empty())
                            || summary.folder == self.folder.get_untracked()
                            || summary
                                .folder
                                .starts_with(&format!("{}/", self.folder.get_untracked()))
                        {
                            items.insert(0, summary);
                        }
                    });
                }
                self.task_query.set(String::new());
                self.dirty.set(false);
                self.error.set(String::new());
                self.message.set("All changes saved".into());
                if self.page.get_untracked() == "Notes" {
                    set_route("Notes", Some(&self.active.get_untracked().id), true);
                }
                if let Some(s) = storage() {
                    let _ = s.remove_item("folio-draft");
                }
                true
            }
            Err(e) => {
                self.error.set(e);
                self.message.set("Save failed · draft preserved".into());
                false
            }
        }
    }
    pub async fn navigate(self, page: &str) {
        if self.busy.get_untracked() || !self.save().await {
            return;
        }
        self.page.set(page.into());
        self.drawer.set(false);
        self.search_open.set(false);
        set_route(page, None, false);
        if page == "Home" {
            self.folder.set(String::new());
            self.load_notes(false).await;
        }
    }
    pub async fn create_note(self) {
        if self.busy.get_untracked() || !self.save().await {
            return;
        }
        let folder = if self.folder.get_untracked().is_empty() {
            default_folder()
        } else {
            self.folder.get_untracked()
        };
        self.active.set(new_note(folder));
        self.page.set("Notes".into());
        self.preview.set(false);
        self.drawer.set(false);
        self.message.set("A new page".into());
        set_route("Notes", None, false);
        set_timeout(
            move || {
                if let Some(el) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("note-title"))
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    let _ = el.focus();
                }
            },
            Duration::from_millis(50),
        );
    }
    pub async fn open_note(self, id: String) {
        if self.busy.get_untracked() || !self.save().await {
            return;
        }
        self.busy.set(true);
        match self
            .api("GET", &format!("/api/notes/{id}"), Value::Null)
            .await
            .and_then(|v| serde_json::from_value::<Note>(v).map_err(|e| e.to_string()))
        {
            Ok(note) => {
                set_route("Notes", Some(&note.id), true);
                self.expand_ancestors(&note.folder);
                self.active.set(note);
                self.drawer.set(false);
                self.page.set("Notes".into());
                self.dirty.set(false);
                self.search_open.set(false);
                self.preview.set(false);
                self.message.set("All changes saved".into());
                self.error.set(String::new());
            }
            Err(e) => self.error.set(e),
        }
        self.busy.set(false);
    }
    pub async fn daily(self) {
        let id = format!("daily-{}", today());
        if !self.save().await {
            return;
        }
        match self
            .api("GET", &format!("/api/notes/{id}"), Value::Null)
            .await
        {
            Ok(v) => {
                if let Ok(n) = serde_json::from_value::<Note>(v) {
                    set_route("Notes", Some(&n.id), true);
                    self.active.set(n);
                    self.page.set("Notes".into());
                }
            }
            Err(e) if e == "Note not found" => {
                self.active.set(Note {
                    id,
                    markdown: format!("# {}\n\n", nice_date(&today())),
                    folder: "Daily".into(),
                    revision: String::new(),
                });
                self.page.set("Notes".into());
                self.mark_dirty();
                self.save().await;
            }
            Err(e) => self.error.set(e),
        }
    }
    pub async fn load_notes(self, more: bool) {
        self.loading.set(true);
        self.notes_epoch.update(|n| *n += 1);
        let epoch = self.notes_epoch.get_untracked();
        let offset = if more {
            self.next_notes.get_untracked().unwrap_or(0)
        } else {
            0
        };
        let folder = js_sys::encode_uri_component(&self.folder.get_untracked())
            .as_string()
            .unwrap_or_default();
        let result = self
            .api(
                "GET",
                &format!("/api/notes?folder={folder}&offset={offset}"),
                Value::Null,
            )
            .await;
        if epoch != self.notes_epoch.get_untracked() {
            return;
        }
        match result {
            Ok(v) => {
                let items = serde_json::from_value::<Vec<NoteSummary>>(v["items"].clone())
                    .unwrap_or_default();
                if more {
                    self.notes.update(|n| n.extend(items))
                } else {
                    self.notes.set(items)
                }
                self.next_notes.set(v["next_offset"].as_u64());
            }
            Err(e) => self.error.set(e),
        }
        self.loading.set(false);
    }
    pub async fn load_tasks(self, query: String, more: bool) {
        if !more && self.task_query.get_untracked() == query {
            return;
        }
        self.task_epoch.update(|e| *e += 1);
        let epoch = self.task_epoch.get_untracked();
        let version = self.task_version.get_untracked();
        let offset = if more {
            self.next_tasks.get_untracked().unwrap_or(0)
        } else {
            0
        };
        match self
            .api(
                "GET",
                &format!("/api/tasks?{query}&offset={offset}"),
                Value::Null,
            )
            .await
        {
            Ok(v) => {
                if epoch != self.task_epoch.get_untracked() {
                    return;
                }
                if version != self.task_version.get_untracked() {
                    self.task_query.set(String::new());
                    Box::pin(self.load_tasks(query, more)).await;
                    return;
                }
                let mut items =
                    serde_json::from_value::<Vec<Task>>(v["items"].clone()).unwrap_or_default();
                let pending = self.pending_tasks.get_untracked();
                let current = self.tasks.get_untracked();
                preserve_pending_tasks(&mut items, &current, &pending);
                if more {
                    self.tasks.update(|t| {
                        for item in items {
                            t.retain(|old| old.id != item.id);
                            t.push(item);
                        }
                    })
                } else {
                    self.tasks.set(items)
                }
                self.next_tasks.set(v["next_offset"].as_u64());
                self.task_query.set(query);
            }
            Err(e) => self.error.set(e),
        }
    }
    pub fn tasks_for_page(self) -> String {
        match self.page.get_untracked().as_str() {
            "Todo" => format!(
                "group={}&today={}",
                self.task_group.get_untracked().to_lowercase(),
                today()
            ),
            "Planner" => {
                let dates = if self.weekly.get_untracked() {
                    week(&self.date.get_untracked())
                } else {
                    vec![self.date.get_untracked()]
                };
                format!(
                    "from={}&to={}",
                    dates.first().unwrap(),
                    dates.last().unwrap()
                )
            }
            _ => format!("group=today&today={}", today()),
        }
    }
    pub fn merge_note_tasks(self, id: &str, v: &Value) {
        if let Ok(tasks) = serde_json::from_value::<Vec<Task>>(v["tasks"].clone()) {
            self.tasks.update(|items| {
                items.retain(|t| t.source_note_id.as_deref() != Some(id));
                items.extend(tasks);
            });
        }
    }
    pub async fn mutate_task(self, id: Option<String>, body: Value) -> bool {
        let key = id.clone().unwrap_or_default();
        if self.pending_tasks.get_untracked().contains(&key) {
            return false;
        }
        self.pending_tasks.update(|pending| {
            pending.insert(key.clone());
        });
        self.task_version.update(|v| *v += 1);
        let previous = self
            .tasks
            .get_untracked()
            .into_iter()
            .find(|t| Some(&t.id) == id.as_ref());
        if let (Some(task), Some(completed)) = (&previous, body["completed"].as_bool()) {
            self.tasks.update(|items| {
                if let Some(item) = items.iter_mut().find(|item| item.id == task.id) {
                    item.completed = completed;
                }
            });
        }
        let path = id
            .as_ref()
            .map(|s| format!("/api/tasks/{s}"))
            .unwrap_or("/api/tasks".into());
        let success = match self
            .api(if id.is_some() { "PUT" } else { "POST" }, &path, body)
            .await
        {
            Ok(v) => {
                if let Ok(summary) =
                    serde_json::from_value::<NoteSummary>(v["note_summary"].clone())
                {
                    self.merge_tree_note(&summary);
                    self.notes.update(|items| {
                        items.retain(|n| n.id != summary.id);
                        items.insert(0, summary);
                    });
                }
                if let Ok(task) = serde_json::from_value::<Task>(v) {
                    self.tasks.update(|items| {
                        items.retain(|t| t.id != task.id);
                        items.push(task);
                    });
                }
                self.error.set(String::new());
                true
            }
            Err(e) => {
                if let Some(previous) = previous {
                    self.tasks.update(|items| {
                        if let Some(item) = items.iter_mut().find(|t| t.id == previous.id) {
                            *item = previous;
                        }
                    });
                }
                self.error.set(e);
                false
            }
        };
        self.pending_tasks.update(|pending| {
            pending.remove(&key);
        });
        self.task_version.update(|v| *v += 1);
        self.task_query.set(String::new());
        success
    }
    pub async fn mutate_goal(self, id: Option<String>, body: Value) -> Result<(), String> {
        let key = id.clone().unwrap_or_default();
        if self.pending_goals.get_untracked().contains(&key) {
            return Err("This goal is already being saved.".into());
        }
        self.pending_goals.update(|pending| {
            pending.insert(key.clone());
        });
        let path = id
            .as_ref()
            .map(|s| format!("/api/goals/{s}"))
            .unwrap_or("/api/goals".into());
        let result = match self
            .api(if id.is_some() { "PUT" } else { "POST" }, &path, body)
            .await
        {
            Ok(v) => {
                if let Ok(goal) = serde_json::from_value::<Goal>(v) {
                    self.goals.update(|items| {
                        items.retain(|g| g.id != goal.id);
                        items.push(goal);
                        items.sort_by_key(|g| g.position);
                    });
                }
                self.error.set(String::new());
                Ok(())
            }
            Err(e) => {
                self.error.set(e.clone());
                Err(e)
            }
        };
        self.pending_goals.update(|pending| {
            pending.remove(&key);
        });
        result
    }
    pub fn toggle_theme(self) {
        self.dark.update(|d| *d = !*d);
        let theme = if self.dark.get_untracked() {
            "dark"
        } else {
            "light"
        };
        if let Some(s) = storage() {
            let _ = s.set_item("folio-theme", theme);
        }
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.document_element())
        {
            let _ = root.set_attribute("data-theme", theme);
        }
    }
}

// List reads may finish while independent task writes are still pending.
fn preserve_pending_tasks(items: &mut [Task], current: &[Task], pending: &BTreeSet<String>) {
    for item in items {
        if pending.contains(&item.id)
            && let Some(local) = current.iter().find(|t| t.id == item.id)
        {
            *item = local.clone();
        }
    }
}

#[cfg(test)]
mod task_sync_tests {
    use super::*;

    #[test]
    fn stale_list_preserves_multiple_pending_checks_but_accepts_other_updates() {
        let task = |id: &str, completed| Task {
            id: id.into(),
            completed,
            ..Task::default()
        };
        let current = vec![task("a", true), task("b", false), task("c", false)];
        let mut incoming = vec![task("a", false), task("b", true), task("c", true)];
        preserve_pending_tasks(
            &mut incoming,
            &current,
            &BTreeSet::from(["a".into(), "b".into()]),
        );
        assert!(incoming[0].completed);
        assert!(!incoming[1].completed);
        assert!(incoming[2].completed);
    }

    #[test]
    fn settled_tasks_accept_server_state_without_adding_items_from_another_view() {
        let current = vec![Task {
            id: "a".into(),
            completed: true,
            ..Task::default()
        }];
        let mut incoming = vec![Task {
            id: "a".into(),
            ..Task::default()
        }];
        preserve_pending_tasks(&mut incoming, &current, &BTreeSet::new());
        assert!(!incoming[0].completed);
        let mut empty = vec![];
        preserve_pending_tasks(&mut empty, &current, &BTreeSet::from(["a".into()]));
        assert!(empty.is_empty());
    }
}
