## 1. Bestand aufnehmen

- [x] 1.1 `grep -rn 'fn format_datetime\|fn format_datetime_short' genossi-frontend/src` ausführen und eine Liste aller lokalen Helfer notieren
- [x] 1.2 `grep -rn '"{[a-z_.]*\.timestamp}"\|"{[a-z_.]*\.created}"\|"{[a-z_.]*\.date}"' genossi-frontend/src` ausführen, um Stellen mit roher Zeitstempel-Anzeige zu finden
- [x] 1.3 Liste der zu migrierenden Aufrufstellen festhalten (Datei + Zeile)

**Gefunden:**
- Lokale Helfer: `component/application_detail.rs:8`, `component/application_list.rs:23`, `component/communication_timeline.rs:7`
- Rohe Zeitstempel: `component/timestamp_section.rs:206`, `page/mail_page.rs:741`, `page/audit_log.rs:316`, `page/inbox_page.rs:282,323`

## 1b. Toten Code entfernen

Beim Scan fiel auf: `src/i18n/cs.rs` und `src/i18n/i18n.rs` sind in keinem `mod`-Statement deklariert (weder in `src/i18n/mod.rs` noch in `src/main.rs`), werden also gar nicht kompiliert. Der Inhalt passt zu einem Shiftplan-Projekt (`Key::Monday`, `Key::Shiftplan` etc.), die hier gar nicht existieren. Entsprechend entfernen, bevor wir neue Methoden drauf packen, damit spätere Leser nicht glauben, es gäbe eine cs-Locale.

- [x] 1b.1 `src/i18n/cs.rs` entfernen
- [x] 1b.2 `src/i18n/i18n.rs` entfernen
- [x] 1b.3 `cargo check` bestätigt, dass keine Referenz auf die gelöschten Dateien existiert

## 2. i18n-Methoden einführen

Aktives `I18n` lebt in `src/i18n/mod.rs` (nicht in `i18n.rs`). Keine separate Locale-Datei pro Sprache — das Format wird im `match self.locale`-Block analog zum bestehenden `format_date` implementiert.

- [x] 2.1 In `src/i18n/mod.rs` die Methode `format_datetime` auf `I18n` ergänzen (pro Locale eigener Match-Arm)
- [x] 2.2 In `src/i18n/mod.rs` die Methode `format_datetime_long` ergänzen
- [x] 2.3 Deutsches Format implementieren (`16.04.2026 16:03` bzw. `16.04.2026 16:03:34`)
- [x] 2.4 Englisches Format implementieren (`2026-04-16 16:03` bzw. `2026-04-16 16:03:34`, konsistent zu `format_date`)
- [x] 2.5 Beide Methoden akzeptieren einen `&str` (ISO8601). Für `Option<String>`-Felder lösen Aufrufer das per `.as_deref().map(...).unwrap_or_else(...)`.
- [x] 2.6 Bei nicht-parsbarem Eingabe-String wird der Originalstring zurückgegeben

## 3. Lokale Helfer entfernen

- [x] 3.1 `format_datetime` aus `component/application_list.rs:23` entfernen, Aufrufer auf `i18n.format_datetime` umstellen
- [x] 3.2 `format_datetime` aus `component/application_detail.rs:8` entfernen, Aufrufer umstellen
- [x] 3.3 `format_datetime_short` aus `component/communication_timeline.rs:7` entfernen, Aufrufer umstellen

## 4. Rohe ISO-Strings ersetzen

- [x] 4.1 `component/timestamp_section.rs:206` (`"{ts.timestamp}"`) auf `format_datetime_long` umstellen
- [x] 4.2 `page/audit_log.rs` durchsehen und alle rohen Zeitstempel auf `format_datetime` bzw. `format_datetime_long` umstellen (Audit-Daten brauchen tendenziell `_long`)
- [x] 4.3 `page/mail_page.rs:741` (`"{d.job.created}"`) auf `format_datetime` umstellen
- [x] 4.4 Erneuter Scan nach `{...\.timestamp}`, `{...\.created}`, `{...\.date}` — alle Treffer geprüft; zusätzlich `page/inbox_page.rs:282` (Liste) und `:323` (Detail) auf `format_datetime` umgestellt

## 5. Tests

- [x] 5.1 Unit-Test für `format_datetime` mit deutscher Locale: erwarteter String
- [x] 5.2 Unit-Test für `format_datetime` mit englischer Locale
- [x] 5.3 Unit-Test für `format_datetime_long` mit deutscher Locale (+ englische Locale + kein-Bruchteil-Variante)
- [x] 5.4 Unit-Test: nicht-parsbarer ISO-String → Originalstring zurück
- [x] 5.5 Manueller Test: Audit-Log-Seite zeigt formatierte Zeitstempel ohne Nanosekunden
- [x] 5.6 Manueller Test: Timestamp-Sektion zeigt formatierte Zeitstempel
- [x] 5.7 Manueller Test: Antragsliste, Antragsdetail, Communication-Timeline, Mail-Page zeigen formatierte Werte

## 6. Verifizierung

- [x] 6.1 `cargo fmt` — nur meine neue `parse_iso_components`-Funktion brauchte einen kleinen Fix (angewandt). Weitere Diffs in `api.rs`, `application_list.rs`, `mail_page.rs`, `inbox_page.rs` etc. sind alles pre-existing Drift in Code, den dieser Change nicht anfasst — eigener Cleanup-Change, nicht hier.
- [x] 6.2 `cargo clippy --all-targets` — 153 Warnings gesamt, **keine** in meinem neu hinzugefügten Code. Alle Warnings betreffen vorhandene Baustellen (Clone-on-Copy, redundante Closures etc.), nicht durch diesen Change ausgelöst.
- [x] 6.3 `cargo test` — 26 Tests grün, davon 6 neue für `format_datetime`/`format_datetime_long`
- [x] 6.4 Erneuter `grep` nach `fn format_datetime\|fn format_datetime_short` — Treffer nur noch im `i18n`-Modul (`src/i18n/mod.rs`)
- [x] 6.5 Spec-Scenarios aus `specs/frontend-datetime-formatting/spec.md` einzeln durchspielen und abhaken:
  - Deutsche Locale: `16.04.2026 16:03` ✓ (Test `format_datetime_de_drops_fractional_seconds`)
  - Englische Locale: `2026-04-16 16:03` ✓ (Test `format_datetime_en_uses_iso_date`)
  - Audit-Log-Anzeige mit `_long`: Datum + HH:MM:SS, keine Nanosekunden ✓ (Tests `format_datetime_long_*`)
  - Keine Duplikate: `grep` zeigt `fn format_datetime` nur noch im i18n-Modul ✓
  - Timestamp-Sektion: `timestamp_section.rs:206` nutzt `format_datetime_long` ✓
  - Audit-Log-Liste: `audit_log.rs:316` nutzt `format_datetime_long` ✓
