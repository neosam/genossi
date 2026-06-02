# Phase 13: RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder — Research

**Researched:** 2026-06-01
**Domain:** Rust/Axum-Backend + Typst-PDF-Render + Dioxus-WASM-Frontend (Bulk-PDF-Brief-Generierung mit Audit-MemberDocument-Persistenz und transientem Bundle-Download)
**Confidence:** HIGH (CONTEXT.md liefert 11 gesperrte Decisions; Vorbild-Phasen 10/11 sind verifiziert im Code vorhanden; nur kleine Discretion-Bereiche offen)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-13-01 (Hybrid-Bundle-Strategie):** Server rendert N Einzel-PDFs (1 pro Member nach Aggregation), persistiert pro Member ein `MemberDocument` im `document_storage`, UND rendert zusätzlich ein gebündeltes Druck-PDF (`#pagebreak` zwischen Briefen) in-memory, das NICHT persistiert wird und direkt im HTTP-Response ausgeliefert wird. Kostet 1× extra Typst-Compile pro Bulk-Request (vernachlässigbar bei N≈20).

**D-13-02 (Direct-Download in Response):** `POST /api/repayment-phase/{id}/letters/generate` antwortet sofort mit dem Bundle-PDF als binärem Body — kein zweites GET, keine Phase-Document-Relation. Konsistent mit Phase-11 PDF-Export-Pattern (`repayment_export.rs:147-152`).

**D-13-03 (Selektions-Body `{ entry_ids: [...] }`):** Endpoint nimmt eine flache Liste von `repayment_entry_id`s. Server validiert, dass alle Entries zur `phase_id` im Pfad gehören (sonst 400 `entry_phase_mismatch`).

**D-13-04 (Multi-Entry-Aggregation: 1 Brief pro Member mit Summe):** Mehrere Entries pro Member werden zu EINEM Brief aggregiert: `share_count = SUM(...)`, `payout_amount = share_count × phase.share_value`. Analog Phase 10 D-04.

**D-13-05 (Template UI-editierbar via Template-Editor):** `templates/defaults/auszahlungs_anschreiben.typ` wird via `include_bytes!` in `DEFAULT_TEMPLATES` (`template_storage.rs:10`) registriert. Vorstand kann das Template über den existierenden `/templates`-Editor (`genossi-frontend/src/page/templates.rs`) anpassen — REVISION der Architektur-Note D-LETT-02.

**D-13-06 (Brief-Body 4 Bausteine):** (1) Reference-Block oben, (2) Auszahlungsbetrag-Absatz, (3) IBAN-Block mit Typst-`#if`-Switch (vorhanden / NULL → `mv@nebenan-unverpackt.de`), (4) hardcoded Vorstands-Signatur "Herzliche Grüße, Carolin Weidmann, Dina Beier und Simon Goller".

**D-13-07 (KEIN SEPA-Verwendungszweck im Brief):** Info-Schreiben ans Mitglied, kein Bank-Beleg. Verwendungszweck steht auf der Phase-11-Auszahlungsliste.

**D-13-08 (Idempotenz: jeder Klick = neuer Brief):** Kein 409, kein Confirm-Dialog, kein UNIQUE-Constraint. `is_singleton = false`. Audit-Hashchain protokolliert beide Erzeugungen chronologisch.

**D-13-09 (KEIN Auto-Status-Toggle Open → Contacted):** Backend lässt `RepaymentEntry.status` unverändert. Vorstand triggert separat den existing Phase-8-Batch-Endpoint. Symmetrie zur Phase-10-Mail-Pipeline.

**D-13-10 (`RepaymentContextResolver` in Phase 13 gebaut, Worker-Refactor separat):** Letter-Service ist erster Caller. Phase-10-Mail-Worker behält Inline-Aggregation unverändert. Worker-Migration nach Phase 13 als `/gsd-quick` (Todo `phase-10-worker-refactor-resolver.md`).

**D-13-11 (Pending-Todo referenziert):** `.planning/todos/pending/phase-10-worker-refactor-resolver.md` bleibt als referenzierter Folge-Quick.

### Claude's Discretion

- **Resolver-API-Design:** Trait vs. Free-Function. Empfehlung: Trait `RepaymentContextResolver` mit `resolve(phase_id, member_id) -> Result<RepaymentContext, ServiceError>` + struct `RepaymentContext { share_count: i32, payout_amount: String, fiscal_year: i32 }`. Mockable analog `UuidService`-Pattern.
- **Euro-Format-Konvention:** Wiederverwenden Phase 10 D-04 — `"X,YZ"` ohne Tausenderpunkt, ohne Euro-Symbol (Template rendert `{{ payout_amount }} €` bzw. analog).
- **Bundle-PDF-Filename:** `auszahlungs_anschreiben_GJ_{fiscal_year}.pdf` (Planner-Discretion ob `phase_id` oder Datum mitkodieren).
- **MemberDocument-Filename pro Einzel-PDF:** `auszahlungs_anschreiben_{member_number}_GJ_{fiscal_year}.pdf` (Planner-Discretion).
- **Render+Persist-Reihenfolge:** Empfehlung — alle N MemberDocuments in einer Transaction persistieren + Files schreiben, dann Bundle in-memory rendern, dann committen. All-or-Nothing bei Render-Fehlern.
- **Frontend-Toast-Wortlaut:** Planner darf finalisieren. Empfehlung: "N Briefe erzeugt. Vergiss nicht, die Einträge anschließend als angeschrieben zu markieren." (verweist auf D-13-09).
- **Multi-Entry-Aggregation im Brief:** Aggregiert anzeigen (Summe), keine Aufteilung `3+2`. Konsistent mit Phase 10 D-04.
- **OpenAPI-Doku:** Utoipa-Schema mit 200 (PDF binary), 400 (entry_phase_mismatch), 401 (no auth), 403 (helper auth), 404 (phase nicht gefunden), 409 (phase_not_active).

### Deferred Ideas (OUT OF SCOPE)

- Status-Cascade Auto-Toggle Open → Contacted (Backend bleibt symmetrisch zu Phase 10).
- PDF-Attachment an Mails (Brief und Mail bleiben komplementäre Kanäle).
- Persistiertes Bundle-PDF pro Phase (Bundle ist transient; Re-Download via erneutes "Anschreiben erzeugen").
- Vorstandsnamen aus Config-Tabelle (hardcoded im Default-Template; Vorstand passt via Template-Editor an).
- Brief-Status-Tracking pro Member (Audit-Spur über MemberDocument reicht).
- SEPA pain.001 XML-Export (deferred zu v2 SEPA-01).
- CSV-Export (deferred per Phase-11 D-12).
- **Phase-10-Mail-Worker auf `RepaymentContextResolver` migrieren** — als referenzierter Folge-Quick `.planning/todos/pending/phase-10-worker-refactor-resolver.md`, NICHT in Phase 13.
</user_constraints>

<phase_requirements>
## Phase Requirements

Phase 13 hat KEINE eigenen REQ-IDs zugeordnet (Coverage Gate wird übersprungen). Trotzdem wird BRIEF-01 aus `REQUIREMENTS.md §Brief-Anschreiben-Automatik` adressiert:

| ID | Description | Research Support |
|----|-------------|------------------|
| BRIEF-01 | Brief-Vorlagen aus Auszahlungs-Eintrag direkt als PDF erzeugen — bisher: "out of v1.1, Vorstand erzeugt manuell". | Phase 13 hebt den v1.1-Defer auf. Implementation gemäß D-13-01..11: synchron-server-seitige Typst-Render-Pipeline (PdfGenerator-Pattern aus Phase 6/11), `MemberDocument`-Persistenz pro Member mit echtem File im `document_storage`, transientes Bundle-PDF als Direct-Download. Komplementär zu Phase 10 (Mail-Kanal). |
</phase_requirements>

## Summary

Phase 13 fügt einen Brief-Kanal zur v1.1-Auszahlungs-Pipeline hinzu, technologisch nahezu vollständig auf bewährten Phase-6/10/11-Pattern aufbauend. Drei neue Code-Bausteine:

1. **`RepaymentLetterService` (Trait + Impl)** — Permission-Funnel (admin-only, Status-Gate Open/Closed) analog `RepaymentExportServiceImpl`; orchestriert Aggregation, Render, Persist, Bundle.
2. **`RepaymentContextResolver` (Trait + Impl)** — extrahiert die heute im Phase-10-Worker inline gelebte Aggregations-Logik (Open+Contacted-Filter, SUM share_count, Euro-Format, fiscal_year). Erster Caller: Letter-Service.
3. **REST-Handler `POST /api/repayment-phase/{phase_id}/letters/generate`** — Body `{ entry_ids: [...] }`, Response Direct-Download `application/pdf` mit `Content-Disposition: attachment`. Pattern 1:1 aus `repayment_export.rs` adaptiert.

