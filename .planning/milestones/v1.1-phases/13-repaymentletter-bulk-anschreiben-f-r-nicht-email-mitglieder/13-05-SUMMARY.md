---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 05
subsystem: rest
tags: [phase-13, rest-handler, openapi, di-wiring, direct-download, axum, utoipa, single-arc]

requires:
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "04"
    provides: "RepaymentLetterService Trait + RepaymentLetterBundle Output + RepaymentLetterServiceImpl + RepaymentContextResolverImpl"
provides:
  - "REST-Endpoint POST /api/repayment-phase/{phase_id}/letters/generate (genossi_rest::repayment_letter)"
  - "GenerateLettersRequest { entry_ids: Vec<Uuid> } body schema (ToSchema)"
  - "RepaymentLetterRestState State-Trait — bound auf create_app + start_server + test_server"
  - "RepaymentLetterServiceImpl + RepaymentContextResolverImpl im Production-RestStateImpl verdrahtet (Single-Arc per Process)"
  - "OpenAPI-Doku mit 6 Status-Codes (200/400/401/403/404/409) + X-Document-Count Header"
affects: [13-06, 13-07]

tech-stack:
  added: []
  patterns:
    - "Direct-Download-Response: Body::from(Vec<u8>) + Content-Type application/pdf + Content-Disposition attachment (Phase 11 D-13-02 1:1)"
    - "X-Document-Count Header (D-13-04): unique-member-count nach Aggregation als Custom-Header — Frontend liest fuer Toast-Pluralisierung"
    - "Lokales map_letter_error: ServiceError::PermissionDenied -> RestError::Forbidden(403), andere Errors via globalem From<ServiceError> (Phase 11 D-11 Pattern)"
    - "Trait-Bound-Parity zu repayment_export: 2 Stellen in create_app + start_server, plus separate test_server-Bound-Liste (genossi_rest/src/test_server.rs traegt eigene Bound-List)"
    - "Single-Arc-per-Process DI: ALLE 10 Letter-Service-Dependencies + 2 Resolver-Dependencies via .clone() von existierenden lokalen Arcs in RestStateImpl::new() — kein neuer DAO-Konstruktor (Plan-10 P07 Lektion eingehalten, baseline Arc::new(.*Dao::new) count 25 unveraendert)"
    - "#[allow(dead_code)] auf repayment_context_resolver-Feld in RestStateImpl: nur Storage zur Vorbereitung des D-13-11 Phase-10-Worker-Refactor-Quicks; aktuell wird der Arc nur via repayment_letter_service genutzt (das den Resolver intern haelt)"

key-files:
  created:
    - "genossi_rest/src/repayment_letter.rs"
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-05-SUMMARY.md"
  modified:
    - "genossi_rest/src/lib.rs"
    - "genossi_rest/src/test_server.rs"
    - "genossi_bin/src/lib.rs"

key-decisions:
  - "Pre-Flight Task 0 (verifikation-only, kein Code-Write): ALLE 10 erforderlichen Arcs existieren bereits als single local bindings in RestStateImpl::new(); document_storage als Arc::new(DocumentStorage::from_env()) line 647 (DocumentStorage ist Type-Alias fuer FilesystemDocumentStorage, line 446) — KEIN neuer Konstruktor in Task 3 noetig, nur .clone() pro Dependency"
  - "test_server.rs als gesonderte Bound-Stelle: das e2e-Test-Harness in genossi_rest/src/test_server.rs traegt eine eigene RestState-Bound-Liste (nicht erkannt im Plan-Text), die ebenfalls um + RepaymentLetterRestState ergaenzt werden musste — sonst kompiliert create_app mit dem TestServer-RestState nicht (Plan 07 E2E-Tests sind sofort blockiert)"
  - "#[allow(dead_code)] auf repayment_context_resolver-Feld: der Resolver-Arc wird heute nur als Inner-Field des RepaymentLetterServiceImpl benutzt. Der Storage auf RestStateImpl ist Vorbereitung fuer D-13-11 (Phase-10-Worker-Refactor), wird sonst noch nicht via Trait gelesen — Compiler-Warning durch gezielten allow-Attribut unterdrueckt mit Inline-Begruendung statt Field zu loeschen"
  - "Bundle-Filename + X-Document-Count kommen aus dem Service: result.filename und result.document_ids.len() — REST-Layer rechnet nichts neu aus, vermeidet Drift zwischen 'wieviele MemberDocuments persistiert' und 'was im Header steht'"
  - "RepaymentContextResolver-Type-Alias separat vom RepaymentLetterService eingefuehrt: das macht die D-13-11-Migration spaeter trivial (Phase-10-Worker bekommt die existierende Type-Alias-Definition + den Arc via Field-Zugriff)"

