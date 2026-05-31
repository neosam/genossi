-- ADR Phase 10 (D-12 / D-03): mail_jobs gets two optional refs so the worker can
-- (a) record which template was used (template_id -> MemberDocument.template_id)
-- and (b) merge job-wide repayment context into per-recipient render.
--
-- FK clauses are documentary only (project does not enable PRAGMA foreign_keys=ON;
-- consistent with repayment_entry migration line 4-7). ON DELETE SET NULL semantics
-- enforced at service layer: deleting a template or phase must not break audit.
--
-- Backward-compat: existing mail_jobs rows have NULL in both columns; legacy code
-- paths (test-mail, single-send, non-repayment bulk-send) keep NULL.
--
-- No down-migration: SQLite < 3.35 cannot remove columns.

ALTER TABLE mail_jobs ADD COLUMN template_id BLOB NULL;
ALTER TABLE mail_jobs ADD COLUMN repayment_phase_id BLOB NULL;
