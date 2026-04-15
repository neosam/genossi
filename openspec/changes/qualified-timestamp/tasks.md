## 1. Database & DAO

- [ ] 1.1 Create SQLite migration for audit_timestamp table (id, timestamp, audit_hash, audit_entry_count, tsr_token, webdav_path, status)
- [ ] 1.2 Define AuditTimestampEntry struct and AuditTimestampDao trait in genossi_dao (create, get_latest, get_all, get_by_id)
- [ ] 1.3 Implement AuditTimestampDao for SQLite in genossi_dao_impl_sqlite
- [ ] 1.4 Write unit tests for AuditTimestampDao (insert, query latest, query all)

## 2. RFC 3161 Client

- [ ] 2.1 Add dependencies: der, cms, x509-cert, sha2 (or evaluate if openssl bindings are simpler for RFC 3161)
- [ ] 2.2 Implement TimeStampReq creation: build ASN.1/DER encoded request with SHA256 hash
- [ ] 2.3 Implement TimeStampResp parsing: extract status, signed token, and embedded hash from response
- [ ] 2.4 Implement HTTP transport: POST to TSA URL with correct Content-Type headers and optional Basic Auth
- [ ] 2.5 Write unit tests for request creation (deterministic DER output)
- [ ] 2.6 Write unit tests for response parsing (valid response, error response)
- [ ] 2.7 Write integration test against a test TSA (e.g. freetsa.org for non-qualified testing)

## 3. Timestamp Service

- [ ] 3.1 Define TimestampService trait (create_timestamp, get_all, get_by_id, verify_token, verify_audit_consistency)
- [ ] 3.2 Implement create_timestamp: read latest audit hash, check for duplicates, send TSA request, store result
- [ ] 3.3 Implement verify_token: validate TSR token signature and check embedded hash matches stored audit_hash
- [ ] 3.4 Implement verify_audit_consistency: replay audit_log hash chain up to audit_entry_count, compare with stored hash
- [ ] 3.5 Write unit tests for duplicate detection (skip when hash unchanged)
- [ ] 3.6 Write unit tests for verification logic (valid, tampered token, hash mismatch, audit log manipulated)

## 4. Backup-Worker Integration

- [ ] 4.1 Add timestamp step to backup worker: after regular backup, check tsa_enabled config, run TimestampService::create_timestamp
- [ ] 4.2 Upload .tsr file to WebDAV `audit-timestamps/` directory after successful TSA response
- [ ] 4.3 Handle TSA failure gracefully: log error, store "tsa_failed" record, continue backup cycle
- [ ] 4.4 Handle WebDAV upload failure: log warning, store "upload_failed" record, token remains in local DB
- [ ] 4.5 Skip timestamp step when audit_log is empty or tsa_enabled is false
- [ ] 4.6 Write tests for backup worker with timestamp step (success, TSA failure, skip scenarios)

## 5. Configuration

- [ ] 5.1 Document config keys: tsa_enabled (bool), tsa_url (string), tsa_user (string), tsa_pass (secret)
- [ ] 5.2 Add config key validation in timestamp service (tsa_url required when tsa_enabled)
- [ ] 5.3 Write tests for config validation (missing URL, disabled, complete config)

## 6. REST API

- [ ] 6.1 Define TimestampResponse and TimestampVerifyResponse REST types in genossi_rest_types
- [ ] 6.2 Implement GET /api/audit/timestamps endpoint (admin-only, list all timestamps)
- [ ] 6.3 Implement GET /api/audit/timestamps/{id}/verify endpoint (admin-only, full verification)
- [ ] 6.4 Register endpoints in OpenAPI/Swagger documentation
- [ ] 6.5 Write E2E tests for timestamp REST endpoints

## 7. Frontend

- [ ] 7.1 Add timestamp REST types to genossi-frontend/rest-types
- [ ] 7.2 Add timestamp status section to audit log page (latest timestamp date, hash, status)
- [ ] 7.3 Add timestamp list view with verification button per entry
- [ ] 7.4 Display verification results (token valid, hash matches, audit log consistent)
- [ ] 7.5 Add i18n translations for timestamp UI elements (de, en)

## 8. Wiring & Integration

- [ ] 8.1 Register AuditTimestampDao in genossi_bin dependency injection
- [ ] 8.2 Wire TimestampService into backup worker
- [ ] 8.3 Register timestamp REST endpoints in the router
- [ ] 8.4 Run full E2E test suite to verify nothing is broken
