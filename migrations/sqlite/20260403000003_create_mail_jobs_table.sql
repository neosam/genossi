CREATE TABLE IF NOT EXISTS mail_jobs (
    id BLOB PRIMARY KEY,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    status TEXT NOT NULL,
    total_count INTEGER NOT NULL,
    sent_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0
);
