## Why

Änderungen an Mitgliederdaten, Aktionen, Dokumenten und Anträgen müssen revisionssicher nachvollziehbar sein. Es muss jederzeit erkennbar sein, wer wann welches Feld geändert hat. Aktuell gibt es keine Änderungshistorie — der `process`-Parameter in den DAO-Methoden wird nicht genutzt, und es gibt keine Möglichkeit, vergangene Zustände oder Manipulationen zu erkennen.

## What Changes

- Neue `audit_log`-Tabelle (append-only) mit einer Zeile pro geändertem Feld
- `Auditable`-Trait auf allen relevanten Entities für automatische Feld-Extraktion und Diff-Berechnung
- Hash-Chain (SHA256): Jeder Eintrag enthält den Hash des vorherigen Eintrags, sodass Manipulationen in der Mitte erkennbar sind
- `transaction_id` zum Gruppieren zusammengehöriger Feldänderungen
- Audit-Macros (`audited_create!`, `audited_update!`, `audited_delete!`) die den DAO-Call und das Logging atomar zusammenfassen
- `AuditLogDao` + SQLite-Implementierung für die Persistenz
- `AuditService` für Diff-Berechnung, Hash-Chain-Verwaltung und Integritätsprüfung
- REST-Endpoints zum Lesen der Audit-Historie und Verifizieren der Hash-Chain
- Frontend-Seite mit filterbarer Audit-Log-Ansicht

## Capabilities

### New Capabilities
- `audit-logging`: Kern-Audit-Log-System mit Hash-Chain, Auditable-Trait, Diff-Berechnung und Audit-Macros auf DAO/Service-Ebene
- `audit-api`: REST-Endpoints zum Abfragen der Audit-Historie und Verifizieren der Hash-Chain-Integrität
- `audit-ui`: Frontend-Seite zur Anzeige und Filterung des Audit-Logs

### Modified Capabilities
- `member-management`: Service-Methoden nutzen Audit-Macros statt direkter DAO-Calls für create/update/delete
- `member-actions`: Service-Methoden nutzen Audit-Macros statt direkter DAO-Calls
- `member-documents`: Service-Methoden nutzen Audit-Macros statt direkter DAO-Calls

## Impact

- **DAO-Layer**: Neues `audit_log` Modul in `genossi_dao` und `genossi_dao_impl_sqlite`
- **Service-Layer**: Neuer `AuditService`, Audit-Macros in `genossi_service_impl`, Anpassung von `MemberServiceImpl`, `MemberActionServiceImpl`, `MemberDocumentServiceImpl`, `ApplicationServiceImpl`
- **REST-Layer**: Neue Audit-Endpoints in `genossi_rest`
- **Frontend**: Neue Audit-Log-Seite in `genossi-frontend`
- **Datenbank**: Neue Migration für `audit_log`-Tabelle
- **Dependencies**: `sha2` Crate für SHA256-Hashing
