-- Phase 25 (APDOC-01..05): stores the original antrag file uploaded to an
-- Application. Semantics per CONTEXT.md decision #5 (schema minimum) and
-- decision #1 (single-slot). The partial unique index below enforces at
-- most one active row per application at the storage boundary
-- (belt-and-suspenders around the Service-layer find + branch).
-- Not audited (roadmap audit hint + CONTEXT.md decision #5): no Auditable
-- impl, no hash-chain overhead.
CREATE TABLE IF NOT EXISTS application_documents (
    id             BLOB PRIMARY KEY NOT NULL,
    application_id BLOB NOT NULL,
    file_name      TEXT NOT NULL,
    mime_type      TEXT NOT NULL,
    relative_path  TEXT NOT NULL,
    size           INTEGER NOT NULL,
    created        TEXT NOT NULL,
    deleted        TEXT,
    version        BLOB NOT NULL,
    FOREIGN KEY (application_id) REFERENCES application(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_application_documents_one_active
    ON application_documents(application_id) WHERE deleted IS NULL;

CREATE INDEX IF NOT EXISTS idx_application_documents_deleted
    ON application_documents(deleted);
