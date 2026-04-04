CREATE TABLE IF NOT EXISTS mail_recipient_attachments (
    recipient_id BLOB NOT NULL REFERENCES mail_recipients(id),
    document_id BLOB NOT NULL REFERENCES member_document(id),
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    PRIMARY KEY (recipient_id, document_id)
);
