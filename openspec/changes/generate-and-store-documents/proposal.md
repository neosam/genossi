## Why

Dokumente wie die Beitrittsbestätigung können bereits per Typst-Template generiert und als PDF heruntergeladen werden. Allerdings werden sie nicht automatisch beim Mitglied hinterlegt — man muss sie manuell als Dokument hochladen. Es fehlt ein "Generieren & Ablegen"-Schritt, der das PDF erzeugt und direkt als MemberDocument speichert.

Zusätzlich ersetzt das bestehende Singleton-Upload-Verhalten vorhandene Dokumente per soft-delete, was unerwünscht ist. Singleton-Dokumente sollen blockiert werden, wenn bereits eins existiert.

## What Changes

- Neuer REST-Endpoint zum Generieren eines PDFs aus einem Template und direktem Ablegen als MemberDocument
- Feste Zuordnung von Template-Namen zu DocumentType (z.B. `join_confirmation.typ` → `JoinConfirmation`)
- Prüfung: Wenn für das Mitglied bereits ein Dokument dieses Typs existiert, wird die Generierung abgelehnt
- **BREAKING**: Singleton-Upload-Verhalten wird geändert — statt auto-replace wird ein Fehler zurückgegeben, wenn ein Singleton-Dokument bereits existiert
- Frontend-Button auf der Member-Detail-Seite: "Beitrittsbestätigung generieren" (nur sichtbar wenn noch kein solches Dokument vorhanden)

## Capabilities

### New Capabilities
- `document-generation`: Generierung von PDF-Dokumenten aus Templates und automatische Ablage als MemberDocument, inklusive Template-zu-DocumentType-Mapping

### Modified Capabilities
- `member-documents`: Singleton-Upload-Verhalten wird geändert — statt auto-replace bei existierendem Singleton-Dokument wird ein Fehler zurückgegeben

## Impact

- `genossi_service/src/member_document.rs`: Neuer Service-Trait-Methode oder separater Service für Generierung
- `genossi_service_impl/src/member_document.rs`: Singleton-Logik ändern (Zeilen 82-98: soft-delete → Fehler)
- `genossi_rest/src/template.rs` oder neuer Endpoint: Render+Store Endpoint
- `genossi-frontend/src/page/member_details.rs`: Button zum Generieren
- `genossi-frontend/rest-types/src/lib.rs`: Ggf. neue API-Typen
