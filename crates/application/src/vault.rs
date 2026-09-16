//! Canonical paths and resumable object moves; SQL is never needed to resolve a path handle.
use super::*;
use folio_core::{
    paths,
    storage::{Entry, Object},
};

fn identity(id: &str) -> String {
    format!(".folio/notes/{}.json", digest(id))
}
fn operation(id: &str) -> String {
    format!(".folio/moves/{}.json", digest(id))
}
impl<S: Store> Application<S> {
    pub(crate) async fn put_record(&mut self, path: &str, value: &Value) -> Result<()> {
        let old = self.store.read(path).await?;
        self.store
            .write(
                path,
                &value.to_string(),
                old.as_ref().map(|o| o.version.as_str()),
            )
            .await?;
        Ok(())
    }
    pub(crate) async fn record(&mut self, path: &str) -> Result<Option<Value>> {
        self.store
            .read(path)
            .await?
            .map(|o| {
                serde_json::from_str(&o.markdown).map_err(|_| fail(503, "Invalid workspace record"))
            })
            .transpose()
    }
    pub(crate) async fn resolve_note(&mut self, id: &str) -> Result<Option<(String, Object)>> {
        if !paths::valid_note_id(id) {
            return Err(fail(400, "Invalid note ID"));
        }
        // Optional identity metadata must never prevent opening a discoverable Markdown file.
        if let Some(object) = self.store.read(&identity(id)).await?
            && let Ok(record) = serde_json::from_str::<Value>(&object.markdown)
        {
            let path = string(&record, "path");
            if paths::note_path(path)
                && let Some(object) = self.store.read(path).await?
            {
                return Ok(Some((path.into(), object)));
            }
        }
        if let Some(path) = paths::id_path(id) {
            return Ok(self.store.read(&path).await?.map(|o| (path, o)));
        }
        let rows = self
            .store
            .query(statement(
                "SELECT path FROM notes WHERE id=?1",
                vec![json!(id)],
            ))
            .await?;
        if let Some(path) = rows
            .first()
            .and_then(|r| r["path"].as_str())
            .filter(|p| paths::note_path(p))
            && let Some(object) = self.store.read(path).await?
        {
            return Ok(Some((path.into(), object)));
        }
        // Compatibility with the pre-path vault, until explicit migration completes.
        let path = format!("{id}.md");
        Ok(self.store.read(&path).await?.map(|o| (path, o)))
    }
    pub(crate) async fn canonical_folder(
        &mut self,
        id: &str,
        path: &str,
        stored: &str,
    ) -> Result<String> {
        if path != format!("{id}.md") || paths::id_path(id).is_some() {
            return Ok(paths::folder(path));
        }
        if paths::id_path(id).is_some() {
            return Ok(String::new());
        }
        if let Some(folder) = notes::content(stored).1 {
            return Ok(folder);
        }
        let rows = self
            .store
            .query(statement(
                "SELECT folder FROM notes WHERE id=?1",
                vec![json!(id)],
            ))
            .await?;
        Ok(rows
            .first()
            .map(|r| string(r, "folder").into())
            .unwrap_or_else(default_folder))
    }
    pub(crate) async fn ensure_folder(&mut self, folder: &str) -> Result<()> {
        if folder.is_empty() {
            return Ok(());
        }
        for path in folder_paths(folder) {
            let marker = format!("{path}/.folio-folder");
            if self.store.read(&marker).await?.is_none() {
                self.store.write(&marker, "", None).await?;
            }
        }
        Ok(())
    }
    /// Journal before copying, verify both copies, then remove the old path. Safe to retry.
    async fn finish_move(&mut self, id: &str, job: &Value) -> Result<()> {
        let from = string(job, "from");
        let to = string(job, "to");
        let body = string(job, "body");
        if !paths::note_path(to) || (!from.is_empty() && !paths::note_path(from)) {
            return Err(fail(503, "Invalid move journal"));
        }
        let source = if from.is_empty() {
            None
        } else {
            self.store.read(from).await?
        };
        if let Some(source) = &source
            && digest(&source.markdown) != string(job, "source_digest")
        {
            return Err(fail(
                409,
                "Source file changed during a move. Both paths have been preserved.",
            ));
        }
        if let Some(target) = self.store.read(to).await? {
            if target.markdown != body {
                return Err(fail(
                    409,
                    "Destination changed during a move. Both files have been preserved.",
                ));
            }
        } else {
            if !from.is_empty() && source.is_none() {
                return Err(fail(
                    409,
                    "Move source is missing. Recovery record preserved.",
                ));
            }
            self.ensure_folder(&paths::folder(to)).await?;
            self.store.write(to, body, None).await?;
        }
        let verified = self
            .store
            .read(to)
            .await?
            .ok_or_else(|| fail(503, "Move verification failed"))?;
        if verified.markdown != body {
            return Err(fail(503, "Move verification failed"));
        }
        self.put_record(&identity(id), &json!({"id":id,"path":to}))
            .await?;
        if !from.is_empty()
            && from != to
            && let Some(current) = self.store.read(from).await?
        {
            if digest(&current.markdown) != string(job, "source_digest") {
                return Err(fail(409, "Source changed; preserved both files."));
            }
            self.store.delete(from).await?;
        }
        self.store.delete(&operation(id)).await?;
        Ok(())
    }
    pub(crate) async fn resume_move(&mut self, id: &str) -> Result<()> {
        if let Some(job) = self.record(&operation(id)).await? {
            self.finish_move(id, &job).await?;
        }
        Ok(())
    }
    pub(crate) async fn write_canonical(
        &mut self,
        id: &str,
        note: &Note,
        current: Option<&(String, Object)>,
        migrating: bool,
    ) -> Result<String> {
        let summary = summarize(note, self.now);
        let filename = if let Some((path, old)) = current {
            let old_note = Note {
                markdown: notes::content(&old.markdown).0.into(),
                ..note.clone()
            };
            if path.contains('/') && summarize(&old_note, 0).title == summary.title {
                path.rsplit('/').next().unwrap().to_string()
            } else {
                paths::filename(&summary.title)
            }
        } else {
            paths::filename(&summary.title)
        };
        let mut target = if note.folder.is_empty() {
            filename
        } else {
            format!("{}/{filename}", note.folder)
        };
        if !paths::note_path(&target) {
            return Err(fail(400, "Invalid note path"));
        }
        if let Some((path, old)) = current
            && *path == target
        {
            self.store
                .write(path, &note.markdown, Some(&old.version))
                .await?;
            self.put_record(&identity(id), &json!({"id":id,"path":target}))
                .await?;
            return Ok(target);
        }
        // Match Windows case-insensitive collision behavior in R2 as well.
        let existing = self.all_entries().await?;
        let occupied = |path: &str| {
            existing
                .iter()
                .any(|e| e.path.to_lowercase() == path.to_lowercase())
        };
        if occupied(&target) {
            if !migrating {
                return Err(fail(
                    409,
                    "A file with this name already exists in the folder. Choose another title.",
                ));
            }
            let stem = target.strip_suffix(".md").unwrap().to_string();
            let suffix = &digest(id)[..12];
            target = format!("{stem} ({suffix}).md");
            if occupied(&target) {
                return Err(fail(
                    409,
                    "Migration destination exists. Originals preserved.",
                ));
            }
        }
        let from = current.map(|(p, _)| p.as_str()).unwrap_or("");
        let job = json!({"id":id,"from":from,"to":target,"body":note.markdown,"source_digest":current.map(|(_,o)|digest(&o.markdown)).unwrap_or_default()});
        self.put_record(&operation(id), &job).await?;
        self.finish_move(id, &job).await?;
        Ok(target)
    }
    pub(crate) async fn all_entries(&mut self) -> Result<Vec<Entry>> {
        let mut cursor = None;
        let mut entries = Vec::new();
        loop {
            let page = self.store.list(cursor.as_deref(), 1000).await?;
            entries.extend(page.entries);
            cursor = page.cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(entries)
    }
    pub(crate) async fn vault_folders(&mut self) -> Result<Value> {
        let entries = self.all_entries().await?;
        self.vault_folders_from_entries(&entries).await
    }
    pub(crate) async fn vault_folders_from_entries(&mut self, entries: &[Entry]) -> Result<Value> {
        let mut folders = std::collections::BTreeSet::new();
        for entry in entries {
            if entry
                .path
                .split('/')
                .any(|s| s.starts_with('.') && s != ".folio-folder")
            {
                continue;
            }
            let folder = paths::folder(&entry.path);
            if valid_folder(&folder) {
                folders.extend(folder_paths(&folder));
            }
        }
        // Legacy folders remain discoverable during migration; physical paths win afterwards.
        if self.store.read(".folio/path-storage.json").await?.is_none() {
            for row in self
                .store
                .query(statement("SELECT path FROM folders", vec![]))
                .await?
            {
                if valid_folder(string(&row, "path")) {
                    folders.insert(string(&row, "path").into());
                }
            }
        }
        Ok(json!(
            folders
                .into_iter()
                .map(|path| json!({"path":path}))
                .collect::<Vec<_>>()
        ))
    }
    pub(crate) async fn browse_notes(
        &mut self,
        q: &std::collections::HashMap<String, String>,
    ) -> Result<Value> {
        let entries = self.all_entries().await?;
        self.browse_notes_from_entries(q, &entries).await
    }
    pub(crate) async fn browse_notes_from_entries(
        &mut self,
        q: &std::collections::HashMap<String, String>,
        entries: &[Entry],
    ) -> Result<Value> {
        let folder = q.get("folder").map(String::as_str).unwrap_or("");
        let exact = q.get("exact").is_some_and(|s| s == "true");
        let archive = folder == "Archive"
            || folder.starts_with("Archive/")
            || q.get("archived").is_some_and(|s| s == "true");
        let offset = q
            .get("offset")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0)
            .min(100_000);
        let mut items = Vec::new();
        // Metadata-only listing. Markdown bodies are read only when opening or rebuilding.
        let mut cached = std::collections::HashMap::new();
        for chunk in entries.chunks(500) {
            let keys: Vec<_> = chunk.iter().map(|e| e.path.clone()).collect();
            let legacy: Vec<_> = chunk
                .iter()
                .filter_map(|e| e.path.strip_suffix(".md").filter(|id| valid_id(id)))
                .collect();
            let rows=self.store.query(statement("SELECT id,title,preview,updated_at,folder,revision,archived,path FROM notes WHERE path IN (SELECT value FROM json_each(?1)) OR (path IS NULL AND id IN (SELECT value FROM json_each(?2)))",vec![json!(serde_json::to_string(&keys).unwrap()),json!(serde_json::to_string(&legacy).unwrap())])).await?;
            for row in rows {
                let key = row["path"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{}.md", string(&row, "id")));
                cached.insert(key, row);
            }
        }
        for entry in entries {
            if !paths::note_path(&entry.path) {
                continue;
            }
            let legacy_id = entry.path.strip_suffix(".md").filter(|p| valid_id(p));
            let rows = cached.remove(&entry.path).into_iter();
            let mut item = rows.into_iter().next().unwrap_or_else(||json!({"id":paths::path_id(&entry.path),"title":entry.path.rsplit('/').next().unwrap_or("").trim_end_matches(".md"),"preview":"","updated_at":entry.updated_at,"revision":""}));
            let actual =
                if legacy_id.is_some() && item["path"].is_null() && item["folder"].is_string() {
                    string(&item, "folder").to_string()
                } else {
                    paths::folder(&entry.path)
                };
            let archived = actual == "Archive" || actual.starts_with("Archive/");
            if (!folder.is_empty()
                && actual != folder
                && (exact || !actual.starts_with(&format!("{folder}/"))))
                || (!archive && archived)
            {
                continue;
            }
            item["folder"] = json!(actual);
            item["archived"] = json!(archived);
            item["path"] = json!(entry.path);
            items.push(item);
        }
        items.sort_by(|a, b| {
            b["updated_at"]
                .as_u64()
                .cmp(&a["updated_at"].as_u64())
                .then_with(|| string(a, "id").cmp(string(b, "id")))
        });
        let more = items.len() > offset + LIST_LIMIT;
        Ok(
            json!({"items":items.into_iter().skip(offset).take(LIST_LIMIT).collect::<Vec<_>>(),"next_offset":if more{Some(offset+LIST_LIMIT)}else{None}}),
        )
    }
    pub(crate) async fn remove_note_record(&mut self, id: &str) -> Result<()> {
        self.store.delete(&identity(id)).await?;
        Ok(())
    }
}
