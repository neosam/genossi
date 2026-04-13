## 1. Datenbank & DAO

- [x] 1.1 SQLite-Migration erstellen: neue Tabelle `member_actions` mit allen Feldern (id, member_id, action_type, date, shares_change, transfer_member_id, effective_date, comment, created, deleted, version)
- [x] 1.2 SQLite-Migration erstellen: Spalte `action_count` zur `members`-Tabelle hinzufuegen (default 0)
- [x] 1.3 `MemberActionEntity` und `MemberActionDao` Trait in `genossi_dao` erstellen (dump_all, create, update, find_by_member_id)
- [x] 1.4 `MemberActionDaoImpl` in `genossi_dao_impl_sqlite` implementieren
- [x] 1.5 `MemberEntity` um `action_count: i32` erweitern und SQLite-Queries anpassen
- [x] 1.6 Unit-Tests fuer MemberActionDao

## 2. Service-Layer

- [x] 2.1 `MemberAction` Service-Typ und `MemberActionService` Trait in `genossi_service` erstellen
- [x] 2.2 ActionType Enum definieren: Eintritt, Austritt, Todesfall, Aufstockung, Verkauf, UebertragungEmpfang, UebertragungAbgabe
- [x] 2.3 `MemberActionServiceImpl` implementieren mit CRUD-Operationen
- [x] 2.4 Validierungslogik: Constraints pro ActionType (shares_change Vorzeichen, transfer_member_id Pflicht, effective_date nur bei Austritt)
- [x] 2.5 Migrations-Validierungslogik implementieren (Summe Aktionen vs. current_shares, Anzahl vs. action_count)
- [x] 2.6 Unit-Tests fuer MemberActionService mit Mocks

## 3. Excel-Import Erweiterung

- [x] 3.1 "Anzahl Aktionen" Spalte im Import-Parser lesen und als `action_count` speichern
- [x] 3.2 Auto-Migration: Eintritts-Aktion und Aufstockungs-Aktion fuer Mitglieder mit action_count==0 und shares_at_joining==current_shares erzeugen
- [x] 3.3 Tests fuer action_count Import und Auto-Migrations-Logik

## 4. REST-Layer

- [x] 4.1 `MemberActionTO` in `genossi_rest_types` erstellen mit Serde und OpenAPI-Annotationen
- [x] 4.2 REST-Endpoints implementieren: POST/GET/PUT/DELETE unter `/api/members/{member_id}/actions`
- [x] 4.3 Migration-Status Endpoint: `GET /api/members/{member_id}/actions/migration-status`
- [x] 4.4 OpenAPI/Swagger-Dokumentation fuer alle neuen Endpoints
- [x] 4.5 Tests fuer REST-Endpoints

## 5. Dependency Injection & Verdrahtung

- [x] 5.1 MemberActionDao und MemberActionService in `genossi_bin` verdrahten
- [x] 5.2 Neue Endpoints in den Axum-Router einhaengen

## 6. E2E-Tests

- [x] 6.1 E2E-Tests: CRUD-Operationen fuer MemberActions
- [x] 6.2 E2E-Tests: Validierung der ActionType-Constraints
- [x] 6.3 E2E-Tests: Migrations-Status Endpoint
- [x] 6.4 E2E-Tests: Excel-Import mit action_count und Auto-Migration
