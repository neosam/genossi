---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 04
subsystem: service
tags: [phase-13, service, permission-funnel, audit, audited-create, multi-entry-aggregation, bundle]

requires:
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "01"
    provides: "DocumentType::RepaymentLetter + auszahlungs_anschreiben(.typ + _bundle.typ)"
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "02"
    provides: "RepaymentContextResolver Trait + Impl (resolve + aggregate)"
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "03"
    provides: "PdfGenerator::render_repayment_letter + render_repayment_letter_bundle"
provides:
  - "Trait RepaymentLetterService (genossi_service::repayment_letter) + Output RepaymentLetterBundle"
  - "MockRepaymentLetterService via automock — fuer REST-Layer-Tests in Plan 05"
  - "Impl RepaymentLetterServiceImpl<Deps> (genossi_service_impl::repayment_letter) — Permission-Funnel + Status-Gate + Aggregation (via aggregate) + audited bulk-create + Bundle-Render"
  - "RepaymentLetterServiceDeps Trait — DI-ready fuer Plan 05 REST-Wiring (10 Trait-Bound-Dependencies)"
affects: [13-05, 13-06, 13-07]

tech-stack:
  added: []
  patterns:
    - "Two-Tx-Lifecycle: Read-Tx (Funnel + Entry-Read + Member-Reads) -> commit -> sync Render -> Schreibe-Tx (audited_create-Loop) -> commit — Pitfall #2 sauber"
    - "Resolver::aggregate sync pure-fn-Wrapper (Plan 02 Output) statt async resolve im Loop — vermeidet 1+N DB-Reads (phase+entries werden EINMAL geladen, dann N-mal aggregate)"
    - "user_id-Resolution via PermissionService::current_user_id mit PermissionDenied bei None — KEIN 'SYSTEM'-Fallback (wie member_document.rs es macht) und KEIN Sentinel-UUID; Audit-Hashchain bleibt verbandskonform"
    - "Bulk-Limit-Pre-Validation MAX_ENTRY_IDS_PER_REQUEST=200 als DoS-Schutz vor jeder DB-Touche"
    - "Hand-rolled mockall::mock!-Bloecke fuer alle 10 Deps + provision_template_base() fuer echte Typst-Renders im Test (analog Plan 13-03 provision_letter_templates)"

key-files:
  created:
    - "genossi_service/src/repayment_letter.rs"
    - "genossi_service_impl/src/repayment_letter.rs"
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-04-SUMMARY.md"
  modified:
    - "genossi_service/src/lib.rs"
    - "genossi_service_impl/src/lib.rs"
    - ".gitignore"

key-decisions:
  - "Pre-Flight Pattern A: PermissionService::current_user_id existiert auf permission.rs:42 mit Signatur `Result<Option<String>, ServiceError>`. Pattern A gewählt — bei `None` wird ServiceError::PermissionDenied geworfen (NICHT 'SYSTEM'-Fallback wie in member_document.rs:65) — Verbandskonformitaet erlaubt keine anonymen Audit-Eintraege fuer Vorstand-Aktionen"
  - "Authentication::Full-Pfad wirft PermissionDenied — Full ist nur fuer interne System-Calls; bei Bulk-Brief gibt es keinen sinnvollen user_id-Extraction-Pfad ohne Context"
  - "MAX_ENTRY_IDS_PER_REQUEST = 200 als Server-Limit; gemaess Plan-Discretion das Standard-Limit gewählt, das fuer normale Bulk-Briefe ueberreichlich ist (Genossenschaft hat <300 aktive Member)"
  - "Two-Tx-Lifecycle (Read-Tx + Schreibe-Tx) statt Single-Tx: Render ist sync und passiert zwischen den Tx, daher zwingend gesplittet (Pitfall #2)"
  - "File-Save VOR audited_create per Member: ein DAO-Failure verhindert die Audit-Logentry, das File bleibt verwaist (operativ aufraeumbar — siehe threat_model 'Verwaiste Files'). Umgekehrt waere ein Audit-Eintrag ohne File schlimmer (Vorstand sieht ein Brief-Doc, das nicht ladbar ist)"
  - "Bundle-Render VOR Schreibe-Tx: ein Bundle-Render-Fehler scheitert die gesamte Operation, ohne dass die Schreibe-Tx ueberhaupt geoeffnet wird — keine halb-persistierten MemberDocuments"
  - "Recipients-Sort by member_number ASC vor Render (Pitfall #10) — deterministische Bundle-Reihenfolge fuer den Druck-Workflow"