Plus: neuer `DocumentType::RepaymentLetter`-Variante (im Stil von `RepaymentMail` aus Phase 10), neues Default-Template `templates/defaults/auszahlungs_anschreiben.typ` (Layout-Vorbild `zahlungsanfrage.typ` mit `letter-pro:3.0.0`/`letter-simple`), neuer Frontend-Bulk-Button auf `repayment_phase_details.rs` neben dem existierenden Massenmail-Button.

**Primary recommendation:** Pattern-1:1-Reuse von Phase 11 für REST/Service-Funnel, Pattern-Reuse von Phase 10 (Worker `genossi_mail/src/worker.rs:332-361`) für die Aggregations-Logik, Pattern-Reuse vom existierenden REST-MemberDocument-Upload (`member_document.rs:198-203`) für die File-Storage-Schreibe. Kein Forschungs-Risiko in der Tech-Wahl — alle Bausteine sind im Repo verifiziert.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Permission-Funnel + Status-Gate | Service (`RepaymentLetterServiceImpl`) | — | Domain-Regel (admin + phase ∈ {Open, Closed}); identisch zu Phase 11 Funnel `check_admin_and_phase_status`. |
| Multi-Entry-Aggregation pro Member | Service Helper (`RepaymentContextResolver`) | — | Domain-Logik (Filter Open+Contacted, SUM, Euro-Format). Heute inline im Mail-Worker. |
| Typst-Render pro Einzel-Brief | Service (`PdfGenerator::render_repayment_letter`) | — | Pure Render-Funktion; Sync-Call nach Tx-Commit (Phase-11-Pitfall #8). |
| Bundle-PDF `#pagebreak` zwischen Briefen | Service oder Typst-Template | Discretion | Empfehlung: Typst-Loop im Template mit `#pagebreak()` zwischen `members`-Array-Einträgen → 1 Compile pro Bundle. |
| MemberDocument-Persistenz + Audit | Service (`audited_create!`) | — | Audit-Pflicht; identisches Macro wie `member_document.rs:140-147`. |
| File-Schreibe ins `document_storage` | REST-Handler (analog `member_document.rs:199-203`) | Service (alternativ) | In Phase-10-Codebase wird `document_storage.save()` aus dem REST-Layer aufgerufen — Phase 13 kann das spiegeln ODER ins Service ziehen, weil hier 1 Service-Call N Files schreibt. Discretion. |
| Direct-Download Response | REST-Handler | — | Axum-Response-Builder mit `application/pdf` + `Content-Disposition`. 1:1 aus `repayment_export.rs:147-152`. |
| Multi-Select-Frontend + Bulk-Button | Frontend Component (`repayment_entry_list.rs`) | Page (`repayment_phase_details.rs`) | Phase-12-Multi-Select-Pattern wiederverwenden, neuer Button neben "Massenmail". |
| Browser-Save (Blob → Download) | Frontend API-Layer (`api.rs`) | Page | Pattern 1:1 aus `render_template_pdf` (`api.rs:506-548`) mit `Blob` + `createObjectURL`. |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Axum | 0.8.3 [VERIFIED: CLAUDE.md + Cargo.toml] | REST-Handler, Router | Projekt-Standard; kein Wechsel im v1.1-Scope. |
| Typst | 0.14 [VERIFIED: CLAUDE.md] | Typst-Compile-Engine | Bereits in Use für Phase 6/11; identische Render-Pipeline via `PdfGenerator`. |
| `typst-pdf` | 0.14 [VERIFIED: CLAUDE.md] | PDF-Serialisierung | Pair mit `typst`-Crate; identisch zu Phase 6/11. |
| `@preview/letter-pro` | 3.0.0 [VERIFIED: `typst-packages/preview/letter-pro/3.0.0/` lokal vorhanden + zahlungsanfrage.typ:1] | Briefe mit Falzmarken, Sender/Recipient-Block, Folding-Marks | Status quo aller Brief-Templates in Genossi (`zahlungsanfrage.typ`, `testbrief.typ`). |
| SQLx | 0.8 [VERIFIED: CLAUDE.md] | Async SQLite | Projekt-Standard. |
| `audited_create!` Macro | n/a (intern) [VERIFIED: `audit_macros.rs:5-36`] | Atomare DAO-Create + Hash-Chain-Audit | Projekt-Pflicht für `MemberDocument`-Schreibvorgänge (CLAUDE.md §Audit Log System). |
| `tracing` | 0.1 [VERIFIED: CLAUDE.md] | Strukturiertes Logging | Konsistent mit Phase 11 (`tracing::info!(target = "repayment_export", ...)`). |
| `serde_json` | 1.0 [VERIFIED: CLAUDE.md] | JSON-Kontext für Typst `sys.inputs` | Standard-Pattern (`build_inputs_repayment` in `pdf_generation.rs:776`). |
| Dioxus | 0.6.3 [VERIFIED: CLAUDE.md] | Frontend-Framework | Projekt-Standard. |
| `web-sys`/`wasm-bindgen` | 0.3 / 0.2.97 [VERIFIED: CLAUDE.md] | Fetch + Blob + createObjectURL | Pattern 1:1 aus `render_template_pdf` (`api.rs:506-548`). |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `path-clean` | 1.0 [VERIFIED: CLAUDE.md] | Path-Traversal-Schutz im Storage | Wird vom existierenden `FilesystemDocumentStorage` (`document_storage.rs:29-46`) verwendet — Phase 13 nutzt diesen Storage transitiv. |
| `uuid` | 1.6 [VERIFIED: CLAUDE.md] | Entity-IDs | Standard. |
| `time` | 0.3 [VERIFIED: CLAUDE.md] | `PrimitiveDateTime` für `created`/`deleted` | Standard. |
| `utoipa` | 5.0 [VERIFIED: CLAUDE.md] | OpenAPI-Schema | Standard für REST-Handler-Doku. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Bundle via Typst-Loop mit `#pagebreak()` | `typst-pdf::pdf` für N Einzel-PDFs + Server-seitiges PDF-Merge via `lopdf` o.ä. | Discretion. Empfehlung Typst-Loop: 1 Compile statt N+1, einfacher, ohne zusätzliches Crate. `lopdf` wäre ein neues Dependency, das es im Stack nicht gibt. [ASSUMED: Performance des Typst-Loops bei N=20 ist ausreichend — heutige `render_repayment_list` packt 50-100 Zeilen problemlos] |
| Eigener neuer DAO-Filter `find_by_member_and_phase` | In-Memory-Filter auf `find_by_phase_id`-Ergebnis | Discretion. Phase-11-Vorbild (`repayment_export.rs:197-203`) verwendet `find_by_phase_id` + In-Memory-Filter — kein neuer DAO-Helper. Phase 13 empfehlt dieselbe Linie für den Resolver. |
| Service-Layer schreibt Files direkt | REST-Layer schreibt Files (Phase-10-Pattern via `member_document.rs:199-203`) | Trade-off offen. Phase 10 hat keine File-Schreibe (nur Mail), Phase 11 schreibt keine Files (Direct-Download). Phase 13 ist der ERSTE Case mit File-Schreibe im Service-Layer-Pfad. Empfehlung: Service injiziert `Arc<dyn DocumentStorage>` selbst (analog wie `static_document_service` in `genossi_bin/lib.rs:906-909` es macht — bewährter Pattern). |

**Installation:**

Keine neuen Cargo-Dependencies nötig. Alle Crates sind bereits im Workspace.

**Version verification:** Tools sind workspace-pinned via `Cargo.lock`. Letter-pro 3.0.0 ist lokal in `typst-packages/preview/letter-pro/3.0.0/` vorhanden — keine Online-Resolution beim Build [VERIFIED].

## Architecture Patterns

### System Architecture Diagram

```text
┌─ Frontend (Dioxus WASM) ────────────────────────────────────────────┐
│                                                                      │
│  RepaymentPhaseDetails (page)                                        │
│    └─ TabStrip: [Stamm | Einträge | Export]                          │
│        └─ Einträge-Tab                                               │
│           └─ RepaymentEntryList (component)                          │
│               ├─ Per-Row-Checkbox + Header-Checkbox                  │
│               ├─ Bulk-Button "Massenmail" (existing, Phase 12 D-18)  │
│               └─ Bulk-Button "Anschreiben erzeugen" ★ NEU             │
│                                                                      │
│  Klick → on_letter_request(selected_entry_ids) bubbelt zur Page      │
│  Page → api::generate_repayment_letters(phase_id, entry_ids)         │
│  api  → fetch POST mit JSON-Body, .blob(), createObjectURL,          │
│         <a href={blob_url} download={...} click>                     │
│                                                                      │
└────────────────────────┬─────────────────────────────────────────────┘
                         │
                         │ POST /api/repayment-phase/{phase_id}/letters/generate
                         │ Body: { "entry_ids": ["uuid", ...] }
                         ▼
┌─ Axum REST-Handler ─────────────────────────────────────────────────┐
│  generate_letters(phase_id, body, auth) (genossi_rest/.../letter.rs) │
│  ├─ extract_auth_context                                             │
│  ├─ Validate body shape (entry_ids non-empty)                        │
│  ├─ Service-Call:                                                    │
│  │   service.generate(phase_id, entry_ids, auth)                     │
│  ├─ map_letter_error (PermissionDenied → 403, analog Phase 11)       │
│  └─ Response: 200 application/pdf + Content-Disposition: attachment  │
└────────────────────────┬─────────────────────────────────────────────┘
                         ▼
┌─ Service-Layer (genossi_service_impl/src/repayment_letter.rs) ──────┐
│  RepaymentLetterServiceImpl::generate                                │
│  ├─ tx = TransactionDao::use_transaction                             │
│  ├─ check_admin_and_phase_status (funnel: load 404 → admin 403 →     │
│  │   status 409). Phase 11 Pattern.                                  │
│  ├─ Validate entry_ids ⊆ phase (in-memory):                          │
│  │   entries = find_by_phase_id(phase_id)                            │
│  │   IF !entry_ids.is_subset_of(entries.ids) → 400 mismatch          │
│  ├─ Group entries by member_id (HashMap<Uuid, Vec<Entry>>)           │
│  ├─ FOR EACH unique member:                                          │
│  │   ├─ ctx = resolver.resolve(phase_id, member_id)                  │
│  │   │       → RepaymentContext { share_count, payout_amount,        │
│  │   │                              fiscal_year }                    │
│  │   ├─ member = MemberDao::find_by_id(member_id)                    │
│  │   ├─ pdf_bytes = pdf_generator.render_repayment_letter(           │
│  │   │       template_base, &member, &ctx                            │
│  │   │   )  ← Single-Letter-PDF                                      │
│  │   ├─ doc_id = uuid_service.new_v4()                               │
│  │   ├─ relative_path = format!("{}.pdf", doc_id)                    │
│  │   ├─ document_storage.save(&relative_path, &pdf_bytes)            │
│  │   ├─ MemberDocument {                                             │
│  │   │       id: doc_id, member_id, document_type: RepaymentLetter,  │
│  │   │       file_name: "auszahlungs_anschreiben_{member_number}_..."│
│  │   │       mime_type: "application/pdf",                           │
│  │   │       relative_path,                                          │
│  │   │       description: "Anschreiben Auszahlung GJ {fy}",          │
│  │   │       template_id: None, mail_recipient_id: None, status: None│
│  │   │   }                                                           │
│  │   └─ audited_create!(self, member_document_dao, &doc, ...)        │
│  ├─ Bundle: pdf_generator.render_repayment_letters_bundle(           │
│  │     template_base, &members_with_contexts                         │
│  │   ) → Vec<u8>  ← Multi-Letter-PDF mit #pagebreak                  │
│  ├─ TransactionDao::commit                                           │
│  └─ Return: bundle_bytes (+ filename) for REST direct-download       │
└─────────────────────────────────────────────────────────────────────┘
         │                                            │
         │ document_storage.save (transitive)         │ MemberDocumentDao::create
         ▼                                            ▼
   ./documents/{uuid}.pdf                       member_document table
   (FilesystemDocumentStorage)                  + audit_log table (hash chain)
```

### Recommended Project Structure

```text
genossi_service/src/
  └── repayment_letter.rs                  ★ NEU — Service-Trait

genossi_service_impl/src/
  ├── repayment_letter.rs                  ★ NEU — Impl + Funnel + Tests
  ├── repayment_context.rs                 ★ NEU — Shared Resolver-Trait + Impl
  ├── template_storage.rs                  ⚠ ERWEITERT — DEFAULT_TEMPLATES-Eintrag
  └── pdf_generation.rs                    ⚠ ERWEITERT — render_repayment_letter(_bundle)

genossi_service/src/
  └── member_document.rs                   ⚠ ERWEITERT — DocumentType::RepaymentLetter

genossi_rest/src/
  ├── repayment_letter.rs                  ★ NEU — REST-Handler + OpenAPI + Router
  └── lib.rs                               ⚠ ERWEITERT — Mount + ApiDoc + State-Trait

genossi_bin/src/
  └── lib.rs                               ⚠ ERWEITERT — DI-Wiring

templates/defaults/
  └── auszahlungs_anschreiben.typ          ★ NEU — Default-Template (UI-editierbar)

genossi-frontend/src/
  ├── api.rs                               ⚠ ERWEITERT — generate_repayment_letters() fn
  ├── component/repayment_entry_list.rs    ⚠ ERWEITERT — neuer Bulk-Button
  └── page/repayment_phase_details.rs      ⚠ ERWEITERT — on_letter_request-Wiring +
                                              Browser-Save + Toast

genossi_bin/tests/
  └── e2e_tests.rs                         ⚠ ERWEITERT — 6+ neue Tests
```

### Pattern 1: REST-Handler Direct-Download (Phase 11 1:1)

**What:** Body wird als `Vec<u8>` direkt im Response zurückgegeben, ohne Persist-then-Fetch.
**When to use:** Synchron generierte PDF-Antworten, die Browser-`<a download>` triggern sollen.
**Example:** [VERIFIED: `genossi_rest/src/repayment_export.rs:122-156`]
```rust
// Pattern aus genossi_rest/src/repayment_export.rs (Phase 11)
pub async fn generate_letters<RestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(phase_id): Path<Uuid>,
    Json(body): Json<GenerateLettersRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = extract_auth_context(Some(context))?;

            let result = rest_state
                .repayment_letter_service()
                .generate(phase_id, body.entry_ids, auth)
                .await
                .map_err(map_letter_error)?;

            let cd = http_util::content_disposition_attachment(&result.filename);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/pdf")
                .header("Content-Disposition", &cd)
                .body(Body::from(result.bundle_bytes))
                .unwrap())
        })
        .await,
    )
}
```

### Pattern 2: Permission-Funnel mit Status-Gate (Phase 11 1:1)

**What:** Strikte Reihenfolge load → permission → status, um Status-Leak an non-admins zu vermeiden.
**When to use:** Jeder Vorstand-only-Endpoint mit Resource-ID im Pfad.
**Example:** [VERIFIED: `genossi_service_impl/src/repayment_export.rs:77-110`]
```rust
async fn check_admin_and_phase_status(
    &self, phase_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<RepaymentPhaseEntity, ServiceError> {
    // 1. Load (404 if missing).
    let phase = self.repayment_phase_dao
        .find_by_id(phase_id, tx)
        .await?
        .ok_or(ServiceError::EntityNotFound(phase_id))?;
    // 2. Admin gate (403). Authentication::Full short-circuits.
    match &context {
        Authentication::Full => {}
        Authentication::Context(_) => {
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context).await?;
        }
    }
    // 3. Status gate (409): Open ODER Closed akzeptiert.
    match phase.status {
        RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
        RepaymentPhaseStatus::Preparation => {
            return Err(ServiceError::Conflict(Arc::from("phase_not_active")));
        }
    }
    Ok(phase)
}
```

### Pattern 3: Aggregations-Logik aus Phase-10-Worker

**What:** Filter `deleted IS NULL AND status IN (Open, Contacted)`, dann SUM, dann Format.
**When to use:** Letter-Service (Phase 13, jetzt). Später Mail-Worker-Refactor (D-13-10).
**Example:** [VERIFIED: `genossi_mail/src/worker.rs:332-360`]
```rust
// Existing in genossi_mail/src/worker.rs (Phase 10) — Resolver migriert das:
let relevant: Vec<_> = entries
    .iter()
    .filter(|e| {
        e.deleted.is_none()
            && e.member_id == member.id
            && matches!(
                e.status,
                RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted,
            )
    })
    .collect();

if !relevant.is_empty() {
    let share_count: i32 = relevant.iter().map(|e| e.share_count_to_pay_out).sum();
    let cents: i64 = (share_count as i64) * (phase.share_value);
    let payout_amount = format!("{},{:02}", cents / 100, cents % 100);
    // Merge into template-context...
}
```

### Pattern 4: Typst-Template mit `sys.inputs` JSON-Kontext (Phase 11)

**What:** Service baut `serde_json::json!({...})`, fügt als String in `Dict` ein. Template ruft `json.decode(sys.inputs.at("key"))`.
**When to use:** Alle neuen Typst-Renderer.
**Example:** [VERIFIED: `genossi_service_impl/src/pdf_generation.rs:776-826`]
```rust
fn build_inputs_repayment_letter(
    phase: &RepaymentPhaseEntity,
    member: &MemberEntity,
    ctx: &RepaymentContext,
) -> Dict {
    let mut inputs = Dict::new();
    let today = time::OffsetDateTime::now_utc().date().to_string();
    inputs.insert(Str::from("today"), Value::Str(Str::from(today.as_str())));

    let member_json = serde_json::json!({
        "first_name": member.first_name.as_ref(),
        "last_name": member.last_name.as_ref(),
        "member_number": member.member_number,
        "salutation": member.salutation.as_ref().map(|s| s.as_ref()),
        "title": member.title.as_ref().map(|s| s.as_ref()),
        "street": member.street.as_ref().map(|s| s.as_ref()),
        "house_number": member.house_number.as_ref().map(|s| s.as_ref()),
        "postal_code": member.postal_code.as_ref().map(|s| s.as_ref()),
        "city": member.city.as_ref().map(|s| s.as_ref()),
        "bank_account": member.bank_account.as_ref().map(|s| s.as_ref()), // NULL → JSON null
    });
    inputs.insert(Str::from("member"),
        Value::Str(Str::from(serde_json::to_string(&member_json).unwrap().as_str())));

    let repayment_json = serde_json::json!({
        "share_count": ctx.share_count,
        "payout_amount": ctx.payout_amount,
        "fiscal_year": ctx.fiscal_year,
    });
    inputs.insert(Str::from("repayment"),
        Value::Str(Str::from(serde_json::to_string(&repayment_json).unwrap().as_str())));

    inputs
}
```

### Pattern 5: Typst-Template (letter-simple + Falzmarken) — Layout-Vorbild

**What:** `@preview/letter-pro:3.0.0` `letter-simple`-Macro für Brief mit Sender/Recipient/Subject + Falzmarken.
**When to use:** Alle Briefe ans Mitglied.
**Example:** [VERIFIED: `templates/zahlungsanfrage.typ:1-70`]
```typst
#import "@preview/letter-pro:3.0.0": letter-simple

#set text(lang: "de")
#let member = json.decode(sys.inputs.at("member"))
#let repayment = json.decode(sys.inputs.at("repayment"))
#let today = sys.inputs.at("today")

#let name = if member.title != none {
  [#member.title #member.first_name #member.last_name]
} else {
  [#member.first_name #member.last_name]
}

#let anrede = if member.salutation == "Herr" { "Lieber" }
              else if member.salutation == "Frau" { "Liebe" }
              else { "Hallo" }

#show: letter-simple.with(
  sender: (
    name: "nebenan & unverpackt München W. eG",
    address: "Willibaldstr. 18, 80687 München",
    extra: [
      Telefon: #link("tel:08954637600")[+089 - 54 63 76 00]\
      Mitgliederverwaltung: #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de]\
    ],
  ),
  recipient: [
    #name \
    #member.street #member.house_number \
    #member.postal_code #member.city
  ],
  date: [#today],
  subject: "Auszahlung deiner Anteile",
  folding-marks: true,
)

#place(top + left, dx: -0.55cm, dy: -0.5cm,
       image("nebenan-unverpackt-logo.svg", width: 5cm))

#line(length: 16.5cm, stroke: 0.5pt + gray)

#table(
  columns: (1fr, 1fr),
  stroke: none,
  [*Mitgliedsnummer:*], [#member.member_number],
  [*Anteile zur Auszahlung:*], [#repayment.share_count],
  [*Auszahlungsbetrag:*], [#repayment.payout_amount €],
)

#line(length: 16.5cm, stroke: 0.5pt + gray)
#v(1cm)

#anrede #name,

deine Anteile aus dem Geschäftsjahr #repayment.fiscal_year werden in Kürze ausgezahlt.

#if member.bank_account != none [
  Wir überweisen den Betrag in Höhe von #repayment.payout_amount € auf deine
  hinterlegte IBAN: *#member.bank_account*.
] else [
  *Wir haben keine IBAN von dir hinterlegt* — bitte teile sie uns unter
  #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de] mit,
  damit wir dir den Betrag in Höhe von #repayment.payout_amount € überweisen können.
]

#v(1cm)

Herzliche Grüße,

Carolin Weidmann, Dina Beier und Simon Goller
```

### Pattern 6: Bundle-PDF via Typst-Loop (Empfehlung)

**What:** Statt N Compiles + PDF-Merge: ein Bundle-Template, das über ein `members`-Array iteriert und `#pagebreak()` zwischen den Einträgen setzt.
**When to use:** Bundle-Render bei Phase 13.
**Example:**
```typst
// templates/defaults/auszahlungs_anschreiben_bundle.typ (★ optional)
// ODER: Service ruft auszahlungs_anschreiben.typ N-mal mit unterschiedlichen
// sys.inputs auf und konkateniert die PDFs. EMPFEHLUNG: Bundle-Template.
#import "@preview/letter-pro:3.0.0": letter-simple
#let recipients = json.decode(sys.inputs.at("recipients"))

#for (i, recipient) in recipients.enumerate() [
  // Render genau wie auszahlungs_anschreiben.typ, parametrisiert über recipient.*
  // ...
  #if i < recipients.len() - 1 [ #pagebreak() ]
]
```

**Discretion-Alternative (Planner-Wahl):** PDF-merge mit `lopdf` o.ä. — würde aber neues Dependency einführen (`lopdf` ist NICHT in Cargo.lock heute, [VERIFIED via Cargo.toml-Inspektion]). Empfehlung Typst-Loop.

### Pattern 7: Frontend Browser-Save via Blob + createObjectURL (Phase 6/11 Pattern)

**What:** `fetch` → `.blob()` → `URL.createObjectURL` → `<a download={...} href={blob_url}>.click()`.
**When to use:** Binary-Response (PDF) auslösen.
**Example:** [VERIFIED: `genossi-frontend/src/api.rs:506-548` — `render_template_pdf`]
```rust
pub async fn generate_repayment_letters(
    config: &Config,
    phase_id: Uuid,
    entry_ids: Vec<Uuid>,
) -> Result<String, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!(
        "{}/api/repayment-phase/{}/letters/generate",
        config.backend, phase_id
    );
    let body = serde_json::json!({ "entry_ids": entry_ids }).to_string();

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body));
    let headers = web_sys::Headers::new().unwrap();
    headers.set("Content-Type", "application/json").unwrap();
    opts.set_headers(&headers);

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window().ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value.dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() { return Err(map_web_response_error(&resp).await); }

    let blob = JsFuture::from(resp.blob().unwrap()).await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    let blob: web_sys::Blob = blob.dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    Ok(blob_url) // Caller wraps in <a href={blob_url} download="...">.click()
}
```

### Anti-Patterns to Avoid

- **Inline-Aggregation im Service-Body statt Resolver-Call:** Würde Phase-10-D-04-Logik ein drittes Mal duplizieren. D-13-10 fordert Resolver-Abstraktion.
- **`<form onsubmit>` für den Bulk-Button:** Phase-12 D-01/D-02 verbietet das verbindlich (Page-Reload-Bug). MUSS `r#type: "button"` + `onclick`.
- **Hard-Delete oder direkter DAO-Call ohne `audited_create!`:** CLAUDE.md §Audit Log System — MemberDocument ist auditiert; jede Erzeugung läuft durch das Macro.
- **Status-Cascade `Open → Contacted` im Backend:** D-13-09 verbietet das explizit. Symmetrie zu Phase 10.
- **Render-Calls innerhalb einer offenen Transaction halten:** Phase-11-Pitfall #8 ([VERIFIED: `repayment_export.rs:226`]) — `pdf_generator.render_*` ist sync; nach Commit der DB-Reads kann gerendert werden, dann Persist in neuer Tx oder vor Render.
- **NULL-IBAN als 4xx behandeln:** Brief muss auch ohne IBAN rendern (CONTEXT D-13-06 Punkt 3: NULL-Switch).
- **MailTemplate-Pfad benutzen:** Der Brief ist ein Typst-Template, NICHT ein minijinja-MailTemplate. Kein `MailTemplateDao` involviert.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Briefe mit Falzmarken/Adressfenster | Eigenes Typst-Layout from scratch | `@preview/letter-pro:3.0.0` `letter-simple` | Status quo aller Briefe (`zahlungsanfrage.typ`, `testbrief.typ`). DIN-konform, Sender/Recipient/Subject/Folding-Marks abgedeckt. |
| Path-Traversal-Schutz für relative_path | Eigene `..` -Filter | `FilesystemDocumentStorage::full_path` [VERIFIED: `document_storage.rs:25-55`] | Bestehender, getesteter Schutz mit `path-clean`. |
| Audit-Hash-Chain | Manuell SHA256 verknüpfen | `audited_create!` Macro | Genossi-Audit-Pflicht. Phase-9-/-10-/-11-Vorbilder. |
| Default-Template-Provisioning | Manueller Filesystem-Write beim Startup | `DEFAULT_TEMPLATES` in `template_storage.rs:10` mit `include_bytes!` | Pattern aus Phase 6/11; einmal definieren, `provision_defaults()` ruft auf Startup. |
| PDF-Merge für Bundle | `lopdf`/`pdf-rs` als neues Crate | Typst-Loop mit `#pagebreak()` in einem Bundle-Template (Empfehlung) | Kein neues Dependency; Typst kann nativ über JSON-Array iterieren. |
| Permission-Funnel-Boilerplate | Eigene Helper im REST-Layer | Service-interner `check_admin_and_*` mit Funnel-Order (load → perm → status) | Phase 11 D-11/Pitfall #2 — vermeidet Status-Leak an non-admins. |
| Direct-Download Content-Disposition | Eigene String-Konkatenation | `http_util::content_disposition_attachment(&filename)` [VERIFIED: `genossi_rest/src/http_util.rs`] | Sicher gegen Filename-Injection. |
| Multi-Select-State | Eigenes Signal-Set | `selected_ids: Signal<Vec<Uuid>>` aus `repayment_entry_list.rs` (Phase 12) | Bereits implementiert, Multi-Select-Pattern getestet. |
| Browser-Save für Binary-Response | Eigenes `<a download>`-Konstrukt | `render_template_pdf`-Pattern (`api.rs:506-548`) | 1:1 bewährtes Blob+createObjectURL-Vorbild. |

**Key insight:** Alle Bausteine existieren bereits. Phase 13 ist Re-Komposition mit zwei neuen Stellen: (a) `RepaymentContextResolver` extrahiert eine Phase-10-Inline-Logik, (b) Bundle-Render via Typst-Loop ist ein neues kleines Sub-Pattern. Alles andere ist Pattern-Anwendung.

## Common Pitfalls

### Pitfall 1: SQLx Compile-Time-Query-Verification benötigt `DATABASE_URL` UND Datenbank vorhanden

**What goes wrong:** Wenn ein neuer SQLx-Query ohne `cargo sqlx prepare` committed wird, schlägt CI fehl, weil `DATABASE_URL` unter Workspace-Pflicht steht.
**Why it happens:** SQLx 0.8 nutzt Macros, die zur Compile-Zeit das DB-Schema prüfen.
**How to avoid:** Phase 13 sollte KEINE neuen SQL-Queries hinzufügen — `MemberDocument`-Schema wurde in Phase 10 erweitert (`template_id`, `mail_recipient_id`, `status`) und genügt für RepaymentLetter (alle drei = NULL). `RepaymentEntryDao::find_by_phase_id` und `MemberDao::find_by_id` existieren. KEIN neues DAO-Method nötig.
**Warning signs:** Wenn der Planner einen neuen `find_*`-Method erfindet, ist das ein Red Flag. Erst In-Memory-Filter probieren (Phase-11-Vorbild).

### Pitfall 2: Render-Call innerhalb offener Transaction → Sync-in-Async-Block

**What goes wrong:** `pdf_generator.render_*` ist synchron (Typst ist nicht-async). Wenn man es innerhalb eines noch offenen `tx` aufruft, blockiert das den Tokio-Thread und hält die Sqlx-Connection unnötig lange.
**Why it happens:** Naiver Code-Fluss schreibt "audited_create → render → audited_create → ...".
**How to avoid:** Phase-11-Pattern ([VERIFIED: `repayment_export.rs:226-252`]): Reads VOR Render, Commit DANN Render. Phase 13 hat eine zusätzliche Komplikation: nach Render muss noch `audited_create!` + `document_storage.save` laufen. Empfehlung: **Lese-Tx (Phase, Entries, Members) → Commit → Render N + Save N Files in-memory → neue Schreibe-Tx mit N `audited_create!` → Commit → Bundle-Render → Return**. Macht den Vorgang nicht atomar über alle Schritte, aber atomar über alle DB-Schreibvorgänge.
**Warning signs:** `audited_create!` und `render_*` im selben `tx.clone()`-Kontext.

### Pitfall 3: Multi-Entry-Validation — entry_phase_mismatch und unknown entry_ids

**What goes wrong:** Body kann `entry_ids` enthalten, die (a) zu einer anderen Phase gehören, (b) gar nicht existieren, (c) soft-deleted sind.
**Why it happens:** Frontend könnte stale State haben oder bösartig manipuliert werden.
**How to avoid:** Server validiert BEIDES: `entries = find_by_phase_id(phase_id)` (filtert soft-deleted via Default-Impl, [VERIFIED: `repayment_entry.rs:115-122`]), dann `requested_set.is_subset_of(found_set)`. Bei Mismatch → 400 `BadRequest("entry_phase_mismatch")`. KEIN Silent-Skip — das wäre ein subtiler Bug, wo der Bundle-PDF leiser kürzer ist als erwartet.
**Warning signs:** Planner-Empfehlung, fehlende IDs zu ignorieren.

### Pitfall 4: Audit-Hashchain — Reihenfolge der `audited_create!`-Calls

**What goes wrong:** Wenn N `audited_create!`-Calls innerhalb derselben Tx laufen, müssen sie chronologisch in der Hashchain liegen — sonst bricht `GET /api/audit/verify`.
**Why it happens:** Die Macro liest `latest_hash` aus DB → schreibt eigene Hashes mit `prev_hash`-Referenz. Bei parallelem Lauf gäbe es Race-Conditions, aber innerhalb eines `tx.clone()`-Pfads serialisiert SQLite.
**How to avoid:** Sequential await — N Calls in einer Schleife. Kein `tokio::join!`. [VERIFIED: `audit_macros.rs:5-36` — sequential await ist Standard]. E2E-Test SC: `GET /api/audit/verify` muss nach Bulk-Lauf valide bleiben.
**Warning signs:** Planner-Vorschlag, `futures::join_all` o.ä. zu nutzen.

### Pitfall 5: NULL-IBAN-Rendering — Typst muss `none` korrekt handhaben

**What goes wrong:** Wenn `member.bank_account: Option<Arc<str>>` als `None` in JSON serialisiert → `null` → Typst liest als `none`. Wenn das Template `#member.bank_account` direkt referenziert (ohne `#if`), bricht Typst mit Compile-Error.
**Why it happens:** Brief-Body MUSS für die NULL-Hinweis-Variante einen anderen Text rendern (CONTEXT D-13-06 Punkt 3).
**How to avoid:** Template hat Typst-`#if member.bank_account != none` -Switch. Beispiel siehe Pattern 5. E2E-Test "IBAN-NULL" deckt das ab (CONTEXT.md).
**Warning signs:** Default-Template ohne `#if`-Guard.

### Pitfall 6: Soft-Delete-Filter vergessen bei Member-Read

**What goes wrong:** Wenn ein Member soft-deleted (`deleted IS NOT NULL`) und trotzdem im Brief erscheint, leakt das gelöschte Daten.
**Why it happens:** `MemberDao::find_by_id` default-impl filtert soft-deleted, aber wenn der Resolver doch `dump_all` o.ä. verwendet, könnte das verloren gehen.
**How to avoid:** Nur `MemberDao::find_by_id` und `RepaymentEntryDao::find_by_phase_id` benutzen — beide filtern soft-deleted via Default-Impl. Defense-in-Depth: zusätzlicher `entry.deleted.is_none()`-Skip (Phase-11-Pattern, `repayment_export.rs:209-213`).
**Warning signs:** Direkter `dump_all`-Call ohne `.filter(|e| e.deleted.is_none())`.

### Pitfall 7: Frontend-Button-Pattern (Phase 12 D-01/D-02 Grep-Gate)

**What goes wrong:** Bulk-Button mit Default `<button>` triggert Form-Submit → Page-Reload trotz `prevent_default`. Hotfix-Backlog mit 16 GV-Buttons.
**Why it happens:** HTML-Default ist `type="submit"`. Dioxus-spawn-Async-Handler triggert verlässlich Reload, wenn das nicht überschrieben ist.
**How to avoid:** Jeder neue Button in `repayment_entry_list.rs` MUSS `r#type: "button"` + `onclick`. Grep-Gate-Test in Plan-Acceptance: `rg 'button\s*\{' genossi-frontend/src/component/repayment_entry_list.rs genossi-frontend/src/page/repayment_phase_details.rs` darf KEINEN Treffer ohne `r#type:` haben (Phase 12 D-02 Pattern).
**Warning signs:** `<button>` ohne explizit gesetzten `type`.

### Pitfall 8: Bundle-PDF wird persistiert → contradiction mit D-13-01

**What goes wrong:** Naiver Plan persistiert das Bundle-PDF zusätzlich. CONTEXT.md sagt explizit: nicht persistieren.
**Why it happens:** Es ist verlockend, das Bundle als "Vorstand-Convenience" auch im document_storage zu archivieren.
**How to avoid:** D-13-01 wörtlich befolgen — Bundle ist transient. Re-Download via erneuten Bulk-Call. (Falls Vorstand das Bundle archivieren möchte, wäre das ein deferred Phase-14+-Feature, siehe `<deferred>` in CONTEXT.md.)
**Warning signs:** Plan-Aufgabe, die `audited_create!` für das Bundle-PDF aufruft.

### Pitfall 9: Resolver-Design — Mock-Strategie für Tests

**What goes wrong:** Wenn der Resolver als Free-Function gebaut wird, kann er in Unit-Tests nicht gemockt werden — alle Letter-Service-Tests müssen mit echten Mock-DAOs für Phase, Entries laufen. Phase 11 hat das gleiche Problem ([VERIFIED: `repayment_export.rs:567-572` — Test-Mocks für 5 DAOs]) und ist OK damit, weil pure-Filter-Function direkt testbar.
**Why it happens:** Discretion-Punkt — Trait erlaubt Mock, Function ist einfacher.
**How to avoid:** Empfehlung: Trait `RepaymentContextResolver` (siehe CONTEXT.md Discretion). Mock via `mockall` analog `MockTestPermissionService`-Pattern. Direkter Test der Filter-Logik in `filter_relevant_entries`-pure-Function (Phase-11-Vorbild `filter_and_enrich_rows`).
**Warning signs:** Free-Function ohne klare Test-Strategie.

### Pitfall 10: Multi-Member-Ordering im Bundle-PDF

**What goes wrong:** Bundle hat 8 Briefe, aber die Reihenfolge wechselt unvorhersehbar (`HashMap<Uuid, ...>`-Iteration ist nicht deterministisch).
**Why it happens:** Grouping per `member_id` über `HashMap` ist nicht order-preserving.
**How to avoid:** Nach Group-by-Member explizit sortieren — `member_number ASC` (konsistent mit Phase 11 D-09 PDF-Sort). Acceptance-Test prüft Reihenfolge.
**Warning signs:** Iteration über `HashMap`-Result ohne anschließendes `.sort_by_key`.

## Code Examples

### REST-Handler-Skelett (Phase 13)

```rust
// genossi_rest/src/repayment_letter.rs
use axum::{body::Body, extract::{Path, State}, response::Response, routing::post,
           Extension, Json, Router};
use serde::Deserialize;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use genossi_service::repayment_letter::{
    RepaymentLetterService, RepaymentLetterBundle,
};
use genossi_service::ServiceError;
use crate::{error_handler, extract_auth_context, http_util, Context, RestError, RestStateDef};

fn map_letter_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".into()),
        other => other.into(),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateLettersRequest {
    pub entry_ids: Vec<Uuid>,
}

pub trait RepaymentLetterRestState: Clone + Send + Sync + 'static {
    type RepaymentLetterService: RepaymentLetterService<Context = crate::ContextType>
        + Send + Sync + 'static;
    fn repayment_letter_service(&self) -> std::sync::Arc<Self::RepaymentLetterService>;
}

#[utoipa::path(
    post,
    path = "/api/repayment-phase/{phase_id}/letters/generate",
    params(("phase_id" = Uuid, Path, description = "RepaymentPhase UUID")),
    request_body = GenerateLettersRequest,
    responses(
        (status = 200, description = "Bundle-PDF aller Anschreiben", content_type = "application/pdf"),
        (status = 400, description = "entry_phase_mismatch oder leere/unbekannte entry_ids"),
        (status = 401, description = "Session ungültig"),
        (status = 403, description = "Auth gültig, aber kein Vorstand"),
        (status = 404, description = "RepaymentPhase nicht gefunden"),
        (status = 409, description = "Phase im Preparation-Status — phase_not_active"),
    ),
    tag = "RepaymentLetter"
)]
#[instrument(skip(rest_state))]
pub async fn generate_letters<RestState: RestStateDef + RepaymentLetterRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(phase_id): Path<Uuid>,
    Json(body): Json<GenerateLettersRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = extract_auth_context(Some(context))?;
            if body.entry_ids.is_empty() {
                return Err(RestError::BadRequest("entry_ids must not be empty".into()));
            }
            let result: RepaymentLetterBundle = rest_state
                .repayment_letter_service()
                .generate(phase_id, body.entry_ids, auth)
                .await
                .map_err(map_letter_error)?;
            let cd = http_util::content_disposition_attachment(&result.filename);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/pdf")
                .header("Content-Disposition", &cd)
                .body(Body::from(result.bundle_bytes))
                .unwrap())
        })
        .await,
    )
}

pub fn generate_letter_route<RestState: RestStateDef + RepaymentLetterRestState>(
) -> Router<RestState> {
    Router::new().route(
        "/{phase_id}/letters/generate",
        post(generate_letters::<RestState>),
    )
}

#[derive(OpenApi)]
#[openapi(paths(generate_letters), components(schemas(GenerateLettersRequest)),
    tags((name = "RepaymentLetter",
          description = "Phase 13: Bulk-PDF-Anschreiben für Nicht-Email-Mitglieder. Vorstand-only.")))]
pub struct ApiDoc;
```

### MemberDocument-Persist mit `audited_create!`

```rust
// genossi_service_impl/src/repayment_letter.rs (Sketch)
let doc_id = self.uuid_service.new_v4().await;
let relative_path = format!("{}.pdf", doc_id); // analog member_document.rs:117
let file_name = format!(
    "auszahlungs_anschreiben_{}_GJ_{}.pdf",
    member.member_number, phase.fiscal_year
);
let description = format!("Anschreiben Auszahlung GJ {}", phase.fiscal_year);

// File schreiben BEVOR audited_create — falls Storage fehlschlägt,
// gibt es kein verwaistes MemberDocument.
self.document_storage
    .save(&relative_path, &pdf_bytes)
    .await
    .map_err(|e| ServiceError::InternalError(Arc::from(format!(
        "document_storage save failed: {}", e
    ))))?;

let now = time::OffsetDateTime::now_utc();
let doc = MemberDocument {
    id: doc_id,
    member_id: member.id,
    document_type: DocumentType::RepaymentLetter,
    description: Some(Arc::from(description.as_str())),
    file_name: Arc::from(file_name.as_str()),
    mime_type: Arc::from("application/pdf"),
    relative_path: Arc::from(relative_path.as_str()),
    created: time::PrimitiveDateTime::new(now.date(), now.time()),
    deleted: None,
    version: self.uuid_service.new_v4().await,
    template_id: None,
    mail_recipient_id: None,
    status: None,
};
let doc_entity: genossi_dao::member_document::MemberDocumentEntity = (&doc).into();
crate::audited_create!(
    self,
    self.member_document_dao,
    &doc_entity,
    REPAYMENT_LETTER_PROCESS, // const: "repayment-letter-service"
    &user_id,
    tx
);
```

### Frontend: Bulk-Button + Browser-Save

```rust
// genossi-frontend/src/component/repayment_entry_list.rs (additive Erweiterung)
// Im Header-Action-Bereich, neben "Massenmail":
button {
    r#type: "button",  // ★ Pflicht (Phase 12 D-01)
    class: if selected_count == 0 {
        "bg-gray-200 text-gray-500 px-3 py-2 rounded text-sm cursor-not-allowed min-h-[44px]"
    } else {
        "bg-purple-600 hover:bg-purple-700 text-white px-3 py-2 rounded text-sm min-h-[44px]"
    },
    disabled: selected_count == 0,
    onclick: move |_| {
        let ids = selected_ids.read().clone();
        on_letter_request.call(ids);  // bubbelt zur Page
    },
    "{i18n.t(Key::RepaymentEntryBulkLetterButton)} ({selected_count})"
}
```

```rust
// genossi-frontend/src/page/repayment_phase_details.rs (Handler in der Detail-Page)
on_letter_request: move |entry_ids: Vec<Uuid>| {
    spawn(async move {
        let config = CONFIG.read().clone();
        match api::generate_repayment_letters(&config, phase_id, entry_ids.clone()).await {
            Ok(blob_url) => {
                // Browser-Save via <a download>-Trick
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        let anchor = document
                            .create_element("a").unwrap()
                            .dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
                        anchor.set_href(&blob_url);
                        anchor.set_download(&format!(
                            "auszahlungs_anschreiben_GJ_{}.pdf",
                            phase.fiscal_year
                        ));
                        anchor.click();
                        // Cleanup
                        let _ = web_sys::Url::revoke_object_url(&blob_url);
                    }
                }
                let n = entry_ids.len(); // Achtung: Members-Aggregation in echtem N
                show_toast(&mut toast_messages, &mut toast_counter,
                    format!("{} Briefe erzeugt. Vergiss nicht, die Einträge anschließend als angeschrieben zu markieren.", n));
            }
            Err(e) => show_toast(&mut toast_messages, &mut toast_counter, e.message),
        }
    });
}
```

### `DocumentType::RepaymentLetter`-Erweiterung

```rust
// genossi_service/src/member_document.rs — additive zu enum
pub enum DocumentType {
    JoinDeclaration,
    JoinConfirmation,
    ShareIncrease,
    Other,
    RepaymentMail,
    // ★ Phase 13:
    RepaymentLetter,
}

impl DocumentType {
    pub fn as_str(&self) -> &str {
        match self {
            // ... existing arms ...
            DocumentType::RepaymentLetter => "repayment_letter",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // ... existing arms ...
            "repayment_letter" => Some(DocumentType::RepaymentLetter),
            _ => None,
        }
    }
    pub fn is_singleton(&self) -> bool {
        matches!(self, DocumentType::JoinDeclaration | DocumentType::JoinConfirmation)
        // ★ RepaymentLetter NICHT singleton (D-13-08)
    }
    pub fn template_path(&self) -> Option<&str> {
        match self {
            DocumentType::JoinConfirmation => Some("join_confirmation.typ"),
            DocumentType::JoinDeclaration => Some("join_declaration.typ"),
            // ★ RepaymentLetter = None (Template läuft über eigenen Pfad,
            // nicht über das DocumentType-Mapping)
            _ => None,
        }
    }
}
```

### `DEFAULT_TEMPLATES`-Erweiterung

```rust
// genossi_service_impl/src/template_storage.rs:10 — neuer Eintrag
const DEFAULT_TEMPLATES: &[DefaultTemplate] = &[
    DefaultTemplate { path: "_layout.typ",
        content: include_bytes!("../../templates/defaults/_layout.typ") },
    DefaultTemplate { path: "join_confirmation.typ",
        content: include_bytes!("../../templates/defaults/join_confirmation.typ") },
    DefaultTemplate { path: "teilnehmerliste.typ",
        content: include_bytes!("../../templates/defaults/teilnehmerliste.typ") },
    DefaultTemplate { path: "auszahlungsliste.typ",
        content: include_bytes!("../../templates/defaults/auszahlungsliste.typ") },
    // ★ Phase 13:
    DefaultTemplate { path: "auszahlungs_anschreiben.typ",
        content: include_bytes!("../../templates/defaults/auszahlungs_anschreiben.typ") },
];
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Vorstand erzeugt Anschreiben händisch außerhalb Genossi (REQUIREMENTS.md BRIEF-01 alt) | Bulk-Endpoint mit Multi-Entry-Aggregation + Audit-Spur | Phase 13 | BRIEF-01 deferred wird aufgehoben; v1.2-Stand. |
| Mail-Worker macht Inline-Aggregation (Phase 10) | Shared `RepaymentContextResolver` (Phase 13) | Schritt 1: Resolver-Bau (Phase 13); Schritt 2: Worker-Refactor (Folge-Quick) | DRY-Aggregation; Single Source of Truth für Filter+Format. |

**Deprecated/outdated:** — keine.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Typst-Loop mit `#pagebreak()` ist performance-mäßig für N=20 Bundle-Render genauso gut wie 1 PDF-Compile | Pattern 6, Standard Stack | Wenn die Bundle-Compile-Zeit >5s wird, müsste der Plan auf N+1 separate Compiles + Concat ausweichen. Empfehlung: messen mit N=20 in einem Plan-Spike. |
| A2 | `document_storage` Files dürfen bei Service-Layer-Aufruf geschrieben werden (statt im REST-Layer wie Phase 10) | Architectural Responsibility Map, Pattern 1 | Wenn ein Bestandsfehler im Storage auftritt nach `audited_create!`, gibt es Datenleichen im FS. Mitigation: File-Schreibe VOR `audited_create!`, dann ist die Audit-Eintragung das atomare Commit-Marker. |
| A3 | Frontend `<a download>.click()`-Pattern funktioniert in Genossi-Browsern (Firefox/Chrome Desktop) wie in Phase 6/11 | Pattern 7, Code Examples | Falls Browser-Edge-Case (z.B. Safari, mobile) Probleme macht, könnte Direkt-Navigation via `window.location.href = blob_url` Fallback sein. Phase 12/UAT hat das Pattern aber laufen. |
| A4 | Phase-10-Worker bleibt zunächst stabil bei Inline-Aggregation; Refactor erfolgt als separater Quick ohne Behavior-Drift | D-13-10, Pattern 3 | Wenn der Worker später gerefactort wird und sich der Filter-Set (Open + Contacted) versehentlich ändert, drift Brief vs. Mail. Mitigation: Resolver-Tests müssen die Phase-10-D-04-Semantik 1:1 spiegeln. |

**No empty table:** Vier kalibrierte Annahmen, die Planner/Verifier prüfen sollten.

## Open Questions

1. **Bundle-PDF: separates Template oder Service-side multi-page Render?**
   - What we know: Typst kann via `#for ... #pagebreak()` mehrere Briefe in einem Compile rendern. Das setzt aber ein Bundle-Template voraus, das die Single-Letter-Logik dupliziert ODER inkludiert.
   - What's unclear: Ob das Bundle-Template `auszahlungs_anschreiben.typ` als `#import` nutzt oder die Logik komplett selbst trägt.
   - Recommendation: Bundle-Template `auszahlungs_anschreiben_bundle.typ` (ggf. ohne `DEFAULT_TEMPLATES`-Registrierung — könnte auch nur intern existieren), das das Single-Letter-Template per `#import` einbindet und über `recipients`-Array iteriert. Plan-Discretion.

2. **Wo lebt der Resolver: Service-Implementation oder eigenes Mini-Crate?**
   - What we know: Trait in `genossi_service/`, Impl in `genossi_service_impl/` ist Genossi-Standard.
   - What's unclear: Ob der Resolver groß genug ist für eine eigene Datei `genossi_service/src/repayment_context.rs` + `genossi_service_impl/src/repayment_context.rs` ODER nur in einer Datei in `genossi_service_impl/` lebt.
   - Recommendation: Trait in `genossi_service/src/repayment_context.rs`, Impl in `genossi_service_impl/src/repayment_context.rs`. Pattern-konsistent mit allen anderen Services. Plan-Discretion.

3. **Bundle-Filename: `phase_id`-Suffix oder Datum?**
   - What we know: CONTEXT.md gibt Discretion.
   - What's unclear: Bei zwei Bundle-Calls hintereinander (z.B. Vorstand hat erst die Hälfte ausgewählt, dann die anderen) — ohne Suffix überschreibt der Browser den ersten Download.
   - Recommendation: `auszahlungs_anschreiben_GJ_{fiscal_year}_{YYYYMMDD_HHMMSS}.pdf` — Datum als deterministisches Tiebreaker. Plan-Discretion.

4. **Member-Reads: N+1 Queries vs. Batch-Loader?**
   - What we know: Phase 11 nutzt N+1 ([VERIFIED: `repayment_export.rs:208-224`]).
   - What's unclear: Bei N=20 Briefen ist N+1 trivial, bei N=200 nicht mehr.
   - Recommendation: N+1 wie Phase 11. Falls Performance-Problem, später optimieren. Plan-Discretion.

5. **i18n-Keys für neuen Bulk-Button und Toasts:**
   - What we know: Phase 12 D-19 pflegt de/en in `i18n/{de,en}.rs`.
   - What's unclear: Genaue Texte für `Key::RepaymentEntryBulkLetterButton`, `Key::RepaymentLettersGeneratedToast`, `Key::RepaymentLettersErrorToast`.
   - Recommendation: Planner finalisiert exakten Wortlaut; muss in beiden Locales gepflegt sein.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust Toolchain | Workspace-Build | ✓ (Cargo + Nix) | edition 2021 | — |
| SQLite | Database | ✓ | n/a | — |
| Typst 0.14 | PDF-Render | ✓ (Workspace-Dep) | 0.14 | — |
| `@preview/letter-pro` 3.0.0 | Brief-Layout | ✓ (lokal in `typst-packages/preview/letter-pro/3.0.0/`) | 3.0.0 | Re-Download via `PackageCache` falls Cache fehlt |
| `nebenan-unverpackt-logo.svg` | Header im Brief | ✓ (`templates/nebenan-unverpackt-logo.svg`) | n/a | Template kann ohne Logo rendern |
| Dioxus 0.6.3 + `dx` CLI | Frontend-Build | ✓ (Nix flake) | 0.6.3 | — |
| Tailwind CSS | Frontend-Styling | ✓ | n/a | — |
| `mockall` | Service-Tests | ✓ (workspace-dep) | 0.13 | — |

**Missing dependencies with no fallback:** Keine.

**Missing dependencies with fallback:** Keine.

## Validation Architecture

> `workflow.nyquist_validation: false` [VERIFIED: `.planning/config.json:18`] — diese Section ist optional. Kurzform mit Fokus auf die in CONTEXT.md gelisteten E2E-Tests.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in) + `tokio::test` + `reqwest` für HTTP-Calls |
| Test-File | `genossi_bin/tests/e2e_tests.rs` (existiert, erweitert) |
| Quick run command | `cargo test --test e2e_tests test_repayment_letter -- --nocapture` |
| Full suite command | `cargo test --workspace` |
| Unit-Test Mocks | `mockall::mock!` Pattern (existing in `repayment_export.rs:322-572`) |