patterns-established:
  - "REST-Handler-Modul-Mount in genossi_rest/src/lib.rs hat heute 5 koordinierte Stellen: pub mod + ApiDoc-Nest + Router-.nest + create_app-Bound + start_server-Bound. Plus eine OFFEN-IM-PLAN-TEXT-NICHT-ERWAEHNTE Stelle: test_server.rs Bound. Folge-Plans sollten test_server.rs explizit als 6. Stelle behandeln."
  - "Single-Arc-Acceptance-Gates (Arc::new\\(.*Dao::new\\() koennen Vorkommen in Kommentaren matchen — bei der Acceptance-Verifikation darauf achten, dass dokumentierende Beispiel-Codes (z.B. 'KEIN neues Arc::new(SomeDao::new(...))') das Grep aufblaehen; bei Plan 13-05 fuehrte das initial zu count=26 statt 25, was via Kommentar-Reformulierung auf 25 zurueckgebracht wurde."

requirements-completed: [BRIEF-01]

duration: ~30min
completed: 2026-06-01
---

# Phase 13 Plan 05: RepaymentLetter REST-Handler + DI-Wiring Summary

**REST-Layer + Binary-Wiring fuer den Bulk-Brief-Service end-to-end: neuer POST-Endpoint mit Direct-Download-Pattern (Phase-11-Konsistenz), 6-Status-Code OpenAPI-Doku inkl. X-Document-Count Header (D-13-04 Frontend-Toast-Pluralisierung), Permission-Override fuer 403-Forbidden, Production-DI mit 10 Letter-Service- + 2 Resolver-Dependencies via Single-Arc-per-Process. Workspace-Build + alle Unit-Tests gruen; baseline Arc-DAO-Count unveraendert bei 25.**

## Performance

- **Duration:** ~30 min (zwischen `9d35376` parent und `04d65b2` Task-3-GREEN)
- **Tasks:** 3 ausgefuehrt (Task 0 verifikation-only, Task 1 = REST-Handler + Tests, Task 2 = Router-Mount + Bounds, Task 3 = DI-Wiring)
- **Files created:** 2 (1 Rust-Modul + SUMMARY)
- **Files modified:** 3 (genossi_rest/lib.rs, genossi_rest/test_server.rs, genossi_bin/lib.rs)
- **Commits:** 3 (1 pro Code-Task; Task 0 hat keinen eigenen Commit weil verifikation-only)

## Accomplishments

### Task 0 — Pre-Flight Single-Arc-Gate (kein Commit, Verifikation-only)

Ausgefuehrte Greps und Resultate:

| Grep | Resultat | Bedeutung |
|---|---|---|
| `let document_storage` | 3 hits — primary Arc bei line 647 (`Arc::new(DocumentStorage::from_env())`), zwei weitere als `.clone()` in initialize_audit_snapshot + worker setup | document_storage EXISTIERT als single Arc; Task 1 Stelle 0 entfaellt |
| `let uuid_service` | 1 hit (line 600) | Single Arc OK |
| `let audit_log_dao` | 3 hits — primary line 609 (`Arc::new(AuditLogDao::new(pool.clone()))`), weitere als worker-clone | Single Arc OK fuer Letter-Service-Wiring |
| `let member_document_dao` | 3 hits — primary line 631 (`Arc::new(MemberDocumentDao::new(pool.clone()))`) | Single Arc OK |
| `let member_dao` | 7 hits, davon 1 primary Arc (line 590), Rest in initialize_audit_snapshot/Test-Server-Helpern | Production-Arc OK |
| `let repayment_phase_dao` | 2 hits (line 796 primary, line 1239 clone) | Single Arc OK |
| `let repayment_entry_dao` | 2 hits (line 797 primary, line 1238 clone) | Single Arc OK |
| `let permission_service` | 1 hit (line 602) | OK |
| `let transaction_dao` | 9 hits, primary line 589, andere in Worker-/Test-Setup | Production-Arc OK |
| `let pdf_generator` | 1 hit (line 728) | OK |
| `let template_storage` | 1 hit (line 726) | OK |
| `Arc::new(FilesystemDocumentStorage` | 0 hits (direkter Match) — der existing Konstruktor nutzt den Type-Alias `DocumentStorage` (line 446: `type DocumentStorage = FilesystemDocumentStorage;`); via Alias-Match `Arc::new(DocumentStorage::from_env` returned 1 hit (line 647) | EXAKT 1 Konstruktor, Single-Arc-Pattern eingehalten |
| `Arc::new\([A-Z][A-Za-z]*Dao::new` baseline | 25 hits | Baseline fuer Single-Arc-Acceptance |

