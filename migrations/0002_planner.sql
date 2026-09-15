ALTER TABLE notes ADD COLUMN folder TEXT NOT NULL DEFAULT 'Personal';
ALTER TABLE notes ADD COLUMN searchable TEXT NOT NULL DEFAULT '';
ALTER TABLE notes ADD COLUMN revision TEXT NOT NULL DEFAULT '';
ALTER TABLE notes ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
CREATE INDEX notes_folder_updated ON notes(folder, archived, updated_at DESC, id);
CREATE TABLE folders (path TEXT PRIMARY KEY NOT NULL);
INSERT INTO folders(path) VALUES ('Daily'),('Projects'),('Personal'),('Archive');
CREATE TABLE tasks (
 id TEXT PRIMARY KEY NOT NULL, title TEXT NOT NULL, completed INTEGER NOT NULL DEFAULT 0,
 due_date TEXT, due_time TEXT, source_note_id TEXT, source_line INTEGER,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX tasks_due ON tasks(completed,due_date,id);
CREATE INDEX tasks_source ON tasks(source_note_id);
CREATE TABLE goals (
 id TEXT PRIMARY KEY NOT NULL, title TEXT NOT NULL, description TEXT NOT NULL DEFAULT '',
 position INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'active',
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE account (id INTEGER PRIMARY KEY CHECK(id=1), username TEXT NOT NULL, salt TEXT NOT NULL, password_hash TEXT NOT NULL);
CREATE TABLE sessions (id TEXT PRIMARY KEY NOT NULL, csrf TEXT NOT NULL, expires_at INTEGER NOT NULL);
CREATE INDEX sessions_expiry ON sessions(expires_at);
CREATE TABLE login_attempts (id TEXT PRIMARY KEY NOT NULL, count INTEGER NOT NULL, reset_at INTEGER NOT NULL);