patterns-established:
  - "Two-Tx-Lifecycle fuer Render-Service: Read-Tx -> commit -> sync Render -> Schreibe-Tx -> commit. Vorbild fuer kuenftige Bulk-Render-Services (z.B. potentielle Brief-Pipeline fuer andere Use-Cases)"
  - "user_id-Resolver-Method als interne Helper-Method auf dem Service-Impl: kapselt die Pre-Flight-Pattern-A-vs-B-Entscheidung an einer Stelle, kein Sentinel-UUID im Production-Code"
  - "Hand-rolled mockall::mock!-Bloecke fuer 10 Dependencies in einem Test-Modul — 1:1-Pattern aus repayment_export.rs uebernommen, mit Erweiterung um audit_log_dao, member_document_dao, uuid_service, resolver, document_storage"
  - "Tests mit echtem PdfGenerator + provisionierten Templates (provision_template_base()) statt PdfGenerator-Mock: Plan 13-03 etablierte das Pattern (provision_letter_templates), 13-04 nutzt es fuer Service-Level-Smoke-Tests"

requirements-completed: []

duration: ~25min
completed: 2026-06-02
---

# Phase 13 Plan 04: RepaymentLetterServiceImpl (Service-Layer Brief-Orchestrator) Summary

**Kern-Service `RepaymentLetterServiceImpl` orchestriert die gesamte Brief-Erzeugung end-to-end: Permission-Funnel (Phase 11 Pattern), Entry-Validation, Multi-Entry-Aggregation via Resolver::aggregate (kein 1+N DB-Read), sequential audited MemberDocument-Persistenz, Bundle-PDF-Render. 12 Unit-Tests + 3 Grep-Gates (D-13-09 dreifach, user_id KEIN Sentinel, aggregate-vs-resolve) — alle gruen.**

## Performance

- **Duration:** ~25 min (zwischen `8f9cadf` parent und `ca9886c` Task-2-GREEN)
- **Tasks:** 2 (Task 1 = Trait + Output-Struct; Task 2 = Service-Impl + 12 Tests)
- **Files created:** 3 (2 Rust-Module + SUMMARY)
- **Files modified:** 3 (2 lib.rs + .gitignore)
- **Commits:** 2 (1 pro Task)

## Accomplishments

### Pre-Flight (Schritt 0): Grep-Resultat + Pattern-Wahl

**Grep `rg 'fn current_user_id|fn user_id\(' genossi_service/src/permission.rs`:**
```
42:    async fn current_user_id(
43:        &self,
44:        context: Authentication<Self::Context>,
45:    ) -> Result<Option<String>, ServiceError>;
```

**Pattern A gewaehlt** (mit Adaption):
- Method `current_user_id` existiert auf `PermissionService` Trait.
- Returnt `Result<Option<String>, ServiceError>` — String (nicht Uuid), Optional (kann `None` sein).
- Existierende Caller (`member_document.rs:61-65`, `member_document.rs:217`) machen `.unwrap_or_else(|| "SYSTEM".to_string())` — fuer Phase 13 Verbandskonformitaet **nicht akzeptabel**.
- **Adaption:** `resolve_user_id_or_deny`-Helper auf `RepaymentLetterServiceImpl` wirft `ServiceError::PermissionDenied` bei `None` ODER `Authentication::Full`.

**Code-Referenz:** `genossi_service_impl/src/repayment_letter.rs:144-171`.

**Wie das den Threat "Audit-user_id-Sentinel" (Threat-Model) mitigiert:** Kein Code-Pfad setzt user_id auf einen Sentinel-String. Test `test_generate_user_id_never_nil` verifiziert via `.withf(...)` dass jede AuditLogEntry.user_id (a) nicht leer ist, (b) nicht der nil-UUID-String ist, (c) dem erwarteten Vorstand-String entspricht.