**Entscheidung:** Alle 10 Arcs existieren als single local bindings. Task 1 hat KEINE neue Arc::new(...)-Konstruktor noetig, NUR .clone() pro Dependency. Plan-Stelle 0 (document_storage-Konstruktor) entfaellt.

### Task 1 — REST-Handler + ApiDoc + Module Mount (Commit `81015a0`)

- Neue Datei `genossi_rest/src/repayment_letter.rs` (~210 Zeilen inkl. 5 Tests).
- **`generate_letters<RestState>` Handler**: extrahiert auth, prueft `entry_ids` nicht leer, ruft `rest_state.repayment_letter_service().generate(...)`, mapped Errors via lokalem `map_letter_error`, baut Response mit 3 Headern: `Content-Type`, `Content-Disposition`, `X-Document-Count`.
- **`GenerateLettersRequest { entry_ids: Vec<Uuid> }`** mit `#[derive(Debug, Deserialize, ToSchema)]` — D-13-03 flache Liste, Server aggregiert serverseitig (D-13-04).
- **`map_letter_error(e: ServiceError) -> RestError`** als lokaler Override: `PermissionDenied -> Forbidden(403)`, andere via `.into()` (globales From mapping). Phase 11 D-11 Pattern 1:1.
- **`RepaymentLetterRestState` Trait** mit Associated-Type `RepaymentLetterService` (bound auf `crate::ContextType`) und Accessor `repayment_letter_service() -> Arc<...>`.
- **`generate_letter_route<RestState>()`**: Axum-Router-Generator fuer Pfad `/{phase_id}/letters/generate` mit POST-Methode.
- **`ApiDoc` struct** mit `#[openapi(...)]` Macro: paths(generate_letters), components(schemas(GenerateLettersRequest)), tag "RepaymentLetter".
- **OpenAPI dokumentiert alle 6 Response-Codes**: 200 (PDF bytes inkl. Header-Beschreibung), 400 (Validation + entry_phase_mismatch), 401 (no session), 403 (helper auth), 404 (phase not found), 409 (phase_not_active).
- **5 Unit-Tests**: `test_map_letter_error_permission_denied_to_403`, `..._entity_not_found_passthrough`, `..._conflict_passthrough`, `test_generate_letters_request_deserialization`, `test_generate_letters_request_empty_list_deserialization`. Alle gruen.
- `pub mod repayment_letter;` zu `genossi_rest/src/lib.rs` ergaenzt (sonst kompiliert die neue Datei nicht).

### Task 2 — Router-Mount + ApiDoc-Nest + Trait-Bounds (Commit `b7006b4`)

- **ApiDoc-Nest** in `genossi_rest/src/lib.rs:280`: `(path = "/api/repayment-phase/{phase_id}/letters", api = repayment_letter::ApiDoc)` direkt nach Export-Eintrag.
- **Trait-Bounds** an BEIDEN bekannten Stellen (Plan-Text-Pflicht >=2) — `+ repayment_letter::RepaymentLetterRestState`:
  - `create_app` Generic-Bound (line 453, direkt nach `+ repayment_export::RepaymentExportRestState`)
  - `start_server` Generic-Bound (line 786, an gleicher Stelle wie create_app)
  - **PLUS test_server.rs Bound** (line 29) — Plan-Text erwaehnt nur 2 Stellen, aber `genossi_rest/src/test_server.rs` traegt seine eigene parallele Bound-List, die ebenfalls ergaenzt werden musste (sonst kompiliert das Test-Harness nicht — siehe Auto-Fix Rule 3 unten).
- **Router-Mount** in `genossi_rest/src/lib.rs:660-670`: zweiter `.nest("/api/repayment-phase", repayment_letter::generate_letter_route::<RestState>())` direkt nach repayment_export-Mount, mit Inline-Kommentar zur Axum-Multi-Nest-Semantik.
- Parity-Check verifiziert: `repayment_export::RepaymentExportRestState` count == `repayment_letter::RepaymentLetterRestState` count in lib.rs (beide 2 — selbe Bound-Stellen).
- Build: `cargo build -p genossi_rest` UND `cargo build -p genossi_rest --all-features` beide clean.

### Task 3 — DI-Wiring in genossi_bin/src/lib.rs (Commit `04d65b2`)

