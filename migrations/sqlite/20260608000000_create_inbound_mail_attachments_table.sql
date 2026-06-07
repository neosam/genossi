CREATE TABLE IF NOT EXISTS inbound_mail_attachments (
    id BLOB PRIMARY KEY NOT NULL,
    inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
    created TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    relative_path TEXT,
    oversized INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_inbound_mail_attachments_mail ON inbound_mail_attachments(inbound_mail_id);
