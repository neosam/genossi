---
phase: 08-repaymententry-auto-bef-llung
plan: 05
subsystem: rest
tags: [rust, axum, utoipa, openapi, di-wiring, rest-handler, repayment-entry, batch-toggle]

requires:
  - phase: 08-repaymententry-auto-bef-llung
    plan: 03
    provides: "RepaymentEntryService-Trait + 6 Methoden (create/update/delete/get/list_by_phase/batch_toggle_status) + RepaymentEntryServiceImpl mit 7 Deps + strukturierter 409-JSON-Body bei Batch-Failures"
  - phase: 08-repaymententry-auto-bef-llung
    plan: 04
    provides: "RepaymentPhaseServiceImpl::open_repayment_phase mit Auto-Fill + close_repayment_phase mit Pending-Validation + RepaymentPhaseServiceDeps um RepaymentEntryDao + MemberDao erweitert + partielles DI-Wiring in genossi_bin"

provides:
  - "6 REST-Endpoints unter /api/repayment-entry: POST (create), GET (list ?phase_id=), GET/{id}, PUT/{id}, DELETE/{id}, POST /batch-status (D-12)"
  - "Router-Reihenfolge /batch-status VOR /{id} (T-08-05-02 mitigation: Axum-Uuid-Parse-Collision-Defense)"
  - "7 TOs in genossi_rest_types: RepaymentEntryStatusTO, RepaymentEntryTO, CreateRepaymentEntryRequest, UpdateRepaymentEntryRequest, BatchStatusRequest, CloseConflictResponse, BatchFailureResponse (W-05)"
  - "RepaymentEntryRestState-Trait + RepaymentEntryServiceDependencies + RepaymentEntryService type-alias"
  - "DI-Wiring in RestStateImpl: RepaymentEntryServiceImpl mit 7 Arc-shared Deps; repayment_entry_dao + repayment_phase_dao GENAU EINMAL gebaut und via Arc::clone an beide Services geteilt (W-02)"
  - "Trait-Bound RepaymentEntryRestState an create_app + start_server + start_test_server"
  - "OpenAPI-Doc unter Tag 'RepaymentEntries' mit 6 Pfaden + 7 Schemas"
  - "10 grüne Tests (7 in genossi_rest_types + 3 in genossi_rest)"

affects:
  - "08-06 (E2E-Tests): kann nun gegen reale SQLite das komplette CRUD + Batch + Close-Validation testen; alle 6 Endpoints + Phase-7-Endpoints sind exponiert; start_test_server hat den Trait-Bound"
  - "09 (PAYO): mark_paid_out-Endpoint hängt sich an /api/repayment-entry/{id} an; PUT-Pfad ist bereits da, Phase 9 erweitert die Service-Layer-Validation"
  - "12 (Frontend): kann gegen die REST-Surface arbeiten; BatchFailureResponse + CloseConflictResponse sind strukturierte TOs, die das Frontend direkt deserialisieren kann"

tech-stack:
  added: []
  patterns:
    - "Router-Reihenfolge-Mitigation für statische Literale vor Path-Parametern: `.route('/batch-status', ...)` VOR `.route('/{id}', ...)` zwingend (T-08-05-02 mitigation)"
    - "Strukturierter 409-JSON-Body als TO formalisiert: Service liefert JSON-in-Arc<str>, REST-Layer reicht 1:1 durch, Frontend deserialisiert in BatchFailureResponse/CloseConflictResponse (W-05)"
    - "Arc-shared DAO-Pattern für Multi-Service-Sharing (W-02): DAO genau einmal konstruiert, via .clone() an beide Service-Konstruktoren — mirror des Phase-3-helper_token_dao-Sharing"
    - "Trait-Bound-Kette: jeder neue REST-Subrouter ergänzt seinen *RestState-Trait an create_app + start_server + start_test_server simultan (sonst Compile-Error oder fehlende E2E-Testbarkeit)"

