## Why

Die Fehlermeldungen im Frontend sind durchgehend technisch und für Endnutzer (Vorstandsmitglieder, Admins) unverständlich. Wenn z.B. ein Mitglied gespeichert werden soll und das Backend 422 antwortet, sieht der User `HTTP status client error (422 Unprocessable Entity) for url ...`. Upload-Fehler zeigen rohen JSON: `Upload failed (415): {"error":"File type 'exe' is not allowed","allowed_extensions":["pdf","png",...]}`. Gleiches gilt für Konflikte (409), fehlende Berechtigungen (401/403), und Server-Fehler (500) — überall wird der technische reqwest-Fehlertext oder die rohe Backend-Response direkt angezeigt.

Das ist nach dem Security-Hardening (neue 415-Responses, Session-Revoke) besonders relevant, weil jetzt mehr Error-Pfade im normalen Betrieb auftreten können.

## What Changes

- **Zentrale Fehler-Mapping-Funktion**: In `api.rs` wird eine Funktion eingeführt, die HTTP-Statuscodes und Backend-Responses in benutzerfreundliche deutsche Fehlertexte übersetzt. Die rohen Texte verschwinden aus der UI.
- **Strukturierte Fehlertypen**: Statt `Result<T, String>` liefern API-Funktionen einen Fehlertyp, der HTTP-Status, eine benutzerfreundliche Nachricht und optional den technischen Detailtext enthält.
- **Konsistente Fehleranzeige-Komponente**: Eine wiederverwendbare Error-Komponente ersetzt die aktuell über Pages verstreuten `div { class: "text-red-..." }` Blöcke. Sie zeigt eine verständliche Nachricht und bietet optional ein aufklappbares "Details"-Feld für den technischen Text (nützlich für Bug-Reports).
- **Spezial-Behandlung bekannter Fehler**: HTTP 415 (Whitelist) zeigt die erlaubten Dateitypen als lesbare Liste, nicht als JSON-Array. HTTP 409 (Konflikt) zeigt z.B. "Es gibt bereits ein Dokument dieses Typs". HTTP 401/403 zeigt "Keine Berechtigung" statt eines reqwest-Stacktrace.

## Capabilities

### New Capabilities

- `frontend-error-display`: Definiert wie Fehler im Frontend dargestellt werden — Mapping von HTTP-Statuscodes zu benutzerfreundlichen Meldungen, strukturierter Fehlertyp, wiederverwendbare Error-Komponente, und i18n-Integration.

### Modified Capabilities

_(keine bestehenden Specs betroffen — das ist eine rein Frontend-seitige UX-Verbesserung)_

## Impact

**Code:**
- `genossi-frontend/src/api.rs` — alle API-Funktionen: Rückgabetyp ändert sich von `Result<T, String>` / `Result<T, reqwest::Error>` zu `Result<T, AppError>` (oder ähnlichem strukturiertem Typ). Fehler-Mapping zentral statt in jeder Funktion einzeln.
- `genossi-frontend/src/component/` — neue `ErrorDisplay`-Komponente (oder ähnlich benannt)
- `genossi-frontend/src/page/*.rs` — alle Stellen mit `error.set(Some(format!(...)))` und `error.set(Some(e))` werden auf die neue Komponente und den neuen Fehlertyp umgestellt
- `genossi-frontend/src/i18n/` — neue Keys für Fehlermeldungen (de, en, cs)

**Keine Backend-Änderungen.** Das Backend liefert bereits strukturierte Fehler (JSON mit `error`-Feld bei 415, Text bei 400/422). Die Verbesserung liegt rein in der Interpretation und Anzeige.

**Keine neuen Dependencies.**

**Benutzer:**
- Statt technischer Texte sehen Admins verständliche Meldungen in ihrer Sprache
- Technische Details bleiben per Aufklapp-Feld zugänglich (für Support/Bug-Reports)
- Kein Verhaltenswechsel — nur bessere Kommunikation bei Fehlern
