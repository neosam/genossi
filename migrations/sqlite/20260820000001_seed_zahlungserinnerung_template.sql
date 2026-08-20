-- Phase 30 (APTPL-03, D-14): Seed der deutschen Antragsteller-Vorlage
-- „Zahlungserinnerung" mit fester UUID 00000000-0000-0000-0000-000000000003
-- (setzt die …0001/…0002-Serie der Anrede-Seeds fort) und template_type
-- 'application'.
--
-- Formeller Sie-Ton. Der Body verwendet AUSSCHLIESSLICH Schlüssel, die der
-- Antragsteller-Kontext (application_to_template_context, Plan 30-03) liefert:
-- salutation, title, first_name, last_name, shares, open_amount, bank_iban,
-- bank_name, bank_bic (optional → guarded), genossenschaft_name. Es werden
-- keinerlei mitgliederspezifische Kontextvariablen verwendet.
--
-- Der Render-Sicherheits-Nachweis (dieser Body validiert strict gegen den
-- Antragsteller-Kontext) kommt in Plan 30-03 via validate_application_template.
-- Text-only (kein body_html) — NULL-Legacy ist zulässig.

INSERT OR IGNORE INTO mail_templates (id, created, version, name, subject, body, template_type)
VALUES (
    X'00000000000000000000000000000003',
    datetime('now'),
    X'00000000000000000000000000000033',
    'Zahlungserinnerung',
    'Zahlungserinnerung — Ihre Beitrittserklärung',
    'Sehr geehrte{% if salutation == "Herr" %}r Herr{% elif salutation == "Frau" %} Frau{% else %}s Mitglied{% endif %}{% if title %} {{ title }}{% endif %} {{ last_name }},


vielen Dank für Ihre Beitrittserklärung zu unserer Genossenschaft. Für Ihre Zeichnung von {{ shares }} Geschäftsanteilen ist derzeit noch ein offener Betrag von {{ open_amount }} zu begleichen.

Wir bitten Sie, diesen Betrag auf folgendes Konto zu überweisen:

Kontoinhaber: {{ genossenschaft_name }}
IBAN: {{ bank_iban }}
Bank: {{ bank_name }}
{% if bank_bic %}BIC: {{ bank_bic }}
{% endif %}Verwendungszweck: Beitritt {{ first_name }} {{ last_name }}

Sollten Sie die Zahlung bereits veranlasst haben, betrachten Sie dieses Schreiben bitte als gegenstandslos.

Mit freundlichen Grüßen,
{{ genossenschaft_name }}',
    'application'
);
