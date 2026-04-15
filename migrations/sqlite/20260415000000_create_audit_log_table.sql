CREATE TABLE audit_log (
    id BLOB NOT NULL PRIMARY KEY,
    timestamp TEXT NOT NULL,
    user_id TEXT NOT NULL,
    process TEXT NOT NULL,
    transaction_id BLOB NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    action TEXT NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    prev_hash TEXT NOT NULL,
    entry_hash TEXT NOT NULL
);

CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_transaction ON audit_log(transaction_id);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_user ON audit_log(user_id);
