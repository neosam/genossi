CREATE TABLE IF NOT EXISTS backup_document_sync (
    relative_path TEXT PRIMARY KEY NOT NULL,
    content_hash TEXT NOT NULL,
    last_uploaded TEXT NOT NULL
);
