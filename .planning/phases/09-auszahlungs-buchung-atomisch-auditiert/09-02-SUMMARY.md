---
phase: 09-auszahlungs-buchung-atomisch-auditiert
plan: 02
subsystem: rest-layer
tags: [rest, openapi, action-endpoint, no-body, axum, utoipa, swagger]

# Dependency graph
requires:
  - phase: 09-auszahlungs-buchung-atomisch-auditiert
    plan: 01
    provides: "RepaymentEntryService::mark_paid_out Trait-Methode + RepaymentEntryServiceImpl mit 12-Schritt-Cascade + MockRepaymentEntryService::expect_mark_paid_out"
  - phase: 08-repaymententry-auto-bef-llung
    provides: "RepaymentEntryRestState-Trait + bestehende REST-Handler-Struktur + RepaymentEntryTO-Response-DTO + Router-Mount in genossi_rest/src/lib.rs"
  - phase: 07-repaymentphase-backend-foundation
    provides: "Action-Endpoint-Pattern (kein Body, kein Version-Body-Feld) via open_repayment_phase / close_repayment_phase"
provides:
  - "POST /api/repayment-entry/{id}/mark-paid-out Axum-Route"
  - "mark_paid_out Handler mit utoipa::path-Annotation (alle 5 Status-Codes 200/400/401/404/409/500)"
  - "ApiDoc paths-Eintrag fuer mark_paid_out (Swagger-UI-Sichtbarkeit)"
affects: [09-03-wiring, 09-04-e2e, 12-frontend-confirm-dialog]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Action-Endpoint ohne Request-Body (Phase-7-D-02/D-03-Pattern wortgleich uebernommen)"
    - "utoipa::path mit allen 5 Status-Codes (200/400/401/404/409/500) per CONTEXT D-06"
    - "Globales From<ServiceError> for RestError reicht — kein lokaler map_*_error-Override"

key-files:
  created: []
  modified:
    - "genossi_rest/src/repayment_entry.rs (+70 LOC: mark_paid_out-Handler + Route-Registration + ApiDoc-Erweiterung)"

key-decisions:
  - "D-07 Single-only umgesetzt: EXAKT EIN Action-Endpoint /{id}/mark-paid-out, KEINE Batch-Variante (Cascade ist sicherheitskritisch/irreversibel, Confirm-Dialog UI-05 pro Eintrag; Batch deferred zu Phase 12). Sanity-Grep auf batch-mark-paid-out-Pattern: 0."
  - "D-03 kein Request-Body, kein Version-Body-Feld — Action-Endpoint-Pattern wortgleich aus open_repayment_phase uebernommen. Concurrency-Defense laeuft ueber Entry-Status-Guard + Version-Check im DAO-UPDATE (siehe 09-RESEARCH Frage 1)."
  - "D-05 Response-Body = RepaymentEntryTO mit aktualisierter version aus Re-Read; KEIN MemberTO/MemberActionTO. Schmales Response-Schema."
  - "D-06 alle 5 Status-Codes in OpenAPI-Annotation: 200 (RepaymentEntryTO), 400 (PAYO-03), 401 (Unauthorized), 404 (Entry not found), 409 (PAYO-04 / Phase-Status / Version-Race), 500 (BL-01 Re-Read-None)."
  - "Reihenfolge der neuen Route am Ende der generate_route()-Builder-Chain (Konvention, nicht funktional erzwungen — Axum matcht /{id}/mark-paid-out spezifischer als /{id})."

patterns-established:
  - "Action-Endpoint /{id}/<action>-<verb> ohne Body + utoipa::path mit allen relevanten Status-Codes; Vorbild fuer kuenftige Lifecycle-Actions auf RepaymentEntry/Application/Member"

requirements-completed: []  # PAYO-01, PAYO-04 werden erst nach E2E in 09-04 markiert (per Konvention)

# Metrics
duration: ~2min
completed: 2026-05-31
---

