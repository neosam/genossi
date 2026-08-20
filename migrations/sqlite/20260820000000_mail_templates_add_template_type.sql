-- Phase 30 (APTPL-01, D-01/D-02): mail_templates.template_type — pool discriminator.
--
-- Trennt den Antragsteller-Vorlagen-Pool ('application') vom Mitglieder-Pool
-- ('member') auf Datenebene, damit eine Mitglieder-Massenmail nie eine reine
-- Antrags-Vorlage rendert. Der Wert ist ein einfacher Typ-Diskriminator und
-- ist nach dem Anlegen unveränderlich (Pitfall 3, Option a — UPDATE schreibt
-- template_type nicht).
--
-- NOT NULL DEFAULT 'member': jede Alt-Zeile (die beiden Legacy-Seeds sowie alle
-- bestehenden Vorstands-Vorlagen) liest verlustfrei 'member' zurück
-- (NULL/DEFAULT-Legacy-Roundtrip, D-02). Der Mitglieder-Selektor-Filter
-- (template_type = 'member') folgt in Phase 32 (D-03).
--
-- Forward-only. SQLite < 3.35 kann Spalten nicht droppen; keine Down-Migration.

ALTER TABLE mail_templates ADD COLUMN template_type TEXT NOT NULL DEFAULT 'member';
