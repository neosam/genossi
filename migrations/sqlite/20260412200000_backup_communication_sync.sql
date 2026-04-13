CREATE TABLE IF NOT EXISTS backup_communication_sync (
    mail_type TEXT NOT NULL,
    mail_id BLOB NOT NULL,
    synced_at TEXT NOT NULL,
    PRIMARY KEY (mail_type, mail_id)
);
