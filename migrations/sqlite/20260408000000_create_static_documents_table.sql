CREATE TABLE IF NOT EXISTS static_documents (
    id BLOB PRIMARY KEY NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    name TEXT NOT NULL,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_static_documents_deleted ON static_documents(deleted);
CREATE INDEX IF NOT EXISTS idx_static_documents_name ON static_documents(name);