5 Stellen edited in genossi_bin/src/lib.rs (Stelle 0 aus Plan entfaellt per Task-0-Pre-Flight: document_storage existiert bereits als Single-Arc).

- **Stelle 1 (Deps-Aliases, line ~318-373):** zwei neue `pub struct + impl + type` Bloecke direkt nach `RepaymentExportServiceDependencies`:
  - `RepaymentContextResolverDependencies` mit 3 assoc types (Transaction + 2 DAOs)
  - `RepaymentLetterServiceDependencies` mit 12 assoc types (Context + Transaction + 8 DAO/Service-Bounds + 2 Helper-Bounds: RepaymentContextResolver + DocumentStorage)
  - jeweils mit `unsafe impl Send/Sync` (Pattern aus existing Deps-Aliases).
- **Stelle 2 (RestStateImpl-Felder, line ~607-617):** zwei neue Arc-Felder direkt nach `repayment_export_service`:
  - `repayment_context_resolver: Arc<RepaymentContextResolver>` mit `#[allow(dead_code)]` und Inline-Begruendung (D-13-11 Storage)
  - `repayment_letter_service: Arc<RepaymentLetterService>`
- **Stelle 3 (Service-Construction in `RestStateImpl::new()`, line ~942-985):** zwei `let ... = Arc::new(...)` Bloecke direkt nach repayment_export_service-Konstruktion:
  - `RepaymentContextResolverImpl` mit 2 DAO-Arcs via .clone()
  - `RepaymentLetterServiceImpl` mit allen 12 Feldern (10 deps + pdf_generator + template_base) — ALLE via .clone() von existierenden lokalen Arcs. KEIN neuer DAO-Konstruktor.
- **Stelle 4 (Struct-Literal-Return, line ~1077-1081):** zwei neue Felder direkt nach `repayment_export_service,` im Return-Block.
- **Stelle 5 (Trait-Impl `RepaymentLetterRestState for RestStateImpl`, line ~1543-1551):** neuer impl-Block direkt nach `RepaymentExportRestState`-Impl, gibt `self.repayment_letter_service.clone()` zurueck.

**Builds verifiziert nach Task 3:**
- `cargo build -p genossi_bin`: clean
- `cargo build -p genossi_bin --all-features`: clean
- `cargo build --workspace --all-features`: clean
- `cargo test -p genossi_rest --lib repayment_letter`: 5/5 pass
- `cargo test -p genossi_service_impl --lib repayment_letter`: 21/21 pass

**Single-Arc-Gate (Plan 10 P07 Lektion):**
- Baseline `Arc::new(*Dao::new(` count vor Plan: 25
- Post-Plan count: 25 (unveraendert — alle neuen Service-Deps via .clone())
- `Arc::new(FilesystemDocumentStorage`: 0 direkter Match (via Alias `DocumentStorage::from_env` 1 hit, line 647 — exakt 1 Konstruktor)
- `Arc::new(MemberDocumentDao::new(self.pool`: 0 (kein neuer Arc fuer MemberDocumentDao im Letter-Service-Pfad)

## Task Commits

1. **Task 1 — REST-Handler + Module-Mount:** `81015a0` (feat) — genossi_rest/src/lib.rs + repayment_letter.rs (217 insertions)
2. **Task 2 — Router-Mount + Bounds:** `b7006b4` (feat) — genossi_rest/src/lib.rs + test_server.rs (13 insertions)
3. **Task 3 — DI-Wiring im Binary-Layer:** `04d65b2` (feat) — genossi_bin/src/lib.rs (115 insertions)

_Note: Task 0 ist verifikation-only (kein Code-Write); Pre-Flight-Resultat ist im Plan-Acceptance dokumentiert und in dieser SUMMARY oben._

## Files Created/Modified

- **Created** `genossi_rest/src/repayment_letter.rs` — REST-Handler + GenerateLettersRequest + RepaymentLetterRestState Trait + ApiDoc + map_letter_error + 5 Unit-Tests (~210 Zeilen)
- **Modified** `genossi_rest/src/lib.rs` — Module-Declaration + ApiDoc-Nest + Router-Mount + 2 Trait-Bounds (~25 Zeilen Delta)
- **Modified** `genossi_rest/src/test_server.rs` — Trait-Bound auf TestServer-RestState ergaenzt (1 Zeile Delta)
- **Modified** `genossi_bin/src/lib.rs` — 5 DI-Stellen: 2 Deps-Structs + 2 Type-Aliases + 2 RestStateImpl-Felder + 2 Arc::new(...)-Konstruktionen + 2 Return-Struct-Eintraege + 1 Trait-Impl (~115 Zeilen Delta)

