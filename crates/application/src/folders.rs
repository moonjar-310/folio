use super::*;

// A short shared lease closes the race between a hierarchy change and a note save.
// It is an internal SQL key, never a canonical object ID.
pub(crate) const FOLDER_LOCK: &str = "_folder_mutation";

fn within(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}/"))
}

impl<S: Store> Application<S> {
    pub(crate) async fn pending_folder_rename(&mut self) -> Result<Value> {
        if let Some(record) = self.record(".folio/folder-rename.json").await? {
            return Ok(record);
        }
        Ok(self
            .store
            .query(statement(
                "SELECT source,target FROM folder_renames WHERE id=1",
                vec![],
            ))
            .await?
            .into_iter()
            .next()
            .unwrap_or(Value::Null))
    }

    pub(crate) async fn check_folder_write(&mut self, path: &str) -> Result<()> {
        let job = self.pending_folder_rename().await?;
        if !job.is_null()
            && [string(&job, "source"), string(&job, "target")]
                .iter()
                .any(|root| within(path, root) || within(root, path))
        {
            return Err(fail(
                409,
                "A folder rename is unfinished. Resume it from the Folders section before editing this hierarchy.",
            ));
        }
        Ok(())
    }

    pub(crate) async fn mutate_folder(&mut self, method: &str, input: Value) -> Result<Value> {
        let owner = self.acquire_note(FOLDER_LOCK).await?;
        let result = self.mutate_folder_locked(method, input).await;
        self.release_note(FOLDER_LOCK, &owner).await;
        result
    }

