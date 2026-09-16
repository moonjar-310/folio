ALTER TABLE sessions ADD COLUMN provider TEXT NOT NULL DEFAULT 'password';
ALTER TABLE sessions ADD COLUMN subject TEXT NOT NULL DEFAULT 'local-owner';
