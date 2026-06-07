---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-03
subsystem: rest-api
tags: [axum, http_util, content-disposition, content-type, utoipa, attachments, idor-mitigation, e2e]

# Dependency graph
requires:
  - phase: 19-01
    provides: "InboundMailAttachment entity + InboundMailAttachmentDao::find_by_id_and_mail (T-03 IDOR-safe lookup)"
  - phase: 19-02
    provides: "InboxService::find_attachment + list_attachments; persist_attachment pipeline; InboxServiceImpl<A, St> generic params"
provides:
  - "http_util::content_disposition_inline (mirrors attachment helper, T-02 + T-05 guarantees)"
  - "InboundMailAttachmentTO + InboundMailDetailTO.attachments field (D-07)"
  - "GET /api/inbox/{mail_id}/attachments/{attachment_id} with ?disposition=inline|attachment switch (D-08)"
  - "InboxRestState trait extension (inbox_document_storage + content_disposition_{attachment,inline})"
  - "5 E2E tests covering attachment embed, default + inline disposition, T-03 IDOR, 410 GONE oversized"
affects: [19-04-frontend-components, 19-05-frontend-page-wiring, 19-06-attachment-icon-row]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Trait-accessor trampoline pattern: handler in `genossi_mail` calls `state.content_disposition_*` (provided by `genossi_bin` which delegates to `genossi_rest::http_util`) — avoids circular `genossi_mail → genossi_rest` dependency"
    - "Method-name disambiguation: `inbox_document_storage` (NOT `document_storage`) to avoid clash with `RestStateDef::document_storage` already on `RestStateImpl`"
    - "Two-layer 404 vs 410 semantics: 404 for missing/cross-mail (T-03), 410 GONE for oversized rows where DB row exists but bytes were rejected at receive (D-02 + UI clarity)"
    - "Defense-in-depth disposition default: any value except literal `inline` falls back to `attachment` — invalid values do not 400, they default to the safer disposition"

key-files:
  created: []
  modified:
    - genossi_rest/src/http_util.rs (content_disposition_inline + 4 unit tests)
    - genossi_mail/src/inbox_rest.rs (InboundMailAttachmentTO, extended DetailTO, download_attachment handler, route, OpenAPI, trait extension)
    - genossi_bin/src/lib.rs (impl InboxRestState extended with 3 new methods)
    - genossi_bin/tests/e2e_tests.rs (seed_inbound_mail_attachment helper + 5 E2E tests)

key-decisions:
  - "Trait method named `inbox_document_storage` rather than `document_storage` — `RestStateImpl` implements both `genossi_mail::InboxRestState` AND `genossi_rest::RestStateDef`, and identical method names produce E0034 multiple-applicable-items errors. Rename is harmless because `genossi_mail` does not have an existing `document_storage()` accessor on the trait."
  - "Default disposition falls back to `attachment` for any value other than literal `inline` (not a 400 error). Browsers default to safer disposition on garbled query strings; matches UX of similar endpoints."
  - "Oversized rows return 410 GONE rather than 404 because the row exists — the bytes were rejected at receive (D-02 hard cap). UI consumes 410 as 'visible-but-not-downloadable' marker (different from 'mail does not exist')."
  - "Trait-accessor trampoline for `content_disposition_*` rather than re-implementing or shipping a shared helper crate. Single source of truth stays in `genossi_rest::http_util`; `genossi_bin` is the only crate that imports both `genossi_mail` and `genossi_rest`, so it's the natural trampoline site."
  - "Storage-path encoding test (filename never appears in path) is the responsibility of Plan 19-02's persist_attachment unit tests; this plan focuses on the read path. T-02 mitigation here is the Content-Disposition encoding via http_util helpers."

patterns-established:
  - "Trait-accessor trampoline for crate-dependency-cycle avoidance: when crate B handler needs a function from crate A but A depends on B, the binary crate (which depends on both) implements the trait accessor by delegating to A's helper."
  - "404-vs-410 semantic split for soft-rejected rows: 404 = row missing or wrong tenant (privacy), 410 = row present but resource gone (clarity for UI)."
  - "Disposition-switch handler with safer-by-default fallback: explicit `inline` opt-in, everything else (including invalid query) gets `attachment`."

requirements-completed: []

# Metrics
duration: 12min
completed: 2026-06-07
---

# Phase 19 Plan 03: REST Endpoints Summary

