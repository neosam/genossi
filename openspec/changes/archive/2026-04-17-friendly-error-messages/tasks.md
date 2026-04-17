## 1. AppError-Typ und zentrales Mapping

- [x] 1.1 `AppError`-Struct in `genossi-frontend/src/api.rs` definieren: Felder `status: Option<u16>`, `message: String`, `detail: Option<String>`; `Display` und `std::error::Error` implementieren
- [x] 1.2 Hilfsfunktion `map_response_error(response: Response) -> AppError` implementieren: HTTP-Status lesen, Body als Text extrahieren, Status-Code auf deutsche Meldung mappen (siehe Spec-Tabelle)
- [x] 1.3 Spezialbehandlung für 415: Body als JSON parsen (`error` + `allowed_extensions`), erlaubte Typen als kommaseparierte Liste in der Message anzeigen; Fallback bei Parse-Fehler
- [x] 1.4 Hilfsfunktion `network_error(e: reqwest::Error) -> AppError` für Netzwerkfehler (status=None, message="Verbindungsfehler")
- [x] 1.5 Unit-Tests: `map_response_error` für Status 400, 401, 403, 404, 409, 415 (mit/ohne JSON), 422, 429, 500, unbekannter Status; `Display`-Impl

## 2. ErrorAlert-Komponente

- [x] 2.1 `ErrorAlert`-Komponente in `genossi-frontend/src/component/error_alert.rs` erstellen: Props `error: AppError`, `on_dismiss: Option<EventHandler<()>>`
- [x] 2.2 Rotes Alert-Banner mit `error.message`, konsistentes Tailwind-Styling
- [x] 2.3 Aufklappbarer "Details"-Bereich wenn `error.detail` vorhanden
- [x] 2.4 Dismiss-Button (X) nur wenn `on_dismiss` gesetzt
- [x] 2.5 Komponente in `component/mod.rs` exportieren

## 3. API-Funktionen migrieren

- [x] 3.1 Alle Funktionen in `api.rs` die `Result<T, reqwest::Error>` zurückgeben auf `Result<T, AppError>` umstellen: `error_for_status_ref()` durch Status-Prüfung + `map_response_error()` ersetzen
- [x] 3.2 Alle Funktionen in `api.rs` die `Result<T, String>` zurückgeben auf `Result<T, AppError>` umstellen: manuelle `format!`-Fehler durch `AppError::new()` ersetzen
- [x] 3.3 Kompilierbarkeit prüfen — alle Aufrufer müssen noch kompilieren (ggf. temporäre `.to_string()`-Aufrufe über Display)

## 4. Pages migrieren

- [x] 4.1 `member_details.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.2 `applications_page.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.3 `static_documents.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.4 `templates.rs`: Error-Signal auf `Option<AppError>` umstellen (inkl. `preview_error`), inline Error-Divs durch `ErrorAlert` ersetzen
- [x] 4.5 `config_page.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.6 `validation.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.7 `inbox_page.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.8 `mail_templates.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.9 `audit_log.rs`: Error-Signal auf `Option<AppError>` umstellen, inline Error-Div durch `ErrorAlert` ersetzen
- [x] 4.10 `mail_page.rs`: Beide Error-Signals auf `Option<AppError>` umstellen, inline Error-Divs durch `ErrorAlert` ersetzen

## 5. Aufräumen

- [x] 5.1 Prüfen ob weitere Stellen im Frontend `String`-Errors anzeigen (Grep nach `error.set(Some(` und `bg-red`) und ggf. migrieren
- [x] 5.2 `cargo build -p genossi-frontend` erfolgreich
- [x] 5.3 `cargo clippy -p genossi-frontend` ohne neue Warnungen (clippy nicht verfügbar in aktueller Nix-Shell — cargo check erfolgreich)
- [x] 5.4 `cargo test` — alle bestehenden Tests passieren
