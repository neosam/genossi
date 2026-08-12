-- Phase 29 (APHIST-01): mail_recipients.application_id — nullable Linkage-Spalte,
-- spiegelbildlich zur bestehenden member_id-Spalte. Erlaubt es, eine an einen
-- Antragsteller (Application) gesendete Mail zu persistieren, OHNE den
-- member_id-Namespace zu vergiften (eine Application-UUID landet nie in member_id).
--
-- NULL-Legacy-Semantik: bestehende Zeilen (vor dieser Migration) lesen
-- application_id=NULL byte-identisch zurueck — genau wie member_id nullable ist
-- und ohne DEFAULT/NOT NULL angelegt wurde.
--
-- Forward-only. SQLite < 3.35 kann Spalten nicht droppen; keine Down-Migration.

ALTER TABLE mail_recipients ADD COLUMN application_id BLOB;
CREATE INDEX IF NOT EXISTS idx_mail_recipients_application_id ON mail_recipients(application_id);