**Download-Endpunkt `GET /api/inbox/{mail_id}/attachments/{attachment_id}` mit `?disposition=inline|attachment`-Switch + `InboundMailDetailTO.attachments` (Embed via `InboxService::list_attachments`) + T-03 IDOR-Guard (cross-mail → 404) + 410 GONE für oversized (D-02). 4 Unit-Tests für `content_disposition_inline` + 5 E2E-Tests grün.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-06-07T10:41:11Z
- **Completed:** 2026-06-07T10:53:01Z
- **Tasks:** 2 (TDD-Cycle für Task 1: RED → GREEN; Task 2 mit 5 e2e tests)
- **Files modified:** 4 (http_util.rs, inbox_rest.rs, lib.rs, e2e_tests.rs)

## Accomplishments

- `content_disposition_inline(filename)` Helper sitzt direkt neben `content_disposition_attachment` in `genossi_rest/src/http_util.rs`, teilt `sanitize_ascii_filename` + `percent_encode_utf8` (DRY) und somit alle T-02/T-05-Garantien
- 4 neue Unit-Tests `test_inline_simple_filename`, `_umlaut_filename`, `_quote_in_filename`, `_newline_in_filename` (T-05 explizit asserted: result enthält weder `\r` noch `\n`)
- Neuer Public-TO `InboundMailAttachmentTO { id, file_name, mime_type, size_bytes, oversized }` + neues Feld `attachments: Vec<InboundMailAttachmentTO>` auf `InboundMailDetailTO` (D-07)
- `to_attachment_to` Free-Function konvertiert Entity → TO; `to_detail_to` Signatur erweitert um `attachments`-Param
- `get_inbox` Handler ruft jetzt zusätzlich `svc.list_attachments(mail_uuid)` und embeddet die TOs
- Neuer Axum-Handler `download_attachment` mit kompletter Auth/Validation-Chain:
  - Path-UUIDs parsen (400 BAD_REQUEST bei Parse-Fail)
  - `state.inbox_service().find_attachment(mail_uuid, att_uuid)` → 404 bei `Ok(None)` (T-03 IDOR delegation an DAO)
  - 410 GONE bei `oversized || relative_path.is_none()` (D-02)
  - `inbox_document_storage().load(rel_path)` → 404 bei `StorageError::NotFound`, 500 sonst
  - `Content-Disposition` via `state.content_disposition_{inline,attachment}` Trait-Accessor (Trampoline-Pattern, T-02)
  - Response: 200 + `Content-Type` (mime_type) + `Content-Disposition` + Body bytes
- Route `/{mail_id}/attachments/{attachment_id}` auf existierender Inbox-Router-Chain registriert (kein neuer Auth-Middleware-Code, D-09 + T-04 erfüllt durch existierenden Vorstand-Funnel)
- `#[utoipa::path]` Annotation: Tag "inbox", drei params (mail_id Path, attachment_id Path, disposition Query Option<String>), Responses 200/401/404/410
- OpenAPI `InboxApiDoc` erweitert um `download_attachment` + `InboundMailAttachmentTO` Schema
- `InboxRestState` Trait erweitert um drei neue Methoden: `inbox_document_storage()`, `content_disposition_attachment(filename)`, `content_disposition_inline(filename)` (Trampoline durch genossi_bin, vermeidet circular `genossi_mail → genossi_rest`-Dependency)
- `RestStateImpl` in `genossi_bin/src/lib.rs` implementiert die 3 neuen Trait-Methoden:
  - `inbox_document_storage` cloned das existierende `document_storage`-Field (gleiche Arc wie `RestStateDef::document_storage`, andere Trait-Methode)
  - `content_disposition_*` delegate an `genossi_rest::http_util::content_disposition_*` (single source of truth)
- `seed_inbound_mail_attachment(pool, mail_id, file_name, mime, bytes, oversized) -> Uuid` Helper in e2e_tests.rs:
  - INSERT in `inbound_mail_attachments` Tabelle mit `relative_path = inbound_mail_attachments/{mail_id}/{att_id}` (NULL wenn oversized=true)
  - Wenn !oversized: tokio::fs::write Bytes in `./documents/{rel_path}` (gleicher Pfad wie `FilesystemDocumentStorage::from_env`)
- **5 neue E2E-Tests** alle grün:
  - `test_get_inbox_detail_includes_attachments` — 2 Attachments (1 normal + 1 oversized) im DetailTO JSON-Body korrekt
  - `test_download_attachment_default_disposition_is_attachment` — GET ohne query → 200 + Content-Type: text/plain + Content-Disposition startet mit "attachment;" + Body == `b"hello world"`
  - `test_download_attachment_inline_query_switches_disposition` — `?disposition=inline` → Disposition startet mit "inline;"
  - `test_download_attachment_cross_mail_returns_404` — `(mail_B_id, attachment_A1_id)` → 404; Positive Control `(mail_A_id, attachment_A1_id)` → 200 (T-03 IDOR)
  - `test_download_attachment_oversized_returns_410` — Row mit `oversized=true, relative_path=NULL` → 410 GONE

