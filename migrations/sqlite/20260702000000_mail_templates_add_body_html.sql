-- Phase 23 (HTML-03, D-06): mail_templates.body_html — optional HTML body per template.
--
-- Adds an OPTIONAL HTML body next to the existing plain-text `body`. Text stays
-- authoritative; when body_html IS NOT NULL the send path emits a
-- multipart/alternative message (text first, HTML second) per the Phase-22
-- build_message helper.
--
-- NULL-legacy semantics: existing templates read back as body_html=NULL, which
-- means "text-only mail" (unchanged behavior). Author HTML is sanitized via
-- ammonia at all entry points (create/update), not at DB write.
--
-- Forward-only. SQLite < 3.35 cannot drop columns; no down migration is provided.

ALTER TABLE mail_templates ADD COLUMN body_html TEXT NULL;