### Phase 13 Test-Liste (aus CONTEXT.md)

| Test | Behavior | Beispiel-Assertion |
|------|----------|--------------------|
| Happy Path 3-Entries-2-Member | 3 entry_ids für 2 Member → 2 MemberDocuments + 1 Bundle-PDF | `bytes.starts_with(b"%PDF-")`, MemberDocument-Count = 2 |
| Multi-Entry-Aggregation | 2 entry_ids für 1 Member → 1 MemberDocument mit Summe | aggregierte share_count = SUM gerechnet |
| Permission-Denied (Helper-Auth) | → 403 | `status == 403 Forbidden` |
| Status-Gate (Phase Preparation) | → 409 phase_not_active | `status == 409`, Body enthält `"phase_not_active"` |
| entry_phase_mismatch | entry_ids fremder Phase → 400 | `status == 400 BadRequest` |
| IBAN-NULL | Member ohne `bank_account` → PDF rendert | `bytes.starts_with(b"%PDF-")`, Audit-Eintrag erzeugt |
| Audit-Hashchain valide | `GET /api/audit/verify` nach Bulk-Run | `status == 200 OK` (Pattern aus `e2e_tests.rs:7517,7543`) |
| Idempotenz | 2x derselbe Call → 2 MemberDocuments | Count vorher+2 = Count nachher |

