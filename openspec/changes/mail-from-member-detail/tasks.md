## 1. i18n-Keys

- [ ] 1.1 Prüfen, ob `Key::SendMail` (oder ähnlich) bereits existiert; sonst neuen Key `MailSendButton` anlegen
- [ ] 1.2 Neuen Key `NoEmailAddressHint` für den Disabled-Hinweis anlegen
- [ ] 1.3 Übersetzungen in `de.rs`, `en.rs`, `cs.rs` einpflegen

## 2. Knopf in member_details.rs

- [ ] 2.1 Geeignete Stelle im Aktionsbereich der Detailseite identifizieren (oberhalb der Stammdaten oder im Header — passend zum bestehenden Layout)
- [ ] 2.2 Knopf rendern, sichtbar nur wenn `!is_new` (also nicht im Anlegen-Modus)
- [ ] 2.3 Knopf disabled, wenn `member.read().email.is_none() || member.read().email.as_deref().map(str::trim).map(str::is_empty).unwrap_or(true)`
- [ ] 2.4 Bei `disabled` einen Tooltip oder Inline-Hinweis mit `Key::NoEmailAddressHint` anzeigen
- [ ] 2.5 Im `onclick`-Handler: `SELECTED_MEMBER_IDS.write().clear(); SELECTED_MEMBER_IDS.write().toggle(member_id); nav.push(Route::MailPage {});`

## 3. Tests

- [ ] 3.1 Manueller Test: Mitglied mit E-Mail → Knopf klickbar → Klick landet auf `/mail` mit korrektem Empfänger
- [ ] 3.2 Manueller Test: Mitglied ohne E-Mail → Knopf disabled, Hinweis sichtbar
- [ ] 3.3 Manueller Test: Vorherige Auswahl auf der Mitgliederliste wird durch Klick ersetzt
- [ ] 3.4 Manueller Test: Anlegen-Modus → kein Knopf

## 4. Verifizierung

- [ ] 4.1 `cargo fmt`
- [ ] 4.2 `cargo clippy --all-targets`
- [ ] 4.3 `cargo test`
- [ ] 4.4 Spec-Scenarios aus `specs/mail-from-member-detail/spec.md` einzeln durchspielen und abhaken
