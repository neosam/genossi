-- Quick 260614-9zf: Persist the per-recipient rendered subject + body.
-- The worker already renders subject/body per recipient (template interpolation)
-- but discarded the result after sending. We now store it on the recipient so the
-- Vorstand can later see exactly what an individual member received (audit/support).
--
-- Both columns are nullable (no NOT NULL, no DEFAULT) on purpose: legacy rows and
-- recipients that have not been sent yet (or failed before rendering) have no
-- rendered content. No down-migration: SQLite < 3.35 cannot drop columns
-- (same convention as 20260603100000_mail_job_attach_repayment_letter.sql).
ALTER TABLE mail_recipients ADD COLUMN rendered_subject TEXT;
ALTER TABLE mail_recipients ADD COLUMN rendered_body TEXT;