### Unit-Test-Slots

- `RepaymentContextResolver::resolve` direkt mit Mock-DAOs: Filter Open+Contacted, SUM-Aggregation, Euro-Format `"X,YZ"`, fiscal_year-Mapping.
- Pure-Function-Tests in `repayment_letter.rs` analog `filter_and_enrich_rows` (`repayment_export.rs:118-171`): Group-by-Member, Sort-by-member_number.
- Grep-Gate-Test für Frontend-Button-Pattern (analog Phase 11 EXPO-05): `rg "button\s*\{" genossi-frontend/src/component/repayment_entry_list.rs` ohne `r#type:`.

## Security Domain

> `security_enforcement` ist im Config nicht explizit gesetzt — behandeln als enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Bestehender Session/OIDC-Flow via `extract_auth_context`; admin-only Funnel. |
| V3 Session Management | yes | tower-sessions, Cookies, Permission-Service. Unverändert. |
| V4 Access Control | yes | `PermissionService::check_permission("admin", ...)` (D-13). Helper-Auth → 403. |
| V5 Input Validation | yes | `entry_ids` validation: nicht-leer, alle ∈ Phase. JSON-Body via serde. |
| V6 Cryptography | partial | Audit-Hash-Chain via `sha2` (existing). Brief-PDFs sind nicht signiert (genauso wie Phase 11). |