    async fn mutate_folder_locked(&mut self, method: &str, input: Value) -> Result<Value> {
        let path = string(&input, "path");
        let folders = self.vault_folders().await?;
        for folder in folders.as_array().into_iter().flatten() {
            self.store
                .execute(statement(
                    "INSERT OR IGNORE INTO folders(path) VALUES(?1)",
                    vec![folder["path"].clone()],
                ))
                .await?;
        }
        if !valid_folder(path) {
            return Err(fail(400, "Invalid folder path"));
        }
        if method == "PUT" {
            return self.rename_folder(path, string(&input, "name")).await;
        }
        self.check_folder_write(path).await?;
        if method == "POST" {
            if folders
                .as_array()
                .into_iter()
                .flatten()
                .any(|f| string(f, "path").to_lowercase() == path.to_lowercase())
            {
                return Err(fail(409, "A folder with this name already exists."));
            }
            if !self
                .store
                .query(statement(
                    "SELECT path FROM folders WHERE path=?1",
                    vec![json!(path)],
                ))
                .await?
                .is_empty()
            {
                return Err(fail(409, "A folder with this name already exists."));
            }
            self.ensure_folder(path).await?;
            let paths = folder_paths(path);
            self.store
                .batch(
                    paths
                        .iter()
                        .map(|p| {
                            statement(
                                "INSERT OR IGNORE INTO folders(path) VALUES(?1)",
                                vec![json!(p)],
                            )
                        })
                        .collect(),
                )
                .await?;
            return Ok(json!({"path":path,"paths":paths}));
        }
        let entries = self.all_entries().await?;
        if entries
            .iter()
            .any(|e| e.path.starts_with(&format!("{path}/")) && !e.path.ends_with("/.folio-folder"))
        {
            return Err(fail(
                409,
                "Move the files out of this folder before deleting it.",
            ));
        }
        let mut markers: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                e.path.starts_with(&format!("{path}/")) && e.path.ends_with("/.folio-folder")
            })
            .map(|e| e.path)
            .collect();
        markers.sort_by_key(|p| std::cmp::Reverse(p.len()));
        for marker in markers {
            self.store.delete(&marker).await?;
        }
        self.store
            .execute(statement(
                "DELETE FROM folders WHERE path=?1 OR substr(path,1,length(?1)+1)=?1||'/'",
                vec![json!(path)],
            ))
            .await?;
        Ok(json!({"deleted":true}))
    }

    // Two notes per request bounds canonical reads/writes on Workers. Successful
    // notes leave the source SQL subtree; a failed canonical/index pair is retried.
    async fn rename_folder(&mut self, source: &str, name: &str) -> Result<Value> {
        if name.contains('/') || !valid_folder(name) {
            return Err(fail(400, "Enter one valid folder name, without slashes."));
        }
        let target = source
            .rsplit_once('/')
            .map(|(parent, _)| format!("{parent}/{name}"))
            .unwrap_or_else(|| name.into());
        if !valid_folder(&target) {
            return Err(fail(400, "The resulting folder path is too long."));
        }
        let job = self.pending_folder_rename().await?;
        if !job.is_null() {
            if string(&job, "source") != source || string(&job, "target") != target {
                return Err(fail(409, "Finish the pending folder rename first."));
            }
        } else {
            if self
                .store
                .query(statement(
                    "SELECT path FROM folders WHERE path=?1",
                    vec![json!(source)],
                ))
                .await?
                .is_empty()
            {
                return Err(fail(404, "Folder not found"));
            }
            if target == source {
                return Ok(json!({"source":source,"path":target,"done":true}));
            }
            if !self.store.query(statement("SELECT path FROM folders WHERE path=?1 OR substr(path,1,length(?1)+1)=?1||'/' LIMIT 1",vec![json!(target)])).await?.is_empty() { return Err(fail(409,"A folder with this name already exists.")); }
            let descendants = self
                .store
                .query(statement(
                    "SELECT path FROM folders WHERE path=?1 OR substr(path,1,length(?1)+1)=?1||'/'",
                    vec![json!(source)],
                ))
                .await?;
            if descendants.iter().any(|row| {
                !valid_folder(&format!("{target}{}", &string(row, "path")[source.len()..]))
            }) {
                return Err(fail(
                    400,
                    "A descendant folder path would exceed 240 bytes.",
                ));
            }
            let actual = self.vault_folders().await?;
            if actual
                .as_array()
                .into_iter()
                .flatten()
                .any(|f| string(f, "path").to_lowercase() == target.to_lowercase())
            {
                return Err(fail(
                    409,
                    "A folder with this name already exists, including its letter case.",
                ));
            }
            if self.all_entries().await?.iter().any(|e| {
                e.path.starts_with(&format!("{source}/"))
                    && !paths::note_path(&e.path)
                    && !e.path.ends_with("/.folio-folder")
            }) {
                return Err(fail(
                    409,
                    "This folder contains files Folio cannot move. Move those files with your storage tools first.",
                ));
            }
            self.put_record(
                ".folio/folder-rename.json",
                &json!({"source":source,"target":target}),
            )
            .await?;
        }
        // Discover source files from storage even when every note index row is gone.
        let entries = self.all_entries().await?;
        for entry in entries
            .iter()
            .filter(|e| paths::note_path(&e.path) && e.path.starts_with(&format!("{source}/")))
            .take(2)
        {
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
        }
        let rows=self.store.query(statement("SELECT id,folder FROM notes WHERE folder=?1 OR substr(folder,1,length(?1)+1)=?1||'/' ORDER BY id LIMIT 2",vec![json!(source)])).await?;
        for row in rows {
            let id = string(&row, "id");
            let note: Note = serde_json::from_value(self.read_note(id).await?)
                .map_err(|_| fail(503, "Invalid canonical note"))?;
            let folder = format!("{target}{}", &string(&row, "folder")[source.len()..]);
            if note.folder != folder && !within(&note.folder, source) {
                return Err(fail(
                    409,
                    "A note has a different canonical folder. Resolve it before resuming this rename.",
                ));
            }
            self.save_note_under_folder_lock(
                id,
                SaveNote {
                    markdown: note.markdown,
                    folder,
                    revision: Some(note.revision),
                },
            )
            .await?;
        }
        let remaining=self.store.query(statement("SELECT id FROM notes WHERE folder=?1 OR substr(folder,1,length(?1)+1)=?1||'/' LIMIT 1",vec![json!(source)])).await?;
        let physical_remaining = self
            .all_entries()
            .await?
            .iter()
            .any(|e| paths::note_path(&e.path) && e.path.starts_with(&format!("{source}/")));
        if !remaining.is_empty() || physical_remaining {
            return Ok(json!({"source":source,"path":target,"done":false}));
        }
        let mut markers: Vec<_> = self
            .all_entries()
            .await?
            .into_iter()
            .filter(|e| {
                e.path.starts_with(&format!("{source}/")) && e.path.ends_with("/.folio-folder")
            })
            .map(|e| e.path)
            .collect();
        for marker in &markers {
            self.ensure_folder(&format!(
                "{target}{}",
                &paths::folder(marker)[source.len()..]
            ))
            .await?;
        }
        self.ensure_folder(&target).await?;
        markers.sort_by_key(|p| std::cmp::Reverse(p.len()));
        for marker in markers {
            self.store.delete(&marker).await?;
        }
        self.store.batch(vec![
            statement("INSERT OR IGNORE INTO folders(path) SELECT ?2||substr(path,length(?1)+1) FROM folders WHERE path=?1 OR substr(path,1,length(?1)+1)=?1||'/'",vec![json!(source),json!(target)]),
            statement("DELETE FROM folders WHERE path=?1 OR substr(path,1,length(?1)+1)=?1||'/'",vec![json!(source)]),
            statement("DELETE FROM folder_renames WHERE id=1",vec![]),
        ]).await?;
        self.store.delete(".folio/folder-rename.json").await?;
        Ok(json!({"source":source,"path":target,"done":true}))
    }
}
