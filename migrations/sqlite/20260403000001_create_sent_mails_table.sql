CREATE TABLE IF NOT EXISTS sent_mails (
    id BLOB PRIMARY KEY,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    to_address TEXT NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL,
    error TEXT,
    sent_at TEXT
);
