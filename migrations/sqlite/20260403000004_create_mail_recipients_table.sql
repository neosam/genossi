CREATE TABLE IF NOT EXISTS mail_recipients (
    id BLOB PRIMARY KEY,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    mail_job_id BLOB NOT NULL REFERENCES mail_jobs(id),
    to_address TEXT NOT NULL,
    member_id BLOB,
    status TEXT NOT NULL,
    error TEXT,
    sent_at TEXT
);