## Task Commits

1. **Task 1 RED:** `81e272c` (test) — 4 failing inline-disposition unit tests
2. **Task 1 GREEN:** `0f44d81` (feat) — `content_disposition_inline` helper, 18 http_util tests pass
3. **Task 2:** `85dccce` (feat) — DetailTO extension, download_attachment handler, route, OpenAPI, trait extension, RestStateImpl impl, 5 E2E tests

## Files Created/Modified

- `genossi_rest/src/http_util.rs` — +55 LOC (helper-Fn 14, Tests 41 LOC)
- `genossi_mail/src/inbox_rest.rs` — +147 / -7 LOC (TO, Converter, extended to_detail_to, extended get_inbox, DispositionQuery, download_attachment Handler, Route, OpenAPI, Trait extension)
- `genossi_bin/src/lib.rs` — +19 / -0 LOC (3 neue Methoden auf `impl InboxRestState for RestStateImpl`)
- `genossi_bin/tests/e2e_tests.rs` — +233 / -0 LOC (seed_inbound_mail_attachment + 5 Tests)

## Decisions Made

- **Trait-Methode heißt `inbox_document_storage`, NICHT `document_storage`:** rustc lehnt mit E0034 ab, wenn `RestStateImpl` zwei Traits (`InboxRestState` + `RestStateDef`) mit identisch benannten Methoden implementiert. Pure Naming-Disambiguation, kein semantischer Unterschied. Member_document.rs (in `genossi_rest`) nutzt `RestStateDef::document_storage` weiterhin direkt.
- **Default `?disposition`-Wert fällt auf `attachment` zurück (kein 400 bei invalid value):** `match q.disposition.as_deref() { Some("inline") => …, _ => attachment }`. Sichereres Default-Verhalten — invalid query strings landen auf der konservativen Variante.
- **Oversized = 410 GONE (nicht 404):** Plan-Vorgabe per D-08; semantische Trennung von "Row missing" (404) und "Row exists, bytes wurden bei IMAP-poll wegen 10MB-Cap abgelehnt" (410). Frontend kann diesen Unterschied visualisieren ("rejected" Badge statt einfach "not found").
- **Trampoline-Pattern für `content_disposition_*`:** `genossi_mail` darf nicht `genossi_rest` importieren (circular dep), also leitet `genossi_bin` (das beide importiert) die Aufrufe weiter. Single source of truth bleibt `genossi_rest::http_util` — kein Code-Duplikat, keine Cargo.toml-Änderungen.
- **`seed_inbound_mail_attachment` schreibt direkt in `./documents/`:** Default-Pfad von `FilesystemDocumentStorage::from_env`. Existierende e2e tests (z.B. `test_static_document_*`) verwenden das gleiche Pattern; keine TempDir-Isolation nötig weil Pfade pro Test-Run unique UUIDs enthalten.

## Deviations from Plan

**Eine Abweichung — Trait-Methoden-Naming:**

- **[Rule 3 - Blocking] `document_storage()` Methode in `InboxRestState` umbenannt zu `inbox_document_storage()`**
  - **Found during:** Task 2 Step 7-8 (cargo check)
  - **Issue:** `RestStateImpl` implementiert bereits `genossi_rest::lib::RestStateDef` mit eigener Methode `fn document_storage(&self) -> Arc<Self::DocumentStorage>`. Das Hinzufügen einer gleichnamigen Methode auf `InboxRestState` erzeugt E0034 multiple-applicable-items-Fehler an mehreren Stellen in `genossi_rest/src/member_document.rs` (Zeilen 249, 395), wo `rest_state.document_storage()` aufgerufen wird.
  - **Fix:** Trait-Methode in `InboxRestState` zu `inbox_document_storage` umbenannt. `download_attachment` Handler ruft jetzt `state.inbox_document_storage()` auf. Existierender `RestStateDef::document_storage` bleibt unverändert; `member_document.rs` weiterhin grün.
  - **Files modified:** `genossi_mail/src/inbox_rest.rs`, `genossi_bin/src/lib.rs`
  - **Commit:** Teil von `85dccce`

Sonst lief der Plan exakt wie geschrieben. Eine kleine Klarstellung: Plan 19-02 hatte das `inbox_attachment_dao` schon in den InboxService verdrahtet (via `InboxServiceImpl::new(...)` ctor-arg), also war der Service-Aufruf `state.inbox_service().find_attachment(mail, att)` in Task 2 unmittelbar verfügbar — kein zusätzliches DAO-Field auf `RestStateImpl` nötig. Die 3 neuen Trait-Methoden bekommen alles aus existierenden `RestStateImpl`-Feldern (`document_storage`, `genossi_rest::http_util`-Free-Functions).

