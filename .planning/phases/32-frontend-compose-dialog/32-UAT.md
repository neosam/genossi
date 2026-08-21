---
status: passed
phase: 32-frontend-compose-dialog
source: [32-VERIFICATION.md]
started: 2026-08-21T00:00:00Z
updated: 2026-08-21T07:30:00Z
---

## Current Test

number: —
name: Abgeschlossen (User-Abnahme 2026-08-21)
expected: —
awaiting: —

## Tests

### 1. Vorbefüllung + Template-Filter (Compose-Seite)
expected: application_detail → „✉ E-Mail senden" (bei vorhandener Adresse) öffnet die Compose-Seite mit der Vorlage „Zahlungserinnerung" vorausgewählt und Betreff/Body befüllt; das TemplateSelector-Dropdown zeigt nur Antragsteller-Vorlagen (keine Mitglieder-Vorlagen).
result: pass — Live im Browser bestätigt (dev): Betreff + Body mit Zahlungserinnerung vorbefüllt, Navigation über den Button funktioniert. Hinweis (kosmetisch, nicht blockierend): das Vorlage-Dropdown zeigt die initiale Auswahl nicht als selektierten Eintrag an — als Beobachtung notiert, kein Funktionsdefekt.

### 2. Debounced Live-Vorschau (~400ms, kein Flackern)
expected: Vorschau aktualisiert sich verzögert mit aufgelösten Platzhaltern; letzte Vorschau bleibt während Pending sichtbar.
result: pass — Live bestätigt: Vorschau rendert mit real aufgelösten Platzhaltern (Anrede „Sehr geehrter Herr Testo", 1 Anteil, offener Betrag 250,00 €, Bankdaten, Verwendungszweck). Exaktes Debounce-Timing/Verwerfen veralteter Läufe nicht separat gestresst → wird auf der Integrationsumgebung mitgeprüft.

### 3. Senden-Button disabled während laufendem Request (kein Doppelversand)
expected: Button disabled solange Request läuft, Label „Wird gesendet…", kein zweiter Request.
result: skipped — bewusst auf Integrationsumgebung verschoben (User-Entscheidung bei Abnahme; Release-Build folgt).

### 4. Post-Send: Erfolgs-Toast + Rücksprung zur Antragsliste
expected: Erfolgs-Toast, danach Navigation zurück.
result: skipped — bewusst auf Integrationsumgebung verschoben (User-Entscheidung bei Abnahme).

### 5. No-Email-Guard + Navigation (application_detail Button)
expected: Ohne Adresse disabled + Hinweistext; mit Adresse Navigation zur Compose-Route.
result: skipped — Navigations-Teil (mit Adresse) implizit live bestätigt (Test 1); der Ohne-Adresse-Fall wird auf der Integrationsumgebung geprüft.

### 6. Timeline-Klick → Body-Panel (echter Body) + Long-Text-Backstop
expected: Panel zeigt echten gespeicherten Body; langer Inhalt bleibt im Scroll-Container.
result: skipped — bewusst auf Integrationsumgebung verschoben (User-Entscheidung bei Abnahme).

## Während des UAT gefundene und behobene Probleme

1. **Umgebung (kein Phase-32-Bug):** dx 0.7.9 aus Flake-Input-Bump `17d0d4d` serviert `/assets/config.json` als SPA-Fallback → Config-Load crasht. Fix: dioxus-cli auf 0.6.3 gepinnt via altem nixpkgs-Input (`8949ef1`).
2. **Dev-DB-Konfiguration (kein Bug):** `share_value_cents`, `genossenschaft_name`, `bank_iban`, `bank_name`, `bank_bic` fehlten in der frischen Dev-DB → per API mit Dummy-Werten gesetzt (Muster eG etc.). Für Integration/Prod über Verwaltung setzen.
3. **Echter Bug (behoben + Regression-Test):** `plain_to_html` escapte `"`/`'` → `&quot;` in `body_html` zerbrach Jinja-String-Literale (`{% if salutation == "Herr" %}`) beim Server-Render — erste Vorlage mit Quotes in Bedingungen. Fix `97196fb`, Test `jinja_string_literals_survive`.

## Summary

total: 6
passed: 2
issues: 0
pending: 0
skipped: 4
blocked: 0

**Abnahme:** User hat das Feature am 2026-08-21 im Browser abgenommen („Ich finde das Feature super"). Die 4 übersprungenen Checks werden auf der Integrationsumgebung nach dem Release-Build nachgeholt.

## Gaps

Keine funktionalen Gaps. Offene Beobachtung (kosmetisch): TemplateSelector zeigt initiale Auswahl nicht im Dropdown an (siehe Test 1).
