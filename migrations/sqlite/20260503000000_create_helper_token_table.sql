-- Phase 2: Helfer-Token-Aggregat
-- Migration filename per D-27 (englisch, konsistent mit Phase-1-Konvention)
--
-- Schema-Begründungen:
--   - assembly_id FK ON DELETE RESTRICT: Assembly-Hard-Delete soll fehlschlagen, solange Tokens existieren (Soft-Delete ist Standard).
--   - session_id FK ON DELETE SET NULL: Cleanup-Jobs dürfen alte Sessions entfernen; Token-Listing zeigt weiterhin "eingelöst" (used_at IS NOT NULL, session_id IS NULL).
--   - UNIQUE INDEX on token_hash: race-hardens den atomaren Redeem-WHERE-Clause; brute-force-resistance ohne Salt (50-bit Crockford-Entropie).
--   - INDEX assembly_id: D-21 Listing-Query.
--   - INDEX deleted: Soft-Delete-Filter (D-05).

CREATE TABLE IF NOT EXISTS helper_token (
    id BLOB PRIMARY KEY NOT NULL,
    assembly_id BLOB NOT NULL,
    memo TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    created TEXT NOT NULL,
    used_at TEXT,
    session_id TEXT,
    revoked_at TEXT,
    deleted TEXT,
    version BLOB NOT NULL,
    FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT,
    FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_helper_token_token_hash ON helper_token(token_hash);
CREATE INDEX IF NOT EXISTS idx_helper_token_assembly ON helper_token(assembly_id);
CREATE INDEX IF NOT EXISTS idx_helper_token_deleted ON helper_token(deleted);
