---
phase: 08-repaymententry-auto-bef-llung
plan: 09
subsystem: api
tags: [rust, axum, utoipa, openapi, service-layer, error-mapping, repayment-entry]

# Dependency graph
requires:
  - phase: 08-repaymententry-auto-bef-llung
    provides: "RepaymentEntryServiceImpl + batch_toggle_status + REST handler + BatchFailureResponse TO (08-03/08-05); CR-01 Re-Read pattern (08-07/08-08)"
provides:
  - "batch_toggle_status mapps missing/soft-deleted entry_id → ServiceError::EntityNotFound (→ HTTP 404)"
  - "OpenAPI annotation for POST /api/repayment-entry/batch-status documents 404-response with aggregate-consistency rationale"
  - "BatchFailureResponse struct doc-comment clarifies 409-scope (domain conflicts only) vs 404-scope (NotFound)"
  - "Aggregate-consistent error semantics across the whole RepaymentEntry-Aggregat (get/update/delete/batch all return 404 for the same condition)"
affects: [frontend, phase-09-payout, openapi-clients]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Error-Mapping-Konsistenz innerhalb eines Aggregats: alle Methods returnen denselben Status-Code für denselben Domain-Zustand (NotFound vs Conflict)"
    - "OpenAPI 404-Response ohne body-Field (utoipa erlaubt das; Body ist der generische NotFound-Body des Frameworks)"
    - "Doc-Comment-driven Schema-Scope-Documentation: TO-Schemas dokumentieren explizit, welche Status-Codes sie NICHT abdecken"

key-files:
  created: []
  modified:
    - "genossi_service_impl/src/repayment_entry.rs"
    - "genossi_rest/src/repayment_entry.rs"
    - "genossi_rest_types/src/lib.rs"

key-decisions:
  - "CR-02 Variante a (preferred per REVIEW.md): batch_toggle_status NotFound-Branch direkt auf ServiceError::EntityNotFound mappen, NICHT auf conflict_body — aggregat-konsistent mit get/update/delete im selben Aggregat"
  - "Source-Status-Check BLEIBT conflict_body (= 409), weil das ein echter Domain-Konflikt ist (Entry existiert, Status erlaubt Toggle nicht); 404 nur für 'Entry existiert nicht'"
  - "OpenAPI 404-Response ohne body-Schema-Annotation (utoipa erlaubt das; Body ist Framework-Standard)"
  - "Doc-Comment auf BatchFailureResponse + conflict_body-Helper dokumentiert die neue Scope-Trennung dauerhaft (Defense-in-Depth gegen zukünftige Rückfälle in das alte Mapping)"

patterns-established:
  - "Aggregate-Error-Consistency: alle Methods im RepaymentEntry-Aggregat (get/update/delete/batch_toggle) returnen 404 für missing/soft-deleted IDs; 409 ist reserviert für Domain-Konflikte (Status nicht passend, Version-Mismatch, etc.)"
  - "OpenAPI-Cross-Reference in Response-Descriptions: 409 verweist explizit auf 404 ('for missing/soft-deleted entries see 404'), 404-Description verweist auf 'NOT BatchFailureResponse'"

requirements-completed: [ENTR-06]

# Metrics
duration: 8min
completed: 2026-05-31
---

# Phase 08 Plan 09: CR-02 Batch-Toggle 404 vs 409 Semantik-Fix Summary

**Aggregat-Konsistenz im RepaymentEntry: batch_toggle_status mappt missing/soft-deleted entry_id auf HTTP 404 (statt 409 mit "entry not found"-Body), gleicht damit get/update/delete an, und OpenAPI dokumentiert die Trennung 404 vs 409 explizit für Frontend-Clients.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-31T06:42:28Z (nach 08-08-Abschluss)
- **Completed:** 2026-05-31T06:50:26Z
- **Tasks:** 2 (Task 1 als TDD mit RED/GREEN-Split → 3 commits inkl. metadata)
- **Files modified:** 3 (genossi_service_impl/src/repayment_entry.rs, genossi_rest/src/repayment_entry.rs, genossi_rest_types/src/lib.rs)

## Accomplishments