key-files:
  created:
    - "genossi_rest/src/repayment_entry.rs (379 LOC: RestState-Trait + ListEntriesQuery + 6 Handler mit utoipa-Annotations + generate_route + ApiDoc + 3 Smoke-Tests)"
  modified:
    - "genossi_rest_types/src/lib.rs (+277 LOC: 7 TOs + bidirektionale From-Impls + 7 Tests)"
    - "genossi_rest/src/lib.rs (+5 LOC: pub mod repayment_entry + OpenAPI-Nest + Router-Mount + 2 Trait-Bounds)"
    - "genossi_rest/src/test_server.rs (+1 LOC: RepaymentEntryRestState Trait-Bound)"
    - "genossi_bin/src/lib.rs (+68 LOC -10 LOC: RepaymentEntryServiceDependencies + RepaymentEntryService type-alias + repayment_entry_service Field + Wiring (Variable repayment_entry_dao_for_phase → repayment_entry_dao via Arc::clone an beide Services) + RestState-Impl-Bridge)"

key-decisions:
  - "Router-Reihenfolge: /batch-status MUSS VOR /{id} stehen — Axum matcht in Deklarations-Reihenfolge. Inline-Doc-Kommentar im generate_route fixiert diese Invariante für künftige Modifikationen (T-08-05-02 mitigation)."
  - "Variable in RestStateImpl::new() umbenannt von repayment_entry_dao_for_phase (Phase-7-Plan-04-Konvention für temporären Single-Use) zu repayment_entry_dao, weil sie nun an zwei Services geteilt wird. Arc::clone() für beide Service-Konstruktoren."
  - "BatchFailureResponse und CloseConflictResponse als ToSchema-TOs in genossi_rest_types formalisiert — der Service-Layer (Plan 03 + Plan 04) emittiert bereits exakt dieses JSON-Schema in ServiceError::Conflict(Arc<str>), REST-Layer reicht den Body 1:1 als 409-Response durch. Frontend kann beide TOs direkt deserialisieren. KEIN serialize-then-parse-then-serialize-Roundtrip im REST-Layer nötig (W-05)."
  - "UpdateRepaymentEntryRequest hat Optional-Field-Pattern (share_count_to_pay_out + status sind Option, version ist Pflicht). PaidOut als Target wird im Service-Layer (Plan 03) abgelehnt — KEIN zusätzlicher Pre-Check im REST-Layer (redundant; Service-Layer ist Single Source of Truth für Edit-Matrix)."
  - "Listing-Query nur mit ?phase_id=<uuid> (D-10), keine status- oder member-Filter. Frontend filtert client-side; künftige Filter sind nachzuziehen ohne API-Breaking-Change."
  - "Kein lokaler map_*_error-Override im REST-Layer (analog Phase-7-Plan-04-Konvention) — globales From<ServiceError> for RestError reicht für alle 6 Endpoints. ServiceError::Conflict(Arc<str>) → RestError::Conflict(String) → HTTP 409 mit Body."
  - "test_list_query_deserializes_from_json nutzt serde_json statt serde_urlencoded — serde_urlencoded ist nicht direkt als Dependency aufgelistet; serde_json reicht für die Validierung dass die Feld-Namen stimmen (phase_id). Axum's Query<T>-Extractor nutzt intern serde_urlencoded, was über axum-Dependencies verfügbar ist."

patterns-established:
  - "Router-Reihenfolge-Doku als Inline-Kommentar bei statischen Literalen vor Path-Parametern (zukünftige Endpoint-Erweiterungen mit /static-name + /{id} müssen das Pattern übernehmen — sonst Uuid-Parse-Collision)"
  - "Variable-Rename-Pattern beim Übergang von Single-Use- zu Multi-Use-DAO im DI-Wiring: descriptive_for_X-Suffix entfernen sobald an mehrere Services geteilt wird"
  - "Multi-Phase-Plan-Konsistenz: Plan 04 hat partielles Wiring vorbereitet (repayment_entry_dao_for_phase + type RepaymentEntryDao-Alias + RepaymentPhaseServiceDeps-Erweiterung), Plan 05 baut darauf auf und vervollständigt das Wiring auf den vollen 7-Deps-Service"

requirements-completed: [ENTR-02, ENTR-04, ENTR-05, ENTR-06, PHAS-02, PHAS-03]

duration: ~10min
completed: 2026-05-31
---

# Phase 08 Plan 05: REST-Layer + DI-Wiring für RepaymentEntry Summary

