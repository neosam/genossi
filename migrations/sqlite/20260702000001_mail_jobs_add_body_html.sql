-- Phase 23 (HTML-03, D-07): mail_jobs.body_html — optional HTML body per job.
--
-- Mirrors mail_templates.body_html on the job side so ad-hoc jobs (and the
-- rendered snapshot of a template-based job) can carry an HTML body. The
-- worker uses this column to decide whether to emit a multipart/alternative
-- message (D-09).
--
-- NULL-legacy semantics: existing jobs read back as body_html=NULL, which
-- means "text-only mail" (unchanged behavior). Author HTML is sanitized via
-- ammonia at create_job before it lands here.
--
-- Forward-only. SQLite < 3.35 cannot drop columns; no down migration is provided.

ALTER TABLE mail_jobs ADD COLUMN body_html TEXT NULL;
