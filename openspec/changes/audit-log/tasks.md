## 1. Database & DAO Foundation

- [x] 1.1 Create SQLite migration for audit_log table with indexes (entity, transaction, timestamp, user)
- [x] 1.2 Define AuditLogEntry struct and AuditLogDao trait in genossi_dao (create_entries, get_latest_hash, get_by_entity, get_all_ordered)
- [x] 1.3 Implement AuditLogDao for SQLite in genossi_dao_impl_sqlite
- [x] 1.4 Write unit tests for AuditLogDao SQLite implementation (insert, query by entity, query ordered)

## 2. Auditable Trait & Diff

- [x] 2.1 Define Auditable trait in genossi_dao with entity_type, entity_id, audit_fields, and default diff implementation
- [x] 2.2 Define AuditFieldChange struct (field_name, old_value, new_value)
- [x] 2.3 Implement Auditable for MemberEntity (all data fields, excluding id/version/created/deleted)
- [x] 2.4 Implement Auditable for MemberActionEntity
- [x] 2.5 Implement Auditable for MemberDocumentEntity
- [x] 2.6 Implement Auditable for ApplicationEntity
- [x] 2.7 Write unit tests for each Auditable implementation (audit_fields correctness, diff with changes, diff without changes)

## 3. Hash Chain & Audit Service

- [x] 3.1 Add sha2 dependency to genossi_service_impl
- [x] 3.2 Implement hash computation function: SHA256(timestamp|user_id|process|transaction_id|entity_type|entity_id|action|field_name|old_value|new_value|prev_hash)
- [x] 3.3 Implement audit entry creation logic: compute diff, sort fields alphabetically, build hash chain, create AuditLogEntry structs
- [x] 3.4 Implement chain verification logic: read all entries ordered, recompute hashes, report broken links
- [x] 3.5 Write unit tests for hash computation determinism
- [x] 3.6 Write unit tests for chain verification (intact chain, broken chain, empty chain)

## 4. Audit Macros

- [x] 4.1 Implement audited_create! macro in genossi_service_impl (dao.create + log all non-None fields)
- [x] 4.2 Implement audited_update! macro (dao.find_by_id + dao.update + log changed fields only)
- [x] 4.3 Implement audited_delete! macro (dao.find_by_id + set deleted + dao.update + log all fields as delete)
- [x] 4.4 Write tests for each macro (create logs all fields, update logs only changed, update with no changes logs nothing, delete logs all)

## 5. Service Integration — Member

- [x] 5.1 Add AuditLogDao as dependency to MemberServiceImpl via gen_service_impl!
- [x] 5.2 Replace direct dao.create call with audited_create! in MemberServiceImpl::create (including auto-created Eintritt/Aufstockung actions)
- [x] 5.3 Replace direct dao.update call with audited_update! in MemberServiceImpl::update
- [x] 5.4 Replace direct dao.update call with audited_delete! in MemberServiceImpl::delete
- [x] 5.5 Extract user_id from Authentication<Context> via permission_service.current_user_id (use "SYSTEM" for Authentication::Full)
- [x] 5.6 Update existing MemberService unit tests to include AuditLogDao mock
- [x] 5.7 Write integration tests verifying audit entries are created on member create/update/delete

## 6. Service Integration — MemberAction

- [x] 6.1 Add AuditLogDao as dependency to MemberActionServiceImpl
- [x] 6.2 Replace direct DAO calls with audit macros in MemberActionServiceImpl (create, update, delete)
- [x] 6.3 Update existing MemberActionService unit tests to include AuditLogDao mock
- [x] 6.4 Write integration tests for member action audit logging

## 7. Service Integration — MemberDocument

- [x] 7.1 Add AuditLogDao as dependency to MemberDocumentServiceImpl
- [x] 7.2 Replace direct DAO calls with audit macros in MemberDocumentServiceImpl
- [x] 7.3 Update existing MemberDocumentService unit tests to include AuditLogDao mock
- [x] 7.4 Write integration tests for member document audit logging

## 8. Service Integration — Application

- [x] 8.1 Add AuditLogDao as dependency to ApplicationServiceImpl
- [x] 8.2 Replace direct DAO calls with audit macros in ApplicationServiceImpl
- [x] 8.3 Update existing ApplicationService unit tests to include AuditLogDao mock
- [x] 8.4 Write integration tests for application audit logging

## 9. REST API

- [x] 9.1 Define AuditLogResponse REST types in genossi_rest_types (with utoipa ToSchema)
- [x] 9.2 Define VerifyResponse REST type (valid, total_entries, broken_links)
- [x] 9.3 Implement GET /api/audit/{entity_type}/{entity_id} endpoint (admin-only)
- [x] 9.4 Implement GET /api/audit endpoint with query parameter filtering (entity_type, entity_id, user_id, from, to, action)
- [x] 9.5 Implement GET /api/audit/verify endpoint (admin-only)
- [x] 9.6 Register audit endpoints in OpenAPI/Swagger documentation
- [x] 9.7 Write E2E tests for audit REST endpoints

## 10. Frontend — Audit Log Page

- [x] 10.1 Add audit-log REST types to genossi-frontend/rest-types
- [x] 10.2 Create audit log page component at /audit route
- [x] 10.3 Implement audit log table with columns: Zeitpunkt, Benutzer, Aktion, Entity-Typ, Entity-ID, Feld, Alter Wert, Neuer Wert
- [x] 10.4 Implement transaction_id grouping (visual grouping of related entries)
- [x] 10.5 Implement filter controls (Entity-Typ dropdown, Benutzer text input, Aktion dropdown, Zeitraum date range)
- [x] 10.6 Implement hash chain verify button with success/failure display
- [x] 10.7 Add audit log link to navigation menu (admin-only visibility)
- [x] 10.8 Add i18n translations for audit log page (de, en)

## 11. Dependency Injection & Wiring

- [x] 11.1 Register AuditLogDao in genossi_bin dependency injection (main.rs)
- [x] 11.2 Wire AuditLogDao into all service impls that use audit macros
- [x] 11.3 Register audit REST endpoints in the router
- [x] 11.4 Run full E2E test suite to verify nothing is broken

## 12. Documentation

- [x] 12.1 Update CLAUDE.md with audit log architecture notes and Auditable trait requirements for new entities
