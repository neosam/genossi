CREATE TABLE audit_timestamp (
    id BLOB NOT NULL PRIMARY KEY,
    timestamp TEXT NOT NULL,
    audit_hash TEXT NOT NULL,
    audit_entry_count INTEGER NOT NULL,
    tsr_token BLOB,
    webdav_path TEXT,
    status TEXT NOT NULL
);

CREATE INDEX idx_audit_timestamp_timestamp ON audit_timestamp(timestamp);
CREATE INDEX idx_audit_timestamp_status ON audit_timestamp(status);
