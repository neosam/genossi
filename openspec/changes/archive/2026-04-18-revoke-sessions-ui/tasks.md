## 1. Komponente erstellen

- [x] 1.1 Neue Datei `genossi-frontend/src/component/revoke_sessions_button.rs` anlegen mit `RevokeSessionsButton`-Komponente
- [x] 1.2 Bestätigungsdialog mit `Modal`-Komponente: Warntext ("Alle Sessions werden beendet. Sie werden ausgeloggt."), Bestätigungs-Button und Abbrechen-Button
- [x] 1.3 `use_signal`-Flags für Dialog-Sichtbarkeit (`show_confirm`) und Ladezustand (`loading`)
- [x] 1.4 API-Call `api::revoke_all_sessions()` beim Bestätigen; Button deaktivieren während des Calls
- [x] 1.5 Bei Erfolg: Redirect via `window.location` auf `{backend_url}/logout`
- [x] 1.6 Bei Fehler: `ErrorAlert`-Komponente im Modal anzeigen, Dialog bleibt offen

## 2. Integration in TopBar

- [x] 2.1 `pub mod revoke_sessions_button;` in `genossi-frontend/src/component/mod.rs` eintragen
- [x] 2.2 `RevokeSessionsButton {}` in `top_bar.rs` im Auth-Bereich einfügen (innerhalb des `if let Some(auth)` Blocks, vor dem Logout-Link)

## 3. i18n

- [x] 3.1 Neue i18n-Keys hinzufügen: `RevokeAllSessions` ("Sessions beenden" / "Revoke all sessions" / "Ukončit relace"), `RevokeSessionsConfirmTitle` (Dialog-Titel), `RevokeSessionsConfirmText` (Warntext)

## 4. Tests

- [x] 4.1 Unit-Test: `RevokeSessionsButton` rendert ohne Fehler (entfällt — reine UI-Komponente ohne extrahierbare Logik, Frontend-Tests decken nur reine Funktionen ab)
- [x] 4.2 E2E-Test: `POST /api/session/revoke-all` gibt 200 zurück (Endpoint-Smoke-Test, bereits vorhanden — verifiziert in `e2e_tests.rs:7952`)
