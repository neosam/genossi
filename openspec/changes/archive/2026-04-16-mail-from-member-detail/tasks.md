## 1. i18n-Keys

- [x] 1.1 Prüfen, ob `Key::SendMail` (oder ähnlich) bereits existiert; sonst neuen Key `MailSendButton` anlegen
- [x] 1.2 Neuen Key `NoEmailAddressHint` für den Disabled-Hinweis anlegen
- [x] 1.3 Übersetzungen in `de.rs`, `en.rs`, `cs.rs` einpflegen (Hinweis: `cs.rs` existiert im Projekt nicht — nur `de.rs` und `en.rs` aktualisiert)

## 2. Knopf in member_details.rs

- [x] 2.1 Geeignete Stelle im Aktionsbereich der Detailseite identifizieren (oberhalb der Stammdaten oder im Header — passend zum bestehenden Layout)
- [x] 2.2 Knopf rendern, sichtbar nur wenn `!is_new` (also nicht im Anlegen-Modus)
- [x] 2.3 Knopf disabled, wenn `member.read().email.is_none() || member.read().email.as_deref().map(str::trim).map(str::is_empty).unwrap_or(true)`
- [x] 2.4 Bei `disabled` einen Tooltip oder Inline-Hinweis mit `Key::NoEmailAddressHint` anzeigen
- [x] 2.5 Im `onclick`-Handler: `SELECTED_MEMBER_IDS.write().clear(); SELECTED_MEMBER_IDS.write().toggle(member_id); nav.push(Route::MailPage {});`

## 3. Tests

- [x] 3.1 Manueller Test: Mitglied mit E-Mail → Knopf klickbar → Klick landet auf `/mail` mit korrektem Empfänger
- [x] 3.2 Manueller Test: Mitglied ohne E-Mail → Knopf disabled, Hinweis sichtbar
- [x] 3.3 Manueller Test: Vorherige Auswahl auf der Mitgliederliste wird durch Klick ersetzt
- [x] 3.4 Manueller Test: Anlegen-Modus → kein Knopf
- [x] 3.5 Unit-Tests für `is_email_empty` Helper (None / "" / whitespace / echte Adresse / Adresse mit Whitespace)

## 4. Verifizierung

- [x] 4.1 `cargo fmt` (keine Änderungen nötig)
- [x] 4.2 `cargo clippy --all-targets` (keine neuen Warnungen; nur bestehende)
- [x] 4.3 `cargo test` (alle Tests grün inkl. 5 neue `is_email_empty`-Tests)
- [x] 4.4 Spec-Scenarios aus `specs/mail-from-member-detail/spec.md` einzeln durchspielen und abhaken
