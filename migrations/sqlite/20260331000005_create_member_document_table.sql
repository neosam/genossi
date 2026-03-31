CREATE TABLE IF NOT EXISTS member_document (
    id BLOB PRIMARY KEY NOT NULL,
    member_id BLOB NOT NULL,
    document_type TEXT NOT NULL,
    description TEXT,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_member_document_member_id ON member_document(member_id);
CREATE INDEX IF NOT EXISTS idx_member_document_deleted ON member_document(deleted);