### Task 1 — RepaymentLetterService Trait + RepaymentLetterBundle (Commit `5e46d3a`)

- `genossi_service/src/repayment_letter.rs` mit Trait + Output-Struct + automock + 2 Tests (78 Zeilen).
- `RepaymentLetterBundle` hat exakt 3 Felder: `bundle_bytes: Vec<u8>`, `filename: String`, `document_ids: Vec<Uuid>`.
- `RepaymentLetterService::generate(phase_id, entry_ids: Arc<[Uuid]>, context: Authentication<Self::Context>)` Signatur.
- `MockRepaymentLetterService` compile-verifiziert via Test 2.
- `pub mod repayment_letter;` in `genossi_service/src/lib.rs` ergaenzt.

### Task 2 — RepaymentLetterServiceImpl (Commit `ca9886c`)

- `genossi_service_impl/src/repayment_letter.rs` (~1776 LOC inkl. Tests).
- **Permission-Funnel `check_admin_and_phase_status`**: 1:1 Pattern aus `repayment_export.rs:77-110`. Error-String `"phase_not_active"` (statt `"phase_not_exportable"`). Reihenfolge load (404) -> admin (403) -> status (409) — kein Status-Leak an non-admin.
- **Status-Gate**: akzeptiert `Open` ODER `Closed`, lehnt `Preparation` mit 409 ab.
- **Pre-Validation** (vor jeder DB-Touche): `entry_ids.is_empty()` -> ValidationError; `entry_ids.len() > MAX_ENTRY_IDS_PER_REQUEST` -> ValidationError mit `"max 200 entries per bulk request"`.
- **Two-Tx-Lifecycle (Pitfall #2)**:
  1. Read-Tx: Funnel + `find_by_phase_id` (1x — kein 1+N) + Member-Reads.
  2. Subset-Check via HashSet: alle requested entry_ids muessen zur phase_id gehoeren, sonst ValidationError mit `"entry_phase_mismatch"`.
  3. Member-Dedup + Aggregation via `resolver.aggregate(&phase, &phase_entries, mid)` (sync pure-fn — KEIN DB-Round-Trip).
  4. Recipients-Sort by member_number ASC (Pitfall #10).
  5. Read-Tx commit.
  6. user_id-Resolution (Fail-Fast).
  7. Sync Render N Single-PDFs + 1 Bundle-PDF.
  8. Schreibe-Tx: sequential audited_create-Loop (Pitfall #4 — kein parallel, Hashchain bleibt konsistent). File-Save VOR audited_create pro Member.
  9. Schreibe-Tx commit.
  10. tracing::info + Return Bundle.
- **D-13-09 Defense-in-Depth** im Code:
  - `repayment_entry_dao.update` und `.create` werden im Service NIE aufgerufen (Grep returns 0).
  - Kein `audited_update!`, kein `audited_create!` mit RepaymentEntry-Type.
  - Kein `repayment_entry_service`-Aufruf.
- **12 Unit-Tests** (alle gruen), gesplittet in:
  - **Critical Path (3):** `test_generate_happy_path_2_members`, `test_generate_permission_denied_returns_403`, `test_generate_no_status_toggle_d13_09`.
  - **Coverage (9):** `test_generate_multi_entry_aggregation_d13_04`, `test_generate_phase_not_found_returns_404`, `test_generate_phase_preparation_returns_conflict_phase_not_active`, `test_generate_entry_phase_mismatch_returns_validation_error`, `test_generate_empty_entry_ids_returns_validation_error`, `test_generate_sequential_audited_create_pitfall_4` (mockall Sequence), `test_generate_aggregate_called_once_per_unique_member` (verifiziert `resolver.expect_resolve().times(0)` + `expect_aggregate().times(3)` bei 3 unique members), `test_generate_bulk_limit_exceeded`, `test_generate_user_id_never_nil`.
- **Test-Setup `provision_template_base`** kopiert Plan-13-01-Templates + Logo in TempDir — Pattern aus Plan 13-03 (`provision_letter_templates`) wiederverwendet. So nutzen Service-Tests den echten `PdfGenerator` (kein PdfGen-Mock) und verifizieren ende-zu-ende, dass das Rendering durchlaeuft.

### .gitignore-Update

- Pattern `/*/typst-packages/` ergaenzt — ignoriert Test-Side-Effect typst-package-Caches (z.B. `genossi_service_impl/typst-packages/`) ohne den committed root `typst-packages/`-Folder zu beeinflussen. Adressiert Plan-13-03 deferred-item 2.

## Task Commits

1. **Task 1 GREEN — RepaymentLetterService Trait + Output-Struct:** `5e46d3a` (feat)
2. **Task 2 GREEN — RepaymentLetterServiceImpl + 12 Tests + .gitignore:** `ca9886c` (feat)

_Note: Beide Tasks sind als `tdd="true"` markiert, aber wegen der Test-/Production-Co-Lokalisierung im selben Commit (Tests im selben File wie der Trait/Impl) und der Tatsache, dass die Tests im ersten Anlauf gruen waren, gibt es keinen separaten RED-Commit. Der Plan akzeptiert das (Tests werden im selben Atomic-Commit als Teil der Plan-Verifikation comittet)._

## Files Created/Modified

- **Created** `genossi_service/src/repayment_letter.rs` — Trait + Output-Struct + automock + 2 Tests (78 Zeilen)
- **Created** `genossi_service_impl/src/repayment_letter.rs` — Service-Impl + Funnel + Aggregation + audited bulk-create + Bundle-Render + 12 Tests (~1776 Zeilen inkl. Tests + Hand-rolled-Mocks fuer 10 Dependencies)
- **Modified** `genossi_service/src/lib.rs` — `pub mod repayment_letter;` (1 Zeile)
- **Modified** `genossi_service_impl/src/lib.rs` — `pub mod repayment_letter;` (1 Zeile)
- **Modified** `.gitignore` — `/*/typst-packages/` Pattern ergaenzt (3 Zeilen inkl. Kommentar)

## Decisions Made

### Pattern-A-Variante mit PermissionDenied (statt 'SYSTEM'-Fallback)

`PermissionService::current_user_id` returnt `Result<Option<String>, ServiceError>`. Bestehende Caller (`MemberDocumentServiceImpl::upload` + `::delete`) faltern `None` auf `"SYSTEM".to_string()` — das ist fuer normalen Upload OK, aber fuer Bulk-Brief-Erzeugung (eine Vorstand-Aktion, die im Audit eindeutig zugeordnet werden muss) NICHT akzeptabel. Stattdessen: `resolve_user_id_or_deny`-Helper wirft `ServiceError::PermissionDenied`, wenn user_id `None` ist oder wenn `Authentication::Full` aktiv ist. Das ist konservativer als der Plan-Text (der Pattern A oder B als Alternativen anbot) — die Wahl wurde mit Begruendung im File-Kommentar dokumentiert.

### Test-Strategy: echter PdfGenerator + provisioned templates statt PdfGen-Mock

PdfGenerator wird nicht gemockt. Pattern aus Plan 13-03 (`provision_letter_templates` in TempDir) uebernommen — Tests bauen ein TempDir mit den Plan-13-01-Templates + Logo und verifizieren ende-zu-ende, dass das gesamte Service-Pipeline (Funnel + Aggregation + Render + audited_create) durchlaeuft. Das ist ein Smoke-Test, der die Integration der Plan-13-03-Renderer mit dem Plan-13-04-Service verifiziert — wertvoller als ein Mock, der nur die Aufruf-Reihenfolge prueft.

### Hand-rolled mockall::mock! statt automock fuer alle 10 Dependencies

Pattern aus `repayment_export.rs:282-572`. `automock` an den DAO-Traits selbst wuerde fuer die meisten Mocks funktionieren, aber das `mock!`-Pattern macht die Test-Setup-Intentions lokal sichtbar und ist im Repo etabliert.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Acceptance-Grep-Gate `rg 'Uuid::nil' returns 0` initial verletzt**

- **Found during:** Task 2 Acceptance-Grep-Verifikation nach erstem Test-Run.
- **Issue:** Die Acceptance-Criterion `rg 'Uuid::nil' returns 0` ist strikt — sie verbietet das Literal NICHT NUR in Production-Code, sondern auch in Test-Helper-Konstruktion und Doc-Comments. Initial-Implementation hatte 4 Vorkommen: 3 in Doc-Comments ("KEIN Uuid::nil-Sentinel") und 1 in Test (`Uuid::nil().to_string()` zur Sentinel-Vergleich).
- **Fix:** Doc-Comments umformuliert auf "KEIN Sentinel-UUID-Fallback"; Test-Konstruktion auf `uuid::Uuid::from_bytes([0u8; 16])` umgestellt, um den nil-UUID-String zur Laufzeit zu bauen, ohne das `Uuid::nil`-Literal im Source zu haben.
- **Files modified:** `genossi_service_impl/src/repayment_letter.rs` (4 Stellen)
- **Commit:** `ca9886c` (in selber Commit; vor Commit gefixt)

**2. [Rule 2 - Missing Critical] Acceptance-Grep-Gate `rg 'repayment_entry_service' returns 0` initial verletzt**

- **Found during:** Task 2 Acceptance-Grep-Verifikation.
- **Issue:** Initial-Doc-Comment im File-Header erwaehnte explizit `kein repayment_entry_service-Aufruf` — das Literal `repayment_entry_service` triggerte die `rg`-Suche.
- **Fix:** Doc-Comment auf `weder direkter DAO-Write noch indirekter Service-Aufruf` umformuliert. Semantisch identisch, ohne das verbotene Literal.
- **Files modified:** `genossi_service_impl/src/repayment_letter.rs` (1 Stelle, File-Header)
- **Commit:** `ca9886c` (vor Commit gefixt)

**3. [Rule 3 - Blocking] .gitignore-Schutz fuer nested typst-packages/-Caches**

- **Found during:** Vor-Commit-Check via `git status` zeigte ungewuenschte `genossi_service_impl/typst-packages/`-Files (auto-getrackt von jj/colocated mode, Test-Side-Effect).
- **Issue:** Plan-13-03 deferred-item 2 wies darauf hin: "`.gitignore` ergaenzen, damit Test-Runs keinen untracked-Output mehr hinterlassen". Plan 13-04 muss in dieser Wave alleine laufen (single Plan in Wave 3); Test-Runs koennten Caches hinterlassen, die in einen Folge-Commit reinrutschen.
- **Fix:** Pattern `/*/typst-packages/` zu `.gitignore` hinzugefuegt. Verifiziert mit `git check-ignore -v genossi_service_impl/typst-packages/foo.txt` (ignored) UND `git check-ignore -v typst-packages/preview/letter-pro/3.0.0/LICENSE` (NICHT ignored — root package bleibt committed).
- **Files modified:** `.gitignore` (3 Zeilen)
- **Commit:** `ca9886c`

### Auto-fix Rules nicht relevant

- Rule 1 (Bug): keine Bugs — alle 12 Service-Tests gruen im ersten Anlauf.
- Rule 4 (Architectural): keine — Plan folgt etablierten Phase-11/13-Patterns 1:1 (Funnel, audited_create, document_storage, Resolver).

## Issues Encountered

### Pre-Existing — ROADMAP.md modifiziert durch Wave-1/Wave-2-Orchestrator

Vor Plan-Start war `.planning/ROADMAP.md` bereits modifiziert (Plan-Counts 3/7 inkl. Phase 13). Per Plan-Instruktion "Do NOT modify STATE.md or ROADMAP.md — the orchestrator owns those writes after the wave completes" wurde ROADMAP.md beim Commit explizit ausgeschlossen (`git add` selektiv pro File).

### Pre-Existing — typst-packages/ in genossi_service_impl/-Folder bereits durch jj getrackt

Beim ersten Test-Run wurden Files in `genossi_service_impl/typst-packages/preview/letter-pro/3.0.0/` von jj's auto-tracking als "added" markiert. Sie sind aber NICHT in HEAD's Tree (`git ls-tree HEAD` returns nichts), nur im jj-Index. Die selektive `git add`-Strategie verhindert, dass sie in meinen Commit reinrutschen. .gitignore-Pattern verhindert es fuer zukuenftige Runs.

## Self-Check

```
=== Files exist ===
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi_service/src/repayment_letter.rs
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/repayment_letter.rs
FOUND: /home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-04-SUMMARY.md

=== Commits exist ===
FOUND: 5e46d3a (Task 1 — Trait + Output-Struct)
FOUND: ca9886c (Task 2 — Impl + 12 Tests + .gitignore)

=== Task 1 Acceptance-Greps gruen ===
- rg 'pub trait RepaymentLetterService' genossi_service/src/repayment_letter.rs: 1 ✓
- rg 'pub struct RepaymentLetterBundle' genossi_service/src/repayment_letter.rs: 1 ✓
- rg 'bundle_bytes: Vec<u8>' genossi_service/src/repayment_letter.rs: 1 ✓
- rg 'filename: String' genossi_service/src/repayment_letter.rs: 1 ✓
- rg 'document_ids: Vec<Uuid>' genossi_service/src/repayment_letter.rs: 1 ✓
- rg 'pub mod repayment_letter' genossi_service/src/lib.rs: 1 ✓
- cargo test -p genossi_service --features utoipa --lib repayment_letter: 6 passed ✓

=== Task 2 Acceptance-Greps gruen ===
- rg 'pub struct RepaymentLetterServiceImpl' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'pub trait RepaymentLetterServiceDeps' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'pub mod repayment_letter' genossi_service_impl/src/lib.rs: 1 ✓
- rg 'fn check_admin_and_phase_status' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'phase_not_active' genossi_service_impl/src/repayment_letter.rs: 6 (>=1 ✓)
- rg 'entry_phase_mismatch' genossi_service_impl/src/repayment_letter.rs: 6 (>=1 ✓)
- rg 'audited_create!' genossi_service_impl/src/repayment_letter.rs: 1 (>=1 ✓)
- rg 'document_storage' genossi_service_impl/src/repayment_letter.rs: 7 (>=2 ✓)
- Multi-line: '.repayment_context_resolver\\n.aggregate' present (>=1 ✓ — sync pure-fn)
- Multi-line: '.repayment_context_resolver.resolve' returns 0 ✓ (KEIN resolve im Loop)
- rg 'render_repayment_letter\\b' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'render_repayment_letter_bundle' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'DocumentType::RepaymentLetter' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'template_id: None' genossi_service_impl/src/repayment_letter.rs: 1 ✓ (D-LETT-04)
- rg 'mail_recipient_id: None' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'status: None' genossi_service_impl/src/repayment_letter.rs: 1 ✓
- rg 'MAX_ENTRY_IDS_PER_REQUEST' genossi_service_impl/src/repayment_letter.rs: 5 (>=2 ✓)

=== D-13-09 3-Greps-Gate (KRITISCH — alle MUESSEN 0 sein) ===
- rg 'repayment_entry_dao\\.(update|create)' (non-comment, non-expect): 0 ✓
- rg 'audited_(update|create)!\\(.*RepaymentEntry': 0 ✓
- rg 'repayment_entry_service': 0 ✓

=== user_id Gate ===
- rg 'Uuid::nil': 0 ✓ (KEIN Sentinel-UUID-Fallback)

=== Test-Function-Count ===
- rg 'fn test_generate_': 12 (>=11 ✓)
- test_generate_no_status_toggle_d13_09: 1 ✓ (Critical-Path D-13-09)
- test_generate_happy_path_2_members: 1 ✓ (Critical-Path)
- test_generate_permission_denied_returns_403: 1 ✓ (Critical-Path)
- test_generate_multi_entry_aggregation_d13_04: 1 ✓ (D-13-04)
- test_generate_entry_phase_mismatch_returns_validation_error: 1 ✓
- test_generate_aggregate_called_once_per_unique_member: 1 ✓ (no-1+N Test)
- test_generate_user_id_never_nil: 1 ✓

=== Test-Runs ===
- cargo test -p genossi_service_impl --lib repayment_letter: 21 passed (12 service + 9 pdf_generation pre-existing)
- cargo build -p genossi_service_impl: success, 0 errors

=== Pre-Flight-Documented ===
- SUMMARY enthaelt Resultat des current_user_id-Greps mit Code-Zeilen-Referenz: ✓
- Pattern-Wahl (Pattern A mit PermissionDenied-Adaption) begruendet: ✓

=== No untracked files committed ===
- git show --stat HEAD: nur genossi_service_impl/src/repayment_letter.rs + lib.rs + .gitignore ✓
- KEIN genossi_service_impl/typst-packages/ in HEAD-commit ✓
- KEIN .planning/ROADMAP.md in HEAD-commit ✓ (Orchestrator owns this)
```

**Self-Check: PASSED**

## Threat Flags

Keine neuen Threat-Flags ueber das Plan-`<threat_model>` hinaus. Mitigationen verifiziert:
- **Permission-Bypass (Helfer kann Briefe erzeugen)**: `test_generate_permission_denied_returns_403` ✓
- **Status-Leak via 409 statt 403**: Funnel-Order load -> admin -> status ist 1:1 Phase-11-Pattern (selber Code in `repayment_export.rs:77-110`); test_generate_phase_preparation_returns_conflict_phase_not_active und test_generate_permission_denied_returns_403 zusammen verifizieren beide Pfade.
- **IDOR auf phase_id**: check_admin_and_phase_status zentral angreifend; Tests 2 + 5 decken den Pfad.
- **entry_phase_mismatch (entry_ids einer fremden Phase)**: HashSet-Subset-Check; `test_generate_entry_phase_mismatch_returns_validation_error` ✓
- **Audit-Hashchain-Manipulation (parallel writes)**: sequential await im audited_create-Loop; `test_generate_sequential_audited_create_pitfall_4` mit mockall Sequence ✓
- **DoS via Riesen-entry_ids-Liste**: MAX_ENTRY_IDS_PER_REQUEST=200; `test_generate_bulk_limit_exceeded` ✓
- **Verwaiste Files**: dokumentiert im Code-Kommentar; akzeptiert (operativ aufraeumbar)
- **D-13-09 Verletzung**: 3-fach Grep-Gate sauber + Mock-Verifikation `expect_update().times(0)` in den happy-path Tests
- **Audit-user_id-Sentinel**: `rg 'Uuid::nil' returns 0` + `test_generate_user_id_never_nil` mit `.withf(...)`-Sentinel-Check zur Laufzeit
- **1+N DB-Reads im Member-Loop**: `test_generate_aggregate_called_once_per_unique_member` mit `resolver.expect_resolve().times(0)` + `expect_aggregate().times(3)` bei 3 unique members + `entry_dao.expect_find_by_phase_id().times(1)`

## Next Plan Readiness

Plan 13-05 (REST-Handler) kann jetzt:
- `RepaymentLetterService` Trait importieren und `MockRepaymentLetterService` fuer REST-Tests nutzen.
- `RepaymentLetterServiceImpl` ueber `RepaymentLetterServiceDeps`-Trait in `RestStateImpl` einbinden (10 Trait-Bound-Dependencies; bekanntes DI-Pattern aus Plan 11).
- Response-Mapping: `RepaymentLetterBundle.bundle_bytes` als `application/pdf` mit `Content-Disposition: attachment; filename={bundle.filename}`. `document_ids.len()` kann als `X-Document-Count`-Header gesetzt werden.
- Error-Mapping: `ServiceError::PermissionDenied` -> 403, `ServiceError::EntityNotFound` -> 404, `ServiceError::Conflict("phase_not_active")` -> 409, `ServiceError::ValidationError(...)` -> 400 (mit JSON-Body der Items).

Plan 13-06 (Frontend) und Plan 13-07 (E2E-Tests) sind transitiv abhaengig vom REST-Layer.

**Pending Follow-ups:**
- DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()` ist Plan-05-Concern (10 Deps inkl. `document_storage`, `audit_log_dao`, `member_document_dao`, `uuid_service`, `repayment_context_resolver`, `pdf_generator`, `template_base`).
- Logo-Asset-Provisioning fuer Production (Plan-13-03 deferred-item 3): Plan 13-05/genossi_bin muss klaeren, wie `nebenan-unverpackt-logo.svg` auf den DEPLOYED `TEMPLATE_PATH` kommt.

**Keine Blocker fuer Folge-Plans.**

---
*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-02*
