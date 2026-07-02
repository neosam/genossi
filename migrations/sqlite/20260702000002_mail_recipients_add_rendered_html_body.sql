-- Phase 23 (HTML-03, D-08): mail_recipients.rendered_html_body — per-recipient
-- rendered HTML body persisted by the worker at send time (parallel to
-- rendered_subject / rendered_body from Quick 260614-9zf).
--
-- "Wir müssen aufbewahren, was verschickt wurde" — the per-recipient rendered
-- HTML is kept byte-identical to what the recipient received, not re-derived
-- on the fly.
--
-- NULL-legacy semantics: legacy recipients (pre-migration) read back as
-- rendered_html_body=NULL, matching the text-only contract. The worker only
-- writes Some(_) when the corresponding job's body_html is Some(_).
--
-- Forward-only. SQLite < 3.35 cannot drop columns; no down migration is provided.

ALTER TABLE mail_recipients ADD COLUMN rendered_html_body TEXT NULL;
