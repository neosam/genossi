-- Phase 8: RepaymentEntry-Aggregat (Plan 01)
-- Migration filename per Phase-7-Konvention (englisch, alphabetisch nach Datum).
--
-- NOTE (WR-03): FOREIGN KEY clauses below are DOCUMENTARY only.
-- This codebase does not enable `PRAGMA foreign_keys=ON`. The Service layer
-- performs explicit phase-status and member-existence checks before any INSERT
-- (D-11.1 + D-11.2), which is the operative protection. The FK clauses document
-- the intended referential semantics for future operators reading the schema.
--
-- Schema-Begruendungen:
--   - id BLOB PRIMARY KEY: Standard-UUID-Persistenz (CLAUDE.md Entity Structure).
--   - member_id BLOB NOT NULL: Verweis auf member(id); FK dokumentarisch.
--   - phase_id BLOB NOT NULL: Verweis auf repayment_phase(id); FK dokumentarisch.
--   - share_count_to_pay_out INTEGER NOT NULL CHECK(> 0): T-08-01-03-Mitigation;
--     verhindert negative oder Null-Auszahlungsmengen auf Schema-Ebene.
--   - status TEXT NOT NULL DEFAULT 'Open': D-05 Eintragsstatus startet immer in
--     Open; Enum-Varianten {Open, Contacted, PaidOut} alle drei von Anfang an.
--   - created TEXT NOT NULL: ISO8601, analog repayment_phase.
--   - deleted TEXT: Soft-Delete-Slot (ENTR-05).
--   - version BLOB NOT NULL: Optimistic-Locking via UUID.
--   - KEIN UNIQUE-Constraint auf (member_id, phase_id): ENTR-03 erlaubt
--     mehrere Eintraege pro Mitglied+Phase (z.B. Teil-Abtretungen, Korrekturen).
--   - 3 Indizes: phase_id (Listing GET ?phase_id=), (phase_id, status) (Close-
--     Validation pending-Filter D-13), deleted (Soft-Delete-Lookup, partial-
--     index-aequivalent).
--   - FK ON DELETE RESTRICT: Soft-Delete-Konvention; Hard-Delete des Members
--     oder der Phase soll fehlschlagen, solange Eintraege existieren.
--
-- Audit-Konformitaet: RepaymentEntryEntity IST Auditable (Plan 01 Task 2).
-- audited_create!/audited_update!/audited_delete! Macros werden in Plan 03
-- via grep-gate verifiziert (T-08-01-01-Mitigation).

CREATE TABLE IF NOT EXISTS repayment_entry (
    id BLOB PRIMARY KEY NOT NULL,
    member_id BLOB NOT NULL,
    phase_id BLOB NOT NULL,
    share_count_to_pay_out INTEGER NOT NULL CHECK(share_count_to_pay_out > 0),
    status TEXT NOT NULL DEFAULT 'Open',
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    FOREIGN KEY (member_id) REFERENCES member(id) ON DELETE RESTRICT,
    FOREIGN KEY (phase_id) REFERENCES repayment_phase(id) ON DELETE RESTRICT
);

CREATE INDEX IF NOT EXISTS idx_repayment_entry_phase ON repayment_entry(phase_id);
CREATE INDEX IF NOT EXISTS idx_repayment_entry_phase_status ON repayment_entry(phase_id, status);
CREATE INDEX IF NOT EXISTS idx_repayment_entry_deleted ON repayment_entry(deleted);
