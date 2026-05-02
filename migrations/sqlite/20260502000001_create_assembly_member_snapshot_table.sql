-- NOTE (WR-03): FOREIGN KEY clauses below are DOCUMENTARY only.
-- This codebase does not enable `PRAGMA foreign_keys=ON` on the SqlitePool
-- (no `after_connect` hook in genossi_bin / genossi_dao_impl_sqlite). Per
-- SQLite default, the FK declarations are parsed but not enforced at
-- runtime. The clauses are kept so the schema clearly expresses the
-- intended referential relationship; existence of `assembly_id` and
-- `member_id` rows is enforced at the service layer (open_assembly only
-- writes snapshots that originate from `member_dao.all()` filtered against
-- the just-created `assembly` row in the same transaction).
-- Phase 2/3 may revisit this and enable `PRAGMA foreign_keys=ON`
-- workspace-wide; that is intentionally out of scope here.
CREATE TABLE IF NOT EXISTS assembly_member_snapshot (
    assembly_id BLOB NOT NULL,
    member_id BLOB NOT NULL,
    captured_at TEXT NOT NULL,
    PRIMARY KEY (assembly_id, member_id),
    FOREIGN KEY (assembly_id) REFERENCES assembly(id),
    FOREIGN KEY (member_id) REFERENCES member(id)
);

CREATE INDEX IF NOT EXISTS idx_assembly_member_snapshot_assembly_id
    ON assembly_member_snapshot(assembly_id);
