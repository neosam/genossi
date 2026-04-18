## Meta
- **Priority:** high
- **Category:** bugfix

## Why

Axum hat ein Default-Body-Limit von 2 MB. Die Service-Layer-Limits für Dokument-Uploads (50 MB für Member-Dokumente in `genossi_service_impl/src/member_document.rs:20`, 10 MB für statische Dokumente in `genossi_mail/src/static_document_service.rs:25`) sind höher, werden aber nie erreicht — Axum blockt vorher mit einem Fehler. Uploads über 2 MB schlagen fehl.

## What Changes

- Axum `DefaultBodyLimit` auf den Upload-Routen auf den jeweiligen Service-Layer-Wert hochsetzen:
  - Member-Dokument-Upload (`/api/member/{id}/document`): 50 MB
  - Statische-Dokument-Upload: 10 MB (bzw. `STATIC_DOCUMENTS_MAX_BYTES`)
- Globales Default-Body-Limit bleibt bei 2 MB für alle anderen Endpoints (Defense-in-Depth).

## Capabilities

### New Capabilities

_(keine)_

### Modified Capabilities

- `member-documents`: Upload-Route akzeptiert Bodies bis 50 MB
- `static-documents`: Upload-Route akzeptiert Bodies bis zum konfigurierten Limit

## Impact

**Code:**
- `genossi_rest/src/lib.rs` — Route-spezifisches `DefaultBodyLimit::max()` auf Upload-Routen
- Alternativ: `axum::extract::DefaultBodyLimit` als Layer auf den betroffenen Routen

**Benutzer:**
- Dokument-Uploads über 2 MB funktionieren wieder wie vorgesehen.
