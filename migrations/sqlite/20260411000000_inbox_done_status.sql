-- Replace single `status` column with independent boolean flags:
--   replied, done, archived
-- SQLite does not support DROP COLUMN in older versions, so we recreate the table.

-- 1. Create new table without `status`, with boolean flags instead
CREATE TABLE inbound_mails_new (
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
    message_id TEXT,
    replied INTEGER NOT NULL DEFAULT 0,
    done INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    assigned_member_id BLOB,
    UNIQUE (uid_validity, imap_uid)
);

-- 2. Migrate data from old table to new, mapping status → boolean flags
INSERT INTO inbound_mails_new (
    id, created, version, uid_validity, imap_uid, from_address, subject,
    received_at, body_text, has_attachments, has_html_body, raw_html_body,
    in_reply_to, message_id, replied, done, archived, assigned_member_id
)
SELECT
    id, created, version, uid_validity, imap_uid, from_address, subject,
    received_at, body_text, has_attachments, has_html_body, raw_html_body,
    in_reply_to, message_id,
    CASE WHEN status = 'replied' THEN 1 ELSE 0 END,
    CASE WHEN status = 'ignored' THEN 1 ELSE 0 END,
    CASE WHEN status = 'archived' THEN 1 ELSE 0 END,
    assigned_member_id
FROM inbound_mails;

-- 3. Drop old table and rename
DROP TABLE inbound_mails;
ALTER TABLE inbound_mails_new RENAME TO inbound_mails;

-- 4. Recreate indexes
DROP INDEX IF EXISTS idx_inbound_mails_status;
CREATE INDEX idx_inbound_mails_done ON inbound_mails(done);
CREATE INDEX idx_inbound_mails_received_at ON inbound_mails(received_at);