**6 REST-Endpoints unter /api/repayment-entry (CRUD + Batch-Toggle) inkl. Router-Reihenfolge-Mitigation, 7 TOs mit strukturiertem 409-Body (BatchFailureResponse/CloseConflictResponse), DI-Wiring teilt RepaymentEntryDao + RepaymentPhaseDao Arc-shared zwischen RepaymentEntryServiceImpl und RepaymentPhaseServiceImpl — W-02 verifiziert (exakt 1 DAO-Konstruktor pro Prozess). 10 grüne Tests; Workspace baut clean.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-31T04:46:43Z
- **Completed:** 2026-05-31T04:56:17Z
- **Tasks:** 3/3 abgeschlossen
- **Files created:** 1 (`genossi_rest/src/repayment_entry.rs`)
- **Files modified:** 4 (`genossi_rest_types/src/lib.rs`, `genossi_rest/src/lib.rs`, `genossi_rest/src/test_server.rs`, `genossi_bin/src/lib.rs`)
- **Tests:** 7 (rest_types) + 3 (rest) = 10 neue grüne Tests; 42 grüne repayment-related Service-Tests bleiben unverändert grün

## Accomplishments

- **7 TOs in genossi_rest_types/src/lib.rs:**
  - `RepaymentEntryStatusTO` (Open/Contacted/PaidOut) mit bidirektionalen From-Impls
  - `RepaymentEntryTO` mit ISO8601-Datetime-Serde auf Optional-Timestamps, version als Option
  - `CreateRepaymentEntryRequest` (phase_id, member_id, share_count_to_pay_out)
  - `UpdateRepaymentEntryRequest` (Optional fields, pflicht-version) — Optional-Field-Pattern (D-12)
  - `BatchStatusRequest` (entry_ids: Vec<Uuid>, target_status)
  - `CloseConflictResponse` (error, pending_count, pending_member_numbers) — D-15 strukturierter 409-Body für /close
  - `BatchFailureResponse` (failure_index, failure_id, failure_reason) — **W-05** strukturierter 409-Body für /batch-status
- **6 REST-Handler in genossi_rest/src/repayment_entry.rs** (379 LOC):
  - POST `/api/repayment-entry` — create (D-11 Validations laufen im Service)
  - GET `/api/repayment-entry?phase_id=<uuid>` — list mit `ListEntriesQuery` (D-10)
  - GET `/api/repayment-entry/{id}` — detail
  - PUT `/api/repayment-entry/{id}` — update (Optional-Field-Pattern; Edit-Matrix im Service)
  - DELETE `/api/repayment-entry/{id}` — soft-delete (ENTR-05)
  - POST `/api/repayment-entry/batch-status` — Batch-Toggle (D-07/D-08)
