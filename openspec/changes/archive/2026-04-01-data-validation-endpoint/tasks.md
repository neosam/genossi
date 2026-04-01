## 1. Service Layer - Trait & Typen

- [x] 1.1 Erstelle `genossi_service/src/validation.rs` mit `ValidationResult`, `MemberNumberGap`, `UnmatchedTransfer` Structs und `ValidationService` Trait
- [x] 1.2 Registriere das Modul in `genossi_service/src/lib.rs`

## 2. Service Layer - Implementierung

- [x] 2.1 Erstelle `genossi_service_impl/src/validation.rs` mit `ValidationServiceImpl` (nutzt `gen_service_impl!` Macro mit MemberDao + MemberActionDao Dependencies)
- [x] 2.2 Implementiere Mitgliedsnummern-Lücken-Erkennung (dump_all, min..max Bereich, fehlende Nummern finden)
- [x] 2.3 Implementiere Übertragungs-Gegenpart-Prüfung (alle Transfer-Aktionen laden, Paare matchen nach member_id/transfer_member_id/shares/date)
- [x] 2.4 Registriere das Modul in `genossi_service_impl/src/lib.rs`
- [x] 2.5 Unit-Tests für Lücken-Erkennung (keine Lücken, Lücken vorhanden, Bereich nicht bei 1, soft-deleted zählen mit, keine Mitglieder)
- [x] 2.6 Unit-Tests für Übertragungs-Prüfung (alle gepaart, fehlender Gegenpart, Shares mismatch, Datum mismatch, soft-deleted ignoriert, keine Übertragungen)

## 3. REST Layer

- [x] 3.1 Erstelle `genossi_rest_types/src/validation.rs` mit Transfer Objects (ValidationResultTO, MemberNumberGapTO, UnmatchedTransferTO)
- [x] 3.2 Erstelle `genossi_rest/src/validation.rs` mit `GET /api/validation` Handler und OpenAPI-Dokumentation
- [x] 3.3 Erweitere `RestStateDef` um `ValidationService` und registriere die Route in `genossi_rest/src/lib.rs`

## 4. Binary Layer - Wiring

- [x] 4.1 Erstelle `ValidationServiceDependencies` in `genossi_bin/src/lib.rs` und verdrahte den Service mit dem RestState

## 5. Tests

- [x] 5.1 E2E-Test: `GET /api/validation` gibt 200 mit leerem Ergebnis bei konsistenten Daten
- [x] 5.2 E2E-Test: `GET /api/validation` erkennt Mitgliedsnummern-Lücken nach Import
- [x] 5.3 E2E-Test: `GET /api/validation` erkennt unmatched Übertragungen

## 6. Frontend

- [x] 6.1 API-Client-Funktion `get_validation` in `genossi-frontend/src/api.rs`
- [x] 6.2 REST-Types für Validierung in `genossi-frontend/rest-types/src/lib.rs`
- [x] 6.3 Erstelle Validierungsseite `genossi-frontend/src/page/validation.rs`
- [x] 6.4 Registriere Route und Navigation für die Validierungsseite
- [x] 6.5 i18n-Keys für Validierungsseite (de + en)
