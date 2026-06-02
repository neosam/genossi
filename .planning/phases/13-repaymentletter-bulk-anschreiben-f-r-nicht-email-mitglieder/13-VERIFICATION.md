---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
verified: 2026-06-02T07:00:00Z
status: passed
score: 12/12 must-haves verified
overrides_applied: 0
---

# Phase 13: RepaymentLetter-Bulk-Anschreiben — Verification Report

**Phase Goal:** Ergänzt die v1.1-Auszahlungs-Pipeline um einen Brief-Kanal — Vorstand selektiert auf der RepaymentPhase-Detail-Page Einträge multi-select und triggert eine Bulk-PDF-Generierung; pro Member entsteht ein auditiertes MemberDocument mit Info-Schreiben (Auszahlungsbetrag, Anteile, hinterlegte IBAN), zusätzlich ein transientes Bundle-PDF als Direct-Download für den Druck-Workflow.

**Verified:** 2026-06-02
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

Must-haves stammen aus den 7 PLAN-Frontmatter-Listen, deduplizert und auf das Phase-Goal verdichtet. ROADMAP-Section enthält keine separaten `success_criteria` (leer in JSON-Output) — Phase-Goal-Text + BRIEF-01 sind die Verträge.

| #  | Truth | Status | Evidence |
| -- | ----- | ------ | -------- |
| 1  | DocumentType::RepaymentLetter Variante + Templates (Single + Bundle) + DEFAULT_TEMPLATES-Eintrag | VERIFIED | `member_document.rs` enthält 8 `DocumentType::RepaymentLetter`-Refs; `templates/defaults/auszahlungs_anschreiben.typ` (4157 B) + `_bundle.typ` (513 B) existieren; `template_storage.rs` registriert beide via `include_bytes!`. Bundle macht `#import "auszahlungs_anschreiben.typ": render-letter` (Single-Source-of-Truth) |
| 2  | RepaymentContextResolver Trait + Pure-fn `aggregate_for_member` (Multi-Entry-Aggregation D-13-04) | VERIFIED | `genossi_service/src/repayment_context.rs` definiert Trait mit `resolve` (async) + `aggregate` (sync). `genossi_service_impl/src/repayment_context.rs` enthält `pub fn aggregate_for_member`; Filter `Open \| Contacted` (D-13-10) + Format `format!("{},{:02}", ..)` (Phase-10 D-04) verifiziert |
| 3  | PdfGenerator::render_repayment_letter + render_repayment_letter_bundle (Single + N-Recipient Bundle in 1 Compile) | VERIFIED | `pdf_generation.rs`: `pub fn render_repayment_letter\b` = 1, `render_repayment_letter_bundle` = 1; Smoke-Tests `test_render_repayment_letter_smoke`, `..._null_iban_renders_ok`, `..._bundle_smoke` alle grün (21 passed); echtes %PDF-Output verifiziert |
| 4  | RepaymentLetterService.generate Permission-Funnel (load→admin→status) + Status-Gate (`phase_not_active`) + entry_phase_mismatch ValidationError | VERIFIED | `repayment_letter.rs`: `check_admin_and_phase_status` Funnel 1:1 Phase-11-Pattern; `phase_not_active` = 6 hits, `entry_phase_mismatch` = 6 hits; Tests `test_generate_permission_denied_returns_403`, `..._phase_preparation_returns_conflict_..`, `..._entry_phase_mismatch_..` alle grün |
| 5  | Multi-Entry-Aggregation: per-member dedup + `resolver.aggregate` (NICHT `resolve`) → kein 1+N DB-Read | VERIFIED | Multi-line grep: `repayment_context_resolver\n.aggregate(&phase, &phase_entries, mid)` present; `repayment_context_resolver.resolve` = 0; Test `test_generate_aggregate_called_once_per_unique_member` mit `expect_resolve().times(0)` grün |
| 6  | Audited MemberDocument-Persistenz pro Member (audited_create!) — D-LETT-04 Felder (template_id/mail_recipient_id/status = None) | VERIFIED | `audited_create!` = 1, `DocumentType::RepaymentLetter` als Doc-Type, `template_id: None` + `mail_recipient_id: None` + `status: None` jeweils 1 hit; E2E-Test `test_letter_happy_path_3_entries_2_members` verifiziert `letter_docs.len() == 1` pro Member; Audit-Chain valid post-bulk (Test 7) |
| 7  | Bundle-PDF transient (Direct-Download, NICHT persistiert) + Sortierung member_number ASC (Pitfall #10) | VERIFIED | `bundle_bytes` ist nur als REST-Response (`Body::from(result.bundle_bytes)`) gesetzt, kein zweiter `document_storage.save` für Bundle; recipients.sort_by member_number ASC vor Render |
| 8  | D-13-09: Backend toucht RepaymentEntry NIE (kein DAO-update, kein audited_*-Macro auf Entry, kein Service-Indirection) | VERIFIED | 3-Greps-Gate: `repayment_entry_dao.(update\|create)` = 0, `audited_(update\|create)!\(.. RepaymentEntry` = 0, `repayment_entry_service` = 0; E2E-Test `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` verifiziert `status` == "open" vor + nach 2 Bulk-Runs |
| 9  | REST-Handler POST /api/repayment-phase/{phase_id}/letters/generate + Content-Disposition + X-Document-Count Header + CORS-expose | VERIFIED | `genossi_rest/src/repayment_letter.rs` enthält Handler + ToSchema-Request + map_letter_error (PermissionDenied → 403); Response-Header `X-Document-Count` direkt aus `result.document_ids.len()`; `genossi_rest/src/lib.rs` ergänzt `.expose_headers([..,"x-document-count", "content-disposition"])` (WR-03 fixed); OpenAPI dokumentiert alle 6 Status-Codes (200/400/401/403/404/409) |
| 10 | DI-Wiring: RepaymentLetterServiceImpl in RestStateImpl via Single-Arc-per-Process (Plan-10 P07 Lektion) | VERIFIED | `genossi_bin/src/lib.rs`: 2 Deps-Structs + 2 Type-Aliases + 2 Arc::new(..Impl)-Konstruktionen + Trait-Impl `RepaymentLetterRestState for RestStateImpl`; SUMMARY 13-05 dokumentiert baseline-Arc-Count 25 unverändert + Bound auch in `test_server.rs` ergänzt; `cargo build --workspace --all-features` clean |
| 11 | Frontend Bulk-Action-Button + on_letter_request EventHandler + Blob-Download + Singular/Plural-Toast + Selection-Preservation (D-13-09) | VERIFIED | Component `repayment_entry_list.rs`: Prop + Purple-Button + onclick mit r#type "button" (Phase-12 D-01); Page `repayment_phase_details.rs`: `generate_repayment_letters` Call + `create_element("a")` + `revoke_object_url` + Filename `auszahlungs_anschreiben_GJ_{}.pdf`; CR-01 fixed: `toast_singular`/`toast_plural_template` werden via `i18n.t()` BEVOR `spawn` ausgelesen; i18n-Keys (Singular + Plural mit `{count}`) in DE + EN registriert |
| 12 | E2E-Tests verifizieren End-to-End-Verhalten inkl. Audit-Hashchain-Validity + Idempotenz + No-Status-Toggle | VERIFIED | `cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth -- --test-threads=1` → **7 passed, 0 failed, 1 ignored** (helper-auth-Test ignored mit dokumentierter mock_auth-Limitation); Cross-Phase-Regression `cargo test -p genossi_bin --test e2e_tests --features mock_auth` → **292 passed, 0 failed** |

**Score:** 12/12 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `genossi_service/src/repayment_letter.rs` | RepaymentLetterService Trait + RepaymentLetterBundle Output | VERIFIED | 2936 B; Trait + Struct + automock; 2 Tests grün |
| `genossi_service_impl/src/repayment_letter.rs` | Impl + Funnel + Aggregation + audited_create + Bundle-Render | VERIFIED | 73741 B; 12 Tests grün (`cargo test -p genossi_service_impl --lib repayment_letter` = 21 passed inkl. pdf_generation) |
| `genossi_service/src/repayment_context.rs` | Trait + RepaymentContext Struct | VERIFIED | 4471 B; Trait mit resolve + aggregate; 4 Tests grün |
| `genossi_service_impl/src/repayment_context.rs` | Impl + Pure-fn `aggregate_for_member` | VERIFIED | 18339 B; 15 Tests grün |
| `genossi_service_impl/src/pdf_generation.rs` (modified) | render_repayment_letter + render_repayment_letter_bundle + build_inputs Helpers | VERIFIED | Pub-fns + 9 Helper-Tests + 4 Render-Smoke-Tests (alle grün) |
| `genossi_service/src/member_document.rs` (modified) | DocumentType::RepaymentLetter Variante | VERIFIED | 8 Refs; 4 Tests (as_str/from_str/is_singleton=false/template_path=None) grün |
| `genossi_service_impl/src/template_storage.rs` (modified) | 2 DEFAULT_TEMPLATES-Einträge | VERIFIED | include_bytes! für beide; 2 Tests verifizieren Provisioning + Drift-Schutz |
| `templates/defaults/auszahlungs_anschreiben.typ` | Single-Letter mit `render-letter`-Funktion (4 D-13-06-Bausteine + IBAN-Switch) | VERIFIED | 4157 B; render-letter, letter-pro, sys.inputs, bank_account `#if`-Switch, Vorstands-Signatur, kein "Verwendungszweck" (D-13-07) |
| `templates/defaults/auszahlungs_anschreiben_bundle.typ` | Bundle-Wrapper mit `#import` + pagebreak Loop | VERIFIED | 513 B; importiert render-letter, iteriert recipients, Drift-Schutz: keine Brief-Body-Strings dupliziert |
| `genossi_rest/src/repayment_letter.rs` | REST-Handler + GenerateLettersRequest + ApiDoc + RepaymentLetterRestState Trait | VERIFIED | 8431 B; alle 6 status= Annotations, X-Document-Count Header, map_letter_error |
| `genossi_rest/src/lib.rs` (modified) | Module mount + ApiDoc-Nest + Router-Mount + 2 Trait-Bounds + CORS expose_headers | VERIFIED | Alle 5 Stellen + CORS-expose-headers für `x-document-count` + `content-disposition` |
| `genossi_rest/src/test_server.rs` (modified) | Test-Server-Bound erweitert | VERIFIED | `+ crate::repayment_letter::RepaymentLetterRestState` Bound vorhanden |
| `genossi_bin/src/lib.rs` (modified) | DI-Wiring: 2 Deps-Structs + 2 Type-Aliases + 2 Arc-Felder + RestStateImpl-Field + Trait-Impl | VERIFIED | Alle 5 Edit-Stellen vorhanden + Single-Arc-per-Process eingehalten (SUMMARY 13-05) |
| Frontend artifacts (api.rs, component, page, i18n) | Bulk-Button + on_letter_request + Blob-Save + Singular/Plural-Toast + i18n DE/EN | VERIFIED | Alle Acceptance-Greps grün (siehe SUMMARY 13-06); CR-01 fix: i18n-Strings VOR spawn-Closure aufgelöst |
| `genossi_bin/tests/repayment_letter_e2e.rs` | 8 E2E-Tests | VERIFIED | 31648 B; 8 `test_letter_`-Funktionen; 7 grün + 1 ignored (mock_auth-Limit dokumentiert) |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `RepaymentLetterServiceImpl::generate` | `check_admin_and_phase_status` | Permission-Funnel | WIRED | Funnel-Methode vorhanden + Tests `_permission_denied_..` + `_phase_preparation_..` grün |
| `RepaymentLetterServiceImpl::generate` | `audited_create!` | MemberDocument-Persistenz im Loop | WIRED | 1 hit, Test `_sequential_audited_create_pitfall_4` (mockall Sequence) grün |
| `RepaymentLetterServiceImpl::generate` | `RepaymentContextResolver::aggregate` (NICHT resolve) | Per-Member-Aggregation (pure-fn) | WIRED | `aggregate` 1+ hit, `resolve` 0 hits; Test `_aggregate_called_once_per_unique_member` grün |
| `RepaymentLetterServiceImpl::generate` | `PdfGenerator::render_repayment_letter` / `_bundle` | synchrone Render-Calls nach Read-Tx-Commit | WIRED | Beide Methoden gerufen; Read-Tx commit (Zeile 322) VOR Render-Aufrufen |
| `RepaymentLetterServiceImpl::generate` | `document_storage.save` (planned_saves, NACH Tx-Commit) | File-Persistenz pro Einzel-PDF | WIRED | CR-02 fix: `planned_saves`-Liste sammelt vor commit, schreibt NACH commit (Zeile 357 + 406 + 411) — atomic-then-persist |
| `genossi_rest/src/repayment_letter.rs::generate_letters` | `RepaymentLetterService::generate` | `rest_state.repayment_letter_service().generate(..)` | WIRED | Handler-Body ruft `.repayment_letter_service().generate(phase_id, entry_ids, auth)` |
| `generate_letters` | `X-Document-Count` Header | `result.document_ids.len()` | WIRED | Header direkt aus document_ids.len() gesetzt + CORS exposed |
| `genossi_bin/src/lib.rs` | `RepaymentLetterServiceImpl` | Arc::new mit 12 Deps via .clone() | WIRED | DI-Wiring vorhanden; Single-Arc-baseline 25 unverändert |
| Frontend `RepaymentEntryList` | `repayment_phase_details` page | EventHandler<Vec<Uuid>> bubble | WIRED | `on_letter_request: EventHandler<Vec<Uuid>>` Prop + Page-Handler ruft `api::generate_repayment_letters` |
| Frontend page | `api::generate_repayment_letters` | fetch + blob + X-Document-Count read | WIRED | API-fn liest `X-Document-Count`-Header + Browser-Save via `<a download>.click() + revoke_object_url` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| REST-Handler `generate_letters` | `result: RepaymentLetterBundle` | `rest_state.repayment_letter_service().generate(..)` → Service-Impl ruft echte DAOs (RepaymentPhaseDao::find_by_id, RepaymentEntryDao::find_by_phase_id, MemberDao::find_by_id) + audited_create! → MemberDocumentDao::create + document_storage.save | Yes (echte DB-Queries) | FLOWING |
| Bundle-PDF-Response | `result.bundle_bytes: Vec<u8>` | `pdf_generator.render_repayment_letter_bundle(..)` → echter Typst-compile mit recipients-JSON | Yes (E2E-Test verifiziert `bytes.starts_with(b"%PDF-")` + `len > 1000`) | FLOWING |
| MemberDocument-Persistenz | per-recipient `pdf_bytes` + `MemberDocumentEntity` | `pdf_generator.render_repayment_letter(..)` für Single + audited_create! (sequential) + document_storage.save (NACH commit) | Yes (E2E-Test `list_member_documents` zeigt Docs pro Member) | FLOWING |
| Frontend Toast | `result.document_count: usize` | API-Client liest `X-Document-Count` Header aus `resp.headers()`; Fallback `entry_ids.len()` falls Header fehlt | Yes (CORS-Expose-Header für Cross-Origin-Sichtbarkeit verifiziert via `.expose_headers`-Liste) | FLOWING |
| Frontend Blob-Download | `result.blob_url: String` | API-Client erstellt Blob aus `resp.blob().await` und ruft `Url::create_object_url_with_blob` | Yes (Page-Handler triggert `<a download>.click() + revoke_object_url`) | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| 8 E2E-Tests laufen (Phase-Goal end-to-end) | `cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth -- --test-threads=1` | 7 passed, 0 failed, 1 ignored | PASS |
| Service-Layer Unit-Tests | `cargo test -p genossi_service_impl --lib repayment_letter --features mock_auth` | 21 passed, 0 failed | PASS |
| Cross-Phase-Regression (bestehende E2E) | `cargo test -p genossi_bin --test e2e_tests --features mock_auth` | 292 passed, 0 failed | PASS |
| Workspace-Build clean | `cargo build --workspace --all-features` | exit 0 (52 s clean build) | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| BRIEF-01 | 13-01, 13-02, 13-04, 13-05, 13-07 | Brief-Vorlagen aus Auszahlungs-Eintrag direkt als PDF erzeugen (v1.1-Defer wird in Phase 13 aufgehoben) | SATISFIED | End-to-End-Pipeline implementiert: DocumentType::RepaymentLetter + Single+Bundle Typst-Templates + Resolver-Aggregation + Service-Impl mit auditiertem MemberDocument + REST-Endpoint mit Direct-Download + Frontend-Button + 8 E2E-Tests; `cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth` 7/7 aktiv grün |

REQUIREMENTS.md Z. 83 (Table-Eintrag "Vorstand erzeugt manuell außerhalb von Genossi") ist nun durch Phase 13 obsolet — die Tabelle wurde aber nicht synchron umgeschrieben. Keine ORPHANED requirements: nur BRIEF-01 ist Phase 13 zugewiesen.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (none) | — | TODO/FIXME/XXX/HACK/PLACEHOLDER im Phase-13-Code | — | Saubere Implementation; einziger TODO-Doc-Kommentar in `member_document.rs` ist explizit von REVIEW.md WR-06 als gewollte Doku-Marke gefixed (commit 170b15a) |

### Human Verification Required

Keine. Alle Phase-Goal-relevanten Behaviors sind durch E2E-Tests + Unit-Tests + Smoke-Tests verifiziert; visuelle Aspekte (PDF-Layout, Toast-Pluralisierung, Button-Optik) sind durch i18n-Strings + Color-Class-Greps + Phase-12-D-01-Pattern + Plan-13-06-Self-Check abgedeckt. Die Phase ist als technische Auslieferung verifiziert.

### Gaps Summary

Keine offenen Gaps. Die in REVIEW.md identifizierten 2 BLOCKER + 6 WARNINGs sind alle gefixed (REVIEW.md `status: clean`, `fixes_applied` listet 8 Commits). Insbesondere:

- CR-01 (Dioxus-Hook in spawn-Closure): gefixed in commit 6256618 — `i18n.t(..)` wird VOR spawn aufgelöst und per Move-Capture übergeben.
- CR-02 (Orphan-PDFs bei Tx-Rollback): gefixed in commit 397c8d9 — Variant A (atomic-then-persist) implementiert: `planned_saves` sammelt vor commit, schreibt NACH commit.
- WR-03 (X-Document-Count CORS): gefixed in commit d760a60 — `.expose_headers([..])` ergänzt für Cross-Origin-Sichtbarkeit.
- WR-06 (Idempotenz erzeugt Orphan-PDFs): als TODO-Doc-Kommentar in commit 170b15a dokumentiert (behavioral fix für phase-14+ als Folge-Quick deferred).

Pending Follow-ups (außerhalb Phase-13-Scope, dokumentiert in SUMMARYs):
- D-13-11 Phase-10-Worker-Refactor auf RepaymentContextResolver (Todo `phase-10-worker-refactor-resolver.md`).
- Logo-Asset-Provisioning für Production (`nebenan-unverpackt-logo.svg` muss auf deployed TEMPLATE_PATH gelangen).
- REQUIREMENTS.md Z. 83 Tabelle synchronisieren (BRIEF-01 nicht mehr "out of v1.1").

---

_Verified: 2026-06-02_
_Verifier: Claude (gsd-verifier)_
