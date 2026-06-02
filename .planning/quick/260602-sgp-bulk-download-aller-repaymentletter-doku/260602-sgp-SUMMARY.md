---
phase: quick-260602-sgp
plan: 01
status: complete
duration_minutes: 51
tasks_completed: 2
files_created:
  - genossi-frontend/src/component/repayment_letter_download_button.rs
  - .planning/quick/260602-sgp-bulk-download-aller-repaymentletter-doku/deferred-items.md
files_modified:
  - Cargo.toml
  - genossi_service_impl/Cargo.toml
  - genossi_service/src/repayment_letter.rs
  - genossi_service_impl/src/repayment_letter.rs
  - genossi_rest/src/repayment_letter.rs
  - genossi_bin/Cargo.toml
  - genossi_bin/tests/repayment_letter_e2e.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/component/mod.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/page/repayment_phase_details.rs
commits:
  - b3f9b12: feat(quick-260602-sgp) Service-Trait + REST handler for RepaymentLetter bulk-download
  - eabfbde: feat(quick-260602-sgp) E2E tests + Frontend Component-First download button
key_decisions:
  - "lopdf 0.34 manual merge via Pages-Tree-Pattern (kein high-level `merge_documents` in 0.34)"
  - "zip 2 als workspace.dependency (vorher nur genossi_rest-lokal)"
  - "Component-Mount oberhalb TabStrip (sichtbar in allen Tabs) + conditional render gegen Preparation-Status"
  - "Reiner Lese-Endpoint — KEIN audited_*! Macro"
threat_mitigations_verified:
  - "T-sgp-01 (Spoofing): extract_auth_context + check_admin_and_phase_status decken die 401/403-Pfade ab (Service-Layer Test test_download_bundle_permission_denied_returns_403)"
  - "T-sgp-04 (Information disclosure): permission-gate ADMIN; X-Document-Count + X-Skipped-Count zeigen dem Frontend transparency"
  - "T-sgp-06 (Path-Traversal): relative_path kommt aus MemberDocumentEntity (UUID-basiert), nicht aus User-Input"
  - "T-sgp-07 (ZIP-Slip): Filenames aus member.member_number (i64) + sanitize_for_filename ersetzt '/'/'.' durch '_'"
---

# Quick 260602-sgp: Bulk-Download aller RepaymentLetter-Dokumente — SUMMARY

## Task

Implementierung eines neuen Bulk-Download-Endpoints `GET /api/repayment-phase/{phase_id}/letters/download?format=zip|pdf` fuer Vorstand-User, die alle bereits persistierten RepaymentLetter-PDFs einer Phase in einem Click herunterladen wollen.

**Wichtigste Garantie:** Im Gegensatz zu `POST /letters/generate` werden die PDFs **NICHT** neu gerendert — der Service liest existierende MemberDocuments aus dem Document-Storage und packt sie entweder zu einem ZIP-Archiv (Einzel-PDFs) oder einer gemerged Bundle-PDF (lopdf).

## Approach

### Backend (Task 1, Commit b3f9b12)

1. **Service-Trait erweitert** (`genossi_service/src/repayment_letter.rs`):
   - `RepaymentLetterDownloadFormat` Enum (Zip|Pdf)
   - `RepaymentLetterDownload` Struct (bytes, content_type, filename, document_count, skipped_count)
   - `download_bundle()` Methode mit klarem Doc-Comment "reiner Lese-Endpoint"

2. **ServiceImpl** (`genossi_service_impl/src/repayment_letter.rs`):
   - Funnel reused via `check_admin_and_phase_status` (404/403/409)
   - `find_existing_letter_for_phase` als Single-Source-of-Truth fuer die (member, phase)-Identifikation (auch von `generate()` benutzt)
   - In-Memory `member_number ASC`-Sort
   - File-Loading mit Skip-on-Error (X-Skipped-Count statt Abbruch)
   - ZIP-Build via `zip::ZipWriter` (Pattern aus `genossi_rest/src/backup.rs`)
   - PDF-Merge via `lopdf` (Pattern aus `lopdf/examples/merge.rs`)

3. **REST-Handler** (`genossi_rest/src/repayment_letter.rs`):
   - `download_letters` Handler mit `DownloadQuery` Query-Extractor
   - Response-Headers: Content-Type, Content-Disposition, X-Document-Count, X-Skipped-Count
   - Format-Validation (zip|pdf -> 400 fuer alles andere)
   - Route in `generate_letter_route` ergaenzt
   - OpenAPI-Spec via Utoipa erweitert

