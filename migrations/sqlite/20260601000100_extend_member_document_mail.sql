-- ADR Phase 10 (D-07 / D-09): member_document gets three optional fields so a
-- repayment-mail send can be persisted as an audited MemberDocument
-- (status='sent'|'failed', linked to MailRecipient + MailTemplate).
--
-- All three columns are NULL-able: existing rows (JoinDeclaration etc.) keep
-- NULL values, backward-compat is preserved (D-08).
--
-- FK clauses are documentary only (project does not enable PRAGMA foreign_keys=ON).
-- ON DELETE SET NULL semantics enforced at service layer; deleting a template or
-- recipient must not break audit-hashchain integrity (template_id/mail_recipient_id
-- already audited via Auditable::audit_fields()).
--
-- Auditable::audit_fields() is extended in genossi_dao/src/member_document.rs to
-- include these three fields APPENDED AT END (FROZEN-order, Phase-7-Lektion):
-- new audits get 9 field entries; existing audits remain valid (their entity
-- snapshots simply did not include these fields).
--
-- No down-migration: SQLite < 3.35 cannot remove columns; project ships forward-only.

ALTER TABLE member_document ADD COLUMN template_id BLOB NULL;
ALTER TABLE member_document ADD COLUMN mail_recipient_id BLOB NULL;
ALTER TABLE member_document ADD COLUMN status TEXT NULL;
