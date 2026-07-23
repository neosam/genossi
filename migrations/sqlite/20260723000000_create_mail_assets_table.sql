-- Phase 27 (IMG-01): stores an inline mail image asset used by the HTML-mail
-- editor and the send-path CID renderer. Bytes are stored INLINE as a SQLite
-- BLOB (`bytes`), NOT on the filesystem — this is the single divergence from
-- the application_documents analog (which uses `relative_path` + DocumentStorage).
-- Not audited (IMG-01): no Auditable impl, no hash-chain overhead. No parent
-- table means no foreign key. No single-slot invariant means no partial unique
-- index.
CREATE TABLE IF NOT EXISTS mail_assets (
    id          BLOB PRIMARY KEY NOT NULL,
    filename    TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    bytes       BLOB NOT NULL,
    uploaded_by TEXT NOT NULL,
    created     TEXT NOT NULL,
    deleted     TEXT,
    version     BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mail_assets_deleted
    ON mail_assets(deleted);
