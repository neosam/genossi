## 1. Dependencies

- [x] 1.1 Entscheidung: eigene Path-Normalisierung vs. `path-clean`-Crate — wenn Crate: `path-clean` zu `genossi_service_impl/Cargo.toml` hinzufügen
- [x] 1.2 `cargo build` zur Verifikation

## 2. Whitelist-Konstante und MIME-Mapping

- [x] 2.1 In `genossi_service/src/member_document.rs` die Konstante `ALLOWED_FILE_TYPES: &[(&str, &str)]` mit (extension, mime_type) laut Design anlegen
- [x] 2.2 Helper-Funktion `lookup_allowed_mime(extension: &str) -> Option<&'static str>` — case-insensitive
- [x] 2.3 Helper-Funktion `allowed_extensions() -> Vec<&'static str>` für 415-Response-Ausgabe
- [x] 2.4 Unit-Tests: `lookup_allowed_mime("PDF")` → `Some("application/pdf")`, `lookup_allowed_mime("exe")` → `None`

## 3. extract_extension-Härtung

- [x] 3.1 In `genossi_service_impl/src/member_document.rs`: `extract_extension` umschreiben, so dass die gelieferte Extension lowercased ist und nur ASCII-alphanumerisch (`[a-z0-9]`) sein darf
- [x] 3.2 Wenn die extrahierte Extension nicht die Regel erfüllt: `None` oder leere Option zurückgeben; der Upload-Pfad behandelt das wie "nicht whitelisted" → 415
- [x] 3.3 Unit-Tests: `foo.PDF` → `pdf`, `foo./bar` → `None` (slash nicht erlaubt), `foo.a..b` → `None`, `foo` ohne Extension → `None`

## 4. Upload-Handler: Whitelist-Check + server-seitige MIME-Ableitung

- [x] 4.1 In `genossi_rest/src/member_document.rs::upload_document`: client-`mime_type` aus Multipart weiterhin lesen, aber **nicht** mehr in `UploadDocument.mime_type` schreiben
- [x] 4.2 Vor dem Service-Call: Extension aus `file_name` extrahieren, gegen Whitelist prüfen
- [x] 4.3 Bei Miss: Response HTTP 415 mit Body `{"error": "...", "allowed_extensions": [...]}`
- [x] 4.4 Bei Hit: `UploadDocument.mime_type` = der aus der Whitelist abgeleitete MIME-Type
- [x] 4.5 Neuer Response-Typ `UnsupportedFileTypeResponse` in `genossi_rest_types` für OpenAPI-Doku
- [x] 4.6 Utoipa-Annotation um 415-Response ergänzen
- [x] 4.7 Unit-Tests (integrationsnah) für upload-handler mit mock storage: erlaubte Extension → 201, verbotene → 415 mit Liste
- [x] 4.8 Generate-Document-Pfad (`generate_document`): hier ist die Extension aus der Template-Logik `.pdf`, MIME-Type fix `application/pdf` → ist bereits safe, aber auch dort auf die Mapping-Tabelle durchlaufen lassen für Konsistenz

## 5. RFC-6266 Content-Disposition Helper

- [x] 5.1 Neue Datei `genossi_rest/src/http_util.rs` (oder in bestehenden util-Bereich) mit Funktion `content_disposition_attachment(filename: &str) -> String`
- [x] 5.2 Implementierung:
  - ASCII-Fallback: alle Non-ASCII, `"`, `\`, `\r`, `\n` → `_`; Ergebnis zu `filename="..."`
  - UTF-8-Teil: Percent-Encoding aller Bytes außer `[A-Za-z0-9._~-]` → `filename*=UTF-8''...`
  - Ergebnis: `format!("attachment; filename=\"{}\"; filename*=UTF-8''{}", ascii, utf8_pct)`
- [x] 5.3 Unit-Tests: einfacher Name `foo.pdf`, mit Umlaut `Müller.pdf`, mit `"` im Namen, mit `\r\n` im Namen, leerer Name, sehr langer Name
- [x] 5.4 In `download_document`-Handler: `format!("attachment; filename=\"{}\"", doc.file_name)` ersetzen durch `content_disposition_attachment(&doc.file_name)`

