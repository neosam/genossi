## 1. Database & DAO

- [x] 1.1 Create SQLite migration for audit_timestamp table (id, timestamp, audit_hash, audit_entry_count, tsr_token, webdav_path, status)
- [x] 1.2 Define AuditTimestampEntry struct and AuditTimestampDao trait in genossi_dao (create, get_latest, get_all, get_by_id)
- [x] 1.3 Implement AuditTimestampDao for SQLite in genossi_dao_impl_sqlite
- [x] 1.4 Write unit tests for AuditTimestampDao (insert, query latest, query all)

## 2. RFC 3161 Client

- [x] 2.1 Add dependencies: x509-tsp, der, cms, const-oid, spki (evaluated: x509-tsp is the dedicated RFC 3161 crate from RustCrypto)
- [x] 2.2 Implement TimeStampReq creation: build ASN.1/DER encoded request with SHA256 hash
- [x] 2.3 Implement TimeStampResp parsing: extract status, signed token, and embedded hash from response
- [x] 2.4 Implement HTTP transport: POST to TSA URL with correct Content-Type headers and optional Basic Auth
- [x] 2.5 Write unit tests for request creation (deterministic DER output)
- [x] 2.6 Write unit tests for response parsing (valid response, error response)
- [x] 2.7 Write integration test against a test TSA (e.g. freetsa.org for non-qualified testing)

## 3. Timestamp Service

- [x] 3.1 Define TimestampService trait (create_timestamp, get_all, get_by_id, verify_token, verify_audit_consistency)
- [x] 3.2 Implement create_timestamp: read latest audit hash, check for duplicates, send TSA request, store result
- [x] 3.3 Implement verify_token: validate TSR token signature and check embedded hash matches stored audit_hash
- [x] 3.4 Implement verify_audit_consistency: replay audit_log hash chain up to audit_entry_count, compare with stored hash
- [x] 3.5 Write unit tests for duplicate detection (skip when hash unchanged)
- [x] 3.6 Write unit tests for verification logic (valid, tampered token, hash mismatch, audit log manipulated)

## 4. Timestamp-Worker

- [x] 4.1 Create independent timestamp worker that runs on its own interval (tsa_interval_hours, default 168h = 7 days)
- [x] 4.2 Worker reads tsa_enabled and tsa_interval_hours from config store on each cycle
- [x] 4.3 Worker calls TimestampService::create_timestamp on each cycle
- [x] 4.4 Upload .tsr file to WebDAV `audit-timestamps/` directory after successful TSA response (if WebDAV configured)
- [x] 4.5 Handle TSA failure gracefully: log error, store "tsa_failed" record, continue sleeping
- [x] 4.6 Handle WebDAV upload failure: log warning, store "upload_failed" record, token remains in local DB
- [x] 4.7 Skip entirely when tsa_enabled is false or audit_log is empty
- [x] 4.8 Write tests for timestamp worker (success, TSA failure, disabled, skip scenarios)

## 5. Configuration

- [x] 5.1 Document config keys: tsa_enabled (bool), tsa_url (string), tsa_user (string), tsa_pass (secret), tsa_interval_hours (integer, default 168)
- [x] 5.2 Add config key validation in timestamp service (tsa_url required when tsa_enabled)
- [x] 5.3 Write tests for config validation (missing URL, disabled, complete config)

## 6. REST API

- [x] 6.1 Define TimestampResponse and TimestampVerifyResponse REST types in genossi_rest_types
- [x] 6.2 Implement GET /api/audit/timestamps endpoint (admin-only, list all timestamps)
- [x] 6.3 Implement GET /api/audit/timestamps/{id}/verify endpoint (admin-only, full verification)
- [x] 6.4 Implement POST /api/audit/timestamps endpoint (admin-only, manual timestamp trigger)
- [x] 6.5 Register endpoints in OpenAPI/Swagger documentation
- [x] 6.6 Write E2E tests for timestamp REST endpoints (list, verify, manual trigger, duplicate skip)

## 7. Frontend

- [x] 7.1 Add timestamp REST types to genossi-frontend/rest-types
- [x] 7.2 Create TSA configuration page (tsa_enabled, tsa_url, tsa_user, tsa_pass, tsa_interval_hours)
- [x] 7.3 Add timestamp status section to audit log page (latest timestamp date, hash, status)
- [x] 7.4 Add "Zeitstempel jetzt erstellen" button with feedback (success, no changes, error)
- [x] 7.5 Add timestamp list view with verification button per entry
- [x] 7.6 Display verification results (token valid, hash matches, audit log consistent)
- [x] 7.7 Add i18n translations for timestamp UI elements (de, en)

## 8. Wiring & Integration

- [x] 8.1 Register AuditTimestampDao in genossi_bin dependency injection
- [x] 8.2 Wire TimestampService with AuditTimestampDao and AuditLogDao
- [x] 8.3 Start timestamp worker as independent background task in genossi_bin
- [x] 8.4 Register timestamp REST endpoints (GET list, GET verify, POST trigger) in the router
- [x] 8.5 Run full E2E test suite to verify nothing is broken
