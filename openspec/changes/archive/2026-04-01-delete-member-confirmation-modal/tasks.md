## 1. Modal-Component verfügbar machen

- [x] 1.1 `Modal`-Component in `genossi-frontend/src/component/mod.rs` als `pub` exportieren

## 2. i18n-Keys hinzufügen

- [x] 2.1 Neue Keys in `src/i18n/mod.rs` hinzufügen: `DeleteMemberConfirmTitle`, `Confirm` (ConfirmDelete existiert bereits)
- [x] 2.2 Englische Übersetzungen in `src/i18n/en.rs` hinzufügen
- [x] 2.3 Deutsche Übersetzungen in `src/i18n/de.rs` hinzufügen
- [x] 2.4 Tschechisch nicht nötig (cs.rs gehört zu anderem Projekt, dieses Projekt hat nur En/De)

## 3. Bestätigungs-Modal in Member-Detail-Seite

- [x] 3.1 Signal `show_delete_modal` in `member_details.rs` hinzufügen
- [x] 3.2 Delete-Button-Handler ändern: Modal anzeigen statt direkt löschen
- [x] 3.3 Modal mit Bestätigungsfrage, Member-Name, Bestätigen- und Abbrechen-Buttons rendern
- [x] 3.4 Bestätigen-Button: Delete-API-Call ausführen und zur Member-Liste navigieren
- [x] 3.5 Abbrechen-Button: Modal schließen ohne Aktion

## 4. Testen

- [x] 4.1 Frontend kompiliert ohne Fehler (`cargo check` im Frontend-Verzeichnis)
- [x] 4.2 Bestehende Tests laufen weiterhin durch (`cargo test`)