## Decisions Made

### test_server.rs als 6. Mount-Stelle (Plan-Discretion-Erweiterung)

Der Plan-Text fuehrt 5 Edit-Stellen in genossi_rest/src/lib.rs auf (Module-Decl, ApiDoc-Nest, 2 Trait-Bound-Stellen, Router-Mount), erwaehnt aber `test_server.rs` nicht. Beim ersten Build-Versuch von Task 2 ergab sich ein Compile-Error im Test-Harness:

```
error[E0277]: the trait bound `RestState: RepaymentLetterRestState` is not satisfied
  --> genossi_rest/src/test_server.rs:41:42
```

Das Test-Harness `genossi_rest/src/test_server.rs` traegt eine eigene parallele Generic-Bound-List (gespiegelt zu `create_app`), die nicht automatisch mitwaechst, wenn `create_app` einen neuen Bound bekommt. Die Loesung war Symmetrie-Erhaltung: dieselbe `+ repayment_letter::RepaymentLetterRestState`-Zeile zusaetzlich in `test_server.rs:29` ergaenzen. Dokumentiert hier weil es eine NICHT-im-Plan-Text-erwaehnte 6. Stelle ist, die Folge-Plans (z.B. Plan 13-07 E2E-Tests) explizit auf dem Schirm haben sollten.

### #[allow(dead_code)] auf repayment_context_resolver-Field

Der RestStateImpl traegt den `repayment_context_resolver: Arc<...>`-Field, obwohl ihn aktuell KEIN REST-Handler direkt liest (der Letter-Service nutzt ihn intern via eigenes Field). Plan-Acceptance fordert das Field explizit (`rg 'repayment_context_resolver: Arc' returns >=2`) — Grund: D-13-11 Pending-Todo `phase-10-worker-refactor-resolver.md` wird in einem Folge-Quick den Phase-10-Mail-Worker auf denselben Resolver-Arc migrieren, der dann via RestStateImpl bezogen werden kann.

Damit ein `cargo build --workspace` ohne dead_code-Warning durchlaeuft, habe ich `#[allow(dead_code)] // kept for D-13-11 follow-up worker refactor` direkt vor dem Field angebracht. Alternative waere gewesen, das Field zu loeschen — das haette aber D-13-11 spaeter "Plan 13-05 nachholen" verursacht. Konservativere Entscheidung: Field halten, Warning unterdruecken, Begruendung inline dokumentieren.

### Bundle-Filename + X-Document-Count aus Service-Result (Single Source of Truth)

Der REST-Handler rechnet weder Filename noch Count selber aus — beide kommen aus dem `RepaymentLetterBundle`-Struct, das Plan 04 Task 1 definiert hat:
- `result.filename: String` -> `Content-Disposition: attachment; filename="..."`
- `result.document_ids.len(): usize` -> `X-Document-Count: N`

Das verhindert Drift zwischen "wieviele MemberDocuments hat der Service tatsaechlich persistiert" und "was steht im Frontend-sichtbaren Header". Plan 07 E2E-Test 1 (Happy-Path) verifiziert end-to-end, dass beide Werte mit der DB-Realitaet uebereinstimmen.

### Kommentar-Reformulierung zur Acceptance-Erfuellung

Initial-Kommentar in der Letter-Service-Konstruktion enthielt das Beispiel-Literal `Arc::new(SomeDao::new(...))`, das die Acceptance-Grep `rg 'Arc::new\\([A-Z][A-Za-z]*Dao::new'` auf 26 statt 25 hochgetrieben hat (Single-Arc-Gate Verletzung im Schein). Loesung: Kommentar reformuliert auf "kein neuer DAO-Konstruktor" — semantisch identisch, ohne das verbotene Literal-Pattern. Lessons-learned-Pattern fuer Folge-Plans hinzugefuegt (`patterns-established` Eintrag).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] test_server.rs-Bound-List nicht im Plan-Text erwaehnt**

- **Found during:** Task 2 erstem `cargo build -p genossi_rest`
- **Issue:** Plan-Text listet 4 Edit-Stellen fuer genossi_rest/src/lib.rs (Module-Decl, ApiDoc-Nest, Trait-Bound, Router-Mount); test_server.rs traegt eine eigene parallele Bound-List, die unabhaengig gewartet werden muss
- **Fix:** Zusaetzliche Bound-Ergaenzung in `genossi_rest/src/test_server.rs:29` (+ `RepaymentLetterRestState`-Bound zwischen RepaymentExportRestState und AuditRestState)
- **Files modified:** `genossi_rest/src/test_server.rs` (1 Zeile)
- **Commit:** `b7006b4` (Task-2-Commit, fix vor Commit)

