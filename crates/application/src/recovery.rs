//! Bounded, explicit migration and cache recovery. Authentication tables are untouched.
use super::*;
impl<S: Store> Application<S> {
    /// For the local operator CLI; never exposed as an unauthenticated HTTP route.
    pub async fn maintain_offline(&mut self, migrate: bool) -> Result<()> {
        let mut input = json!({});
        let mut skipped = 0;
        loop {
            let result = if migrate {
                self.migrate(&input).await?
            } else {
                self.rebuild(&input).await?
            };
            skipped += result["warnings"].as_array().map(Vec::len).unwrap_or(0);
            if result["done"] == true {
                break;
            }
            input = result;
        }
        if skipped > 0 {
            return Err(fail(
                503,
                &format!(
                    "Rebuild completed with {skipped} skipped records. Inspect storage records or use Settings for details."
                ),
            ));
        }
        Ok(())
    }
    pub(crate) async fn rebuild(&mut self, input: &Value) -> Result<Value> {
        let owner = self.acquire_note(folders::FOLDER_LOCK).await?;
        let result = self.rebuild_locked(input).await;
        self.release_note(folders::FOLDER_LOCK, &owner).await;
        result
    }
    async fn rebuild_locked(&mut self, input: &Value) -> Result<Value> {
        if let Some(phase) = input["phase"].as_str() {
            return self.prune_index(phase, string(input, "after")).await;
        }
        let page = self.store.list(input["cursor"].as_str(), 2).await?;
        let mut warnings = Vec::new();
        let mut restored = 0;
        for entry in &page.entries {
            let result: Result<()> = async {
            if paths::note_path(&entry.path) {
                let rows = self
                    .store
                    .query(statement(
                        "SELECT id FROM notes WHERE path=?1",
                        vec![json!(entry.path)],
                    ))
                    .await?;
                let id = rows
                    .first()
                    .map(|r| string(r, "id").to_string())
                    .unwrap_or_else(|| paths::path_id(&entry.path));
                let note: Note = serde_json::from_value(self.read_note(&id).await?)
                    .map_err(|_| fail(503, "Invalid note"))?;
                self.index_note(&note, &entry.path, entry.updated_at)
                    .await?;
                restored += 1;
            } else if entry.path.starts_with(".folio/notes/") && entry.path.ends_with(".json") {
                if let Some(record) = self.record(&entry.path).await? {
                    let id = string(&record, "id");
                    let path = string(&record, "path");
                    if !paths::valid_note_id(id) || !paths::note_path(path) {
                        return Err(fail(503, "Invalid note identity"));
                    }
                    if let Some(object) = self.store.read(path).await? {
                        let folder = paths::folder(path);
                        let note = Note {
                            id: id.into(),
                            revision: digest(&format!("{}\0{folder}", object.markdown)),
                            markdown: object.markdown,
                            folder,
                        };
                        self.index_note(&note, path, entry.updated_at).await?;
                    }
                }
            } else if entry.path.starts_with(".folio/tasks/") && entry.path.ends_with(".json") {
                if let Some(record) = self.record(&entry.path).await? {
                    let task: Task = serde_json::from_value(record)
                        .map_err(|_| fail(503, "Invalid task record"))?;
                    if !valid_id(&task.id) || task.source_note_id.is_some() {
                        return Err(fail(503, "Invalid standalone task record"));
                    }
                    self.store.execute(planning::task_statement(&task)).await?;
                    restored += 1;
                }
            } else if entry.path.starts_with(".folio/goals/") && entry.path.ends_with(".json") {
                if let Some(goal) = self.record(&entry.path).await? {
                    if !valid_id(string(&goal, "id")) {
                        return Err(fail(503, "Invalid goal record"));
                    }
                    self.store.execute(statement("INSERT INTO goals(id,title,description,position,status,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET title=excluded.title,description=excluded.description,position=excluded.position,status=excluded.status,updated_at=excluded.updated_at",vec![goal["id"].clone(),goal["title"].clone(),goal["description"].clone(),goal["position"].clone(),goal["status"].clone(),goal["created_at"].clone(),goal["updated_at"].clone()])).await?;
                    restored += 1;
                }
            } else if entry.path.ends_with("/.folio-folder") {
                let folder = paths::folder(&entry.path);
                if valid_folder(&folder) {
                    self.store
                        .execute(statement(
                            "INSERT OR IGNORE INTO folders(path) VALUES(?1)",
                            vec![json!(folder)],
                        ))
                        .await?;
                }
            }
                    Ok(())
            }.await;
            if let Err(error) = result {
                warnings.push(json!({"path":entry.path,"message":error.message}));
            }
        }
        Ok(
            json!({"cursor":page.cursor,"phase":if page.cursor.is_none(){Some("notes")}else{None},"after":"","done":false,"restored":restored,"warnings":warnings}),
        )
    }
    async fn prune_index(&mut self, phase: &str, after: &str) -> Result<Value> {
        let (sql, next) = match phase {
            "notes" => (
                "SELECT id,path FROM notes WHERE path IS NOT NULL AND id>?1 ORDER BY id LIMIT 2",
                "tasks",
            ),
            "tasks" => (
                "SELECT id FROM tasks WHERE source_note_id IS NULL AND id>?1 ORDER BY id LIMIT 2",
                "goals",
            ),
            "goals" => (
                "SELECT id FROM goals WHERE id>?1 ORDER BY id LIMIT 2",
                "done",
            ),
            "done" => return Ok(json!({"done":true})),
            _ => return Err(fail(400, "Invalid rebuild phase")),
        };
        if phase != "notes" && self.store.read(".folio/path-storage.json").await?.is_none() {
            return Ok(json!({"done":true}));
        }
        let rows = self.store.query(statement(sql, vec![json!(after)])).await?;
        for row in &rows {
            let id = string(row, "id");
            let path = if phase == "notes" {
                string(row, "path").to_string()
            } else {
                format!(".folio/{phase}/{id}.json")
            };
            if self.store.read(&path).await?.is_none() {
                if phase == "notes" {
                    self.store
                        .execute(statement(
                            "DELETE FROM tasks WHERE source_note_id=?1",
                            vec![json!(id)],
                        ))
                        .await?;
                }
                self.store
                    .execute(statement(
                        &format!("DELETE FROM {phase} WHERE id=?1"),
                        vec![json!(id)],
                    ))
                    .await?;
            }
        }
        Ok(
            json!({"phase":if rows.len()==2{phase}else{next},"after":if rows.len()==2{rows.last().map(|r|string(r,"id")).unwrap_or("")}else{""},"done":false}),
        )
    }
    pub(crate) async fn migrate(&mut self, input: &Value) -> Result<Value> {
        let owner = self.acquire_note(folders::FOLDER_LOCK).await?;
        let result = self.migrate_locked(input).await;
        self.release_note(folders::FOLDER_LOCK, &owner).await;
        result
    }
    async fn migrate_locked(&mut self, input: &Value) -> Result<Value> {
        let phase = input["phase"].as_str().unwrap_or("notes");
        let after = input["after"].as_str().unwrap_or("");
        if phase == "notes" {
            let rows = self
                .store
                .query(statement(
                    "SELECT id FROM notes WHERE path IS NULL ORDER BY id LIMIT 2",
                    vec![],
                ))
                .await?;
            for row in &rows {
                let id = string(row, "id");
                let old = self.store.read(&format!("{id}.md")).await?;
                if let Some(old) = old {
                    let backup = format!(".folio/legacy/{id}.md");
                    if self.store.read(&backup).await?.is_none() {
                        self.store.write(&backup, &old.markdown, None).await?;
                    }
                    let check = self
                        .store
                        .read(&backup)
                        .await?
                        .ok_or_else(|| fail(503, "Migration backup missing"))?;
                    if check.markdown != old.markdown {
                        return Err(fail(409, "Legacy backup differs. Original preserved."));
                    }
                }
                let note: Note = serde_json::from_value(self.read_note(id).await?)
                    .map_err(|_| fail(503, "Invalid legacy note"))?;
                self.resume_move(id).await?;
                let current = self.resolve_note(id).await?;
                let path = self
                    .write_canonical(id, &note, current.as_ref(), true)
                    .await?;
                self.index_note(&note, &path, self.now).await?;
            }
            return Ok(
                json!({"phase":if rows.len()==2{"notes"}else{"tasks"},"after":"","done":false}),
            );
        }
        let (sql, next) = match phase {
            "tasks" => (
                "SELECT * FROM tasks WHERE source_note_id IS NULL AND id>?1 ORDER BY id LIMIT 2",
                "goals",
            ),
            "goals" => (
                "SELECT * FROM goals WHERE id>?1 ORDER BY id LIMIT 2",
                "folders",
            ),
            "folders" => (
                "SELECT path AS id FROM folders WHERE path>?1 ORDER BY path LIMIT 2",
                "done",
            ),
            "done" => {
                self.put_record(".folio/path-storage.json", &json!({"version":1}))
                    .await?;
                return Ok(json!({"done":true}));
            }
            _ => return Err(fail(400, "Invalid migration phase")),
        };
        let rows = self.store.query(statement(sql, vec![json!(after)])).await?;
        for row in &rows {
            let id = string(row, "id");
            if phase == "folders" {
                self.ensure_folder(id).await?;
            } else {
                let path = format!(".folio/{phase}/{id}.json");
                if self.store.read(&path).await?.is_none() {
                    let record = if phase == "tasks" {
                        normalize(vec![row.clone()], &["completed"]).remove(0)
                    } else {
                        row.clone()
                    };
                    self.store.write(&path, &record.to_string(), None).await?;
                }
            }
        }
        Ok(
            json!({"phase":if rows.len()==2{phase}else{next},"after":if rows.len()==2{rows.last().map(|r|string(r,"id")).unwrap_or("")}else{""},"done":false}),
        )
    }
}
