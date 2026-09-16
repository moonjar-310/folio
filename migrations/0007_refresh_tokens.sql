-- Existing opaque sessions and Cloudflare audit rows cannot be refreshed.
DELETE FROM sessions;
ALTER TABLE sessions ADD COLUMN username TEXT NOT NULL DEFAULT '';