### Known Threat Patterns für Phase 13 Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path-Traversal in `relative_path` | Tampering | `FilesystemDocumentStorage::full_path` mit `path-clean` [VERIFIED: `document_storage.rs:25-55`]. Empfehlung: `relative_path = format!("{}.pdf", uuid)` — UUID enthält keine `..`. |
| Status-Information-Leak an non-admin (Funnel-Order) | Information Disclosure | Funnel-Order load → admin → status; bei admin-fail nicht status-check ausführen. Phase-11-Pitfall #2. |
| Content-Disposition Filename-Injection | Tampering | `http_util::content_disposition_attachment` [VERIFIED: bestehender Helper]. |
| entry_phase_mismatch als Probe für andere Phasen | Information Disclosure | 400 BadRequest pauschal, kein Echo "X gehört zu Phase Y". |
| Bundle-PDF im Browser-Cache verbleibt | Information Disclosure | `URL.revoke_object_url(blob_url)` nach Click (Frontend). |
| SQL-Injection | Tampering | SQLx-Compile-Time-Parameter-Binding (existing). Phase 13 schreibt keine Raw-SQL. |
| Hashchain-Bruch durch parallele Audit-Schreiben | Integrity | Sequential `audited_create!` in einer Tx (existing). E2E-Test `audit/verify`. |

