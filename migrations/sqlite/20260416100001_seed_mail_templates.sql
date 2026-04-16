-- Seed predefined mail templates with fixed UUIDs
-- Formal: 00000000-0000-0000-0000-000000000001
-- Informal: 00000000-0000-0000-0000-000000000002

INSERT OR IGNORE INTO mail_templates (id, created, version, name, subject, body)
VALUES (
    X'00000000000000000000000000000001',
    datetime('now'),
    X'00000000000000000000000000000011',
    'Formelle Anrede',
    '',
    'Sehr geehrte{% if salutation == "Herr" %}r Herr{% elif salutation == "Frau" %} Frau{% else %}s Mitglied{% endif %}{% if title %} {{ title }}{% endif %} {{ last_name }},



Mit freundlichen Grüßen,'
);

INSERT OR IGNORE INTO mail_templates (id, created, version, name, subject, body)
VALUES (
    X'00000000000000000000000000000002',
    datetime('now'),
    X'00000000000000000000000000000022',
    'Informelle Anrede',
    '',
    '{% if salutation == "Herr" %}Lieber{% elif salutation == "Frau" %}Liebe{% else %}Hallo{% endif %}{% if title %} {{ title }}{% endif %} {{ first_name }},



Viele Grüße,'
);
