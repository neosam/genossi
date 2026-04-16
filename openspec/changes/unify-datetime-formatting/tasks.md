## 1. Bestand aufnehmen

- [ ] 1.1 `grep -rn 'fn format_datetime\|fn format_datetime_short' genossi-frontend/src` ausführen und eine Liste aller lokalen Helfer notieren
- [ ] 1.2 `grep -rn '"{[a-z_.]*\.timestamp}"\|"{[a-z_.]*\.created}"\|"{[a-z_.]*\.date}"' genossi-frontend/src` ausführen, um Stellen mit roher Zeitstempel-Anzeige zu finden
- [ ] 1.3 Liste der zu migrierenden Aufrufstellen festhalten (Datei + Zeile)

## 2. i18n-Methoden einführen

- [ ] 2.1 In `genossi-frontend/src/i18n/i18n.rs` die Methode `format_datetime` auf `I18n` ergänzen, die an die jeweilige Locale delegiert
- [ ] 2.2 In `genossi-frontend/src/i18n/i18n.rs` die Methode `format_datetime_long` ergänzen
- [ ] 2.3 In `genossi-frontend/src/i18n/de.rs` beide Methoden mit deutschem Format implementieren (`16.04.2026 16:03` bzw. `16.04.2026 16:03:34`)
- [ ] 2.4 In `genossi-frontend/src/i18n/en.rs` mit englischem Format implementieren
- [ ] 2.5 In `genossi-frontend/src/i18n/cs.rs` mit tschechischem Format implementieren
- [ ] 2.6 Beide Methoden akzeptieren sowohl `&time::OffsetDateTime` als auch `&str` (z. B. via Trait oder Overloads)
- [ ] 2.7 Bei nicht-parsbarem Eingabe-String wird der Originalstring zurückgegeben

## 3. Lokale Helfer entfernen

- [ ] 3.1 `format_datetime` aus `component/application_list.rs:23` entfernen, Aufrufer auf `i18n.format_datetime` umstellen
- [ ] 3.2 `format_datetime` aus `component/application_detail.rs:8` entfernen, Aufrufer umstellen
- [ ] 3.3 `format_datetime_short` aus `component/communication_timeline.rs:7` entfernen, Aufrufer umstellen

## 4. Rohe ISO-Strings ersetzen

- [ ] 4.1 `component/timestamp_section.rs:206` (`"{ts.timestamp}"`) auf `format_datetime_long` umstellen
- [ ] 4.2 `page/audit_log.rs` durchsehen und alle rohen Zeitstempel auf `format_datetime` bzw. `format_datetime_long` umstellen (Audit-Daten brauchen tendenziell `_long`)
- [ ] 4.3 Weitere Treffer aus 1.2 abarbeiten

## 5. Tests

- [ ] 5.1 Unit-Test für `format_datetime` mit deutscher Locale: erwarteter String
- [ ] 5.2 Unit-Test für `format_datetime` mit englischer Locale
- [ ] 5.3 Unit-Test für `format_datetime` mit tschechischer Locale
- [ ] 5.4 Unit-Test für `format_datetime_long` mit deutscher Locale
- [ ] 5.5 Unit-Test: nicht-parsbarer ISO-String → Originalstring zurück
- [ ] 5.6 Manueller Test: Audit-Log-Seite zeigt formatierte Zeitstempel ohne Nanosekunden
- [ ] 5.7 Manueller Test: Timestamp-Sektion zeigt formatierte Zeitstempel
- [ ] 5.8 Manueller Test: Antragsliste, Antragsdetail, Communication-Timeline zeigen formatierte Werte

## 6. Verifizierung

- [ ] 6.1 `cargo fmt`
- [ ] 6.2 `cargo clippy --all-targets`
- [ ] 6.3 `cargo test`
- [ ] 6.4 Erneuter `grep` nach `fn format_datetime\|fn format_datetime_short` — Treffer nur noch im `i18n`-Modul
- [ ] 6.5 Spec-Scenarios aus `specs/frontend-datetime-formatting/spec.md` einzeln durchspielen und abhaken
