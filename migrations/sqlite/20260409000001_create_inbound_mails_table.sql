CREATE TABLE inbound_mails (
    id BLOB PRIMARY KEY NOT NULL,
    created TEXT NOT NULL,
    version BLOB NOT NULL,
    uid_validity INTEGER NOT NULL,
    imap_uid INTEGER NOT NULL,
    from_address TEXT NOT NULL,
    subject TEXT NOT NULL,
    received_at TEXT NOT NULL,
    body_text TEXT NOT NULL,
    has_attachments INTEGER NOT NULL,
    has_html_body INTEGER NOT NULL,
    raw_html_body TEXT,
    in_reply_to TEXT,
    status TEXT NOT NULL,
    assigned_member_id BLOB,
    UNIQUE (uid_validity, imap_uid)
);

CREATE INDEX idx_inbound_mails_status ON inbound_mails(status);
CREATE INDEX idx_inbound_mails_received_at ON inbound_mails(received_at);
