# Milestones

## v1.4 Mail-Formatierung & Antrags-Dokumente (Shipped: 2026-07-03)

**Phases completed:** 4 phases, 16 plans, 43 tasks

**Key accomplishments:**

- Adds `pub enum MailEncoding { QuotedPrintable, EightBit }` and threads a new `SmtpConfig.encoding` field through `load_smtp_config`, driven by the optional `smtp_encoding` KV key with tolerant fallback — production default `quoted-printable` is preserved.
- Extracts MIME construction from `worker.rs::send_mail_for_recipient` into a new pure, synchronous `genossi_mail::send::build_message`; rewires both `service.rs::send_test_mail` paths (which the digest inherits) through it; fixes the historic missing-charset bug at all three sites and threads the `MailEncoding` opt-in from `SmtpConfig.encoding` to the ONE place where the Content-Transfer-Encoding is decided.
- Three forward-only ADD COLUMN … TEXT NULL migrations plus DAO wiring add body_html (mail_templates + mail_jobs) and rendered_html_body (mail_recipients) with byte-identical NULL-legacy roundtrip guarantees
- Add ammonia-backed `sanitize_html`, autoescaping `html_env`/`render_html_template`, FMT-01 `format_de` German date helper, and a `RenderedContent` return struct — the three Service-layer primitives Plans 03 and 04 will consume.
- Extend `build_message` in `genossi_mail::send` with an optional `html_body: Option<&str>` and a 4-branch decision tree producing `multipart/alternative{text, html}` (nested inside `mixed{…, attachments}` when attachments are present), with text-first ordering pinned by byte-offset assertion.
- Wire the Plan-02 sanitize helper and the Plan-03 MIME extension through the entire mail send stack — 4 D-03 entry points now sanitize author HTML at the store boundary; the worker persists `rendered_html_body` and forwards it to `build_message`; every REST DTO on the compose/detail path carries the new `body_html` field with backward-compatible wire shape.
- 1. [Rule 3 - Blocking] `MailJob.body_html` expects `Arc<str>`, not `String`
- 1. [Rule 3 — Blocker] Dioxus 0.6.3 ClipboardData has no `.get_data()` method
- All three MailBodyEditor call sites migrated to WysiwygEditor with body_html signal wiring end-to-end; TemplatePreview renders backend HTML preview via dangerous_inner_html; body_editor.rs deleted.
- Two new e2e tests pin Plan 24-01's backend seams (preview HTML round-trip + inbox reply sanitize-on-store), and a 12-step Vorstand-facing UAT checklist covers the browser-side behaviors automated tests cannot reach. Auto-mode approved the human-verify checkpoint after confirming the automated regression portion (cargo test --workspace: 305 pass, 1 pre-existing Phase 22 failure, 0 new regressions).
- REQUIREMENTS.md and ROADMAP.md aligned on Move / Ownership-Übergabe semantics for APDOC-03 with the verbatim MemberDocument description format „Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)".
- SQLite `application_documents` table with single-slot partial unique index, plus a narrow-schema `ApplicationDocumentDao` trait (no Auditable, no document_type/description) and its SQLite implementation with optimistic-locking update.
- Wave 2: `ApplicationDocumentService` trait in `genossi_service` and `ApplicationDocumentServiceImpl` in `genossi_service_impl`. Four methods (upload/get/download/delete) all enforce CR-02 ordering; upload branches into create-new OR replace-in-place with a mockall-Sequence-pinned save→update→delete-old flow. Seven unit tests including a dedicated CR-02 regression guard.
- Wave 3 shipped the outward-facing surface for the single-slot ApplicationDocument.
- 1. [Rule 1 — Bug] Plan-04 confirm() passed a fresh v4 UUID as the "old_version" for the app_doc soft-delete WHERE clause.

---

## v1.3 Posteingang-Benachrichtigung & Reply-Komfort (Shipped: 2026-06-28)

**Phases completed:** 3 phases, 11 plans, 18 tasks

**Key accomplishments:**

- Read-only InboundMailAttachment DAO with 4 methods (incl. T-03 IDOR-safe `find_by_id_and_mail`), SQLite impl, and idempotent migration for inbound_mail_attachments table.
- Attachment-Pipeline (parse → 10 MB cap → save-then-DB → rollback) + fetch_one_by_uid (UIDVALIDITY-guard) + InboxService API surface for attachment listing/lookup — all wired through genossi_bin so the existing inbox worker persists attachments automatically after each successful mail-create.
- Download-Endpunkt `GET /api/inbox/{mail_id}/attachments/{attachment_id}` mit `?disposition=inline|attachment`-Switch + `InboundMailDetailTO.attachments` (Embed via `InboxService::list_attachments`) + T-03 IDOR-Guard (cross-mail → 404) + 410 GONE für oversized (D-02). 4 Unit-Tests für `content_disposition_inline` + 5 E2E-Tests grün.
- One-shot attachment backfill worker — `run_attachment_backfill` iterates legacy inbound mails (`has_attachments=true` + `count_for_mail==0`), refetches each from IMAP via `fetch_one_by_uid`, and runs the same `persist_attachment` pipeline as the poll worker. Best-effort (D-05/D-06): IMAP-Err / Ok(None) → silent-skip. Idempotent on restart via the `count_for_mail == 0` filter. Spawned at server boot from `genossi_bin/src/main.rs` immediately after the inbox worker.
- Two new Dioxus components — `InboxAttachmentList` (section wrapper) and `InboxAttachmentListItem` (per-row layout) — plus `format_size` integer-math util, 7 i18n keys × 2 locales, and the `InboundMailAttachmentTO` frontend mirror. All actions are anchor-only (no `<button onclick>`), every `target="_blank"` has `rel="noopener"`, filename flows only into RSX text content (T-05 + T-08 mitigations grep-gated). 4 unit tests for `format_size` green; WASM build green.
- Inbox detail pane wired to `InboxAttachmentList` — MVP-amber-hint deleted, single component invocation inserted between `<pre>` body and assignment-section divider, Component-First principle enforced (zero inline RSX iteration in page file). WASM build green; manual smoke test pending checkpoint.
- Probe-Read-Pattern in `extract_attachments` schliesst CR-01 BLOCKER: 10-MB-Cap greift jetzt VOR Heap-Allokation, nicht erst beim Persist-Schritt.
- Dedizierte SQLite-Tabelle `digest_state` + `DigestStateDao`/`DigestStateDaoSqlite` mit Upsert-Singleton-Semantik für das letzte Digest-Versanddatum (D-03) — inklusive 3 In-Memory-Unit-Tests.
- Config-getriebener Tokio-Poll-Loop (genossi_mail/src/digest.rs), der zur konfigurierten Uhrzeit pro Empfänger genau eine Plain-Text-Posteingangs-Digest-Mail pro Kalendertag verschickt (mit Catch-up nach verpasstem Fenster), inklusive 21 Unit-Tests für alle reinen Helfer und vollständigem DI-Wiring beim Serverstart.
- Neuer Config-Abschnitt „Posteingangs-Benachrichtigung" auf der Config-Seite mit komma-getrenntem Empfänger-Feld, HH:MM-Uhrzeit, Speichern und pure-funktional unit-getesteter Inline-Validierung — persistiert `digest_recipients` + `digest_send_time` für den Plan-02-Worker.

