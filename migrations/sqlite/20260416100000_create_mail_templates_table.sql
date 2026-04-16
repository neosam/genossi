CREATE TABLE IF NOT EXISTS mail_templates (
    id BLOB PRIMARY KEY NOT NULL,
    created TEXT NOT NULL DEFAULT (datetime('now')),
    deleted TEXT,
    version BLOB NOT NULL,
    name TEXT NOT NULL,
    subject TEXT NOT NULL,
    body TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mail_templates_name_not_deleted
ON mail_templates(name) WHERE deleted IS NULL;