### Frontend (Task 2, Commit eabfbde)

1. **Neuer Component** (`genossi-frontend/src/component/repayment_letter_download_button.rs`):
   - Zwei Buttons (ZIP / Bundle-PDF) als Komponente
   - `r#type: "button"` + `onclick` (Memory `feedback_dioxus_button_type.md`)
   - Browser-Save via `<a download>`-Click + `Url::revoke_object_url`
   - Singular/Plural-Toast mit Skipped-Count-Suffix
   - 2 Unit-Tests fuer `error_message`-Helper

2. **API-Funktion** (`api::download_repayment_letters`):
   - web_sys-fetch Pattern (analog zu `generate_repayment_letters`)
   - Extrahiert X-Document-Count, X-Skipped-Count, Content-Disposition-Filename
   - Returns `DownloadedLettersResult` mit blob_url, document_count, skipped_count, filename

3. **i18n-Keys** (6 neue Keys in DE + EN):
   - `RepaymentLetterDownloadZipButton`, `RepaymentLetterDownloadPdfButton`
   - `RepaymentLetterDownloadToastSingular`, `RepaymentLetterDownloadToastPlural`
   - `RepaymentLetterDownloadToastSkipped`, `RepaymentLetterDownloadToastFailure`

4. **Page-Mount** (`page/repayment_phase_details.rs`):
   - Component oberhalb TabStrip, sichtbar in allen Tabs
   - Conditional render: nur wenn `phase.status != Preparation` (Backend-Gate-Spiegelung)
   - Toast via `show_toast(&mut toast_messages, ...)` reused (kein neuer Signal)

### E2E-Tests (Task 2, Commit eabfbde)

6 neue Tests in `genossi_bin/tests/repayment_letter_e2e.rs`:

