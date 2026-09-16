//! Native persistence. The HTTP entrypoint serializes access outside the async executor.
use folio_core::{
    paths::valid_path,
    storage::{Entry, Listing, Object, Statement, Store},
};
use rusqlite::{
    Connection,
    types::{Value as SqlValue, ValueRef},
};
use serde_json::{Map, Value};
use std::{io::Write, path::PathBuf};
pub struct LocalStore {
    vault: PathBuf,
    db: Connection,
}
fn sql_values(values: &[Value]) -> Vec<SqlValue> {
    values
        .iter()
        .map(|v| match v {
            Value::Null => SqlValue::Null,
            Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
            Value::Number(n) => SqlValue::Integer(n.as_i64().unwrap_or(0)),
            Value::String(s) => SqlValue::Text(s.clone()),
            _ => SqlValue::Text(v.to_string()),
        })
        .collect()
}
impl LocalStore {
    pub fn open(root: PathBuf) -> Result<Self, String> {
        let vault = root.join("vault");
        std::fs::create_dir_all(&vault).map_err(|e| e.to_string())?;
        let mut db = Connection::open(root.join("index.sqlite")).map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000; CREATE TABLE IF NOT EXISTS folio_migrations(version INTEGER PRIMARY KEY);").map_err(|e|e.to_string())?;
        for (version, sql) in [
            (1, include_str!("../../../migrations/0001_notes.sql")),
            (2, include_str!("../../../migrations/0002_planner.sql")),
            (3, include_str!("../../../migrations/0003_note_locks.sql")),
            (
                4,
                include_str!("../../../migrations/0004_folder_renames.sql"),
            ),
            (5, include_str!("../../../migrations/0005_vault_paths.sql")),
            (
                6,
                include_str!("../../../migrations/0006_session_identity.sql"),
            ),
            (
                7,
                include_str!("../../../migrations/0007_refresh_tokens.sql"),
            ),
        ] {
            let applied: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM folio_migrations WHERE version=?1)",
                    [version],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if !applied {
                let tx = db.transaction().map_err(|e| e.to_string())?;
                tx.execute_batch(sql).map_err(|e| e.to_string())?;
                tx.execute("INSERT INTO folio_migrations VALUES (?1)", [version])
                    .map_err(|e| e.to_string())?;
                tx.commit().map_err(|e| e.to_string())?;
            }
        }
        Ok(Self { vault, db })
    }
    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if !valid_path(id) {
            return Err("Invalid note ID".into());
        }
        let mut path = self.vault.clone();
        for segment in id.split('/') {
            path.push(segment);
            if let Ok(meta) = std::fs::symlink_metadata(&path) {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err("Reparse points are not allowed in the vault".into());
                    }
                }
                if meta.is_symlink() {
                    return Err("Links are not allowed in the vault".into());
                }
            }
        }
        Ok(path)
    }
}
impl Store for LocalStore {
    async fn list(&mut self, cursor: Option<&str>, limit: u32) -> Result<Listing, String> {
        fn walk(
            root: &std::path::Path,
            base: &std::path::Path,
            out: &mut Vec<Entry>,
        ) -> Result<(), String> {
            for entry in std::fs::read_dir(base).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let meta = entry.path().symlink_metadata().map_err(|e| e.to_string())?;
                if meta.is_symlink() {
                    continue;
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        continue;
                    }
                }
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".folio-tmp-")
                {
                    continue;
                }
                if meta.is_dir() {
                    let path = entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push(Entry {
                        path: format!("{path}/.folio-folder"),
                        updated_at: 0,
                    });
                    walk(root, &entry.path(), out)?;
                } else if meta.is_file() {
                    out.push(Entry {
                        path: entry
                            .path()
                            .strip_prefix(root)
                            .map_err(|e| e.to_string())?
                            .to_string_lossy()
                            .replace('\\', "/"),
                        updated_at: meta
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                    });
                }
            }
            Ok(())
        }
        let mut entries = Vec::new();
        walk(&self.vault, &self.vault, &mut entries)?;
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        entries.dedup_by(|a, b| a.path == b.path);
        entries.retain(|e| cursor.is_none_or(|c| e.path.as_str() > c));
        let limit = limit.clamp(1, 1000) as usize;
        let more = entries.len() > limit;
        entries.truncate(limit);
        let cursor = if more {
            entries.last().map(|e| e.path.clone())
        } else {
            None
        };
        Ok(Listing { entries, cursor })
    }

    async fn query(&mut self, statement: Statement) -> Result<Vec<Value>, String> {
        let mut stmt = self.db.prepare(&statement.sql).map_err(|e| e.to_string())?;
        let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let values = sql_values(&statement.params);
        let rows = stmt
            .query_map(rusqlite::params_from_iter(values), |row| {
                let mut result = Map::new();
                for (i, name) in names.iter().enumerate() {
                    let v = match row.get_ref(i)? {
                        ValueRef::Null => Value::Null,
                        ValueRef::Integer(n) => Value::from(n),
                        ValueRef::Real(n) => Value::from(n),
                        ValueRef::Text(s) => Value::String(String::from_utf8_lossy(s).to_string()),
                        ValueRef::Blob(_) => Value::Null,
                    };
                    result.insert(name.clone(), v);
                }
                Ok(Value::Object(result))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    async fn execute(&mut self, s: Statement) -> Result<(), String> {
        self.db
            .execute(&s.sql, rusqlite::params_from_iter(sql_values(&s.params)))
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    async fn batch(&mut self, statements: Vec<Statement>) -> Result<(), String> {
        let tx = self.db.transaction().map_err(|e| e.to_string())?;
        for s in statements {
            tx.execute(&s.sql, rusqlite::params_from_iter(sql_values(&s.params)))
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
    async fn read(&mut self, id: &str) -> Result<Option<Object>, String> {
        match std::fs::read_to_string(self.path(id)?) {
            Ok(markdown) => Ok(Some(Object {
                version: markdown.clone(),
                markdown,
            })),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    async fn write(
        &mut self,
        id: &str,
        markdown: &str,
        version: Option<&str>,
    ) -> Result<(), String> {
        let current = self.read(id).await?;
        if current.as_ref().map(|o| o.version.as_str()) != version {
            return Err("conflict".into());
        }
        let path = self.path(id)?;
        let parent = path.parent().ok_or("Invalid parent")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        self.path(id)?;
        let mut temp = tempfile::Builder::new()
            .prefix(".folio-tmp-")
            .tempfile_in(parent)
            .map_err(|e| e.to_string())?;
        temp.write_all(markdown.as_bytes())
            .map_err(|e| e.to_string())?;
        temp.as_file().sync_all().map_err(|e| e.to_string())?;
        if version.is_none() {
            temp.persist_noclobber(self.path(id)?).map_err(|e| {
                if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                    "conflict".into()
                } else {
                    e.to_string()
                }
            })?;
        } else {
            temp.persist(self.path(id)?).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    async fn delete(&mut self, id: &str) -> Result<(), String> {
        let path = self.path(id)?;
        if id.ends_with("/.folio-folder") {
            match std::fs::remove_file(&path) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.to_string()),
            }
            if let Some(parent) = path.parent() {
                match std::fs::remove_dir(parent) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.to_string()),
                }
            }
            return Ok(());
        }
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