- **Router-Reihenfolge (T-08-05-02 Mitigation):** `.route("/batch-status", ...)` VOR `.route("/{id}", ...)` mit Inline-Doc-Kommentar als Invariante
- **OpenAPI-Doc** (`ApiDoc`) mit 6 Pfaden unter Tag `RepaymentEntries` + 7 Component-Schemas
- **Router-Mount + Trait-Bounds in genossi_rest/src/lib.rs:** `pub mod repayment_entry;` + OpenAPI-Nest `/api/repayment-entry` + Router `.nest("/api/repayment-entry", generate_route::<RestState>())` + `RepaymentEntryRestState`-Trait-Bound an `create_app` und `start_server`
- **Trait-Bound in test_server.rs** für `start_test_server` (Plan-06 E2E-Tests können den vollen Stack starten)
- **DI-Wiring in genossi_bin/src/lib.rs:**
  - `RepaymentEntryServiceDependencies` struct mit 7 assoc-types
  - `RepaymentEntryService` type-alias auf `RepaymentEntryServiceImpl<...>`
  - `repayment_entry_service: Arc<RepaymentEntryService>` field in `RestStateImpl`
  - `RestStateImpl::new()`: **Variable repayment_entry_dao_for_phase → repayment_entry_dao** umbenannt, jetzt Arc-shared via `.clone()` an `RepaymentPhaseServiceImpl` UND `RepaymentEntryServiceImpl`. Plus `repayment_phase_dao` ebenfalls Arc-shared (war vorher move'd, jetzt `.clone()`).
  - `impl RepaymentEntryRestState for RestStateImpl` bindet die REST-Handler
- **W-02 verifiziert:** `grep -c "let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new" genossi_bin/src/lib.rs` == 1; `grep -c "let repayment_entry_dao = Arc::new(RepaymentEntryDao::new" genossi_bin/src/lib.rs` == 1. Exakt 1 DAO-Konstruktor pro Prozess.
- **10 grüne Tests:**
  - 7 in `genossi_rest_types::repayment_entry_to_tests`: status-roundtrip, domain-conversion, create-serde, update-optional-fields (incl. version-Pflicht-Test), batch-serde, close-conflict-serializes-with-pending-numbers, batch-failure-serde (W-05)
  - 3 in `genossi_rest::repayment_entry::tests`: ApiDoc::openapi() kompiliert, JSON-Deserialisierung von CreateRepaymentEntryRequest + ListEntriesQuery
- **42 grüne repayment-related Service-Tests** bleiben unverändert grün (23 in repayment_phase + 19 in repayment_entry)

## Task Commits

Jede Task atomar committed:

1. **Task 1: TOs in genossi_rest_types** — `6bef223` (feat, +277 LOC)
2. **Task 2: REST-Handler + Router + Trait-Bounds** — `b8e5c14` (feat, +380 LOC)
3. **Task 3: DI-Wiring in genossi_bin** — `00ddfc5` (feat, +68 LOC -10 LOC)

**Plan metadata:** *(folgt mit dem nächsten Commit)*

## Files Created/Modified

- **NEW** `genossi_rest/src/repayment_entry.rs` (379 LOC): Imports + `RepaymentEntryRestState`-Trait + `ListEntriesQuery` + 6 Handler mit `#[instrument]` + `#[utoipa::path]` Annotations + `generate_route` (mit T-08-05-02-Mitigation: batch-status VOR /{id}) + `ApiDoc` mit 7 Schemas + 3 Smoke-Tests
- **MOD** `genossi_rest_types/src/lib.rs` (+277 LOC): Phase-8-Block direkt NACH Phase-7-RepaymentPhase-Block (Z. 1261-1407): 7 TOs + bidirektionale From-Impls; neues Test-Modul `repayment_entry_to_tests` direkt NACH `repayment_phase_to_tests` mit 7 Tests
- **MOD** `genossi_rest/src/lib.rs` (+5 LOC): `pub mod repayment_entry;` (Z. 20, alphabetisch vor `repayment_phase`) + OpenAPI-Nest-Entry (`(path = "/api/repayment-entry", api = repayment_entry::ApiDoc)`) + Router-Mount (`.nest("/api/repayment-entry", repayment_entry::generate_route::<RestState>())`) + `RepaymentEntryRestState`-Trait-Bound an `create_app` und `start_server`
- **MOD** `genossi_rest/src/test_server.rs` (+1 LOC): `RepaymentEntryRestState`-Trait-Bound an `start_test_server`
- **MOD** `genossi_bin/src/lib.rs` (+68 LOC -10 LOC):
  - `RepaymentEntryServiceDependencies` struct + `Send`/`Sync`-Impls + `RepaymentEntryServiceDeps`-Impl mit 7 assoc-types (direkt NACH `RepaymentPhaseService` type-alias)
  - `RepaymentEntryService` type-alias
  - `repayment_entry_service: Arc<RepaymentEntryService>` field in `RestStateImpl`
  - Wiring im `RestStateImpl::new()`: Variable umbenannt von `repayment_entry_dao_for_phase` zu `repayment_entry_dao`; `repayment_phase_dao.clone()` und `repayment_entry_dao.clone()` an beide Services
  - `repayment_entry_service` ins Self-Init
  - `impl genossi_rest::repayment_entry::RepaymentEntryRestState for RestStateImpl` direkt NACH `RepaymentPhaseRestState`-Impl

## Decisions Made

Alle wesentlichen Decisions kamen aus `08-CONTEXT.md` (D-07/D-08/D-09/D-10/D-12/D-15), `08-PATTERNS.md §7-§9` und dem PLAN-Block, und wurden 1:1 umgesetzt.

Klarstellungen während der Implementierung:

- **Variable-Rename `repayment_entry_dao_for_phase` → `repayment_entry_dao`:** Plan 04 hatte den Suffix `_for_phase` als Marker dass die DAO nur einmal für RepaymentPhaseServiceImpl gebraucht wird (move statt clone). Plan 05 teilt die DAO an zwei Services — Suffix entfernt, `Arc::clone()` für beide Konstruktoren.
- **`repayment_phase_dao` Variable wurde ebenfalls von move zu `.clone()` umgestellt:** war in Plan 04 als reine Move-Variable konstruiert (`repayment_phase_dao,` ohne clone), muss jetzt Arc-shared sein. Eine Move-Variable hätte den 2. Service-Konstruktor verhindert (use-of-moved-value-Error).
- **`BatchFailureResponse` und `CloseConflictResponse` als ToSchema-TOs formalisiert** statt opake `Conflict(String)`-Bodies: das Frontend kann den 409-Body strukturiert deserialisieren und die `failure_index` / `pending_member_numbers` direkt in der UI verarbeiten. Der Service-Layer (Plan 03 + Plan 04) emittiert bereits exakt dieses JSON-Schema; REST-Layer reicht 1:1 durch — KEIN serialize-parse-serialize-Roundtrip im REST-Layer nötig (W-05).
- **`ListEntriesQuery` mit utoipa-IntoParams-Derive** statt manueller Schema-Definition — Pattern aus `attendance.rs::ListMembersQuery` 1:1 übernommen. Axum's `Query<T>`-Extractor nutzt intern serde_urlencoded.
- **Listing-Test `test_list_query_deserializes_from_json` nutzt serde_json statt serde_urlencoded:** serde_urlencoded ist nicht direkt als dev-dependency aufgelistet im genossi_rest Cargo.toml (zwar transitive über axum verfügbar, aber nicht garantiert exportiert). serde_json reicht zur Validierung dass das Feld exakt `phase_id` heisst — die tatsächliche URL-Encoded-Deserialisierung wird durch axum's Query-Extractor in Plan 06 E2E-Tests erprobt.
- **PUT-Pfad nutzt das Service-Layer-Edit-Matrix-Verbot für PaidOut:** keine zusätzlichen Pre-Checks im REST-Handler. Body-Mapping reicht — PaidOut als Target wird vom Service-Layer (Plan 03) abgelehnt mit klarer Fehlermeldung; globales From<ServiceError> for RestError mapped das auf 409.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Auto-Fix Blocking] Variable-Rename und Arc-Sharing in genossi_bin/src/lib.rs**