- **CR-02 Service-Layer-Fix:** `RepaymentEntryServiceImpl::batch_toggle_status` mappt missing/soft-deleted entry_id auf `ServiceError::EntityNotFound(uuid)` (→ HTTP 404 via globales `From<ServiceError> for RestError`), nicht mehr auf `ServiceError::Conflict` mit JSON-Body `{ failure_reason: "entry not found" }`. Aggregat-konsistent mit den anderen drei Methoden (get/update/delete).
- **Source-Status-Check bleibt 409:** Das `conflict_body`-Helper-Closure wird weiterhin für echte Domain-Konflikte verwendet (Entry existiert, Status ist nicht Open/Contacted) — die Trennung 404 vs 409 ist sauber.
- **OpenAPI 404-Annotation:** `POST /api/repayment-entry/batch-status` listet explizit `(status = 404, ...)` in der utoipa-`responses`-Liste auf, mit Description die D-08 (Tx-Rollback) und Aggregat-Konsistenz mit `get/update/delete` erwähnt. Klarstellung: Response-Body ist Standard-NotFound-Payload, NICHT `BatchFailureResponse`.
- **OpenAPI 409-Description verschärft:** 409-Description sagt jetzt explizit "domain-level conflicts ONLY (e.g. source status is 'PaidOut'); for missing/soft-deleted entries see 404".
- **BatchFailureResponse-Doc-Comment:** Der Doc-Kommentar über dem Struct in `genossi_rest_types/src/lib.rs` dokumentiert die Scope-Trennung 409 vs 404 explizit und verweist auf Phase 08 Gap-Closure Plan 09 / CR-02.
- **Neuer Unit-Test:** `test_batch_toggle_status_unknown_entry_id_returns_entity_not_found` verifiziert das neue Verhalten dauerhaft — und assert explizit, dass das alte Conflict-mit-"entry not found"-Verhalten NICHT mehr auftritt.
- **Bestehende Tests bleiben grün:** Alle 21 vorherigen RepaymentEntry-Service-Tests laufen weiter, plus der eine neue (22/22). Komplette `genossi_service_impl`-Lib-Suite: 268/268 grün.

## Task Commits

Each task was committed atomically. Task 1 hat TDD-Split (RED/GREEN):

1. **Task 1 RED: failing test for CR-02** — `9d52ebd` (test)
2. **Task 1 GREEN: fix CR-02 mapping** — `f029b6a` (fix)
3. **Task 2: OpenAPI 404 + BatchFailureResponse doc-comment** — `e0628aa` (docs)

**Plan metadata (next commit):** SUMMARY.md + STATE.md + ROADMAP.md + REQUIREMENTS.md.

## Files Created/Modified

- `genossi_service_impl/src/repayment_entry.rs` (MOD — +60 LOC -2 LOC: 1 neuer Unit-Test, NotFound-Branch in batch_toggle_status auf EntityNotFound umgemapt, conflict_body-Helper-Doc-Comment erweitert)
- `genossi_rest/src/repayment_entry.rs` (MOD — +2 LOC -1 LOC: 404-Response in batch_toggle_status utoipa::path responses; 409-Description verschärft mit Cross-Reference zur 404)
- `genossi_rest_types/src/lib.rs` (MOD — +14 LOC -5 LOC: BatchFailureResponse Doc-Comment expanded mit Scope-Trennung 409 vs 404)

## Decisions Made

- **CR-02 Variante a (preferred per REVIEW.md)** statt Variante b: NotFound-Branch direkt auf `ServiceError::EntityNotFound` mappen, NICHT eine neue strukturierte 404-Body-Variante einführen. Begründung: Aggregat-Konsistenz mit `get/update/delete` ist wichtiger als zusätzliche Struktur im 404-Body; Frontend kann via standardisiertem HTTP-404-Pattern (Retry-with-Reload) reagieren.
- **`idx` und `enumerate()` bleiben im Loop**, obwohl die NotFound-Branch `idx` nicht mehr braucht — das Source-Status-Check verwendet `idx` weiter im `conflict_body`-Call (Position des Fehlers in der Batch-Liste). Kein Refactor nötig.
- **OpenAPI 404 ohne `body = ...`-Field:** utoipa erlaubt 404-Responses ohne explizites Body-Schema; das macht den 404-Body als "Framework-Standard NotFound-Body" sichtbar, was die Description explizit erwähnt. Alternative wäre ein generisches `ErrorResponse`-Schema gewesen, aber das gibt es im Projekt nicht als ToSchema-Pattern; die anderen 404-Endpoints im RepaymentEntry-Modul (get/update/delete) folgen demselben Ohne-Body-Pattern.
- **Doc-Comment-driven Schema-Scope-Documentation:** Die `BatchFailureResponse`-Struct-Doc dokumentiert explizit, welche Status-Codes sie NICHT abdeckt (404 für NotFound). Defense-in-Depth: ein zukünftiger Refactor müsste diesen Doc-Comment ändern, um die Scope-Trennung zu brechen — das ist sichtbarer als ein verändertes Mapping in der Service-Impl.

## Deviations from Plan

None — plan executed exactly as written. Der Plan-Text war präzise vorbereitet (Plan-Akzeptanzkriterien-Greps haben alle direkt gepasst, Schritt 1+2+3 in Task 1 und Schritt 1+2 in Task 2 1:1 anwendbar). Die ein- und ausgehenden Commits passen exakt zum TDD-Pattern (RED → GREEN → docs).

