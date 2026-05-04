-- Phase 3: Attendance-Aggregat (Plan 01)
-- Migration filename per D-10 (englisch, konsistent mit Phase-1/2-Konvention).
--
-- NOTE (WR-03): FOREIGN KEY clauses below are DOCUMENTARY only.
-- This codebase does not enable `PRAGMA foreign_keys=ON`. The Service layer
-- (Plan 05: AttendanceServiceImpl::mark_present/mark_absent) performs an
-- explicit snapshot-membership check (D-27) before any UPSERT, which is the
-- operative protection. The FK clauses document the intended referential
-- semantics for future operators reading the schema.
--
-- Schema-Begruendungen:
--   - Composite-PK (assembly_id, member_id): leichtgewichtige Join-Tabelle (D-01, D-02).
--     Automatisch UNIQUE -> ON CONFLICT(assembly_id, member_id)-Targeting fuer UPSERT (D-04, D-05).
--   - KEIN id/version-Feld: Anwesenheit hat keine eigene Identitaet jenseits des
--     (assembly_id, member_id)-Pairs; Idempotenz loest Concurrency (D-01).
--   - marked_at TEXT NOT NULL: letzter Toggle-On-Zeitpunkt, wird beim UPSERT ueberschrieben.
--   - marked_by_user_id TEXT NOT NULL: synthetisch "helper:<token_id>" ODER OIDC-User-ID,
--     wird beim UPSERT ueberschrieben.
--   - deleted TEXT (nullable): Soft-Delete-Slot fuer Toggle-Off (D-03, D-06, D-09).
--     Toggle-On (UPSERT) setzt deleted=NULL; Toggle-Off (UPDATE) setzt deleted=now().
--   - KEIN unmarked_by-Feld: minimaler Schema-Footprint (D-07).
--   - FK ON DELETE RESTRICT: Soft-Delete-Konvention; Hard-Delete der Assembly oder des
--     Members soll fehlschlagen, solange Anwesenheits-Eintraege existieren.
--   - Optional partial index auf assembly_id WHERE deleted IS NULL: beschleunigt
--     count_present_by_assembly + LEFT JOIN auf is_present.
--
-- Audit-Konformitaet: AttendanceEntity ist NICHT Auditable (D-08, ATTN-05).
-- Keine Audit-Macros werden auf attendance ausgefuehrt.

CREATE TABLE IF NOT EXISTS attendance (
    assembly_id BLOB NOT NULL,
    member_id BLOB NOT NULL,
    marked_at TEXT NOT NULL,
    marked_by_user_id TEXT NOT NULL,
    deleted TEXT,
    PRIMARY KEY (assembly_id, member_id),
    FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT,
    FOREIGN KEY (member_id) REFERENCES member(id) ON DELETE RESTRICT
);

-- Composite-PK ist automatisch UNIQUE -> ON CONFLICT(assembly_id, member_id) target works (D-04, D-05).
-- Optional partial index -- beschleunigt count_present_by_assembly + LEFT JOIN auf is_present.
CREATE INDEX IF NOT EXISTS idx_attendance_assembly_present
    ON attendance(assembly_id) WHERE deleted IS NULL;
