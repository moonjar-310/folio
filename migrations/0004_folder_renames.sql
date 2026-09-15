-- Resumable canonical-folder changes; one hierarchy operation at a time.
CREATE TABLE folder_renames (
 id INTEGER PRIMARY KEY CHECK(id=1),
 source TEXT NOT NULL,
 target TEXT NOT NULL
);
