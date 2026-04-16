CREATE INDEX IF NOT EXISTS idx_audit_log_entity_type_timestamp
    ON audit_log(entity_type, timestamp);