**2. [Rule 2 - Missing Critical] dead_code-Warning auf neuem RestStateImpl-Feld**

- **Found during:** Task 3 `cargo build -p genossi_bin`
- **Issue:** Plan-Acceptance fordert `repayment_context_resolver: Arc` als RestStateImpl-Feld, aber kein aktueller REST-Handler liest dieses Feld direkt — Letter-Service traegt seinen eigenen Resolver-Field, der Storage auf dem RestStateImpl ist Vorbereitung fuer D-13-11. Compiler wirft `field repayment_context_resolver is never read`-Warning, was bei strikteren CI-Builds (deny_warnings) fail wuerde
- **Fix:** `#[allow(dead_code)]` mit Inline-Begruendung `// kept for D-13-11 follow-up worker refactor` direkt vor dem Field-Decl
- **Files modified:** `genossi_bin/src/lib.rs` (1 Zeile)
- **Commit:** `04d65b2` (Task-3-Commit, fix vor Commit)

**3. [Rule 3 - Blocking] Acceptance-Grep `Arc::new(*Dao::new(` initial bei 26 statt 25**

- **Found during:** Task 3 Acceptance-Verifikation nach dem ersten Wiring-Commit-Versuch
- **Issue:** Initial-Kommentar im Letter-Service-Konstruktor enthielt das Beispiel-Literal `KEIN neues Arc::new(SomeDao::new(...))` — das Grep matched darauf, was die Baseline-Acceptance scheinbar verletzte (26 statt 25)
- **Fix:** Kommentar reformuliert auf "kein neuer DAO-Konstruktor (Single-Arc-per-Process, Plan 10 P07 Lektion)" — semantisch identisch, ohne verbotenes Pattern
- **Files modified:** `genossi_bin/src/lib.rs` (Kommentar-Block, 5 Zeilen)
- **Commit:** `04d65b2` (vor Commit gefixt)

### Auto-fix Rules nicht relevant

- Rule 1 (Bug): keine Bugs — Service-Trait und Impl aus Plan 04 sind klar definiert, der Wire-up war mechanisch
- Rule 4 (Architectural): keine architektonischen Aenderungen — Plan folgt 1:1 dem Phase-11-Pattern (repayment_export)

## Issues Encountered

### Pre-Existing — ROADMAP.md durch Wave-Orchestrator modifiziert

`.planning/ROADMAP.md` war vor Plan-Start bereits modifiziert (Plan-Counts inkl. Phase 13). Per Plan-Instruktion ("Do NOT modify STATE.md or ROADMAP.md — the orchestrator owns those writes after the wave completes") wurde ROADMAP.md beim Commit explizit ausgeschlossen (`git add` selektiv pro File).

### Pre-Existing — typst-packages/ in genossi_service_impl/-Folder werden von jj getrackt

Beim Plan-Start zeigte `git status` Files in `genossi_service_impl/typst-packages/preview/letter-pro/3.0.0/` als untracked. Plan 13-04 SUMMARY dokumentierte das bereits + hat ein `.gitignore`-Pattern `/*/typst-packages/` eingefuehrt. Die Files bleiben dauerhaft im Working-Tree (jj-colocated mode + Build-Cache), aber `.gitignore` haelt sie aus dem Git-Index. Bei allen 3 Plan-13-05 Commits wurde via selektiver `git add` sichergestellt, dass diese Files NICHT in den Commit reinrutschen.

### Plan-Text-Gap — test_server.rs als 6. Mount-Stelle

Siehe Auto-Fix #1 oben. Folge-Plans und Phase-Templates sollten "test_server.rs Bound-List" explizit als bekannte Mount-Stelle auflisten.

## Self-Check

