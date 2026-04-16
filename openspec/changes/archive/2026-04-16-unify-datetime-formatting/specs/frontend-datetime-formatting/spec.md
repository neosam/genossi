## ADDED Requirements

### Requirement: i18n.format_datetime für Anzeige mit Stunden und Minuten

Die `I18n`-Struktur SHALL eine Methode `format_datetime` bereitstellen, die einen ISO8601-String entgegennimmt und einen lokalisierten String mit Datum und Uhrzeit (Stunden und Minuten, ohne Sekunden) zurückgibt. Kann der String nicht geparst werden, wird er unverändert zurückgegeben.

#### Scenario: Deutsche Locale

- **WHEN** die UI-Sprache auf Deutsch eingestellt ist
- **AND** der Zeitstempel `2026-04-16T16:03:34.512345678Z` formatiert wird
- **THEN** das Ergebnis lautet `16.04.2026 16:03` (oder die lokal übliche Form mit Stunden/Minuten)

#### Scenario: Englische Locale

- **WHEN** die UI-Sprache auf Englisch eingestellt ist
- **AND** derselbe Zeitstempel formatiert wird
- **THEN** das Ergebnis nutzt das englische Locale-Format mit Stunden und Minuten, ohne Sekunden

### Requirement: i18n.format_datetime_long für Sekundengenauigkeit

Für Anzeigen, in denen Sekundengenauigkeit relevant ist (z. B. Audit-Log, Timestamp-Verifikation), SHALL eine Methode `format_datetime_long` existieren, die zusätzlich die Sekunden mit ausgibt — Nanosekunden bleiben in jedem Fall ausgeschlossen.

#### Scenario: Audit-Log-Anzeige

- **WHEN** ein Audit-Log-Eintrag den Zeitstempel `2026-04-16T16:03:34.512345678Z` trägt
- **AND** mit `format_datetime_long` formatiert wird
- **THEN** das Ergebnis enthält Datum, Stunden, Minuten und Sekunden, aber keine Nanosekunden

### Requirement: Keine lokalen Datums-/Zeit-Helfer mehr in Komponenten und Pages

Komponenten und Pages SHALL für Datums- und Zeitanzeigen ausschließlich die `i18n`-Methoden nutzen. Eigene `format_datetime`/`format_datetime_short`-Hilfsfunktionen in Komponenten oder Pages werden entfernt.

#### Scenario: Keine Duplikate

- **WHEN** der Code des Frontends nach `fn format_datetime` oder `fn format_datetime_short` durchsucht wird
- **THEN** es existieren keine lokalen Definitionen außerhalb des `i18n`-Moduls

### Requirement: Keine rohen ISO-Strings mit Nanosekunden in der UI

Wenn ein Zeitstempel als String angezeigt wird, SHALL er zuvor durch eine `i18n`-Format-Methode geleitet worden sein. Rohe ISO8601-Strings mit Bruchteilen von Sekunden tauchen nicht in der UI auf.

#### Scenario: Timestamp-Sektion

- **WHEN** die Timestamp-Sektion einen Eintrag mit dem ISO-String `2026-04-16T16:03:34.512345678Z` darstellt
- **THEN** die UI zeigt einen formatierten Wert (mit oder ohne Sekunden, abhängig vom Kontext)
- **AND** der Bruchteil von Sekunden ist nicht sichtbar

#### Scenario: Audit-Log-Liste

- **WHEN** die Audit-Log-Liste Einträge anzeigt
- **THEN** jeder Zeitstempel ist über `format_datetime` oder `format_datetime_long` geformt