- **Found during:** Task 3 (DI-Wiring)
- **Issue:** Plan 04 hatte das Wiring der `RepaymentPhaseServiceImpl` mit `repayment_entry_dao_for_phase` (move) gemacht. Plan 05 wollte denselben DAO an `RepaymentEntryServiceImpl` teilen. Das ist mit move-Variablen unmöglich (use-of-moved-value-Error). Außerdem hatte Plan 04 `repayment_phase_dao` ebenfalls als Move-Variable gebaut (`repayment_phase_dao,` ohne clone), was zum gleichen Problem für die phase_dao-Sharing geführt hätte.
- **Fix:** Beide Variablen wurden in Plan 05 als reine `Arc`-Variablen behandelt und an beide Services via `.clone()` übergeben. Variable `repayment_entry_dao_for_phase` wurde zu `repayment_entry_dao` umbenannt (Suffix `_for_phase` impliziert single-use, was nicht mehr stimmt).
- **Files modified:** `genossi_bin/src/lib.rs` (Z. 746-787)
- **Verification:** `cargo build --workspace` clean; `grep -c "let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new" genossi_bin/src/lib.rs` == 1; `grep -c "let repayment_entry_dao = Arc::new(RepaymentEntryDao::new" genossi_bin/src/lib.rs` == 1 (W-02 verifiziert)
- **Committed in:** `00ddfc5` (Task 3 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Notwendige Anpassung an die Multi-Service-Sharing-Anforderung. Plan 04 hatte unbewusst die spätere Sharing-Anforderung von Plan 05 nicht antizipiert (verständlich — die zwei Pläne wurden sequentiell ausgeführt). Kein Scope-Creep; reine Mechanik-Fix.

## Issues Encountered

- **`serde_urlencoded` Dependency-Frage:** Erster Test-Entwurf für `ListEntriesQuery` nutzte `serde_urlencoded::from_str()` — aber diese Crate ist nicht direkt im `genossi_rest`-Cargo.toml aufgelistet. Statt eine neue Dependency hinzuzufügen (was eine Rule-4-Architektur-Frage wäre) wurde der Test auf `serde_json`-Roundtrip umgestellt; die tatsächliche URL-Query-Deserialisierung läuft über axum's `Query<T>`-Extractor zur Laufzeit und wird in Plan 06 E2E-Tests mit echtem HTTP-Request verifiziert.

## User Setup Required

None — REST-Layer und DI-Wiring sind komplett. Das Backend exponiert die neuen Endpoints beim Server-Start automatisch unter `/api/repayment-entry`. Keine externen Service-Konfigurationen, keine Environment-Variablen, keine manuellen Schritte.

## Next Phase Readiness

- **Plan 06 (E2E-Tests):** Foundation komplett. `start_test_server` hat den `RepaymentEntryRestState`-Trait-Bound; `RestStateImpl` implementiert ihn. Alle 6 Endpoints + die Phase-7-Endpoints `/api/repayment-phase/{id}/open` (mit Auto-Fill aus Plan 04) und `/close` (mit Pending-Validation aus Plan 04) sind erreichbar. Strukturierte 409-Bodies (BatchFailureResponse, CloseConflictResponse) sind als TOs verfügbar — E2E-Tests können `serde_json::from_str::<BatchFailureResponse>(body)` für strukturierte Assertions nutzen.
- **Plan 9 (PAYO):** REST-Surface für `mark_paid_out` kann sich an `/api/repayment-entry/{id}/mark-paid-out` als Action-Endpoint anhängen (Pattern analog `/api/repayment-phase/{id}/open`). Die existierende `update_repayment_entry`-Methode blockt PaidOut als Target — Phase 9 muss einen separaten Endpoint anlegen, nicht den PUT-Pfad erweitern.
- **Phase 12 (Frontend):** REST-Surface komplett dokumentiert via OpenAPI; Frontend kann gegen `/swagger-ui/` arbeiten. BatchFailureResponse + CloseConflictResponse sind als ToSchema-TOs im OpenAPI sichtbar.
- **Keine Blocker.**

## Threat Coverage

| Threat ID | Mitigation | Verified-by |
|-----------|------------|-------------|
| T-08-05-01 (Endpoint exposed without ADMIN_PRIVILEGE check) | Alle 6 Handler nutzen `extract_auth_context(Some(context))?` und übergeben die `Authentication` an den Service-Layer. Service-Layer (Plan 03) prüft `ADMIN_PRIVILEGE` als erste DAO-touchende Aktion in jeder Methode (verifiziert via Plan-03-Permission-Tests). | Code-Review jeder Handler (`extract_auth_context(Some(context))?`-Pattern); Plan-03-Permission-Tests verifizieren Service-Layer-Gate |
| T-08-05-02 (Axum routing collision /{id} parses 'batch-status' as Uuid) | `.route("/batch-status", post(...))` wird VOR `.route("/{id}", ...)` deklariert. Inline-Doc-Kommentar im generate_route fixiert die Invariante. | Code-Inspektion `grep -n 'route' genossi_rest/src/repayment_entry.rs` — batch-status auf Z. 304, /{id} auf Z. 309. Plan 06 E2E-Tests werden Real-HTTP-Requests gegen /batch-status feuern. |
| T-08-05-03 (TO-Layer deserializes invalid status enum via serde) | `RepaymentEntryStatusTO` ist exhaustive enum (Open/Contacted/PaidOut); serde rejected unbekannte Varianten mit 422. Service-Layer rejected PaidOut als PUT/Batch-Target zusätzlich (Plan 03 D-05/D-07). | Test `test_repayment_entry_status_to_roundtrip` verifiziert alle 3 Varianten; Test `test_batch_status_request_serde` enthält gültiges Enum |
| T-08-05-04 (DI miswiring: two service instances see different DAO state) | `RepaymentEntryDaoImpl` und `RepaymentPhaseDaoImpl` werden EINMAL gebaut und via `Arc::clone()` an beide Services übergeben. W-02-Grep-Gate verifiziert exakt 1 Konstruktor pro DAO. | `grep -c "let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new" genossi_bin/src/lib.rs` == 1 ✓; `grep -c "let repayment_entry_dao = Arc::new(RepaymentEntryDao::new" genossi_bin/src/lib.rs` == 1 ✓ |
| T-08-05-05 (OpenAPI schema mismatch lets clients send invalid bodies) | utoipa-ToSchema-Derive auf allen 7 TOs (inkl. BatchFailureResponse); `ApiDoc::openapi()` im Smoke-Test (`test_apidoc_compiles`) geprüft. Tests `test_*_serde` verifizieren JSON-Roundtrip für alle Requests. | Test `test_apidoc_compiles` in genossi_rest::repayment_entry::tests; Tests `test_create_repayment_entry_request_serde`, `test_update_repayment_entry_request_optional_fields`, `test_batch_status_request_serde` in genossi_rest_types |

## Self-Check: PASSED

**Verified files exist:**
- `genossi_rest/src/repayment_entry.rs`: FOUND
- `genossi_rest_types/src/lib.rs` (modified): FOUND
- `genossi_rest/src/lib.rs` (modified): FOUND
- `genossi_rest/src/test_server.rs` (modified): FOUND
- `genossi_bin/src/lib.rs` (modified): FOUND

**Verified commits exist:**
- `6bef223` (Task 1): FOUND in git log
- `b8e5c14` (Task 2): FOUND in git log
- `00ddfc5` (Task 3): FOUND in git log

**Verified tests pass:**
- 7/7 in `genossi_rest_types::repayment_entry_to_tests`: passed
- 3/3 in `genossi_rest::repayment_entry::tests`: passed
- 42/42 in `genossi_service_impl::repayment_*::tests` (Phase-7+8 Bestand): passed
- `cargo build --workspace`: clean (nur pre-existing Warnings in genossi_mail/genossi_rest/genossi_bin, ausserhalb Plan-Scope)

**Verified acceptance criteria (grep counts):**

Task 1 (genossi_rest_types/src/lib.rs):
- `pub enum RepaymentEntryStatusTO` == 1 ✓
- `pub struct RepaymentEntryTO` == 1 ✓
- `pub struct CreateRepaymentEntryRequest` == 1 ✓
- `pub struct UpdateRepaymentEntryRequest` == 1 ✓
- `pub struct BatchStatusRequest` == 1 ✓
- `pub struct CloseConflictResponse` == 1 ✓
- `pub struct BatchFailureResponse` == 1 ✓ (W-05)
- `failure_index` == 4 ✓ (>= 1)
- `PaidOut` == 9 ✓ (>= 3 — Enum + From-Impls + Tests)

Task 2 (genossi_rest/src/repayment_entry.rs + lib.rs + test_server.rs):
- `pub trait RepaymentEntryRestState` == 1 ✓
- `pub async fn create_repayment_entry` == 1 ✓
- `pub async fn batch_toggle_status` == 1 ✓
- `pub fn generate_route` == 1 ✓
- `.route("/batch-status"` == 1 ✓ (mit `\` Escape in grep)
- `#[utoipa::path` == 6 ✓ (6 Handler)
- `BatchFailureResponse` in repayment_entry.rs == 4 ✓ (>= 2: import + ApiDoc-schemas + response-body-Annotation)
- `pub struct ApiDoc` == 1 ✓
- `pub mod repayment_entry;` in lib.rs == 1 ✓
- `/api/repayment-entry` in lib.rs == 2 ✓ (OpenAPI-Nest + Router-Mount)
- `repayment_entry::RepaymentEntryRestState` in lib.rs == 2 ✓ (create_app + start_server)
- `repayment_entry::RepaymentEntryRestState` in test_server.rs == 1 ✓
- Router-Reihenfolge: `/batch-status` Z. 304 < `/{id}` Z. 309 ✓

Task 3 (genossi_bin/src/lib.rs):
- `type RepaymentEntryDao = genossi_dao_impl_sqlite::repayment_entry::RepaymentEntryDaoImpl` == 1 ✓
- `type RepaymentEntryService = genossi_service_impl::repayment_entry::RepaymentEntryServiceImpl` == 1 ✓
- `pub struct RepaymentEntryServiceDependencies` == 1 ✓
- `type RepaymentEntryDao = RepaymentEntryDao;` == 2 ✓ (RepaymentPhaseServiceDeps + RepaymentEntryServiceDeps)
- `type MemberDao = MemberDao;` == 10 ✓ (>= 2 in beiden Deps-Impls + andere Services)
- `repayment_entry_service: Arc<RepaymentEntryService>` == 1 ✓
- `let repayment_entry_dao = Arc::new(RepaymentEntryDao::new` == 1 ✓
- **W-02 CRITICAL:** `let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new` == 1 ✓
- `impl genossi_rest::repayment_entry::RepaymentEntryRestState for RestStateImpl` == 1 ✓

---

*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