```
=== Files exist ===
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi_rest/src/repayment_letter.rs
FOUND: /home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-05-SUMMARY.md

=== Commits exist ===
FOUND: 81015a0 (Task 1 — REST-Handler + module mount)
FOUND: b7006b4 (Task 2 — Router-mount + Bounds incl. test_server.rs)
FOUND: 04d65b2 (Task 3 — DI-Wiring in genossi_bin)

=== Task 1 Acceptance-Greps gruen ===
- test -f genossi_rest/src/repayment_letter.rs: ✓
- rg 'pub async fn generate_letters' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'pub struct GenerateLettersRequest' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'pub trait RepaymentLetterRestState' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg '#\[derive\(OpenApi\)\]' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'pub fn generate_letter_route' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'map_letter_error' genossi_rest/src/repayment_letter.rs: 9 (>=2 ✓)
- rg 'PermissionDenied => RestError::Forbidden' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'application/pdf' genossi_rest/src/repayment_letter.rs: 5 (>=2 ✓)
- rg 'Content-Disposition' genossi_rest/src/repayment_letter.rs: 4 (>=1 ✓)
- rg '"X-Document-Count"' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'document_ids\.len\(\)' genossi_rest/src/repayment_letter.rs: 1 ✓
- rg 'status = 200' .. 'status = 409' alle present: ✓ (200,400,401,403,404,409 alle 1×)
- cargo build -p genossi_rest exit 0: ✓
- cargo test -p genossi_rest --lib repayment_letter: 5 passed ✓

=== Task 2 Acceptance-Greps gruen ===
- rg 'pub mod repayment_letter' genossi_rest/src/lib.rs: 1 ✓
- rg 'repayment_letter::ApiDoc' genossi_rest/src/lib.rs: 1 ✓
- rg 'repayment_letter::RepaymentLetterRestState' genossi_rest/src/lib.rs: 2 (>=2 ✓)
- rg 'repayment_letter::generate_letter_route' genossi_rest/src/lib.rs: 1 ✓
- rg -U '\.nest\(\s*"/api/repayment-phase"' genossi_rest/src/lib.rs: 3 nests (>=2 ✓)
- Parity check: repayment_export bounds (2) == repayment_letter bounds (2) ✓
- cargo build -p genossi_rest exit 0: ✓
- cargo build -p genossi_rest --all-features exit 0: ✓

=== Task 3 Acceptance-Greps gruen ===
- rg 'pub struct RepaymentContextResolverDependencies' genossi_bin/src/lib.rs: 1 ✓
- rg 'pub struct RepaymentLetterServiceDependencies' genossi_bin/src/lib.rs: 1 ✓
- rg 'type RepaymentContextResolver\s*=' genossi_bin/src/lib.rs: 2 (alias decl + Impl-Type-Generic ✓)
- rg 'type RepaymentLetterService\s*=' genossi_bin/src/lib.rs: 2 ✓
- rg 'RepaymentContextResolverImpl' genossi_bin/src/lib.rs: 4 ✓ (Deps-impl + alias + Konstruktor + Type-Generic)
- rg 'RepaymentLetterServiceImpl' genossi_bin/src/lib.rs: 5 ✓
- rg 'impl genossi_rest::repayment_letter::RepaymentLetterRestState' genossi_bin/src/lib.rs: 1 ✓
- repayment_context_resolver: 4 references (field + let-binding + service-field + return) — semantically equivalent to >=2 acceptance
- repayment_letter_service: 4 references — semantically equivalent to >=2 acceptance

=== Single-Arc Gate (Plan 10 P07) ===
- rg 'let document_storage' genossi_bin/src/lib.rs: 3 (>=1 ✓; primary Arc at line 647)
- Arc::new(FilesystemDocumentStorage|DocumentStorage::from_env) constructors: EXAKT 1 ✓
- rg 'Arc::new\(MemberDocumentDao::new\(self\.pool' genossi_bin/src/lib.rs: 0 ✓ (kein neuer MemberDocumentDao-Arc)
- rg 'Arc::new\([A-Z][A-Za-z]*Dao::new' genossi_bin/src/lib.rs: 25 (baseline preserved ✓)

=== Builds ===
- cargo build -p genossi_rest: clean
- cargo build -p genossi_rest --all-features: clean
- cargo build -p genossi_bin: clean (with #[allow(dead_code)] annotation)
- cargo build -p genossi_bin --all-features: clean
- cargo build --workspace: clean
- cargo build --workspace --all-features: clean

=== Tests ===
- cargo test -p genossi_rest --lib repayment_letter: 5 passed, 0 failed
- cargo test -p genossi_service_impl --lib repayment_letter: 21 passed, 0 failed

=== Pre-Flight Documented ===
- Task 0 Resultat aller 10 Greps: ✓ (oben in Section "Task 0")
- Baseline-Zaehler `Arc::new(...Dao::new` mit Wert 25 dokumentiert: ✓
- document_storage als single Arc identifiziert (line 647 via Alias DocumentStorage::from_env): ✓
- Entscheidung "alle Arcs vorhanden → nur .clone()" dokumentiert + in Task 3 ausgefuehrt: ✓

=== No untracked files committed ===
- git show --stat 81015a0: 2 files (lib.rs + repayment_letter.rs) ✓
- git show --stat b7006b4: 2 files (lib.rs + test_server.rs) ✓
- git show --stat 04d65b2: 1 file (genossi_bin/lib.rs) ✓
- KEIN genossi_service_impl/typst-packages/ in commits ✓
- KEIN .planning/ROADMAP.md in commits ✓ (Orchestrator owns this)
```

**Self-Check: PASSED**

## Threat Flags

Keine neuen Threat-Flags ueber das Plan-`<threat_model>` hinaus. Mitigationen verifiziert:

- **Mount-Path-Konflikt (`/letters` ueberlappt mit `/export`)**: LOW — `cargo build` mit beiden Mounts unter `/api/repayment-phase` erfolgreich; Axum-Path-Matching pro Segment, `/{phase_id}/letters/generate` disjunkt von `/{phase_id}/export/{format}`. Plan 07 verifiziert end-to-end.
- **Trait-Bound-Vergessen → Compile-Error**: MITIGIERT — initial vergessen in test_server.rs, sofort gefunden via `cargo build` (klarer E0277-Error). Nach Fix Parity-Check zu repayment_export gruen.
- **DI-Wiring Multi-Arc-Konflikt**: VERIFIZIERT-MITIGIERT — Single-Arc-Gate-Greps gruen, baseline count 25 unveraendert, kein neuer DAO-Konstruktor.
- **Permission-Bypass im Handler (vergessen, map_letter_error zu rufen)**: VERIFIZIERT — `rg 'map_letter_error' genossi_rest/src/repayment_letter.rs` returns 9 (definition + usage + 4 Test-Assertions); `rg 'PermissionDenied => RestError::Forbidden' genossi_rest/src/repayment_letter.rs` returns 1. Test `test_map_letter_error_permission_denied_to_403` verifiziert das Mapping.
- **OpenAPI-Doku-Drift (Status-Codes falsch dokumentiert)**: VERIFIZIERT — alle 6 Codes (200, 400, 401, 403, 404, 409) als `status = N`-Annotations im `#[utoipa::path]`-Macro, jeweils 1× hit. Plan 07 E2E-Tests verifizieren semantisch (alle 6 Pfade werden getriggert).
- **X-Document-Count-Drift (Header-Wert != document_ids.len())**: VERIFIZIERT — Header wird in einer Zeile direkt aus `result.document_ids.len()` gesetzt, keine Zwischen-Variable; `rg 'document_ids\.len\(\)' genossi_rest/src/repayment_letter.rs` returns exakt 1. Plan 07 E2E-Test 1 verifiziert end-to-end.

## Next Plan Readiness

Plan 13-06 (Frontend) kann jetzt:
- API-Client-Methode `generate_repayment_letters(config, phase_id, entry_ids: Vec<Uuid>) -> Result<(String, usize), AppError>` bauen — `String` = blob_url fuer Browser-Save, `usize` = X-Document-Count-Header fuer Toast-Pluralisierung
- POST `/api/repayment-phase/{phase_id}/letters/generate` mit JSON-Body `{ "entry_ids": [...] }`
- Response-Header lesen: `Content-Disposition` (Filename-Extraction), `X-Document-Count` (Toast)
- Error-Mapping: 401 -> "Session abgelaufen", 403 -> "Kein Vorstand", 404 -> "Phase nicht gefunden", 409 -> "Phase nicht aktiv", 400 -> Validation-Message (entry_phase_mismatch, leere Liste)

Plan 13-07 (E2E-Tests) kann jetzt:
- TestServer-Bound bereits inkl. RepaymentLetterRestState (siehe test_server.rs Edit)
- Reqwest-POST mit `client.post().json(&body)` gegen den neuen Endpoint
- Response-Bytes via `resp.bytes().await` validieren (start with `%PDF-`)
- X-Document-Count Header via `resp.headers().get("X-Document-Count").and_then(|h| h.to_str().ok())` lesen

**Keine Blocker fuer Folge-Plans.**

**Pending Follow-ups (durch dieses Plan NICHT abgedeckt):**
- D-13-11 Phase-10-Worker-Refactor: `.planning/todos/pending/phase-10-worker-refactor-resolver.md` — Worker auf den jetzt im RestStateImpl bereitgestellten `repayment_context_resolver` migrieren. Aktuell ist das Feld via `#[allow(dead_code)]` annotiert; nach Refactor wird die Annotation entfernt.
- Logo-Asset-Provisioning fuer Production (Plan-13-03 / Plan-13-04 pending): `nebenan-unverpackt-logo.svg` muss auf den deployed TEMPLATE_PATH kopiert werden (kein File-Path im Plan 05 Scope).

---

*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-01*
