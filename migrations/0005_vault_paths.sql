ALTER TABLE notes ADD COLUMN path TEXT;
CREATE UNIQUE INDEX notes_path ON notes(path);
