-- Quick 260603-cz6: mail_jobs.attach_repayment_letter — opt-in bool flag.
--
-- When true AND mail_jobs.repayment_phase_id is set, the worker resolves the
-- per-recipient RepaymentLetter MemberDocument (filtered by document_type and
-- the description fingerprint "Anschreiben Auszahlung GJ {fiscal_year}",
-- consistent with Phase 13 D-LETT-04) and attaches it in-memory before send,
-- analog to the existing D-03 repayment-context-merge pattern.
--
-- Stored as INTEGER NOT NULL DEFAULT 0 — Phase-10-Pattern for SQLite booleans.
-- Backward-compat: existing rows get 0 (false), matching prior behavior.
--
-- REST-layer enforces: attach_repayment_letter=true requires repayment_phase_id
-- to be set (400 BadRequest otherwise). The DB column itself accepts any
-- combination; no CHECK constraint to keep migration trivial.
--
-- No down-migration: SQLite < 3.35 cannot remove columns.

ALTER TABLE mail_jobs ADD COLUMN attach_repayment_letter INTEGER NOT NULL DEFAULT 0;
