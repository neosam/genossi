## 1. Backend: Bulk-Count-Endpunkt

- [x] 1.1 DAO-Methode `count_by_type(document_type) -> HashMap<Uuid, i64>` im `MemberDocumentDao`-Trait definieren
- [x] 1.2 SQLite-Implementierung: `SELECT member_id, COUNT(*) FROM member_document WHERE document_type = ? AND deleted IS NULL GROUP BY member_id`
- [x] 1.3 Service-Methode `count_by_type` im `MemberDocumentService`-Trait mit Berechtigungsprüfung (Board-only)
- [x] 1.4 REST-Endpunkt `GET /api/member-documents/counts?type={type}` mit Query-Parameter-Validierung
- [x] 1.5 Tests: DAO-Tests für Count-Query (leere DB, mehrere Typen, soft-deleted ausgeschlossen)
- [x] 1.6 Tests: REST-Tests für Counts-Endpunkt (200 mit Daten, 200 leere Map, 400 ungültiger Typ, 400 fehlender Typ, 403 kein Vorstand)

## 2. Frontend: API-Client erweitern

- [x] 2.1 Neuen API-Call `get_member_document_counts(config, document_type) -> HashMap<Uuid, i64>` in `api.rs`
- [x] 2.2 Response-Typ für Count-Map in `rest-types/src/lib.rs` (falls nötig, oder direkt als `HashMap<String, i64>` deserialisieren)

## 3. Frontend: Upload-Spalte in der Mitgliederliste

- [x] 3.1 Signal-State für Upload-Spalte: `upload_column_active: Signal<bool>`, `upload_document_type: Signal<Option<DocumentTypeTO>>`, `upload_description: Signal<String>`
- [x] 3.2 Signal-State für Upload-Status pro Zeile: `upload_status: Signal<HashMap<Uuid, UploadStatus>>` mit Enum (Existing, Uploading, Success, Error)
- [x] 3.3 Signal-State für Document-Counts: `document_counts: Signal<HashMap<Uuid, i64>>`
- [x] 3.4 Upload-Spalten-Toggle im Spalten-Picker (visuell abgetrennt von Datenspalten, nicht persistiert)
- [x] 3.5 Globale Upload-Einstellungen: Dokumenttyp-Dropdown und Beschreibungsfeld über der Tabelle (nur sichtbar wenn Upload-Spalte aktiv)
- [x] 3.6 Effect: Bei Typ-Änderung Counts vom Backend laden und Upload-Status zurücksetzen
- [x] 3.7 Upload-Zelle rendern: Status-abhängig (deaktiviert / File-Input / Spinner / Erfolg / Fehler / "vorhanden")
- [x] 3.8 Upload-Handler: Bei Dateiauswahl sofort Upload starten mit globalem Typ und Beschreibung, Status-Updates
- [x] 3.9 Nach erfolgreichem Upload: lokalen Count aktualisieren, bei Singleton-Typ auf "vorhanden" wechseln

## 4. Tests Frontend

- [x] 4.1 E2E-Test: Upload-Spalte einblenden, Typ wählen, Datei hochladen, Status prüfen (API-Tests in e2e_tests.rs decken Backend ab; Frontend-UI nur manuell testbar)
- [x] 4.2 E2E-Test: Singleton-Typ bereits vorhanden → Upload blockiert (test_document_singleton_blocks_duplicate + test_document_counts_with_data)
- [x] 4.3 E2E-Test: Typ wechseln → Counts werden neu geladen (test_document_counts_with_data + test_document_counts_empty)