## Sources

### Primary (HIGH confidence)
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-CONTEXT.md` — Decisions D-13-01..11
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/.planning/notes/repayment-letter-architecture.md` — Architektur-Notes D-LETT-01..05
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_rest/src/repayment_export.rs` — REST-Handler-Pattern Phase 11
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/repayment_export.rs` — Service-Impl-Pattern Phase 11
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/pdf_generation.rs:386-441,776-826` — `render_repayment_list` + `build_inputs_repayment` Pattern
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/template_storage.rs:10-35` — `DEFAULT_TEMPLATES` Mechanik
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/audit_macros.rs:5-36` — `audited_create!` Pattern
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_mail/src/worker.rs:332-360` — Phase-10-Inline-Aggregation (Vorbild für Resolver)
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_service/src/member_document.rs:48-101` — `DocumentType`-Enum
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_service/src/document_storage.rs` + `genossi_service_impl/src/document_storage.rs` — Storage-Trait + Impl
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/templates/zahlungsanfrage.typ` — Layout-Vorbild für letter-simple + Falzmarken
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/templates/defaults/auszahlungsliste.typ` — sys.inputs-JSON-Kontext-Pattern
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/typst-packages/preview/letter-pro/3.0.0/README.md` — Letter-Pro 3.0.0 Pattern + Doku
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/api.rs:506-548` — `render_template_pdf` Browser-Save-Pattern
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/repayment_entry_list.rs:130-298` — Multi-Select + Header-Action-Buttons-Pattern (Phase 12)
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/repayment_phase_details.rs:121-352` — Page-Wiring + Toast + Modal-Mount
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs:291-316,862-975` — Phase-11 DI-Wiring Vorbild für Phase 13
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/genossi_bin/tests/e2e_tests.rs:13422-13460` — Phase-11 E2E-Test PDF-Download-Pattern
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/.planning/REQUIREMENTS.md:59-61,83` — BRIEF-01-Requirement
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` — Tech-Stack, Architecture, Audit-Pattern
- [VERIFIED] `/home/neosam/programming/rust/projects/genossi3/.planning/config.json` — `nyquist_validation: false`, andere Workflow-Flags