## Issues Encountered

- **E0034 multiple-applicable-items für `document_storage()`:** siehe Deviation oben, durch Umbenennung gelöst.
- **Pre-existing test failure `test_mail_preview_repayment_no_entries_does_not_default_to_one`:** beim vollen `cargo test -p genossi_bin --test e2e_tests` Lauf failed dieser Test mit "errors must be array" — bekannter Pre-Existing-Failure aus `PROJECT.md` Tech-Debt-Section ("Mail-Subsystem-Triage — pre-existing failure seit Quick-c19"). NICHT durch Plan 19-03 verursacht; 298/299 Tests grün, der eine Failure ist unverändert seit `c19`.

## User Setup Required

Keine — keine neuen Env-Variablen, keine Migration (Plan 19-01 hat die `inbound_mail_attachments` Tabelle bereits angelegt), keine OIDC/IMAP-Config. Der Endpunkt geht live mit dem nächsten `cargo run --bin genossi`.

## Next Phase Readiness

- **Ready for Plan 19-04 (Backfill worker):** N/A — Backfill-Worker nutzt `InboxImapClient::fetch_one_by_uid` (Plan 19-02) und `persist_attachment` (Plan 19-02). REST-Layer hat damit nichts zu tun.
- **Ready for Plan 19-05 (Frontend components):** `GET /api/inbox/{id}` liefert nun `attachments: [{ id, file_name, mime_type, size_bytes, oversized }]` — direkter Input für Attachment-Icon-Row-Component. `oversized: true` ist der Marker für "rejected"-Badge ohne Download-Link.
- **Ready for Plan 19-06 (Frontend page wiring):** Download-URL ist `/api/inbox/{mail_id}/attachments/{attachment_id}` (default disposition) oder mit `?disposition=inline` für PDF-Preview im iframe. Beide Modi single endpoint, response identisch in body, unterschiedlich nur im `Content-Disposition` Header.

## Self-Check: PASSED

- `pub fn content_disposition_inline`: 1 occurrence in `genossi_rest/src/http_util.rs`
- `inline; filename=`: 2 occurrences (helper format-string + assertion)
- `test_inline_simple_filename|_umlaut|_quote|_newline`: 4 occurrences
- `cargo test -p genossi_rest http_util`: 18 passed / 0 failed
- `pub struct InboundMailAttachmentTO`: 1 occurrence in `genossi_mail/src/inbox_rest.rs`
- `pub attachments: Vec<InboundMailAttachmentTO>`: 1 occurrence
- `fn to_attachment_to`: 1 occurrence
- `async fn download_attachment`: 1 occurrence
- `/{mail_id}/attachments/{attachment_id}`: 2 occurrences (route registration + `#[utoipa::path]`)
- `StatusCode::GONE`: 1 occurrence (oversized → 410)
- `DispositionQuery`: 2 occurrences (struct decl + extractor type)
- `audited_create|audited_update|audited_delete` in `inbox_rest.rs`: 0 occurrences (D-10 enforced — read endpoint, no audit)
- `inbox_document_storage`: 2 occurrences in `inbox_rest.rs` (trait + handler call), 1 occurrence in `genossi_bin/src/lib.rs` (impl)
- `InboundMailAttachmentDaoType` in `genossi_bin/src/lib.rs`: 5 occurrences (already there from Plan 19-02 wiring; no new occurrences needed)
- `cargo check -p genossi_rest -p genossi_mail -p genossi_bin`: exits 0
- `cargo test -p genossi_bin --test e2e_tests test_get_inbox_detail_includes_attachments test_download_attachment_default_disposition_is_attachment test_download_attachment_inline_query_switches_disposition test_download_attachment_cross_mail_returns_404 test_download_attachment_oversized_returns_410`: 5 passed / 0 failed
- Commit `81e272c` exists (RED): confirmed via `git log`
- Commit `0f44d81` exists (GREEN): confirmed via `git log`
- Commit `85dccce` exists (Task 2): confirmed via `git log`

## TDD Gate Compliance

- **RED gate:** `81e272c` (`test(19-19-03): add failing inline-disposition tests`) — 4 tests compile-fail because helper not yet exists
- **GREEN gate:** `0f44d81` (`feat(19-19-03): add content_disposition_inline helper`) — helper added, all 18 http_util tests pass
- **REFACTOR gate:** Not required — helper is intentionally a minimal mirror of the attachment-variant; no cleanup necessary.

Plan 19-03 RED/GREEN gates correctly sequenced in git log.

---
*Phase: 19-e-mail-anhaenge-anzeigen*
*Plan: 19-03-rest-endpoints*
*Completed: 2026-06-07*
