use super::*;
const FOLDER_PREFIX: &str = "<!-- folio-folder:";
pub(crate) fn content(stored: &str) -> (&str, Option<String>) {
    if let Some((first, body)) = stored.split_once('\n')
        && let Some(value) = first
            .strip_prefix(FOLDER_PREFIX)
            .and_then(|s| s.strip_suffix(" -->"))
        && let Ok(folder) = serde_json::from_str::<String>(value)
        && valid_folder(&folder)
    {
        return (body, Some(folder));
    }
    (stored, None)
}
impl<S: Store> Application<S> {
    pub(crate) async fn list_notes(
        &mut self,
        q: &std::collections::HashMap<String, String>,
    ) -> Result<Value> {
        if q.get("q").is_none_or(|s| s.is_empty()) {
            return self.browse_notes(q).await;
        }
        let search = q.get("q").cloned().unwrap_or_default().to_lowercase();
        if search.len() > 200 {
            return Err(fail(400, "Search is too long"));
        }
        let folder = q.get("folder").cloned().unwrap_or_default();
        let offset = q
            .get("offset")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
            .min(100_000);
        let archived = folder == "Archive"
            || folder.starts_with("Archive/")
            || q.get("archived").is_some_and(|s| s == "true");
        let exact = q.get("exact").is_some_and(|v| v == "true");
        let rows=self.store.query(statement("SELECT id,title,preview,updated_at,folder,revision,archived FROM notes WHERE (?1='' OR instr(lower(title||' '||searchable),?1)>0) AND (?2='' OR folder=?2 OR (?5=0 AND substr(folder,1,length(?2)+1)=?2||'/')) AND (?3=1 OR archived=0) ORDER BY updated_at DESC,id LIMIT 51 OFFSET ?4",vec![json!(search),json!(folder),json!(archived),json!(offset),json!(exact)])).await?;
        let more = rows.len() > LIST_LIMIT;
        Ok(
            json!({"items":normalize(rows.into_iter().take(LIST_LIMIT).collect(),&["archived"]),"next_offset":if more{Some(offset+LIST_LIMIT as u32)}else{None}}),
        )
    }
    pub(crate) async fn read_note(&mut self, id: &str) -> Result<Value> {
        let (path, object) = self
            .resolve_note(id)
            .await?
            .ok_or_else(|| fail(404, "Note not found"))?;
        let folder = self.canonical_folder(id, &path, &object.markdown).await?;
        let markdown = if path.as_str() == format!("{id}.md") && paths::id_path(id).is_none() {
            content(&object.markdown).0
        } else {
            &object.markdown
        };
        let revision = digest(&format!("{markdown}\0{folder}"));
        Ok(json!(Note {
            id: id.into(),
            markdown: markdown.into(),
            folder,
            revision
        }))
    }
    pub(crate) async fn acquire_note(&mut self, id: &str) -> Result<String> {
        let owner = uid();
        self.store.execute(statement("INSERT INTO note_locks(id,owner,expires_at) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET owner=excluded.owner,expires_at=excluded.expires_at WHERE note_locks.expires_at<=?4",vec![json!(id),json!(owner),json!(self.now+120_000),json!(self.now)])).await?;
        let lock = self
            .store
            .query(statement(
                "SELECT owner FROM note_locks WHERE id=?1",
                vec![json!(id)],
            ))
            .await?;
        if lock.first().is_none_or(|r| string(r, "owner") != owner) {
            return Err(fail(
                409,
                "A save is already in progress for this note. Keep your draft and retry shortly.",
            ));
        }
        Ok(owner)
    }
    pub(crate) async fn release_note(&mut self, id: &str, owner: &str) {
        let _ = self
            .store
            .execute(statement(
                "DELETE FROM note_locks WHERE id=?1 AND owner=?2",
                vec![json!(id), json!(owner)],
            ))
            .await;
    }
    pub(crate) async fn save_note(&mut self, id: &str, input: SaveNote) -> Result<Value> {
        validate_note(id, &input.markdown).map_err(|s| fail(400, s))?;
        let guard = self.acquire_note(folders::FOLDER_LOCK).await?;
        let result = async {
            self.check_folder_write(&input.folder).await?;
            let old = self
                .store
                .query(statement(
                    "SELECT folder FROM notes WHERE id=?1",
                    vec![json!(id)],
                ))
                .await?;
            if let Some(row) = old.first() {
                self.check_folder_write(string(row, "folder")).await?;
            }
            self.save_note_under_folder_lock(id, input).await
        }
        .await;
        self.release_note(folders::FOLDER_LOCK, &guard).await;
        result
    }
    pub(crate) async fn save_note_under_folder_lock(
        &mut self,
        id: &str,
        input: SaveNote,
    ) -> Result<Value> {
        let owner = self.acquire_note(id).await?;
        let result = self.save_note_locked(id, input).await;
        self.release_note(id, &owner).await;
        result
    }
    async fn save_note_locked(&mut self, id: &str, input: SaveNote) -> Result<Value> {
        validate_note(id, &input.markdown).map_err(|s| fail(400, s))?;
        if !input.folder.is_empty() && !valid_folder(&input.folder) {
            return Err(fail(400, "Invalid folder path"));
        }
        if checkboxes(&input.markdown).len() > 2000 {
            return Err(fail(
                400,
                "A note can contain at most 2,000 indexed checkboxes.",
            ));
        }
        self.resume_move(id).await?;
        let current = self.resolve_note(id).await?;
        if let Some((path, old)) = &current {
            let old_folder = self.canonical_folder(id, path, &old.markdown).await?;
            let old_markdown =
                if path.as_str() == format!("{id}.md") && paths::id_path(id).is_none() {
                    content(&old.markdown).0
                } else {
                    &old.markdown
                };
            let revision = digest(&format!("{old_markdown}\0{old_folder}"));
            if input.revision.as_deref() != Some(&revision)
                && !(old_markdown == input.markdown && old_folder == input.folder)
            {
                return Err(fail(
                    409,
                    "This note changed elsewhere. Your draft is preserved; reopen it or save as a new note.",
                ));
            }
        } else if input.revision.as_ref().is_some_and(|s| !s.is_empty()) {
            return Err(fail(
                409,
                "This note was deleted elsewhere. Save your draft as a new note.",
            ));
        }
        let note = Note {
            id: id.into(),
            revision: digest(&format!("{}\0{}", input.markdown, input.folder)),
            markdown: input.markdown,
            folder: input.folder,
        };
        let path = self
            .write_canonical(id, &note, current.as_ref(), false)
            .await?;
        self.index_note(&note, &path, self.now).await.map_err(|_| {
            fail(
                503,
                "Markdown saved; index update failed. Retry Save or rebuild the index.",
            )
        })
    }
    pub(crate) async fn index_note(
        &mut self,
        note: &Note,
        path: &str,
        updated_at: u64,
    ) -> Result<Value> {
        let id = &note.id;
        let summary = summarize(note, updated_at);

        self.store.batch(vec![
            statement("UPDATE tasks SET source_note_id=?2 WHERE source_note_id IN (SELECT id FROM notes WHERE path=?1 AND id<>?2)",vec![json!(path),json!(id)]),
            statement("DELETE FROM notes WHERE path=?1 AND id<>?2",vec![json!(path),json!(id)]),
        ]).await?;
        let old_tasks = normalize(
            self.store
                .query(statement(
                    "SELECT * FROM tasks WHERE source_note_id=?1 ORDER BY source_line",
                    vec![json!(id)],
                ))
                .await?,
            &["completed"],
        );
        let mut old_tasks: Vec<Task> = old_tasks
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        let mut statements = vec![
            statement(
                "INSERT INTO notes(id,title,preview,updated_at,folder,searchable,revision,archived,path) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET title=excluded.title,preview=excluded.preview,updated_at=excluded.updated_at,folder=excluded.folder,searchable=excluded.searchable,revision=excluded.revision,archived=excluded.archived,path=excluded.path",
                vec![
                    json!(id),
                    json!(summary.title),
                    json!(summary.preview),
                    json!(updated_at),
                    json!(note.folder),
                    json!(note.markdown.to_lowercase()),
                    json!(note.revision),
                    json!(summary.archived),
                    json!(path),
                ],
            ),
            statement(
                "INSERT OR IGNORE INTO folders(path) VALUES(?1)",
                vec![json!(note.folder)],
            ),
            statement("DELETE FROM tasks WHERE source_note_id=?1", vec![json!(id)]),
        ];
        statements.extend(folder_paths(&note.folder).into_iter().map(|p| {
            statement(
                "INSERT OR IGNORE INTO folders(path) VALUES(?1)",
                vec![json!(p)],
            )
        }));
        let mut indexed = Vec::<Task>::new();
        if !summary.archived {
            let parsed = checkboxes(&note.markdown);
            if parsed.len() > 2000 {
                return Err(fail(
                    400,
                    "A note can contain at most 2,000 indexed checkboxes.",
                ));
            }
            let titles: Vec<String> = parsed
                .iter()
                .map(|(_, text, _)| planning::task_text(text).0)
                .collect();
            for (line, text, completed) in parsed {
                let (title, due_date, due_time) = planning::task_text(&text);
                if title.is_empty() {
                    continue;
                }
                let matched = old_tasks.iter().position(|t| t.title == title).or_else(|| {
                    old_tasks
                        .iter()
                        .position(|t| t.source_line == Some(line) && !titles.contains(&t.title))
                });
                let old = matched.map(|i| old_tasks.remove(i));
                let task = Task {
                    id: old.as_ref().map(|t| t.id.clone()).unwrap_or_else(uid),
                    title,
                    completed,
                    due_date,
                    due_time,
                    source_note_id: Some(id.into()),
                    source_line: Some(line),
                    created_at: old.map(|t| t.created_at).unwrap_or(self.now),
                    updated_at: self.now,
                };
                indexed.push(task);
            }
        }
        statements.push(statement("INSERT INTO tasks(id,title,completed,due_date,due_time,source_note_id,source_line,created_at,updated_at) SELECT json_extract(value,'$.id'),json_extract(value,'$.title'),json_extract(value,'$.completed'),json_extract(value,'$.due_date'),json_extract(value,'$.due_time'),json_extract(value,'$.source_note_id'),json_extract(value,'$.source_line'),json_extract(value,'$.created_at'),json_extract(value,'$.updated_at') FROM json_each(?1)",vec![json!(serde_json::to_string(&indexed).map_err(|_|fail(500,"Task serialization failed"))?)]));
        self.store.batch(statements).await?;
        let mut response = json!(summary);
        response["tasks"] = json!(indexed);
        Ok(response)
    }
    pub(crate) async fn delete_note(&mut self, id: &str, input: Value) -> Result<Value> {
        let guard = self.acquire_note(folders::FOLDER_LOCK).await?;
        let result = async {
            let old = self
                .store
                .query(statement(
                    "SELECT folder FROM notes WHERE id=?1",
                    vec![json!(id)],
                ))
                .await?;
            if let Some(row) = old.first() {
                self.check_folder_write(string(row, "folder")).await?;
            }
            let owner = self.acquire_note(id).await?;
            let result = self.delete_note_locked(id, input).await;
            self.release_note(id, &owner).await;
            result
        }
        .await;
        self.release_note(folders::FOLDER_LOCK, &guard).await;
        result
    }
    async fn delete_note_locked(&mut self, id: &str, input: Value) -> Result<Value> {
        self.resume_move(id).await?;
        if let Some((path, object)) = self.resolve_note(id).await? {
            let note: Note = serde_json::from_value(self.read_note(id).await?)
                .map_err(|_| fail(503, "Invalid note"))?;
            if string(&input, "revision") != note.revision {
                return Err(fail(409, "The note changed. Reopen it before deleting."));
            }
            let latest = self
                .store
                .read(&path)
                .await?
                .ok_or_else(|| fail(409, "Note changed"))?;
            if latest.version != object.version {
                return Err(fail(409, "Note changed"));
            }
            self.store.delete(&path).await?;
        }
        self.remove_note_record(id).await?;
        self.store
            .batch(vec![
                statement("DELETE FROM tasks WHERE source_note_id=?1", vec![json!(id)]),
                statement("DELETE FROM notes WHERE id=?1", vec![json!(id)]),
            ])
            .await?;
        Ok(json!({"deleted":true}))
    }
}