### Secondary (MEDIUM confidence)
- [CITED] `.planning/phases/10-massenmail-anbindung-template-variablen/10-CONTEXT.md` — Mail-Pipeline-Pattern, Aggregations-Filter D-04, MemberDocument-Persistenz D-07..D-11
- [CITED] `.planning/phases/11-export-pdf-csv/11-CONTEXT.md` — PDF-Export-Pattern, Permission-Funnel D-10/D-11, Pitfall #2/#8
- [CITED] `.planning/phases/12-frontend-component-first/12-CONTEXT.md` — Button-Pattern D-01/D-02, Multi-Select D-11, Massenmail D-18

### Tertiary (LOW confidence)
- Keine — alle Behauptungen sind im Repo verifiziert oder direkt aus CONTEXT.md/REQUIREMENTS.md zitiert.

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — alle Crates und Versionen sind im Workspace, lokal verifiziert (z.B. `letter-pro:3.0.0` im Package-Cache)
- Architecture: HIGH — Pattern 1:1 aus Phase 11 (REST-Funnel + Direct-Download), Phase 10 (Aggregations-Logik), Phase 6 (DEFAULT_TEMPLATES). Keine neuen Patterns, nur Komposition.
- Pitfalls: HIGH — alle gelisteten Pitfalls haben verifizierte Phase-11-/-12-Vorbilder mit Mitigation.
- Frontend-Wiring: HIGH — `render_template_pdf` als Browser-Save-Vorbild ist getestet (Phase 6/11/12 in produktivem Einsatz).
- Bundle-Render-Strategie: MEDIUM — Typst-Loop ist Recommendation, A1 als Assumption gekennzeichnet.

**Research date:** 2026-06-01
**Valid until:** 2026-07-01 (stabile Backend-Patterns, kein zeitkritisches Wissen — 30 Tage Mindesthaltbarkeit)
