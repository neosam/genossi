## 1. Singleton-Blockierung (Breaking Change)

- [x] 1.1 Ändere `MemberDocumentServiceImpl::upload` — ersetze die soft-delete-Logik für Singletons (Zeilen 82-98) durch einen Fehler (z.B. `ServiceError::Conflict`) wenn bereits ein aktives Dokument desselben Singleton-Typs existiert
- [x] 1.2 Füge `Conflict`-Variante zu `ServiceError` hinzu (falls nicht vorhanden) und mappe sie auf HTTP 409 im REST-Layer
- [x] 1.3 Schreibe/aktualisiere Unit-Tests für das neue Singleton-Blockier-Verhalten im Upload
- [x] 1.4 Aktualisiere E2E-Tests für Singleton-Upload-Blockierung

## 2. Template-zu-DocumentType-Mapping

- [x] 2.1 Erstelle eine statische Mapping-Struktur (z.B. `DocumentTypeMapping`) die Document-Type-Identifier auf Template-Pfade und `DocumentType`-Enum-Werte abbildet
- [x] 2.2 Schreibe Unit-Tests für das Mapping (bekannte Typen, unbekannte Typen)

## 3. Generate-and-Store Service-Methode

- [x] 3.1 Füge eine `generate`-Methode zum `MemberDocumentService`-Trait hinzu die member_id und document_type_identifier entgegennimmt
- [x] 3.2 Implementiere `generate` in `MemberDocumentServiceImpl`: Mapping auflösen, Member laden, Singleton-Prüfung, PDF rendern, als MemberDocument speichern
- [x] 3.3 Schreibe Unit-Tests für die generate-Methode (Erfolg, Dokument existiert bereits, unbekannter Typ, Template nicht gefunden)

## 4. REST-Endpoint

- [x] 4.1 Erstelle den Endpoint `POST /api/members/{member_id}/documents/generate/{document_type}` im REST-Layer
- [x] 4.2 Mappe Fehler korrekt: 409 Conflict, 404 Not Found, 400 Bad Request
- [x] 4.3 Schreibe E2E-Tests für den neuen Endpoint (Erfolg, Duplikat, unbekannter Typ)

## 5. Frontend

- [x] 5.1 Füge einen "Beitrittsbestätigung generieren"-Button auf der Member-Detail-Seite hinzu
- [x] 5.2 Button nur anzeigen wenn kein JoinConfirmation-Dokument existiert
- [x] 5.3 API-Aufruf bei Klick und Dokumentliste nach Erfolg aktualisieren