## 6. Download-Handler: Content-Type aus Whitelist

- [x] 6.1 In `download_document`: der zurückgegebene `Content-Type` wird `doc.mime_type.as_ref()` bleiben (für Backwards-Kompat mit Altdaten), aber mit Warnung im Code-Kommentar, dass neue Uploads server-validiert sind
- [x] 6.2 Falls gewünscht (Nice-to-have, in diesem Change aber **Skip**): Beim Download den MIME-Type noch einmal aus der Extension re-ableiten. → In Open Questions aufnehmen, nicht implementieren (Scope-Kontrolle)

## 7. Filename-Normalisierung für generated Documents

- [x] 7.1 Neue Helper-Funktion `sanitize_filename_component(s: &str) -> String` in `genossi_service_impl` oder `genossi_rest`
- [x] 7.2 Regeln: Umlaut-Transliteration (`ä`→`ae`, `Ä`→`Ae`, `ö`→`oe`, `Ö`→`Oe`, `ü`→`ue`, `Ü`→`Ue`, `ß`→`ss`); alles außer `[a-zA-Z0-9_-]` → `_`; leading/trailing `_` trimmen; leer → `_`; Ergebnis lowercased
- [x] 7.3 In `genossi_rest/src/member_document.rs::generate_document`: Filename-Konstruktion so umbauen, dass `sanitize_filename_component(member.last_name)` und `sanitize_filename_component(member.first_name)` verwendet werden statt `.to_lowercase()` direkt
- [x] 7.4 Unit-Tests: `"Müller"` → `mueller`, `"O'Brien"` → `o_brien`, `"Anna-Lena"` → `anna-lena`, `""` → `_`, `"___"` → `_`

## 8. Path-Traversal-Defense im document_storage

- [x] 8.1 In `genossi_service_impl/src/document_storage.rs`: `full_path`-Signatur ändern zu `fn full_path(&self, relative_path: &str) -> Result<PathBuf, StorageError>`
- [x] 8.2 Implementation:
  - `base_path` kanonisieren (einmalig beim Init oder bei jedem Call; einmalig ist schneller, dann Cachen auf dem Struct)
  - Gejointen Pfad normalisieren (via `path-clean` oder eigene Impl)
  - `if !normalized.starts_with(&canonical_base) { return Err(StorageError::ValidationError(...)) }`
- [x] 8.3 `save`, `load`, `delete` aktualisieren: `self.full_path(relative_path)?`
- [x] 8.4 `StorageError::ValidationError(Arc<str>)` als neue Variante einführen, falls nicht vorhanden
- [x] 8.5 Unit-Tests: `save("normal-uuid.pdf", ...)` → ok, `save("../evil", ...)` → ValidationError, `save("/absolute/evil", ...)` → ValidationError
- [x] 8.6 Verify: Bei Service-Start wird `base_path` kanonisiert; wenn nicht existent, anlegen

## 9. Frontend-Anpassungen (minimal)

- [x] 9.1 In der Upload-Komponente im `genossi-frontend`: `<input type="file" accept="...">` mit den erlaubten Extensions setzen (`.pdf,.png,.jpg,.jpeg,.webp,.txt,.doc,.docx,.odt,.xls,.xlsx,.ods`)
- [x] 9.2 415-Response vom Backend freundlich rendern: Toast oder inline-Fehler mit "Datei-Typ nicht erlaubt. Erlaubt: pdf, png, ..."
- [x] 9.3 Falls es ein Shared-Component für Upload gibt (vgl. `frontend-member-actions` Spec), dort anpassen, nicht inline duplizieren

## 10. Dokumentation & Release

- [x] 10.1 `doc/` oder Projektnotiz: Whitelist der erlaubten Datei-Typen dokumentieren
- [x] 10.2 Release-Notes: "Upload ist jetzt auf bestimmte Datei-Typen beschränkt; MIME-Type wird server-seitig gesetzt"
- [x] 10.3 Smoke-Test nach Deploy: .pdf-Upload, .exe-Upload (→ 415), Download mit Umlaut im Namen (→ DevTools zeigt korrekte Header), bestehendes Dokument lässt sich weiter herunterladen