# Phase 9 Plan 02: REST-Exposure mark_paid_out Summary

**Ein neuer Axum-Handler `mark_paid_out` exposed den Plan-09-01-Cascade-Service unter `POST /api/repayment-entry/{id}/mark-paid-out` mit kompletter OpenAPI-Dokumentation aller 5 Status-Codes.**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-05-31T10:26:48Z
- **Completed:** 2026-05-31T10:28:52Z
- **Tasks:** 1 (T1: Handler + Route + ApiDoc)
- **Files modified:** 1

## Accomplishments

- **`mark_paid_out` Handler** in `genossi_rest/src/repayment_entry.rs` implementiert: `pub async fn mark_paid_out<RestState: RestStateDef + RepaymentEntryRestState>(...) -> Response` mit `#[instrument(skip(rest_state))]` und `#[utoipa::path(post, ...)]` Annotation. Delegiert direkt an `rest_state.repayment_entry_service().mark_paid_out(id, auth)` (Plan 09-01-Cascade).
- **Route-Registration:** `.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))` am Ende der `generate_route()`-Builder-Chain angehaengt. Single-only — keine Batch-Variante (D-07).
- **ApiDoc-Erweiterung:** `mark_paid_out` in `ApiDoc::paths(...)`-Liste als 7. Eintrag aufgenommen mit Inline-Kommentar `// Phase 9 (PAYO-01):`. Swagger-UI zeigt den neuen Endpoint unter Tag `RepaymentEntries`.
- **Status-Code-Vollstaendigkeit:** alle 5 erwarteten Codes in der utoipa-Annotation dokumentiert mit aussagekraeftigen Descriptions:
  - 200 OK + `RepaymentEntryTO` (Cascade-Beschreibung im Description-Text)
  - 400 Bad Request (PAYO-03 ValidationError mit Feldnamen)
  - 401 Unauthorized (missing/invalid admin auth)
  - 404 Not Found (Entry not found or soft-deleted)
  - 409 Conflict (PAYO-04 final, Phase-Status, oder Version-Race)
  - 500 Internal Server Error (BL-01 Re-Read-None Pattern)
