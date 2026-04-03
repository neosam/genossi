# SMTP Config UI

## Problem

Die aktuelle Config-Seite zeigt eine generische Key-Value-Tabelle. Um E-Mail (SMTP) einzurichten, muss man die internen Schlüsselnamen kennen (`smtp_host`, `smtp_port`, `smtp_user`, `smtp_pass`, `smtp_from`, `smtp_tls`), die richtigen `value_type`-Werte wählen und die Einträge einzeln anlegen. Das ist für Nicht-Entwickler nicht zumutbar.

## Solution

Eine dedizierte SMTP-Einstellungen-Sektion auf der Config-Seite, die ein benutzerfreundliches Formular mit beschrifteten Feldern anzeigt. Dazu eine "Test-E-Mail senden"-Funktion, um die Konfiguration direkt zu verifizieren.

## Scope

- **Frontend**: Neue SMTP-Formular-Komponente auf der Config-Seite (oberhalb der generischen Tabelle)
- **Backend**: Neuer `POST /api/mail/test` Endpoint, der eine Test-E-Mail an eine angegebene Adresse sendet
- **i18n**: Neue Übersetzungsschlüssel für SMTP-Formularfelder
- Die generische Config-Tabelle bleibt erhalten (unterhalb, evtl. einklappbar)
- Das bestehende Key-Value-Backend bleibt unverändert — das Frontend mapped die Formularfelder auf die bekannten Config-Keys

## Non-goals

- Kein Umbau des Config-Store-Backends (bleibt Key-Value)
- Keine zusätzlichen Config-Gruppen (nur SMTP vorerst)
- Keine Verschlüsselung von Secrets in der Datenbank