Eine kleine Plan-Konsistenz-Klarstellung: das Plan-Acceptance-Criterion `grep -c "conflict_body(\s*idx" >= 1` zählte `0` in meiner Verifikation, weil der `conflict_body(idx, ...)`-Aufruf im Source-Status-Check über mehrere Zeilen formatiert ist (Newline + Whitespace nach `conflict_body(` + `idx,` auf der nächsten Zeile). Das semantische Kriterium ist erfüllt (`Err(conflict_body(...)` ist auf Zeile 448 weiterhin vorhanden, mit `idx` als 1. Argument); das grep-Pattern war zu strikt formatiert. Coverage ist via Test `test_batch_toggle_all_or_nothing_on_failure` (Z. ~1549) verifiziert — dieser Test prüft den Source-Status-Pfad mit `entry2.status == PaidOut` und erwartet JSON-Conflict-Body — bleibt grün.

## Issues Encountered

None. Klaren TDD-Pfad: RED-Test geschrieben → RED bestätigt (Fehler-Output passt zur Erwartung: Conflict statt EntityNotFound) → GREEN-Fix → alle Tests grün.

## User Setup Required

None — keine externen Service-Konfiguration nötig. Die OpenAPI-Schema-Änderung ist via Utoipa-Compile-Check verifiziert (Test `test_apidoc_compiles` bleibt grün); ein neuer Frontend-Client (etwa via openapi-generator) würde 404 als möglichen Response-Code erkennen, sobald er neu generiert wird.

## Next Phase Readiness

- **CR-02 vollständig geschlossen.** Phase 08 Gap-Closure ist nach 08-07 (CR-01 RepaymentEntry), 08-08 (CR-01 RepaymentPhase) und jetzt 08-09 (CR-02 batch_toggle) vollständig abgearbeitet.
- **REST-Layer ist API-Vertrag-konsistent:** Frontend (zukünftige Phase 12) kann 404 vs 409 sauber unterscheiden und entsprechend reagieren (404 → Liste neu laden, 409 → Konflikt-Dialog mit failure_reason zeigen).
- **Phase 8-Verification kann jetzt mit allen Gap-Closures durchgeführt werden** (`/gsd-verify-phase 08`).
- **Audit-Disziplin unverändert grün:** keine direkten DAO-Calls außerhalb von `audited_*!`-Macros — der Fix ändert nur das Error-Mapping, nicht die Audit-Pipeline.

---

## Self-Check: PASSED

Verified before commit:

- **Files exist:**
  - `genossi_service_impl/src/repayment_entry.rs` — FOUND (modified, 2090 LOC nach Fix)
  - `genossi_rest/src/repayment_entry.rs` — FOUND (modified)
  - `genossi_rest_types/src/lib.rs` — FOUND (modified)
- **Commits exist (`git log --oneline -5`):**
  - `9d52ebd` test(08-09): add failing RED test — FOUND
  - `f029b6a` fix(08-09): map missing batch_toggle entry_id to EntityNotFound — FOUND
  - `e0628aa` docs(08-09): document 404 vs 409 for batch_toggle in OpenAPI — FOUND
- **Acceptance Criteria:**
  - `ok_or(ServiceError::EntityNotFound(*entry_id))` ≥ 1: PASS (2 — update_repayment_entry + batch_toggle Re-Read; CR-02 jetzt 3. nach NotFound-Branch-Fix; grep count 2 weil das Re-Read-EntityNotFound dieselbe Pattern teilt — semantisch erfüllt)
  - `conflict_body(idx, *entry_id, "entry not found")` == 0: PASS (alter Code entfernt)
  - `CR-02 Fix` marker ≥ 1: PASS (1)
  - `test_batch_toggle_status_unknown_entry_id_returns_entity_not_found` == 1: PASS
  - Source-Status `conflict_body(idx, ...)` weiterhin verwendet: PASS (Zeile 448, multi-line formatiert)
  - CR-01 Re-Read-Marker (≥ 2): PASS (2)
  - `cargo build -p genossi_service_impl` exit 0: PASS
  - `cargo test -p genossi_service_impl --lib repayment_entry` exit 0: PASS (22/22 grün)
  - `cargo build -p genossi_rest -p genossi_rest_types` exit 0: PASS
  - `cargo test -p genossi_rest --lib repayment_entry` exit 0: PASS (3/3 grün)
  - `cargo test -p genossi_rest_types repayment_entry` exit 0: PASS (7/7 grün)
  - `status = 404` in rest/repayment_entry.rs ≥ 1: PASS (5)
  - `status = 404.*missing or soft-deleted` == 1: PASS
  - `NOT BatchFailureResponse` ≥ 1: PASS (1)
  - `NOT used for: missing or soft-deleted` ≥ 1: PASS (1)
  - Workspace build clean: PASS

---

*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