---

## v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres (Shipped: 2026-06-07)

**Phases completed:** 5 phases (14-18), 24 plans
**Timeline:** 2026-06-04 → 2026-06-07 (4 Tage, 127 commits)
**Requirements:** 31/31 v1.2 satisfied (alle erfüllt — 7 Doku-Drift-Items in REQUIREMENTS.md beim Archivieren korrigiert)
**Tests:** ~67 neue Tests (Phase 14: 17, 15: 19, 16: ~25, 17: ~25, 18: ~21 inkl. 9 DTO-Roundtrip + 12 Modal-Pure-Helper); Audit-Hashchain bleibt valid
**Audit-Status:** `tech_debt` (siehe `milestones/v1.2-MILESTONE-AUDIT.md`)
**Release-Tag:** v1.2.0 (vorab durch `/release-version` erzeugt; kein Milestone-Re-Tag)

**Delivered:** Vorstand kann ab v1.2 die vier Operationen Kündigung, Teil-Rückgabe, Übertragung und Aufstockung direkt am Mitglied auslösen — Single-Button „Mitgliedschaft anpassen" auf der Member-Detail-Page (admin-only) mit `MembershipAdjustModal` als shared Component, 4 Sub-Views, Live-Preview-Confirmation, FiscalYearDateInput mit GJ-Bounds, und vollständigem Audit-Trail via gemeinsamem Process-String pro Operation. v1.2 erzeugt nur Intent-Datensätze (`MemberAction::Austritt/Aufstockung/UebertragungAbgabe/Empfang` + `RepaymentEntry`) — v1.1's PaidOut-Cascade bleibt Single-Source-of-Truth für `MemberAction::Verkauf`-Erzeugung und `current_shares`-Reduktion. Vorstand-UAT-Sign-Off durch Browser-Walk-Through aller 6 Szenarien am 2026-06-07.

### Key Accomplishments (per Phase)

**Phase 14 — DAO/Domain Foundation:** Pure-Function `compute_effective_date` (H1/H2-Stichtag-Berechnung mit 6 Edge-Case-Tests), `RepaymentEntryDao::find_by_member_and_phase` (Trait-Default-Impl + SQLite-SQL-Override) als Foundation für Phase-16-Sum-Check und Auto-Fill-Skip-Pattern, `MemberService::list_transfer_recipients` (admin-gated, exit_date+self-Filter) + REST-Endpoint `GET /api/members/transfer-recipients` mit 6-Feld-PII-guarded `MemberSlimTO` (DSGVO-Whitelist gegen IBAN/Email/Adresse-Leak, Sub-Route VOR `/{id}` catch-all gegen Axum-UUID-Parse-Pitfall).

**Phase 15 — Service+REST: Kündigung + Aufstockung:** `MembershipAdjustService`-Trait + `cancel_membership` (atomare Single-Tx mit `audited_create!(MemberAction::Austritt, CANCEL_PROCESS)`, `effective_date` via Phase-14-Pure-Function, `exit_date` via refactored `recalc_dates`-Free-Function) + `increase_shares` (atomare Multi-Write: `audited_create!(Aufstockung) + audited_update!(Member.current_shares += n)`, Block für gekündigte Mitglieder via HTTP 400). Server-Layer-Datum-Validierung (`validate_willensbekundung_date` Pure-Function, GJ-Bounds aktuell+nächstes), ADMIN_PRIVILEGE-Permission-Funnel, 9 aktive + 2 dokumentierte `#[ignore]` E2E-Tests (mock_auth-Limitation), v1.2-Audit-Pattern für Phase 16+17 etabliert.

**Phase 16 — Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase:** `partial_repayment` mit 14-Schritt-Pipeline: Permission-Funnel → exit_date-Conflict (409) → Range-Validation → H1/H2-Stichtag → inlined Phase-Auto-Create (Variante B: Status=Open, `share_value` aus Vorgänger oder `DEFAULT_SHARE_VALUE_CENT=10000`) → Sum-Check via `find_by_member_and_phase` (PaidOut-exclusive) → `audited_create!(RepaymentEntry, PARTIAL_REPAYMENT_PROCESS)`. Auto-Fill-Skip-Pattern in `open_repayment_phase` (D-16-03/PITFALLS-Kat-1-Mitigation: Phase-Open skippt Member mit existierendem v1.2-Entry → keine Doppelbuchung). Plan 16-05 Gap-Closure ergänzte Closed-Phase-Status-Guard (HTTP 409 vor jedem audited_create) — Re-Verification flipped von `gaps_found` auf `passed`. 18 E2E-Tests + 11 Service-Unit-Tests.

