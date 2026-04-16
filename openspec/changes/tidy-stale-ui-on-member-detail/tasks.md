## 1. Verifikation des aktuellen Verhaltens

- [ ] 1.1 Im laufenden System ein Mitglied mit existierender Eintrittsbestätigung öffnen und Browser-Devtools/Backend-Logs konsultieren: welchen String liefert das Backend in `MemberDocumentTO.document_type`?
- [ ] 1.2 `DocumentTypeTO::JoinConfirmation.as_str()` aus `rest_types` prüfen und mit (1.1) abgleichen
- [ ] 1.3 Falls die Strings nicht übereinstimmen: Ursache im Backend dokumentieren oder im Frontend `from_str` benutzen

## 2. Eintrittsbestätigung-Knopf (Bug-Fix)

- [ ] 2.1 In `member_details.rs:1005` den Vergleich auf `d.document_type == DocumentTypeTO::JoinConfirmation.as_str()` umstellen
- [ ] 2.2 Manueller Test: Mitglied ohne Eintrittsbestätigung → Knopf sichtbar
- [ ] 2.3 Manueller Test: Mitglied mit Eintrittsbestätigung → Knopf nicht sichtbar
- [ ] 2.4 Manueller Test: Knopf klicken → nach Generierung verschwindet der Knopf

## 3. Migrationsstatus-Badge

- [ ] 3.1 In `member_details.rs:322-371` den `if status.status == "migrated"`-Zweig entfernen
- [ ] 3.2 Der `else`-Zweig (Pending-Block mit Anteilen, Aktionen und Bestätigungs-Knopf) bleibt unverändert erhalten
- [ ] 3.3 Manueller Test: Migriertes Mitglied → kein Badge, sonst alles unverändert
- [ ] 3.4 Manueller Test: Pending-Mitglied → Pending-Block weiterhin sichtbar mit allen Details
- [ ] 3.5 Prüfen, ob die i18n-Keys `Key::Migrated` jetzt unbenutzt sind und ggf. entfernen

## 4. Verifizierung

- [ ] 4.1 `cargo fmt`
- [ ] 4.2 `cargo clippy --all-targets`
- [ ] 4.3 `cargo test`
- [ ] 4.4 Spec-Scenarios aus `specs/member-detail-ui-tidy/spec.md` einzeln durchspielen und abhaken