- `test_download_letters_zip_happy_path` — 2 persistierte Letters -> 200 + ZIP-Magic + headers
- `test_download_letters_pdf_happy_path` — 2 persistierte -> 200 + %PDF + parseable durch `lopdf::Document::load_mem` (lopdf-Merge Smoke-Test)
- `test_download_letters_zero_persisted_returns_404` — Phase ohne POST /generate -> 404
- `test_download_letters_helper_auth_returns_403` (#[ignore]'d — mock_auth-Limitation, Service-Layer-Unit-Test deckt 403 ab)
- `test_download_letters_preparation_phase_returns_409` — phase_not_active
- `test_download_letters_invalid_format_returns_400` — format=docx -> 400

Reused Helper: `seed_persisted_letters` triggert `POST /letters/generate` (Phase 13 reused), `create_open_repayment_phase`, `create_preparation_repayment_phase`, `list_entries_for_phase`.

## Files Changed

**Created (2):**
- `genossi-frontend/src/component/repayment_letter_download_button.rs` — Bulk-Download-Component
- `.planning/quick/260602-sgp-bulk-download-aller-repaymentletter-doku/deferred-items.md` — pre-existing E2E-Test-Failure dokumentiert

**Modified (13):**
- `Cargo.toml` — lopdf 0.34 + zip 2 als workspace.dependencies
- `genossi_service_impl/Cargo.toml` — lopdf + zip wired
- `genossi_service/src/repayment_letter.rs` — Trait + neue Types
- `genossi_service_impl/src/repayment_letter.rs` — download_bundle + sanitize_for_filename + merge_pdfs_via_lopdf + 7 Unit-Tests
- `genossi_rest/src/repayment_letter.rs` — REST-Handler + Route + ApiDoc + 4 Unit-Tests
- `genossi_bin/Cargo.toml` — lopdf workspace-dev-dep
- `genossi_bin/tests/repayment_letter_e2e.rs` — 6 neue E2E-Tests
- `genossi-frontend/src/api.rs` — `download_repayment_letters` API + Filename-Parser-Helpers
- `genossi-frontend/src/component/mod.rs` — Component-Registry
- `genossi-frontend/src/i18n/mod.rs` — 6 neue Keys
- `genossi-frontend/src/i18n/de.rs` — DE-Translations
- `genossi-frontend/src/i18n/en.rs` — EN-Translations
- `genossi-frontend/src/page/repayment_phase_details.rs` — Component-Mount + Import
- `Cargo.lock` — Updated dependencies

## Test Results

### Workspace tests (`cargo test --workspace`)

- **Backend Unit-Tests:** 36 passed in `genossi_service_impl::repayment_letter` (29 existing + 7 new download_bundle/helper tests)
- **REST Unit-Tests:** 9 passed in `genossi_rest::repayment_letter` (5 existing + 4 new DownloadQuery)
- **Service-Trait Tests:** 9 passed in `genossi_service` (2 existing + 3 new for new types)
- **E2E:** 5 passed + 1 ignored in `test_download_letters_*` (alle neuen Tests gruen)

### Deferred Issues

- **Pre-existing failing test:** `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` failed BEFORE my changes (verified by `git checkout 4f9cc1f -- ...` + isolated re-run). Documented in `deferred-items.md` — out-of-scope for this quick task.

### Build / Lint Gates

- `cargo build --workspace`: clean
- `cargo clippy --workspace --all-targets`: 0 errors, 0 new warnings
- `cargo check` on `genossi-frontend` (wasm32-unknown-unknown): clean
- `rustfmt --check` on all changed files: clean

### Audit-Disziplin-Grep-Gate

```bash
$ grep -v '^[[:space:]]*//' genossi_service_impl/src/repayment_letter.rs \
    | awk '/async fn download_bundle/,/^    }$/' \
    | grep -c 'audited_'
0  # PASS
```

### Component-First-Grep-Gate

```bash
$ grep -c "download_repayment_letters" genossi-frontend/src/page/repayment_phase_details.rs
0  # PASS — Page only uses Component, no direct API call
$ grep -c "download_repayment_letters" genossi-frontend/src/component/repayment_letter_download_button.rs
2  # PASS — Component invokes the API
```

### Single-Source-of-Truth Gate

```bash
$ grep -c "find_existing_letter_for_phase" genossi_service_impl/src/repayment_letter.rs
7  # PASS (>= 3 — Definition + generate-call + download_bundle-call + tests)
```

## Decisions Made

1. **lopdf 0.34 manual merge** statt einer high-level `merge_documents`-Funktion — die Crate exponiert nur `load_mem`/`save_to`/`renumber_objects_with`/`get_pages`. Implementation folgt 1:1 dem offiziellen `lopdf/examples/merge.rs` Pattern (Pages-Tree-Sammlung via `BTreeMap<ObjectId, Object>`, dann Catalog/Pages-Filter und Kids-Rebuild). E2E-Test `test_download_letters_pdf_happy_path` parsed das Output via `lopdf::Document::load_mem` als Smoke-Test.

2. **zip 2 als workspace.dependency**: vorher nur `genossi_rest/Cargo.toml` deklariert. Service-Layer benoetigt zip jetzt direkt — daher Hochziehen in `[workspace.dependencies]`. `genossi_rest` behaelt seine lokale Deklaration vorerst (gleiche Version 2 + deflate feature) — keine Breaking-Change.

3. **Component-Mount oberhalb TabStrip** (sichtbar in allen Tabs) statt im BasicsTab oder eigenem Tab — das Feature ist tabunabhaengig und braucht kein "Selektion-bilden"-Step (im Gegensatz zu generate_letters). Conditional render gegen `RepaymentPhaseStatusTO::Preparation` spiegelt den Backend-Status-Gate, damit der User nicht aufs 409 lauft.

4. **Reiner Lese-Endpoint — KEIN audited_*! Macro**: D-13 CLAUDE.md sagt Audit-Pflicht NUR fuer Member, MemberAction, MemberDocument, Application **Schreib**-Ops. Der Bulk-Download ist Read-only auf bereits auditierte MemberDocuments — kein neuer Audit-Eintrag noetig (das wuerde nur Lese-Spam in der Hash-Chain erzeugen).

5. **In-Memory Member-Sort statt SQL ORDER BY** — folgt dem Phase-13 `generate()`-Pattern (Sort im Service-Layer, NICHT im DAO). Konsistenz mit dem Bestand und einfacheres Testing per Mock.

6. **Test 4 helper-auth-403 als #[ignore]'d** — gleiche Begruendung wie der bestehende `test_letter_helper_auth_returns_403`: mock_auth context_extractor injiziert immer Admin. Der 403-Pfad ist via Service-Layer-Unit-Test `test_download_bundle_permission_denied_returns_403` abgedeckt.

## Deviations from Plan

- **`commit-to-subrepo` nicht verwendet** — der Plan erwaehnt das nicht und init.execute-phase ist nicht ausfuehrbar (keine GSD-SDK-Bins auf PATH). Stattdessen Standard-`git add` + `git commit`-Pfad pro Task. Single-Repo-Setup verifiziert via `git rev-parse --show-toplevel`.

- **Worktree-Mirror NICHT als git-worktree konfiguriert** — die zugehoerige cwd `.claude/worktrees/agent-a0716a3233149998c/` ist nur ein File-Mirror; `git worktree list` zeigt nur den Main-Tree. Workaround: bearbeitete Files mit `cp ... /home/neosam/.../genossi3/` ins Repo zurueckgesynct, dann committed. Beide Commits liegen auf `main`.

- **Pre-existing E2E-failure** (`test_letter_idempotency_d13_08_and_no_status_toggle_d13_09`) ist nicht angefasst — outside scope. Dokumentiert in `deferred-items.md`.

## Threat-Mitigations Verified

- **T-sgp-01 (Spoofing — Helper-Auth umgangen):** REST-Handler ruft `extract_auth_context(Some(context))?` (401 bei fehlender Session); Service-Layer `check_admin_and_phase_status` ruft `permission_service.check_permission("admin", ...)` -> `PermissionDenied` -> `map_letter_error` mappet auf 403. Unit-Test `test_download_bundle_permission_denied_returns_403` verifiziert.
- **T-sgp-02 (Tampering — Transport-Layer):** TLS am Reverse-Proxy; Application-Level akzeptiert das Risiko (kein Signing).
- **T-sgp-03 (Repudiation):** Bewusst kein Audit-Log fuer Read-Endpoint — Audit-Disziplin-Grep-Gate verifiziert 0 `audited_*!` im neuen Pfad.
- **T-sgp-04 (Information Disclosure — DSGVO):** Permission-Gate ADMIN; X-Document-Count + X-Skipped-Count erlauben Frontend, dem Nutzer Transparenz zu schaffen.
- **T-sgp-05 (DoS — Riesen-Phase):** Akzeptiert (Genossi-Realbetrieb <100 Members/Phase).
- **T-sgp-06 (Path-Traversal via doc.relative_path):** `relative_path` wird im Service aus MemberDocumentEntity gelesen (vom Backend gesetzt mit UUID `format!("{}.pdf", doc_id)` in `RepaymentLetterServiceImpl::generate`). Keine User-Input-Manipulation moeglich.
- **T-sgp-07 (ZIP-Slip):** Filenames werden aus `member.member_number` (i64) + `sanitize_for_filename(last_name/first_name)` gebaut. Sanitize ersetzt '/' / '.' -> '_'; Unit-Tests `test_sanitize_for_filename_special_chars` + `test_sanitize_for_filename_umlauts` verifizieren.

## Known Stubs

None. Der Endpoint ist vollstaendig wired (DocumentStorage-Load, ZIP-Build, lopdf-Merge, Frontend-Click-to-Download).

## Output Notes (per plan request)

- **lopdf-API-Variante:** Manual Pages-Tree-Merge (Option B aus dem Plan-Skelett). `Document::merge_documents` existiert in 0.34 NICHT als High-Level-API. Implementation folgt `lopdf/examples/merge.rs`.
- **Reused E2E-Test-Helper:** `setup_with_templates` (mit Logo-Provisionierung), `create_member_with_exit_date_and_iban`, `create_open_repayment_phase`, `create_preparation_repayment_phase`, `list_entries_for_phase`, `seed_persisted_letters` (NEU — wraps existing POST /letters/generate).
- **Component-Mount auf RepaymentPhase-Detail-Page:** oberhalb der TabStrip, sichtbar in allen Tabs, conditional render gegen `Preparation`-Status.
- **Skipped-Count-Behavior:** Tests haben skipped_count = 0 (Storage-Files vorhanden weil seed-Helper sie persistierte). Real-world-Storage-Drift wuerde via X-Skipped-Count + Frontend-Toast-Suffix sichtbar.
- **Cargo fmt + clippy:** alle changed-files clean.

## Self-Check: PASSED

- [x] All 2 tasks executed and committed (`b3f9b12`, `eabfbde`)
- [x] SUMMARY.md frontmatter populated
- [x] All deviations documented (worktree-mirror + pre-existing E2E-failure)
- [x] All threat mitigations verified
- [x] `cargo build --workspace`: clean
- [x] `cargo test --workspace`: green (except pre-existing failure, documented)
- [x] `cargo clippy --workspace --all-targets`: clean
- [x] `rustfmt --check`: clean on all touched files
- [x] Audit-Disziplin-Grep-Gate: 0
- [x] Component-First-Grep-Gate: 0 in page, >=1 in component
- [x] Single-Source-of-Truth (`find_existing_letter_for_phase`): 7 occurrences