- **Kein Request-Body** in utoipa-Annotation (D-03 / Action-Endpoint-Pattern; Pitfall #8 vermieden).
- **Kein lokaler `map_*_error`-Override** — globales `From<ServiceError> for RestError` (`genossi_rest/src/lib.rs:97-113`) deckt alle 5 Mappings ab (Phase-7/8-Pattern-Konsistenz).

## Task Commits

1. **Task 1: Handler + Route + ApiDoc** — `a746db0` (feat)
   - 3 Edits in `genossi_rest/src/repayment_entry.rs`: Handler-Funktion zwischen `batch_toggle_status` und `generate_route`; neue Route-Zeile am Builder-Chain-Ende; `mark_paid_out`-Eintrag in `ApiDoc::paths`.
   - +70 LOC ohne Loeschungen.

## Files Created/Modified

- `genossi_rest/src/repayment_entry.rs` — +70 LOC (1 neuer Handler `mark_paid_out` mit utoipa::path-Annotation und Inline-Doc-Kommentar zur Phase-9-D-07-Begruendung; 1 neue Route in `generate_route()`; 1 neuer Eintrag in `ApiDoc::paths()`-Liste).

## Verification

**Build:**

```text
$ cargo build -p genossi_rest 2>&1 | tail -5
warning: `genossi_rest` (lib) generated 2 warnings (run `cargo fix --lib -p genossi_rest` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.50s
```

`cargo build -p genossi_rest` ist clean (nur pre-existing warnings: `unused import: put` in `permission.rs:780` und `unused import: response::IntoResponse` in `lib.rs:32` — beide nicht durch Plan 09-02 eingefuehrt).

**Tests:**

```text
$ cargo test -p genossi_rest --lib repayment_entry
test repayment_entry::tests::test_list_query_deserializes_from_json ... ok
test repayment_entry::tests::test_create_request_deserializes ... ok
test repayment_entry::tests::test_apidoc_compiles ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out
```

3/3 Tests gruen, kein Regress. `test_apidoc_compiles` ist die wichtigste Regression-Sicherung — sie verifiziert dass `ApiDoc::openapi()` mit der neuen `paths(..., mark_paid_out)`-Liste sauber compiliert (jeder Eintrag muss eine `#[utoipa::path]`-annotierte Funktion mit passender Signatur referenzieren).

**Acceptance-Criteria-Greps:**

| Grep | Result | Expected |
|------|--------|----------|
| `pub async fn mark_paid_out<RestState` | 1 | ≥ 1 (Handler-Signatur) |
| `path = "/{id}/mark-paid-out"` | 1 | ≥ 1 (utoipa-path) |
| `tag = "RepaymentEntries"` | 7 | ≥ 1 (existierende + neuer Handler) |
| `.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))` | 1 | = 1 (Route-Registration) |
| `(batch.*mark.paid.out\|mark.paid.out.*batch)` (D-07 Sanity) | 0 | = 0 (keine Batch-Variante) |
| `status = (200\|400\|401\|404\|409\|500)` (Datei-weit) | 28 | ≥ 5 (neuer Handler addiert 6: 200/400/401/404/409/500) |
| `mark_paid_out,` (ApiDoc-Liste mit Komma) | 1 | ≥ 1 (paths-Eintrag) |
| `request_body` im `mark_paid_out`-Handler-Block | 0 | = 0 (D-03 / Action-Endpoint) |

Alle Acceptance-Criteria erfuellt.

**Endpoint-Topologie-Verifikation:**

```text
$ grep -E '^        .route\("' genossi_rest/src/repayment_entry.rs
        .route("/batch-status", post(batch_toggle_status::<RestState>))
        .route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))
```

`/batch-status` bleibt VOR `/{id}` (T-08-05-02 Mitigation aus Phase 8), `/{id}/mark-paid-out` am Ende (Konvention).

**genossi_bin Build-Status (erwartet weiterhin fehlschlagend bis Plan 09-03):**

Diese Plan-Ausfuehrung hat den `genossi_bin`-Compile-Zustand nicht beruehrt — Plan 09-03 wird das DI-Wiring (8. Konstruktor-Parameter `member_action_dao` an `RepaymentEntryServiceImpl::new`) anlegen. Bis dahin ist `cargo build -p genossi_rest` und `cargo test -p genossi_rest --lib` der gueltige Verifikations-Befehl, beide gruen.

## Decisions Made

- **Reihenfolge der neuen Route ans Ende der Builder-Chain** (statt z.B. zwischen `/batch-status` und `/`). Begruendung: Axum matcht nach Pfad-Spezifitaet, `/{id}/mark-paid-out` ist spezifischer als `/{id}` — Reihenfolge ist funktional egal. Konvention der Codebase (vgl. `repayment_phase.rs::generate_route` mit `/{id}/open` und `/{id}/close` am Ende) setzt Action-Endpoints ans Ende; folgen wir.
- **Inline-Block-Kommentar mit D-07-Begruendung** ueber der Handler-Funktion und ueber der Route-Zeile. Begruendung: Die D-07-Entscheidung ("keine Batch-Variante in Phase 9") ist nicht offensichtlich aus dem Code lesbar; der Kommentar verhindert dass spaetere Refactoring-Versuche unbedacht eine `/batch-mark-paid-out`-Route hinzufuegen.
- **OpenAPI-Status-Code-Beschreibungen detailliert mit Cascade-Erklaerung** (statt nur "OK"/"Conflict"). Begruendung: Phase-12-Frontend liest die OpenAPI-Doku als Vertrag fuer den Confirm-Dialog UI-05; die Beschreibungen erklaeren explizit dass 409 drei Sub-Faelle abdeckt (PAYO-04 final, Phase-Status, Version-Race) und 500 ein BL-01-Pattern ist (kein User-Bug).
- **`#[instrument(skip(rest_state))]` analog zu bestehenden Handlern** — kein Skip auf `context` (kleine Datenmenge) und `id` (Trace-relevant fuer Audit-Korrelation).

## Deviations from Plan

None — Plan wurde 1:1 ausgefuehrt.

- Edit 1 (Handler-Funktion): genau wie im Plan-Action-Block spezifiziert (~35 LOC + Inline-D-07-Kommentar zusaetzlich).
- Edit 2 (Route-Registration): genau wie spezifiziert (`.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))` am Ende der Chain, mit Inline-Kommentar-Block der D-07/Reihenfolge-Konvention dokumentiert).
- Edit 3 (ApiDoc): genau wie spezifiziert (`mark_paid_out,` nach `batch_toggle_status,` mit Kommentar `// Phase 9 (PAYO-01):`).

## Issues Encountered

Keine. Erster `cargo build -p genossi_rest` direkt gruen, erster `cargo test -p genossi_rest --lib repayment_entry` direkt 3/3 gruen.

## User Setup Required

None — keine externe Service-Konfiguration noetig.

## Next Phase Readiness

- **Plan 09-03 (DI-Wiring in `genossi_bin`):** Der REST-Handler ruft `rest_state.repayment_entry_service().mark_paid_out(...)` auf — die Trait-Methode existiert seit 09-01 in `RepaymentEntryService`, automock erzeugt `MockRepaymentEntryService::expect_mark_paid_out` automatisch. Plan 09-03 muss `member_action_dao` als 8. Argument an `RepaymentEntryServiceImpl::new(...)` in `genossi_bin/src/lib.rs` weiterreichen. Erst dann compiliert `cargo build -p genossi_bin`. Die REST-Route ist schon registriert; sobald `genossi_bin` baut, kann der Server gestartet werden und Swagger-UI zeigt `POST /api/repayment-entry/{id}/mark-paid-out`.
- **Plan 09-04 (E2E-Tests):** Lebender HTTP-Endpoint ist vorbereitet. Die 4 geplanten E2E-Tests (Happy-Cascade, PAYO-03-ValidationError, PAYO-04-Double-mark-paid-out, Race-via-tokio::join) koennen direkt gegen `client.post(server.url(&format!("/api/repayment-entry/{}/mark-paid-out", entry_id))).send()` arbeiten.
- **Plan 12 (Frontend Confirm-Dialog UI-05):** OpenAPI-Schema enthaelt jetzt den Endpoint mit allen 5 Status-Codes; Frontend-Toast-Mapping fuer 400 (Validation), 409 (Conflict mit Sub-Cases), 500 (Internal) kann anhand der Descriptions formuliert werden.

## Self-Check: PASSED

- File `genossi_rest/src/repayment_entry.rs` exists: FOUND
- Commit `a746db0` (Task 1) exists: FOUND (via `git log --oneline -3`)
- `pub async fn mark_paid_out<RestState` Handler-Signatur: FOUND (1 match)
- `path = "/{id}/mark-paid-out"` utoipa-Path: FOUND (1 match)
- `.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))` Route-Registration: FOUND (1 match)
- `mark_paid_out,` in ApiDoc-paths-Liste: FOUND (1 match)
- D-07 Sanity: 0 batch-mark-paid-out-Routes: PASSED (grep result = 0)
- `request_body` im mark_paid_out-Handler-Block: NOT FOUND (0 matches — Action-Endpoint korrekt)
- `cargo build -p genossi_rest` exits 0: PASSED
- `cargo test -p genossi_rest --lib repayment_entry`: 3/3 passed
- No accidental file deletions in commit: PASSED (diff-filter=D returns empty)

---

*Phase: 09-auszahlungs-buchung-atomisch-auditiert*
*Plan: 02*
*Completed: 2026-05-31*