**Phase 17 — Service+REST: Übertrag (Atomare 2-Action-Cascade):** `transfer_shares` als 15-Schritt-Single-Tx-Cascade mit Pre-Write-Detection des Voll-Übertrags (D-17-01). 5 `audited_*!`-Aufrufe teilen `TRANSFER_PROCESS="member-adjust.transfer"`: 2× `audited_create!` (UebertragungAbgabe + Empfang), 2× `audited_update!` (from/to current_shares), 1× optional `audited_create!` (Austritt bei Voll-Übertrag mit `effective_date=transfer_date`, `transfer_member_id=to.id`). `recalc_dates(from)` läuft exakt einmal nach Voll-Übertrag (D-17-02). REST-Endpoint `POST /api/members/{from_id}/transfer-shares` (Sub-Route VOR `/{id}` Catch-All). Self-Transfer-Block (HTTP 400 via `validate_transfer_inputs`), Empfänger-aktiv-Guard (HTTP 409 via `ServiceError::Conflict`, ROADMAP-SC#4-Wortlaut sprach von 400 — Plan-Spec maßgeblich). 8 E2E-Tests + 7 Pure-Function-Tests + 10 Mock-Service-Tests inkl. Same-/Cross-Direction-Race-Patterns (sortiert `[200, 409|500]`, NIE `[200, 200]`).

**Phase 18 — Frontend Component-First:** `MembershipAdjustModal` als shared Component in `genossi-frontend/src/component/membership_adjust_modal.rs` (1078 LOC, Single-File-Dioxus mit `ModalStep`-Enum + 4 Sub-Views Kündigung/Teil-Rückgabe/Übertrag/Aufstockung). Live-Preview-Section pro Sub-View zeigt konkrete Zahlen vor Commit (Stichtag, FY, Anteils-Delta, Voll-Übertrag-Warnung orange). `FiscalYearDateInput`-Component mit GJ-Bounds (default `today`, min/max aus aktuellem+nächstem GJ). `ToastVariant{Success,Error}` + `show_success_toast` + `SuccessToastContainer` (grüne Success-Toasts, Zero-Blast-Radius zu v1.0/v1.1). `MemberSearch.members_override`-Prop für Transfer-Empfänger-Decoupling. 8 neue Frontend-DTOs + 5 API-Client-Funktionen + 46+ i18n-Keys DE/EN mit Symmetrie-Test. Member-Detail-Page-Integration via Admin-only Button (`RequirePrivilege "admin"`), Modal-Mount, zentrale `today`-Variable, beide Toast-Container. 247 Frontend-Tests bestanden, Vorstand-UAT signed-off durch Simon Goller am 2026-06-07. 7 Pläne in 3 Wellen (Wave 1: 5 parallele Foundation-Plans → Wave 2: Modal-Composition → Wave 3: Page-Integration).

### Known Gaps (None — alle 31 REQs satisfied)

Keine — alle 31 REQ-IDs sind in den Phase-VERIFICATIONs als SATISFIED markiert. Beim Archivieren wurden 7 Doku-Drift-Items in REQUIREMENTS.md (CANC-02, PART-01..03, PART-05, PART-06, TRSF-06 — alle technisch erfüllt aber Checkbox + Traceability nicht synchronisiert) korrigiert in `milestones/v1.2-REQUIREMENTS.md`.

### Tech-Debt (per Phase, dokumentiert in `v1.2-MILESTONE-AUDIT.md`)

**Carry-forward BLOCKER (Phase 16 → projektweit):**

- **CR-02 (Permission-Check-Ordering)** in allen 4 v1.2-MembershipAdjustService-Methoden + bestehend in allen 5 v1.1-RepaymentPhaseService-Methoden: `current_user_id()` läuft VOR `check_permission()` → Side-Channel-Risiko bei abgelaufenen Sessions + `"SYSTEM"`-Audit-Fallback bei `Ok(None)`. Explicitly out-of-scope für v1.2; projektweite Cleanup-Phase empfohlen (extrahierbar in `gen_auth_admin!`-Helper).

**Critical UX-Findings (Phase 18, kein Korrektheits-Bug):**

- **CR-01:** `date_signal` überlebt Sub-Choice-Wechsel in MembershipAdjustModal — User kann altes Datum unbemerkt in eine andere Sub-View übertragen. Submit-`is_valid`-Check verhindert Datenfehler.
- **CR-02:** `Signal::set` im Render-Pfad von `render_sub_choice` — Re-Render-Loop-Risiko + User-Datenverlust beim Zurück-Navigieren. Side-Effects gehören in onclick-Handler.

**Warnings:**

- **Phase 16 WR-01..05:** Inkonsistente Check-Reihenfolge (Date-Validation nach Member-Load → SELECT-Spam bei Bad-Date), PII-Leak in 409-Conflict-Message (`exit_date={:?}`), `unwrap()` auf `Response::builder()` im REST-Layer, pre-Read Member ohne re-read post-commit, `REPAYMENT_PHASE_CREATE_PROCESS`-String-Duplikat zu v1.1 (forensisch nicht unterscheidbar).
- **Phase 18:** Alle Submit-Buttons `bg-red-600` (auch konstruktive Operationen wie Aufstockung) — visuell destruktiv. Unused i18n-Keys: 4 Operations-spezifische Success-Keys + 2 AutoCreate-Hint-Keys (User sieht generische statt kontextspezifische Toasts). `format_date_input`/`parse_date_input` doppelt in `fiscal_year_date_input.rs` und `member_details.rs` — Extract-to-shared-module empfohlen.

**Smell:**

- Wire-Asymmetrie `PartialRepaymentResponseTO.entry/.phase` als `serde_json::Value` im Frontend vs. typisiert im Backend. Wire-kompatibel, Modal verwirft Body — zukünftiges `entry.id`-Read braucht `Value::get()`-Workarounds.

**Pre-existing (NICHT v1.2-verursacht):**

- `test_mail_preview_repayment_no_entries_does_not_default_to_one` (`genossi_bin/tests/e2e_tests.rs:13964`) — pre-existing seit Commit `1e48b2f` (Quick-c19 vor Phase 14). Triage in separater Mail-Subsystem-Iteration.
- `cargo test --all-features` Compile-Error auf `transfer_recipients_e2e` (vermutlich oidc-Feature-Konflikt).
- Disk Space Critical (`/home/neosam/programming` 929/950 GB). Workaround: shared `CARGO_TARGET_DIR`.

**Known deferred items at close: 18** (16 v1.1-Quick-Tasks ohne SUMMARY + 2 low-prio Repayment-Letter-Todos — alle aus v1.1-Ära, nicht v1.2-relevant; siehe STATE.md Deferred Items)

## v1.1 Anteile-Rückzahlungsphase (Shipped: 2026-06-02)

**Phases completed:** 7 phases (07-13), 56 plans, 91 tasks
**Timeline:** 2026-04-01 → 2026-06-02 (~62 Tage, 651 commits)
**Code-Änderungen:** 1.536 Dateien, +345.146 / -2.205 LOC
**Requirements:** 33/34 v1 satisfied (UI-06 partial → siehe Known Gaps)
**Tests:** ~600 Workspace-Tests + 292 E2E-Tests grün; Audit-Hashchain bleibt valid

**Delivered:** Vollständige Anteile-Rückzahlungs-Pipeline — vom RepaymentPhase-Aggregat über atomare Auszahlungs-Buchung bis hin zu Massenmail- und Bulk-PDF-Brief-Versand. Ersetzt manuelle Excel-Listen für den Rückzahlungs-Workflow vollständig.

### Key Accomplishments (per Phase)

**Phase 7 — RepaymentPhase Backend Foundation:** Auditpflichtiges Aggregat mit Lifecycle `Vorbereitung → Offen → Abgeschlossen`, i64-Cent-Konvention für `share_value`, 5 Audit-Prozesse, Frozen-Order-Audit-Felder; 7 E2E-Tests verifizieren alle 5 SC + D-04..D-12 inkl. Audit-Hashchain `valid=true`.

**Phase 8 — RepaymentEntry + Auto-Befüllung:** RepaymentEntry-Aggregat (10 Pläne) mit Auto-Befüllung beim Phase-Open (atomar in Status-Übergangs-Tx), manuellem Picker, Status-Toggle `offen ↔ angeschrieben`, Batch-Endpoint mit strukturiertem 409-JSON-Body, Pending-Entry-Validation beim Close, 404-vs-409-Trennung bei missing/soft-deleted Entries; 15 E2E-Tests.

**Phase 9 — Atomare Auszahlungs-Buchung:** 12-Schritt-Cascade `ausbezahlt`-Toggle → `audited_create!(MemberAction::Verkauf)` + `audited_update!(Member, RepaymentEntry)` in einer SQLite-Tx mit gemeinsamem Process-String; PAYO-03/04 final-Semantik; 4 E2E-Tests inkl. Race-Defense (`tokio::join!`) und Audit-Chain-Multi-Endpoint-Verify.

**Phase 10 — Massenmail + Template-Variablen:** Wiederverwendung `POST /api/mail/send-bulk` mit `{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}`; minijinja-strict + `{% if X is defined %}`-Pattern; Mail-Worker integriert Repayment-Variablen-Aggregation + audited `MemberDocument`-Create via inlined `worker_audit`-Modul (Cross-Crate-Audit ohne Dependency-Cycle); 5 E2E-Tests mit deterministischem SMTP-Stub (127.0.0.1:1 + RFC5321-fail-fast).

**Phase 11 — Export (PDF):** Typst-basiertes 6-Spalten-Auszahlungslisten-PDF (Nr./Name/IBAN/Anteile/Betrag/Verwendungszweck), Repeat-Header, optionale Summenzeile; Permission-Funnel `load → admin → status`, `tx.commit()` VOR PdfGenerator-Render; Filename-Schema `auszahlung-{fy}-{include}.pdf` + Filter `?include=open|all|paid`; 8 E2E-Tests inkl. Umlaut-Member „Hans Müller".

**Phase 12 — Frontend (Component-First):** 15 Pläne — `/repayment-phases` Liste, `/repayment-phases/{id}` Detail mit 3-Tab-Layout (Stammdaten, Einträge, Export), Shared-Component `RepaymentEntryList` mit Multi-Select + Status-Filter + Inline-Cell-Edit für `share_count_to_pay_out`, Confirm-Dialog für `ausbezahlt` mit Final-Warnung, 4 Pure-Reuse-Bausteine (`format_payout_eur`, `parse_euro_to_cents`, `RepaymentPhaseStatusBadge`, `RepaymentEntryStatusBadge`).

**Phase 13 — RepaymentLetter-Bulk-Anschreiben:** Brief-Kanal für Nicht-Email-Mitglieder — `DocumentType::RepaymentLetter`, `auszahlungs_anschreiben.typ` + `_bundle.typ` mit `#pagebreak()`; `RepaymentContextResolver`-Trait (`resolve` + `aggregate`) eliminiert N+1-DB-Reads; `RepaymentLetterServiceImpl` mit Permission-Funnel + sequential audited MemberDocument-Persistenz + Bundle-PDF-Render; `POST /api/repayment-phase/{id}/letters/generate` mit Direct-Download + `X-Document-Count`-Header für Frontend-Toast-Pluralisierung; Frontend "Anschreiben erzeugen"-Button mit Selection-Preservation (D-13-09); 8 E2E-Tests verifizieren gesamte Pipeline inkl. Audit-Hashchain.

### Known Gaps (Acknowledged at close)

- **UI-06 (partial)** — Massenmail-Aktion im Tabellen-Header (`partial`): Code-Pfade grep-verifiziert (`RequirePrivilege { fallback: AccessDeniedPage }`), Service-Layer-403 unit-getestet, aber 3 HUMAN-UAT-Items pending (Non-Admin-OIDC-Session lokal nicht verfügbar während UAT 2026-06-01).

### Tech-Debt (per Phase, dokumentiert in `v1.1-MILESTONE-AUDIT.md`)

- **Phase 7:** Optimistic-Locking Stale-Retry-Pattern — DAO bumpt DB-Version, propagiert sie aber nicht zurück (codebase-weite Service-Konvention).
- **Phase 8:** `format_dt`-Helper lokal in `repayment_entry.rs` dupliziert (Phase-7-Variante ist privat, nicht `pub(crate)`).
- **Phase 9:** SQLITE_BUSY Race-Path im E2E-Test akzeptiert sortierte Statuses `[200, 409|500]` statt strict `[200, 409]`; DAO-Layer-Mapping (SQLite-Lock → ConflictError) wäre Rule-4-Change.
- **Phase 10:** `DocumentType::is_singleton()` TODO — Idempotency-Storage-Growth bei Re-Generierung (3 deferred Strategien dokumentiert; siehe WR-06 in 13-REVIEW.md).
- **Phase 11:** `from_env()` defaults zu relativen Pfaden (`./templates`, `./typst-packages`) — unsafe unter parallelen Cargo-Tests (siehe IN-04 in 13-REVIEW.md).
- **Phase 12:** 3 Auth-Gate-UAT-Items in `12-HUMAN-UAT.md` pending (Helper-OIDC-Session lokal nicht verfügbar).
- **Phase 13:** Bundle-Template `auszahlungs_anschreiben_bundle.typ` Side-Effect via `#import` des Single-Templates (Refactor zu `default: none`-Pattern als deferred); `std::mem::forget(tempdir)` im Test-Helper leakt `/tmp`-Dirs (IN-01 in 13-REVIEW.md).

**Known deferred items at close: 5** (1 partial requirement, 2 quick-tasks index out-of-sync, 1 low-prio todo `phase-10-worker-refactor-resolver.md`, 1 UAT-status `partial` in Phase 12 — siehe STATE.md Deferred Items)

### Original CLI-extracted Accomplishments (verbose log)

<details>
<summary>Click to expand full SUMMARY.md one-liner extraction (verbose, raw)</summary>

- SQLite-Implementierung des RepaymentPhaseDao-Traits aus Plan 01 — 1:1-Replikat des Assembly-DAO-Impl-Patterns mit Domain-Substitutionen (`fiscal_year: i32` + `share_value: i64 Cent`, ORDER BY `fiscal_year DESC, created DESC`), 4 grüne Tokio-Integrationstests gegen in-memory SQLite, Optimistic-Locking via Pre-Exists-Check + rows_affected-Detection.
- SQLite-Implementierung des RepaymentPhaseDao-Traits aus Plan 01 — 1:1-Replikat des Assembly-DAO-Impl-Patterns mit Domain-Substitutionen (`fiscal_year: i32` + `share_value: i64 Cent`, ORDER BY `fiscal_year DESC, created DESC`), 4 grüne Tokio-Integrationstests gegen in-memory SQLite, Optimistic-Locking via Pre-Exists-Check + rows_affected-Detection.
- Service-Trait (`RepaymentPhaseService` mit 7 Methoden) + Service-Impl (`RepaymentPhaseServiceImpl`) mit Edit-Matrix (D-04), atomarer fiscal_year-Locking in Open (D-07), Lifecycle-Guards (D-05/D-06), Soft-Delete-Restriction (D-09), Field-Validation (D-11/D-12), Optimistic-Locking und 5 Audit-Prozessen — 17 grüne Unit-Tests (4 im Trait, 13 im Impl), 0 direkte DAO-Calls außerhalb Audit-Macros (T-07-03-01 Mitigation verifiziert per Grep-Gate).
- Phase 7 wird HTTP-bereit: 4 neue TOs in `genossi_rest_types` mit ISO8601-Datetime-Serde + Utoipa-Schemas, 7 REST-Handler in `genossi_rest/src/repayment_phase.rs` (414 LOC) inkl. RestState-Trait + generate_route + ApiDoc, Router-Mount + OpenAPI-Nest in `genossi_rest/src/lib.rs`, Trait-Bound-Erweiterung in `test_server.rs`, vollständige DI-Wiring in `genossi_bin/src/lib.rs` (type-Alias + Deps-Struct + Service-Konstruktion + RestState-Impl + Struct-Field) — `cargo build` und `cargo build --tests -p genossi_bin` grün, 35 neue Tests passed (28 in genossi_rest_types + 4 TO-Tests + 3 Handler-Smoke-Tests).
- Phase 7 ist verifikations-vollständig: 7 neue End-to-End-Tests gegen den real laufenden In-Memory-HTTP-Server verifizieren alle 5 ROADMAP-Success-Criteria sowie alle Phase-7-Edit-Matrix-/Lifecycle-/Validation-Decisions (D-04..D-12). Lifecycle-Test verifiziert ROADMAP SC#4 (Audit-Hashchain `valid=true` mit `broken_links=[]` nach create→open→update→close) und SC#5 (share_value-Korrektur erzeugt Audit-Entry mit `field_name=\"share_value\"`, `old_value=Some(\"12000\")`, `new_value=Some(\"13000\")` unter Process `\"repayment-phase.update\"`). 6 Negative-Path-Tests prüfen D-04/D-07 (fiscal_year-Change in Open → 409), D-05/D-06 (close from Preparation → 409), D-06 (reopen from Closed → 409), D-09 (DELETE in Open → 409), D-11 (fiscal_year=1999 → 400), D-12 (share_value=0 → 400). Gesamt-Test-Set: 255 passed; 0 failed (Baseline 248 + 7 neu).
- Migration für `repayment_entry`-Tabelle plus DAO-Trait, Entity, Auditable-Impl und drei Default-Methoden — Foundation für alle nachfolgenden Phase-8-Plans (Service, REST, Phase-Erweiterung, E2E).
- SQLite-Persistenzschicht fuer RepaymentEntry — dump_all/create/update + Pre-Exists-Check + Optimistic-Locking, 1:1 nach Phase-7-Vorlage (`repayment_phase.rs`) mit 6 gruenen Tokio-Tests gegen in-memory SQLite.
- Service-Layer für RepaymentEntry-CRUD plus Batch-Toggle: Validation gegen Phase/Member, Edit-Matrix mit PaidOut-Doppel-Guard, all-or-nothing Batch-Tx mit strukturiertem 409-JSON-Body — 19 grüne Unit-Tests; Plan forderte mind. 14.
- Erweitert die Phase-7-`RepaymentPhaseServiceImpl` um Auto-Befüllung der RepaymentEntries beim `open_phase` und Pending-Entry-Validation beim `close_phase` — beides atomar in der bestehenden Status-Übergangs-Transaktion. 9 neue Unit-Tests grün; alle 14 Phase-7-Tests bleiben unverändert grün. Plan forderte mind. 8 neue Tests.
- 6 REST-Endpoints unter /api/repayment-entry (CRUD + Batch-Toggle) inkl. Router-Reihenfolge-Mitigation, 7 TOs mit strukturiertem 409-Body (BatchFailureResponse/CloseConflictResponse), DI-Wiring teilt RepaymentEntryDao + RepaymentPhaseDao Arc-shared zwischen RepaymentEntryServiceImpl und RepaymentPhaseServiceImpl — W-02 verifiziert (exakt 1 DAO-Konstruktor pro Prozess). 10 grüne Tests; Workspace baut clean.
- 15 E2E-Tests gegen real-laufenden HTTP-Server mit in-memory SQLite verifizieren Phase 8 end-to-end: Auto-Fill beim Phase-Open + 3 Edge-Cases (zero-members/no-exit-date/outside-FY), manueller Create + Validation (Phase-not-Open 409 + Range 400), Update-Edit-Matrix (Open↔Contacted + PaidOut-Reject 409), Soft-Delete, Batch-Toggle (Happy + PaidOut-Target 400), Close-Validation (409 mit pending_count + member_number sowie 0-Entry-Erlaubt), und Audit-Hashchain bleibt valid nach komplettem Phase-8-Lifecycle. 270 grüne E2E-Tests (Phase-7-Baseline 255 + 15 neue Phase-8).
- Re-Read nach audited_update! in update_repayment_entry und batch_toggle_status — Clients erhalten jetzt die frische DAO-generierte version-UUID statt der stale pre-update Version, sodass realistische Edit-Flows keine 409-Endlosschleife mehr produzieren
- Re-Read nach audited_create! / audited_update! in allen 4 RepaymentPhase-Lifecycle-Methoden (create / update / open / close) — Clients erhalten jetzt die frische DAO-generierte version-UUID statt der stale pre-update Version, sodass realistische Edit- und Lifecycle-Flows keine 409-Endlosschleife mehr produzieren. Phase-7-erbte Bug-Klasse damit beseitigt; selbe Pattern wie 08-07 für RepaymentEntry.
- Aggregat-Konsistenz im RepaymentEntry: batch_toggle_status mappt missing/soft-deleted entry_id auf HTTP 404 (statt 409 mit "entry not found"-Body), gleicht damit get/update/delete an, und OpenAPI dokumentiert die Trennung 404 vs 409 explizit für Frontend-Clients.
- 5 E2E-Tests in genossi_bin/tests/e2e_tests.rs (281 LOC) zementieren die 08-07/08/09-Fixes gegen zukünftige Rückfälle und schließen IN-04 (Test-Coverage-Lücke fürs 2nd-PUT-mit-Response-version-Szenario)
- 12-Schritt-Cascade fuer atomare Auszahlungs-Buchung: 1x audited_create! (MemberAction::Verkauf) + 2x audited_update! (Member, RepaymentEntry) in einer SQLite-Tx mit gemeinsamem Process-String und BL-01 Re-Read-Defense.
- Ein neuer Axum-Handler `mark_paid_out` exposed den Plan-09-01-Cascade-Service unter `POST /api/repayment-entry/{id}/mark-paid-out` mit kompletter OpenAPI-Dokumentation aller 5 Status-Codes.
- Workspace-blockierender 2-Zeilen-Fix in `genossi_bin/src/lib.rs`: `type MemberActionDao = MemberActionDao;` im RepaymentEntryServiceDeps-Block + `member_action_dao: member_action_dao.clone(),` im Konstruktor-Aufruf — heilt E0046+E0063 aus Plan 09-01, macht Workspace-Build clean.
- 4 End-to-End-Tests beweisen den atomaren mark_paid_out-Cascade gegen einen echten HTTP-Server: Happy-Path mit Audit-Chain-Verify, PAYO-03-Validation, PAYO-04-Final-Block und Race-Defense via tokio::join!. Alle 5 ROADMAP-Success-Criteria fuer Phase 9 sind End-to-End verifiziert.
- Edit 1
- 1. [Rule 1 — Bug] Acceptance-criterion conflict on `grep DROP COLUMN` returns 0
- Migration + Auditable-Extension fuer member_document mit 3 neuen Optional-Spalten (template_id, mail_recipient_id, status) und neue DocumentType::RepaymentMail-Variante; FROZEN-Order halt Audit-Hashchain konsistent.
- 1. [Rule 1 — Bug] Plan example used wrong field name `to_address` in `RecipientInput`
- SendBulkMailRequest gets two optional `Option<String>` UUID fields (template_id D-12, repayment_phase_id D-03), parsed via `uuid::Uuid::parse_str` with `MailServiceError::BadRequest` -> HTTP 400 echoing the malformed input, replacing the two `TODO(10.04)` placeholders in the bulk-send `create_job(...)` call-site from Plan 10.03's commit 82c8515.
- merge_repayment_context-Helper + validate_template_with_repayment-D-14-Validator + 5 dedizierte Tests dokumentieren das `{% if X is defined %}`-Pattern unter minijinja-strict; Plan-Spec-Bugs (`..base`-Spread, `{% if %}`-Guard ohne `is defined`, D-14-is_ok-Erwartung) Rule-1-korrigiert.
- Mail-Worker integriert D-04 Repayment-Variablen-Aggregation und D-10/D-11 audited MemberDocument-Create via inlined worker_audit-Modul (Cross-Crate-Audit ohne Dependency-Cycle); 6 neue Generic-Deps am start_mail_worker, fail-tolerant per Recipient, hash-chain byte-identisch zu genossi_service_impl.
- RestStateImpl persists 5 new DAO fields (member_document, repayment_phase, repayment_entry, mail_template, transaction) and start_mail_worker Arc::clone-s 6 deps into the spawn block — workspace compiles clean, binary boots without panic, mail worker is now functional with single-hash-chain guarantee preserved.
- 5 E2E-Tests verifizieren SC#1-4 + Audit-Chain-Integrity + PII-Safety + D-10 ad-hoc-skip end-to-end gegen den live REST-Stack + Mail-Worker; deterministische SMTP-Stub-Strategie via 127.0.0.1:1 + RFC5321-fail-fast; Rule-2 fix in rest.rs routet repayment-linked validations durch validate_template_with_repayment.
- Typst-basiertes Auszahlungslisten-Rendering mit 6-Spalten-Tabelle (Nr./Name/IBAN/Anteile/Betrag/Verwendungszweck), Repeat-Header, optionaler Summenzeile, UTF-8-Verwendungszweck-Strings und fresh-install Default-Template-Provisioning.
- Service-Layer-Interface `RepaymentExportService` mit Pdf-only ExportFormat (D-12), Open-default ExportInclude (D-03) und `RepaymentExport`-Bundle (bytes/content_type/filename) — 1:1-Mirror des Phase-6-AttendanceExportService-Patterns, vollständig automock-fähig.
- `RepaymentExportServiceImpl<Deps>` mit Permission-Funnel `load -> admin -> status` (D-10/D-11/Pitfall #2), N+1-DAO-Read-Pipeline in einer Tx, In-Memory-Include-Filter (D-01/D-02), stabile Sortierung (D-09), Verwendungszweck-Pre-Computing mit ORIGINAL-Umlaut `Anteilsrückzahlung` (D-04/D-05), Euro-Format-Pre-Computing OHNE `.abs()` (REVISION-Fix B3), `tx.commit()` VOR PdfGenerator-Render (Pitfall #8), und 5 Service-Layer-Tests (Grep-Gate + B1/W6 + W1 + B3 + B2/Pitfall #2 Mock).
- REST-Handler `export_repayment` unter `GET /api/repayment-phase/{phase_id}/export/{format}` mit Format-Whitelist NUR `pdf` (D-12), Default-Include `Open` (D-03), lokales `map_export_error` (PermissionDenied -> 403 per D-11), OpenAPI-Schema mit allen 6 Status-Codes, Router-Generator und vollstaendigem lib.rs-Wiring inkl. RestStateDef-Bound-Count-Sync (REVISION-Fix W2 deterministisch).
- 5 additive Edit-Stellen in `genossi_bin/src/lib.rs` verdrahten den `RepaymentExportServiceImpl` (Plan 11.03) ueber das `RepaymentExportRestState`-Trait (Plan 11.04) in die `RestStateImpl`. Alle DAOs + `pdf_generator` + `template_storage` werden via Arc::clone aus den bereits konstruierten Arcs geteilt (Single-Arc-per-Process); `cargo build` (full workspace) ist clean.
- 8 neue E2E-Tests + 1 neuer Helper `create_member_without_iban` in `genossi_bin/tests/e2e_tests.rs` verifizieren die gesamte Phase 11 (EXPO-01/02/03/05) gegen einen real-running Server mit In-Memory-SQLite. Happy-Path inkl. Umlaut-Member `Hans Müller` (REVISION-Fix W6); Filename-Schema `auszahlung-{fy}-{include}.pdf` in jedem PDF-Erfolgsfall asserted (REVISION-Fix W4); Pitfall #2 (Status-Leak) bleibt durch Plan 11.03 Service-Layer-Mock abgedeckt — KEIN E2E-403-Test (REVISION-Fix B2). Plan-11.03-Grep-Gate (`no_audit_macros_used`) und Service-Layer-Pitfall-#2-Test bleiben gruen.
- One-liner:
- Vier Pure-Reuse-Bausteine (format_payout_eur, parse_euro_to_cents, RepaymentPhaseStatusBadge, RepaymentEntryStatusBadge) als Component-First-Foundation für alle nachfolgenden Phase-12-Plans.
- Two new Dioxus Routes (RepaymentPhases + RepaymentPhaseDetails) wired in `src/router.rs`, Stub-Page für Details mit `TODO Plan 12-05` Marker, plus admin-gated 'Anteils-Rückzahlung' NavItem in der Vorstand-TopBar — `/repayment-phases` ist jetzt navigierbar und zeigt die schon in 12-04 implementierte Listen-Page, `/repayment-phases/:id` zeigt die Plan-12-05-Stub-Markierung.
- One-liner:
- One-liner:
- One-liner:
- Phase-12-Eigen-Design Inline-Cell-Edit-Component fuer share_count_to_pay_out — i32-spezialisiert, status-aware via disabled-Prop, Backend-CHECK-Validierung (n > 0) als testbare pure-fn extrahiert.
- One-liner:
- One-liner:
- One-liner:
- One-liner:
- One-liner:
- One-liner:
- One-liner:
- Datei:
- Neue `RepaymentLetter`-Variante im `DocumentType`-Enum plus zwei Default-Templates (`auszahlungs_anschreiben.typ` + `_bundle.typ`) registriert in `DEFAULT_TEMPLATES`; Single-Source-of-Truth-Vertrag via exportierter Typst-`render-letter`-Funktion mit Drift-Schutz-Tests.
- Neue `RepaymentContextResolver`-Trait in `genossi_service::repayment_context` mit zwei Methoden (`resolve` + `aggregate`), Impl in `genossi_service_impl::repayment_context` mit Pure-Function `aggregate_for_member` als 1:1-Mirror der Phase-10-Worker-Inline-Aggregation; 19 Unit-Tests gruen, Worker bleibt per D-13-10 unveraendert.
- PdfGenerator bekommt zwei neue synchrone Render-Methoden — `render_repayment_letter` (1 Member → 1 PDF) und `render_repayment_letter_bundle` (N Members → 1 PDF mit `#pagebreak()`) — plus zwei build_inputs-Helper (sys.inputs JSON-Pattern). TDD RED-GREEN-Cycle pro Task; 9 Helper-Tests + 4 Smoke-Tests; Plan-13-01-Bundle-Template-Bug per Compat-Layer abgefangen.
- Kern-Service `RepaymentLetterServiceImpl` orchestriert die gesamte Brief-Erzeugung end-to-end: Permission-Funnel (Phase 11 Pattern), Entry-Validation, Multi-Entry-Aggregation via Resolver::aggregate (kein 1+N DB-Read), sequential audited MemberDocument-Persistenz, Bundle-PDF-Render. 12 Unit-Tests + 3 Grep-Gates (D-13-09 dreifach, user_id KEIN Sentinel, aggregate-vs-resolve) — alle gruen.
- REST-Layer + Binary-Wiring fuer den Bulk-Brief-Service end-to-end: neuer POST-Endpoint mit Direct-Download-Pattern (Phase-11-Konsistenz), 6-Status-Code OpenAPI-Doku inkl. X-Document-Count Header (D-13-04 Frontend-Toast-Pluralisierung), Permission-Override fuer 403-Forbidden, Production-DI mit 10 Letter-Service- + 2 Resolver-Dependencies via Single-Arc-per-Process. Workspace-Build + alle Unit-Tests gruen; baseline Arc-DAO-Count unveraendert bei 25.
- Frontend-Komplement zu Phase 12 D-18: neuer Bulk-Action-Button "Anschreiben erzeugen" (Purple) neben dem Massenmail-Button in der RepaymentEntryList-Component. POST + JSON-Body mit `entry_ids` der Multi-Selektion, Browser-Save des Bundle-PDFs via `<a download>`-Trick mit revoke_object_url, Toast nutzt `X-Document-Count`-Header (Server-Aggregations-Count nach D-13-04) mit Singular/Plural-aware i18n. Selection bleibt nach Download UNVERAENDERT (D-13-09 Selection-Preservation), damit Vorstand direkt mit dem Phase-8-Batch-Endpoint "Als angeschrieben markieren" auf der gleichen Auswahl fortsetzen kann. cargo check und cargo build --release beide clean.
- 8 End-to-End-Tests verifizieren das gesamte POST /api/repayment-phase/{id}/letters/generate Pipeline durch echte HTTP-Calls — vom Auth-Funnel über Service-Logik bis MemberDocument-Persistenz und Audit-Hashchain. 7 aktiv (gruen), 1 #[ignore] (mock_auth-Limit dokumentiert). Cross-Phase-Regression-Gate gruen (292 bestehende e2e_tests). Phase 13 ist end-to-end-validiert.

</details>

---

## v1.0 GV-Anwesenheits-Erfassung (Shipped: 2026-05-29)

**Phases completed:** 5 phases, 34 plans, 68 tasks

**Key accomplishments:**

- SQLite migrations and DAO traits/impls for the Assembly aggregate plus the

composite-PK assembly_member_snapshot join table — including English-only
AssemblyStatus enum, Auditable impl with 6 audit fields, optimistic-locking
update path, and 17 unit tests.

- Five public TOs (AssemblyStatusTO, AssemblyTO, AssemblyDetailTO, CreateAssemblyRequest, UpdateAssemblyRequest) with ToSchema for OpenAPI, ISO8601 datetime serde on every Optional<PrimitiveDateTime>, and bidirectional Status-enum conversion between DAO and wire format.
- AssemblyService trait + DTOs in `genossi_service::assembly` and full

`AssemblyServiceImpl` in `genossi_service_impl::assembly` covering the
Preparation→Open→Closed lifecycle. `open_assembly` is atomic (single Tx
for status flip, audit entry, and snapshot population). 12 unit tests
total (6 in service-trait crate, 6 mockall-based in service-impl).

- Six Axum handlers in `genossi_rest::assembly` (list/create/get/update/open/close) plus full DI wiring of `AssemblyServiceImpl` into `genossi_bin::RestStateImpl`. Validation helpers, ApiDoc, router registration, and three type-bound updates (`create_app`, `start_server`, `start_test_server`) — workspace builds and 215 e2e tests stay green.
- Three new e2e tests in `genossi_bin/tests/e2e_tests.rs` covering the full Assembly lifecycle (Preparation → Open → Closed) with audit hash chain verification (ASSY-07) and two negative tests for illegal state transitions (Pitfall 3). 218/218 e2e tests green; full workspace test suite green; release build clean. Phase 01 goal end-to-end test-belegt.
- SQLite helper_token table + Auditable DAO trait + race-safe atomic_redeem on UPDATE...RETURNING — proven by 11 unit tests including double-redeem regression
- Typsichere `AuthContext::Helper { session_id, assembly_id }`-Variante als Phase-3-Vorbereitung — ohne cfg-Gate verfügbar in mock_auth und oidc, mit zwei Konstruktions-/Distinktheits-Tests und Smoke-Test gegen versehentliche Feature-Gate-Regression.
- Six REST TOs (HelperTokenStatusTO, HelperTokenTO, HelperTokenCreateResponseTO, CreateHelperTokenRequest, RedeemRequest, RedeemResponse) plus a 4-method HelperTokenService trait with #[automock] mock — proven by 13 unit tests including a defensive token_hash leak guard and a Debug-output guard.
- HelperTokenServiceImpl with gen_service_impl! over 8 deps, 4 service methods (create+list+revoke+redeem), Crockford+SHA256+QR+atomic-redeem orchestration, and ServiceError-discriminator-string convention proven by 11 unit tests including all four D-24 mapping branches.
- Helper-Sessions werden im SessionService an `claims.kind=="helper"` erkannt; D-18 invalidiert die Session sofort, wenn die gebundene Assembly nicht mehr `Open` ist — Pitfall 2 Early-Return verhindert DB-Roundtrip auf dem User-Session-Hot-Path. Mock-Variante erkennt Cookie-Format `helper:<uuid>:<tok>` und cascadiert via optionalem AssemblyStatusProbe.
- Vier Axum-Handler (3 Vorstand admin + 1 Public mit Set-Cookie und Pro-IP-Rate-Limit), zwei neue RestError-Varianten (403/410) für die D-24-Differenzierung, vollständiges DI-Wiring in genossi_bin mit DbAssemblyStatusProbe für HLPR-05-Cascade in mock_auth-Builds — proven by 4 grünen Validation-Tests im genossi_rest und 189 grünen workspace-tests in mock_auth + oidc.
- 10 E2E-Tests in `genossi_bin/tests/e2e_tests.rs` decken HLPR-01/02/04/05/06/07 ab; aufgedeckt + behoben wurden zwei Plan-05-Service-Bugs (redeem pool-deadlock, revoke version-mismatch) und der Mock-Session FK-Constraint-Mismatch — alle 228 e2e_tests.rs-Tests grün, alle 528 workspace-lib-tests grün in beiden Feature-Builds.
- Lightweight Attendance-Join-Tabelle mit atomarem SQLite-UPSERT, idempotentem Soft-Delete-Toggle, DSGVO-Whitelist-View und Snapshot-Membership-Check — alles ohne Audit-Log und ohne Optimistic-Locking.
- Eine neue Trait-Method `list_session_ids_for_assembly` auf HelperTokenDao + SQLite-Impl + 3 grüne Tests; Cascade-Anker (D-12) für Plan 05's `AssemblyServiceImpl::close_assembly`-Erweiterung.
- Trait-Erweiterung `ClaimContext::as_helper(&self) -> Option<Uuid>` mit Default-Impl (failure-closed → None) und einem AuthenticatedContext-Override, der Phase-2-HelperClaims-JSON defensiv parst — die typsichere Brücke für Plan 05's Helper-Permission-Branch.
- Service-Interface-Layer für die GV-Anwesenheits-Erfassung — 4-Methoden-Trait (`list_members`, `mark_present`, `mark_absent`, `stats`), `AttendanceStats`-Domain-Type, `AttendanceMemberTO`-Whitelist mit 7 Feldern, `AttendanceStatsTO` für den Live-Counter — alles ohne ServiceImpl-Wiring, bereit für Plan 05+06 als Konsumenten.
- Service-logic core of Phase 3: a 4-method `AttendanceServiceImpl` plus the central `check_assembly_access` permission funnel (Helper / Vorstand / Full discrimination), AND the cascade-extension to `AssemblyServiceImpl::close_assembly` that invalidates every helper-session bound to the closing GV. All 19 new tests green; all 188 pre-existing service-impl tests stay green.
- Phase 3 final integration: 4 attendance REST handlers, DI-wiring of AttendanceServiceImpl into the binary's RestStateImpl, OpenAPI doc registration, and 6 end-to-end tests against a real-running HTTP server with in-memory SQLite. All 9 Phase-3 requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) plus the SC#8 cascade-DB invariant verified at the integration level.
- GET /api/helper/session und POST /api/helper/logout als append-only Routen im existierenden helper_redeem_router — Frontend kann jetzt Auto-Redirect und Logout ohne /api/attendance-Probe machen.
- Foundation-Layer für Phase-4-Frontend gebaut: web-sys Camera-Features, vendored ZXing-JS-Polyfill, JS-Bridge für nativen BarcodeDetector mit Feature-Detection, Print-CSS für QR-Karten, Tailwind-Safelist und cargo-testbares Crockford-Validation-Modul mit 9 grünen Unit-Tests.
- 14 Phase-4-TOs, 16 async API-Funktionen, 410-Mapping und 67 i18n-Keys (de+en) — Voraussetzung für alle Wave-2-Components und Pages.
- Vier shared Components (AttendanceSearch, AttendanceList, LiveCounter, ConnectionBanner) inklusive Pure-Function-Helpers für unit-testable UI-Logik — der Component-First-Anker für ATTN-06-Reuse zwischen Helfer- und Vorstand-Anwesenheits-Pages.
- Vier Helper-Login-/Token-Components: ManualCodeInput (HLPR-03 iOS-Fallback), QrScanner (BarcodeDetector + ZXing-Polyfill mit Camera-Lifecycle-Cleanup), QrCard (printable Token-Card), HelperShell (no-Vorstand-Chrome Layout mit Locale::De-Forcing).
- Phase:
- mod.rs nimmt 15 Wave-2.1-Components in Empfang — Pages 07-09 koennen jetzt sauber importieren.
- Zwei Vorstand-Pages voll ausimplementiert: `/assemblies` (Liste + Create-Modal) und `/assemblies/{id}` (3-Tab-Detail mit Stammdaten, Tokens, Anwesenheit). Anwesenheits-Tab nutzt EXAKT dieselben 4 Components, die Plan 04-09 für /helper/attendance verwenden wird — ATTN-06 Reuse-Anker etabliert.
- Beide Helfer-Pages voll ausimplementiert: `/helper` (Login mit QR-Scan + Manual-Code parallel + Auto-Redirect) und `/helper/attendance` (4 shared Components in HelperShell-Layout). ATTN-06 Component-Reuse bewiesen: helper_attendance.rs nutzt identische Component-Invocations wie assembly_details.rs Anwesenheits-Tab — einziger Unterschied ist der HelperShell- vs. RequirePrivilege-Wrapper.
- Workspace-Dependency-Promotion fuer rust_xlsxwriter/csv plus neues Typst-Template `teilnehmerliste.typ` mit konditionalem X-von-Y-Kopf und 6-spaltiger Repeat-Header-Tabelle.
- AttendanceExportService Trait + Impl mit Admin+Closed-Funnel, drei Format-Writern (CSV BOM/Semikolon, XLSX rust_xlsxwriter, PDF via Typst-Template), 6-Spalten-DSGVO-Whitelist, kein Audit-Log (D-17), strukturiertes tracing::info! (D-18) — 16/20 Phase-6-Decisions in Code uebertragen.
- HTTP-Endpoint `GET /api/assembly/{aid}/attendance-export/{format}` ist live aufrufbar; AttendanceExportServiceImpl ist in RestStateImpl gewired; 9 E2E-Tests decken PDF/CSV/XLSX-Erfolgspfade, 409 fuer Open/Preparation, 400 fuer unbekanntes Format, include=present-Filter, Filename-Schema und D-12-Post-Close-Edit-Reflexion ab. Plus Rule-2-Auto-Fix: teilnehmerliste.typ in DEFAULT_TEMPLATES — ohne das funktioniert PDF-Export nicht out-of-the-box.
- Closed-only Export-Tab in assembly_details.rs lets Vorstand download attendance lists as PDF/CSV/XLSX via a blob-URL pipeline — Task 1 (i18n) and Task 2 (API + ExportTab) are committed; Task 3 (browser verification checkpoint) is pending human approval.

---
