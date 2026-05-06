-- ADR 2026-05-06: persist helper_token plaintext code so the Vorstand can
-- re-display QR + manual-code at any time (Phase 2 D-11 / D-21 trade-off).
--
-- Single-tenant self-hosted scope; encryption-at-rest is handled at the
-- filesystem / Restic-backup layer, not at the app layer. token_hash redeem
-- semantics are unchanged (`UPDATE ... WHERE token_hash = ? AND used_at IS NULL
-- RETURNING ...`) — code is read-only state for the Vorstand UI.
--
-- Audit-Log MUST exclude this column (parallel rationale to D-06: avoid a
-- second persistent code store in the audit hash chain). See
-- `genossi_dao::helper_token::HelperTokenEntity::audit_fields()`.
--
-- Pre-update tokens have NULL code → frontend renders a "revoke + recreate"
-- hint instead of the QR/code button.
--
-- No down-migration: SQLite < 3.35 has no `DROP COLUMN`; the project ships
-- only forward migrations (see migrations/sqlite/*).

ALTER TABLE helper_token ADD COLUMN code TEXT NULL;
