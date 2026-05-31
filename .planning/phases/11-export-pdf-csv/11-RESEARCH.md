# Phase 11: Export (PDF) — Research

**Researched:** 2026-05-31
**Domain:** Rust/Axum service + Typst-PDF-Template, Phase-6-AttendanceExport ist 1:1-Vorbild
**Confidence:** HIGH (alle Code-Pfade gegen Source verifiziert; Pattern existiert produktiv seit Phase 6)

## Summary

Phase 11 ist eine **mechanische Pattern-Replikation von Phase 6** (`AttendanceExport`). Service-Trait, Permission-Funnel, Format-Whitelist, Filename-Bundle, `tracing::info!`-Logging, OpenAPI-Schema, lokales `map_export_error`, Wiring im `RestStateImpl` — alles existiert bereits funktionstüchtig in `genossi_service/src/attendance_export.rs` + `genossi_service_impl/src/attendance_export.rs` + `genossi_rest/src/attendance_export.rs`. Phase 11 ersetzt nur die Datenquelle (`AttendanceMemberRow` → `RepaymentEntry × Member × Phase`-Join), das Status-Gate (`Closed`-only → `Open OR Closed`), und das Typst-Template (`teilnehmerliste.typ` → `auszahlungsliste.typ`).

**Primary recommendation:** Diff-Strategie statt Greenfield. Phase 6 Service-Impl `genossi_service_impl/src/attendance_export.rs` (1199 LOC) öffnen, `s/Attendance/Repayment/`, Sub-Aggregation hinzufügen, XLSX+CSV-Writer entfernen, Status-Gate auf zwei Stati erweitern. Plan-Geschwindigkeit: ~40% kürzer als Phase 6 (kein XLSX, kein CSV, kein PII-Whitelist, kein Helper-Branch).

**Sekundär:** Für die Member-Read-Strategie reicht das in `genossi_dao::repayment_entry::find_by_phase_id` etablierte "Filter `dump_all()` in-memory"-Pattern — `dump_all()` ist im SQLite-Impl ein einziger SELECT, danach sind alle Reads RAM-only. Eine neue `MemberDao::find_by_ids`-Batch-Methode ist nicht nötig (siehe Q5).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| PDF-Rendering via Typst | Service-Impl (`PdfGenerator`) | — | PDF-Pipeline lebt schon in `genossi_service_impl/src/pdf_generation.rs`; nur neue `render_repayment_list`-Methode |
| Permission-Funnel (admin + status) | Service-Impl | — | Pattern aus `attendance_export.rs:100` `check_admin_and_closed` |
| Format-Whitelist (`pdf` only) | REST-Handler | — | Phase 6 D-14: `match format_str.as_str()` im Handler vor Service-Call (siehe `attendance_export.rs:135`) |
| `?include`-Filter (Open/All/Paid) | Service-Impl (in-memory) | — | Phase 6 filtert in-memory (`attendance_export.rs:167-169`); SQL-Filter unnötig komplex |
| Filename-Bundle generation | Service-Impl (im `RepaymentExport`-Bundle) | — | Phase 6 D-15: Server-generated, kein User-Input (`attendance_export.rs:234`) |
| Content-Disposition Header | REST-Handler (`http_util::content_disposition_attachment`) | — | Existierender Helper in `genossi_rest/src/http_util.rs:43`, RFC 6266 |
| Member-Daten-Read (IBAN, Name) | Service-Impl via `MemberDao::find_by_id` | — | Existierende DAO-Methode; N+1-Pattern in derselben Tx ist OK für SQLite-In-Memory |
| Typst-Template-Auflösung | Disk (`TemplateStorage::base_path()`) | `DEFAULT_TEMPLATES` registriert für Fresh-Install | `template_storage.rs:10` provisioniert beim Start |
| OpenAPI-Schema | REST (lokaler `ApiDoc` + Merger in `lib.rs:271`) | — | Phase 6 D-22: lokaler `ApiDoc`, dann in `lib.rs::ApiDoc::nests` aufnehmen |
| DI-Wiring | `genossi_bin/src/lib.rs::RestStateImpl::new()` | — | Reuse existierender Arcs (`pdf_generator`, `template_storage`, `repayment_phase_dao`, `repayment_entry_dao`, `member_dao`, `permission_service`, `transaction_dao`) |

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** `?include=open` = `RepaymentEntryStatus ∈ {Open, Contacted}` (Recommended-Default; Banking-Use-Case "noch nicht ausbezahlt")
- **D-02:** `?include=all` = `Open ∪ Contacted ∪ PaidOut`; `?include=paid` = nur `PaidOut`. Soft-Deleted (`entry.deleted IS NOT NULL` ODER `member.deleted IS NOT NULL`) wird in JEDEM Filter ausgeschlossen
- **D-03:** Default-Parameter ist `open` per `#[derive(Default)]` auf REST-lokalem `ExportIncludeQuery`-Enum; Service-Domain-`ExportInclude` hat ebenfalls `Default = Open`
- **D-04:** Verwendungszweck-Schema hardcoded: `"Anteilsrückzahlung GJ {fiscal_year} {member_number} {first_name} {last_name}"` (kein Komma/Bindestrich, Leerzeichen-getrennt)
- **D-05:** Keine SEPA-Zeichensatz-Sanitization (Sonderzeichen ä/ö/ü/ß bleiben drin)
- **D-06:** Fehlende IBAN (`Member.bank_account IS NULL`) → leere IBAN-Spalte im PDF, alle anderen Spalten gefüllt; Export blockiert nie; kein Skip, kein Visual-Highlight, kein 409
- **D-07:** Empty-String-Pattern: `member.bank_account.unwrap_or_default()` analog `attendance_export.rs:276`
- **D-08:** Eine Zeile pro `RepaymentEntry` (1:1-Mapping zur DB, keine Per-Mitglied-Aggregation)
- **D-09:** Sortierung primär `member.member_number ASC`, sekundär `entry.created ASC`
- **D-10:** Export erlaubt für `RepaymentPhaseStatus ∈ {Open, Closed}`. `Preparation` → `ServiceError::Conflict("phase_not_exportable")` → 409. Permission-Check (admin) läuft VOR Status-Check
- **D-11:** Vorstand-only via `PermissionService::check_permission("admin", ...)`; `Helper`-Auth liefert `RestError::Forbidden(403)`; lokales `map_export_error` mappt `PermissionDenied → Forbidden(403)`. **Null `audited_*!`-Calls** im Service-Impl (Grep-Gate-Test)
- **D-12:** CSV-Export (EXPO-04) komplett aus Phase 11 entfernt; Format-Whitelist im REST nur `pdf` (alles andere → 400)

### Claude's Discretion

- **Member-Read-Strategie:** N+1 via `MemberDao::find_by_id` pro Entry vs. neue `MemberDao::find_by_ids`-Batch-Methode. Empfehlung dieser Research: N+1 reicht (siehe Q5)
- **Betrag-Rendering:** Service-Pre-Computing als `"60,00"`-String (deutsche Lokalisierung) per `format!("{},{:02}", cents / 100, cents % 100)` analog Phase 10 D-04. Alternative: minijinja-Filter im Typst — wird hier NICHT empfohlen (Typst hat keine native Locale-Formatierung)
- **`format`-Path-Whitelist:** Empfehlung: gleiches Pattern wie Phase 6 D-14 (`match format_str.as_str()`)
- **Permission-Privilege-String:** `"admin"` (Konstante `ADMIN_PRIVILEGE`) — gleicher String wie Phase 6
- **E2E-Helper-Wiederverwendung:** `create_member_with_exit_date()` + `create_open_repayment_phase()` aus Phase 9/10 (siehe Q9)
- **Typst-Layout:** Repeat-Header, 6 Spalten, auto/1fr/1fr/auto/auto/1fr-Verteilung; optional Summenzeile am Ende (nicht REQ)

### Deferred Ideas (OUT OF SCOPE)

- CSV-Export (EXPO-04) → v1.2
- XLSX-Export → nie im v1.1-Scope
- Frontend-Integration (Tab + Download-Button) → Phase 12 UI-02
- SEPA pain.001 XML-Export → v2 (SEPA-01)
- Audit-Hashchain-Eintrag pro Export-Call → explizit nicht gewollt (EXPO-05)
- Per-Mitglied-Aggregation → User-Decision D-08 (eine Zeile pro Entry)
- Konfigurierbarer Verwendungszweck-Text pro Phase → User-Decision D-04 (hardcoded)
- Visual-Highlight für fehlende IBAN im PDF → User-Decision D-06
- Sekundär-Status-Spalte im PDF → Status-Sicht gehört ins Frontend
- Verwendungszweck-SEPA-Sanitization (ä→ae etc.)
- Summenzeile (Anzahl + Gesamtbetrag) → Claude's-Discretion-NTH

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EXPO-01 | PDF-Export verfügbar für `Open` UND `Closed`-Phasen | Status-Gate-Code in Q14; Funnel-Reihenfolge Q7 |
| EXPO-02 | PDF enthält 6 Spalten + Sortierung nach `member_number ASC` | Typst-Template Q2; Sort-Logik im Service Q4 |
| EXPO-03 | `?include=open\|all\|paid` Filter, Default `open` | Filter-Semantik Q4; REST-Query-Param Q13 |
| EXPO-05 | Vorstand-only, read-only, kein Audit-Hashchain-Eintrag | Grep-Gate Q10; Permission-Funnel Q7 |

## Project Constraints (from CLAUDE.md)

- **Tech stack locked:** Rust + Axum + SQLx + SQLite Backend, Dioxus WASM Frontend — keine Sprachwechsel
- **Architektur-Konformität:** Layered DAO/Service/REST muss eingehalten werden; neue Entitäten implementieren bestehende Trait-Patterns
- **Audit-Macros sind RESERVIERT für Member/MemberAction/MemberDocument/Application** — neue GV-Entitäten brauchen das nicht; Phase 11 explizit ohne Audit-Macros (EXPO-05)
- **No-Mocking-DB für E2E:** Backend-Tests gegen In-Memory-SQLite (`sqlite::memory:`), siehe `e2e_tests.rs:29`
- **Test-Pflicht:** Globale CLAUDE.md sagt "Always make sure you have tests for the changes" — Phase 11 hat 6+ E2E-Tests + Unit-Tests im Service-Impl
- **Verbandskonformität:** PDF muss für Banking-Workflow optimiert sein (klare Spalten, IBAN gut lesbar)
- **GSD-Workflow-Pflicht:** Edits nur durch `/gsd-execute-phase`
- **OIDC-Provider ist Nextcloud** (nicht WordPress; ältere Specs irrtümlich falsch)

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `axum` | 0.8.3 | HTTP-Handler, Routing, Extension/Path/Query Extractors | Genossi-Standard; alle anderen Endpoints bereits darin |
| `typst` | 0.14 | Template-Compiler für PDF | Existierende Pipeline in `genossi_service_impl/src/pdf_generation.rs`; eingebundene Liberation-Sans-Fonts |
| `typst-pdf` | 0.14 | PDF-Serialisierung | Direktes Companion zu typst; einheitlich mit allen Genossi-PDFs |
| `serde_json` | 1.0 | `sys.inputs`-Payload-Encoding | Phase-6-Pattern: JSON-String-Inputs in `Dict`; `json.decode()` im Typst-Template |
| `utoipa` | 5.0 | OpenAPI-Schema für `ExportQuery`, `ExportIncludeQuery` | Genossi-Standard für alle REST-Module |
| `tracing` | 0.1 | Strukturiertes Logging statt Audit | Phase 6 D-18: ersetzt `audited_*!` für read-only-Service |
| `async-trait` | — | Trait-Definition `RepaymentExportService` | Genossi-Service-Trait-Konvention |
| `mockall` | 0.13 | `#[automock]` für Service-Mock | Unit-Test-Standard |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `time` | 0.3 | Date-Format für Filename / Verwendungszweck | Wenn fiscal_year aus Phase + iso-date für Filename gebraucht wird |
| `uuid` | 1.6 | Path-Param + Entity-IDs | Standard |
| `genossi_rest::http_util::content_disposition_attachment` | (local) | RFC-6266-konformer Header | IMMER für Download-Endpoints, sonst Filename-Injection-Risiko |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| N+1 `MemberDao::find_by_id` pro Entry | Neue `MemberDao::find_by_ids(ids: &[Uuid])` Batch-Methode | N+1 in SQLite-In-Memory ist mikrosekundenschnell; Batch wäre architektonisch sauberer, aber DAO-Trait erweitern + alle Mocks anpassen kostet 2-3 Plans extra. Bei ~50-100 Entries pro Phase ist N+1 ohne messbare Latenz-Auswirkung |
| In-Memory-Filter für `?include` | SQL-`WHERE`-Filter in neuer DAO-Methode | Phase 6 filtert in-memory (`attendance_export.rs:167`). RepaymentEntryDao::`find_by_phase_id` liest schon ALLE Phase-Entries; In-Memory-Filter ist konsistent + DAO-Trait bleibt unverändert |
| Neue `render_repayment_list`-Methode im `PdfGenerator` | Generalisierte `render(template, base, inputs: Dict)`-Methode | Phase 6 hat `render_attendance_list` als private Sub-Routine (`pdf_generation.rs:279`); Replikation ist konsistent. Generalisierung wäre netter, aber Refactor ist out-of-scope für Phase 11 |
| Eigenes Verwendungszweck-Pre-Computing im Service | Minijinja-Filter im Typst-Template | Typst hat keine native Locale-Formatierung (kein `"60,00"`); Service-Pre-Computing ist sauberer. Phase 10 D-04 ist bereits Vorbild für Euro-Format |

**Installation:** Keine neuen Crate-Dependencies — alle Bibliotheken sind bereits in `Cargo.lock`.

**Version verification:** Cargo.lock-pinned Versionen sind die Wahrheit für dieses Repo; keine npm-Registry-Probe nötig (Rust-Workspace).

## Architecture Patterns

### System Architecture Diagram

```
Browser (Phase 12) / curl
        │
        ▼ GET /api/repayment-phase/{id}/export/{format}?include=...
        │
        ▼  cookies/auth header
[Axum Router] ── extract_auth_context() ──► auth: Authentication<Context>
        │
        ▼
[REST handler: export_repayment_list<RestState>]
        │
        ▼ format match (whitelist "pdf")
[ExportInclude conversion from ExportIncludeQuery]
        │
        ▼ rest_state.repayment_export_service().export(phase_id, fmt, inc, auth)
        │
        ▼
[RepaymentExportServiceImpl::export]
        ├──► transaction_dao.use_transaction(None)
        ├──► check_admin_and_phase_status(phase_id, ctx, tx) ──┐
        │                                                       │
        │           ┌──► RepaymentPhaseDao::find_by_id ──► 404 if None
        │           ├──► PermissionService::check_permission("admin") ──► 403
        │           └──► Phase.status ∈ {Open, Closed} ──► 409 if Preparation
        │                                                       │
        ├──► RepaymentEntryDao::find_by_phase_id(phase_id, tx)  │
        │       (returns ALL non-deleted entries for this phase)│
        │                                                       │
        ├──► For each entry: MemberDao::find_by_id              │
        │       (N+1 in same tx, OK for SQLite in-memory)       │
        │                                                       │
        ├──► transaction_dao.commit(tx)                         │
        │                                                       │
        ├──► Filter rows in-memory:                             │
        │       - entry.deleted IS NULL AND member.deleted IS NULL (always)
        │       - include=open  → status ∈ {Open, Contacted}    │
        │       - include=all   → all 3 stati                   │
        │       - include=paid  → status == PaidOut             │
        │                                                       │
        ├──► Sort: (member_number ASC, created ASC)             │
        │                                                       │
        ├──► Compute amount = entry.share_count_to_pay_out × phase.share_value
        │       Format: "{euros},{:02}"  (deutsche Lokalisierung)
        │                                                       │
        ├──► Compute Verwendungszweck per entry:                │
        │       "Anteilsrückzahlung GJ {fy} {mn} {fn} {ln}"     │
        │                                                       │
        ├──► tracing::info!(target=EXPORT_TARGET, ...)          │
        │                                                       │
        └──► pdf_generator.render_repayment_list(               │
                "auszahlungsliste.typ",                         │
                 template_base, &phase, &enriched_rows)         │
              │                                                 │
              ▼                                                 │
          [PdfGenerator]                                        │
              ├──► fs::read_to_string(template_base/auszahlungsliste.typ)
              │     → on ENOENT: ServiceError::InternalError("template not found: ...")
              ├──► Build Dict for sys.inputs:                   │
              │       "meta" → JSON {fiscal_year, share_value, ...}
              │       "rows" → JSON [{member_number, ..., amount_str, purpose_str}, ...]
              ├──► TemplateWorld::new (Library + LazyHash<FontBook>)
              ├──► typst::compile::<PagedDocument>(&world)      │
              └──► typst_pdf::pdf(&doc, &PdfOptions::default()) │
                    → Vec<u8> ─────────────────────────────────►┘
                                                                │
        ▼  RepaymentExport { bytes, content_type, filename }    │
[REST handler]                                                  │
        ├──► http_util::content_disposition_attachment(filename)│
        └──► Response 200 with                                  │
               Content-Type: "application/pdf"                  │
               Content-Disposition: "attachment; filename=...; filename*=UTF-8''..."
               body = export.bytes
```

### Recommended Project Structure

```
genossi_service/src/
└── repayment_export.rs                  # NEW — Trait + Domain-Types

genossi_service_impl/src/
├── repayment_export.rs                  # NEW — Impl + Permission Funnel + Format-Writer
├── pdf_generation.rs                    # MODIFY — Add render_repayment_list method
└── template_storage.rs                  # MODIFY — DEFAULT_TEMPLATES += auszahlungsliste.typ

genossi_rest/src/
├── repayment_export.rs                  # NEW — Handler + Query Params + Local map_export_error + ApiDoc
└── lib.rs                               # MODIFY — Mount Router + ApiDoc nest

genossi_bin/src/
└── lib.rs                               # MODIFY — RestStateImpl::new() wiring + RestStateImpl impl RepaymentExportRestState

templates/defaults/
└── auszahlungsliste.typ                 # NEW — Repeat-Header table

genossi_bin/tests/
└── e2e_tests.rs                         # MODIFY — 6+ tests, reuse Phase-9/10 helpers
```

### Pattern 1: Permission-Funnel (Admin + Phase-Status)

**What:** Private async-Methode auf `RepaymentExportServiceImpl<Deps>`, die Load+PermCheck+StatusCheck atomar (in derselben Tx) bündelt.

**When to use:** Vor jedem Export-Read; identische Reihenfolge wie Phase 6 verhindert Status-Information-Leak an non-admin.

**Example (1:1 Replikation aus `genossi_service_impl/src/attendance_export.rs:100-131`, nur Status-Set erweitert auf 2 Stati):**

```rust
// Source: genossi_service_impl/src/attendance_export.rs:100-131 (adapted)
async fn check_admin_and_phase_status(
    &self,
    phase_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<RepaymentPhaseEntity, ServiceError> {
    // 1. Load (404 if missing)
    let phase = self
        .repayment_phase_dao
        .find_by_id(phase_id, tx)
        .await?
        .ok_or(ServiceError::EntityNotFound(phase_id))?;

    // 2. Admin gate (403). Authentication::Full short-circuits (Phase-6-Pattern).
    match &context {
        Authentication::Full => {}
        Authentication::Context(_) => {
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context)
                .await?;
        }
    }

    // 3. Status gate (409). Open OR Closed allowed; Preparation rejected.
    use genossi_dao::repayment_phase::RepaymentPhaseStatus;
    match phase.status {
        RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
        RepaymentPhaseStatus::Preparation => {
            return Err(ServiceError::Conflict(Arc::from("phase_not_exportable")));
        }
    }

    Ok(phase)
}
```

### Pattern 2: Format-Whitelist im REST-Handler

**What:** `match format_str.as_str()` VOR Service-Call; nur `pdf` durchgereicht; alles andere → 400.

**Example (Source: `genossi_rest/src/attendance_export.rs:135-145`, gekürzt auf pdf-only):**

```rust
let format = match format_str.as_str() {
    "pdf" => ExportFormat::Pdf,
    other => {
        return Err(RestError::BadRequest(format!(
            "unknown export format: {}",
            other
        )))
    }
};
```

### Pattern 3: Filename-Bundle (Service-generated)

**What:** Service erzeugt Filename im `RepaymentExport`-Bundle; REST-Handler liest nur Wert; KEIN User-Input zum Filename.

**Example (Source: `genossi_rest/src/attendance_export.rs:155-167`):**

```rust
let cd = crate::http_util::content_disposition_attachment(&export.filename);

Ok(Response::builder()
    .status(200)
    .header("Content-Type", export.content_type)
    .header("Content-Disposition", &cd)
    .body(Body::from(export.bytes))
    .unwrap())
```

Filename-Schema gemäß ROADMAP SC #2: **`auszahlung-{fiscal_year}-{include}.pdf`** (z.B. `auszahlung-2026-open.pdf`).

### Pattern 4: Typst-Template mit `sys.inputs` + Repeat-Header

**What:** Template liest `meta` + `rows` als JSON-Strings aus `sys.inputs`; Repeat-Header auf jeder Seite; `_layout.typ`-Import für gemeinsames Letter-Layout.

**Example (Vorbild: `templates/defaults/teilnehmerliste.typ`):**

```typst
// Auszahlungsliste fuer RepaymentPhase — Phase 11
// Inputs via sys.inputs:
//   meta (JSON string): {"fiscal_year": int, "share_value_cent": int,
//                        "title": str, "date": str, "phase_id": str,
//                        "row_count": int, "total_amount_str": str}
//   rows (JSON string): [{"member_number": int, "name": str, "iban": str,
//                         "share_count": int, "amount_str": str,
//                         "purpose": str}, ...]

#import "_layout.typ": letter

#let meta = json.decode(sys.inputs.at("meta"))
#let rows = json.decode(sys.inputs.at("rows"))

#show: letter.with(
  title: meta.title,
  date: meta.date,
)

#text(size: 11pt)[
  *Geschäftsjahr #meta.fiscal_year — #meta.row_count Auszahlung(en)*
]

#v(0.5cm)

#table(
  columns: (auto, 1fr, auto, auto, auto, 1fr),
  align: (right, left, left, right, right, left),
  stroke: 0.5pt,
  table.header(
    repeat: true,
    [*Nr.*], [*Name*], [*IBAN*], [*Anteile*], [*Betrag*], [*Verwendungszweck*],
  ),
  ..rows.map(r => (
    [#r.member_number],
    [#r.name],
    [#r.iban],
    [#r.share_count],
    [#r.amount_str],
    [#r.purpose],
  )).flatten()
)
```

### Anti-Patterns to Avoid

- **Audit-Macro im Service:** `audited_create!`/`audited_update!`/`audited_delete!` darf NICHT in `genossi_service_impl/src/repayment_export.rs` vorkommen (EXPO-05; D-11; Grep-Gate-Test). Read-only Service.
- **Status-Check VOR Permission-Check:** Liefert Status-Info an non-admin (Information-Leak). Reihenfolge ist `load → perm → status` (Phase-6 D-13).
- **Helper-Branch im Service:** Phase 11 ist Vorstand-only. Kein `match context { Helper(_) => ... }`. Helper-Auth landet im normalen `check_permission("admin")`-Pfad → `PermissionDenied` → 403.
- **CSV-Render-Code im Service-Impl:** D-12 streicht CSV; jede `csv`-Variante im Match-Arm bricht den Format-Whitelist-Test.
- **`println!` statt `tracing::info!`:** Phase 6 D-18; strukturierte Logs für Operations.
- **Filename aus User-Input:** Phase 6 D-15; Server-generated only, sonst Path-Injection-Risiko im Header.
- **Inline-RSX-Duplikate im Frontend:** Out-of-scope (Phase 12), aber MEMORY-Note erinnert dran.
- **Hand-Rolled Content-Disposition:** Es gibt `http_util::content_disposition_attachment` — nie selbst bauen, sonst UTF-8-Filename-Bugs.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| RFC-6266-konformer `Content-Disposition` mit UTF-8-Filenames | Eigener Header-Builder | `genossi_rest::http_util::content_disposition_attachment(&filename)` | Bereits sicher gegen Quote/Newline/Umlaut-Injection (`http_util.rs:43`) |
| Typst-PDF-Generierung | Typst-CLI-Shell-Out, eigener Renderer | `PdfGenerator::render_*` aus `pdf_generation.rs` | Font-Cache, Package-Cache, World-Resolver schon implementiert |
| Default-Template-Provisioning auf Fresh-Install | `tokio::fs::write` an `RestStateImpl::new()` | `TemplateStorage::provision_defaults()` + neuer `DEFAULT_TEMPLATES`-Eintrag | Existierender Mechanismus (`template_storage.rs:94`); idempotent |
| Permission-Auth-Extraction aus Axum-Extension | Direkter `Extension<Context>`-Pattern-Match | `crate::extract_auth_context(Some(context))?` aus `genossi_rest/src/lib.rs` | Behandelt OIDC + Mock + Helper konsistent; existiert in Phase 6 |
| Format-Whitelist als String-Vergleich im Service | `Service::export(format: &str, ...)` | REST-lokales `ExportFormat`-Enum + `match` im Handler | Schichtsauberkeit; Service-Trait bleibt typed |
| Euro-Cent → "60,00" Konvertierung | Eigener `format_locale` | `format!("{},{:02}", cents / 100, cents % 100)` Pattern aus Phase 10 D-04 | Bewährter Genossi-Stil; keine Dependency auf eine Locale-Library |
| Audit-Hash-Chain-Integrität | Eigener SHA256-Code | (NICHT-TUN für Phase 11; EXPO-05) Audit-Log bleibt unverändert | Phase 11 ist read-only |
| ISO-Date-Format für Filename | `chrono`/eigener Format-String | `time::format_description::parse("[year]-[month]-[day]")` Pattern aus Phase 6 (`attendance_export.rs:188`) | Genossi-`time`-Crate-Standard |

**Key insight:** Phase 6 hat das gesamte Export-Pattern bereits ausgereift implementiert — incl. Edge-Cases (UTF-8-Filenames, Template-Not-Found, Permission-Leak-Verhinderung). Phase 11 ist ein Replikations-Diff, kein Greenfield-Design. Jede selbst gebaute Lösung ist ein Pattern-Bruch.

## Common Pitfalls

### Pitfall 1: Vergessen, `auszahlungsliste.typ` in `DEFAULT_TEMPLATES` zu registrieren

**What goes wrong:** Fresh-Install ohne den `template_storage.rs:10`-Eintrag schreibt das Template nicht ins Filesystem; erster Export-Aufruf liefert `ServiceError::InternalError("template not found: ...")`.
**Why it happens:** `TemplateStorage::provision_defaults()` iteriert nur über `DEFAULT_TEMPLATES`. Das Template selbst zu erzeugen reicht NICHT — es muss ein neuer `DefaultTemplate { path: "auszahlungsliste.typ", content: include_bytes!(...) }`-Eintrag hinzukommen.
**How to avoid:** Plan-Task explizit für `template_storage.rs`-Edit; E2E-Test nutzt `setup_with_templates()`-Helper (siehe Q9), der das Provisioning ausführt.
**Warning signs:** PDF-E2E-Test bricht mit 500-Status statt 200; Body enthält "template not found".

### Pitfall 2: Status-Check vor Permission-Check (Information Leak)

**What goes wrong:** Non-admin-User sieht, ob eine Phase existiert (404 vs. 409 vs. 200), obwohl er gar nicht berechtigt ist.
**Why it happens:** Falsche Reihenfolge in `check_admin_and_phase_status` — Status-Match VOR `check_permission`.
**How to avoid:** Reihenfolge wie in Phase 6 D-13: `load → perm → status`. Test: non-admin auf eine Phase in `Preparation` muss 403 bekommen, NICHT 409. Siehe `attendance_export.rs:748-784` `test_non_admin_returns_permission_denied`.
**Warning signs:** E2E-Test "non-admin auf Preparation-Phase" liefert 409.

### Pitfall 3: `csv` in Format-Whitelist vergessen zu blocken

**What goes wrong:** D-12 streicht CSV, aber Vorbild-Code (`attendance_export.rs:136`) hat `"csv" => ExportFormat::Csv` — beim 1:1-Kopieren ohne Entfernen bleibt der Match-Arm drin. Erster `?format=csv`-Request liefert 500 (oder schlimmer: leeres PDF mit CSV-Content-Type).
**Why it happens:** Pattern-Replikation ohne Subtraktion.
**How to avoid:** Format-Whitelist im REST-Handler MUSS minimal sein: `"pdf" => Pdf` und sofort `other => 400`. Domain-`ExportFormat`-Enum hat NUR die `Pdf`-Variante. E2E-Test deckt `?format=csv → 400` ab.
**Warning signs:** ROADMAP SC #4 "400 unbekanntes Format (`csv` blockiert mit 400)" — fehlt der Test, wird die Regression nicht erkannt.

### Pitfall 4: Soft-Deleted Member vergessen zu filtern

**What goes wrong:** Wenn ein Mitglied nach RepaymentEntry-Erzeugung soft-deleted wird, bleibt der Entry bestehen (`entry.deleted IS NULL`), aber `member.deleted IS NOT NULL`. D-02 verlangt expliziten Filter; ohne ihn würde der Member-Name auf "<unknown>" oder PII-leakend stehen.
**Why it happens:** N+1-Member-Read über `find_by_id` filtert bereits soft-deleted via Default-Impl (`member.rs:141` `e.deleted.is_none()`). Aber wenn man `dump_all`+manual-find verwendet, fehlt der Filter.
**How to avoid:** `MemberDao::find_by_id` verwenden (default-impl filtert deleted bereits); falls Entry's Member None liefert → Entry skippen (oder Error-Log).
**Warning signs:** Test mit deleted Member, der einen offenen Entry hatte, liefert weniger Zeilen oder bricht.

### Pitfall 5: `current_shares` ändert sich nach PaidOut — Verwendungszweck zeigt Stand zum Export-Zeitpunkt, nicht zum Entry-Zeitpunkt

**What goes wrong:** Nicht direkt ein Bug, aber semantische Falle: Verwendungszweck enthält `member_number + first_name + last_name`, NICHT `share_count_to_pay_out × share_value`. Falls Member umbenannt wurde nach Auto-Fill, zeigt das PDF den AKTUELLEN Namen — was korrekt ist (Banking-Form muss aktuelle Stammdaten haben).
**Why it happens:** `MemberDao::find_by_id` liest den aktuellen Member, nicht einen Snapshot zum Entry-Zeitpunkt.
**How to avoid:** Dokumentieren in Service-Comment, dass Verwendungszweck Live-Daten zeigt. Phase 6 hat dasselbe Pattern (`attendance_export.rs:99-103` Comment "frische Transaction").
**Warning signs:** Vorstand erwartet "den Namen, der bei Phase-Open gespeichert war" — D-04 hardcoded das Schema, also Comment ausreicht.

### Pitfall 6: `assert!(res.is_ok(), "{:?}", res)` ohne custom `Debug` druckt Bytes-Hex-Spam

**What goes wrong:** Wenn `RepaymentExport`-Bundle ein `#[derive(Debug)]` hat, druckt Test-Failure den vollen PDF-Bytes-Dump (Megabyte-Logs).
**Why it happens:** Default-Debug auf `Vec<u8>` ist verbose.
**How to avoid:** Manuelles `Debug`-Impl wie in `attendance_export.rs:64-72` (`bytes_len` statt `bytes`).
**Warning signs:** CI-Log-Größe explodiert bei Test-Failure.

### Pitfall 7: Send-Tracker-Conflict mit `rust_xlsxwriter` (irrelevant für Phase 11)

Phase 6 hat einen Send-Marker-Bug mit `Workbook` (`attendance_export.rs:299` Comment "no .await between Workbook operations"). Phase 11 hat KEIN XLSX, also irrelevant — aber falls in v1.2 XLSX-Export nachgezogen wird: Pattern aus Phase 6 lesen.

### Pitfall 8: PdfGenerator-Tx-Lebenszyklus

`pdf_generator.render_*` ist SYNC (`fn`, kein `async`). Das ist OK, weil Phase 6 zeigt, dass der Tx vorher committed wird (`attendance_export.rs:175`). Wichtig: KEINE Tx-Operation NACH `pdf_generator.render_*` — der Tx ist schon committed/dropped.

**Warning signs:** Compile-Error "tx moved" bei nachträglichen DAO-Calls.

## Code Examples

### Q1 Example — `PdfGenerator::render_attendance_list`-Signatur (Vorbild für `render_repayment_list`)

```rust
// Source: genossi_service_impl/src/pdf_generation.rs:279-336
pub fn render_attendance_list(
    &self,
    template_path: &str,      // "teilnehmerliste.typ"
    template_base: &Path,     // TemplateStorage::base_path()
    assembly: &AssemblyEntity,
    rows: &[AttendanceMemberRow],
    present: u64,
    total: Option<u64>,
) -> Result<Vec<u8>, ServiceError> {
    let full_path = template_base.join(template_path);
    let source_text = std::fs::read_to_string(&full_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ServiceError::InternalError(Arc::from(format!(
                "template not found: {}",
                full_path.display()
            )))
        } else {
            ServiceError::InternalError(Arc::from(format!("template io error: {}", e)))
        }
    })?;

    let inputs = build_inputs_attendance(assembly, rows, present, total);

    let world = TemplateWorld::new(
        &source_text, template_path, template_base.to_path_buf(),
        inputs, &self.fonts, &self.book, &self.package_cache,
    );

    let result = typst::compile::<PagedDocument>(&world);
    // ...
}
```

**For Phase 11:** Neue Methode `render_repayment_list(&self, template_path, template_base, phase, enriched_rows)` mit eigenem `build_inputs_repayment` (Vorbild: `build_inputs_attendance` Z. 602-652). Signatur:

```rust
pub fn render_repayment_list(
    &self,
    template_path: &str,                        // "auszahlungsliste.typ"
    template_base: &Path,
    phase: &RepaymentPhaseEntity,
    rows: &[RepaymentExportRow],                // neue Bundle-Struct
) -> Result<Vec<u8>, ServiceError>
```

Wobei `RepaymentExportRow` (im Service-Impl-Modul, NICHT im DAO) die enrichten Felder hält:

```rust
struct RepaymentExportRow {
    member_number: i64,
    name: String,              // "first_name last_name"
    iban: String,              // bank_account.unwrap_or_default()
    share_count: i32,
    amount_str: String,        // "60,00"
    purpose: String,           // "Anteilsrückzahlung GJ 2026 1234 Max Mustermann"
}
```

### Q3 Example — `DEFAULT_TEMPLATES` Eintrag-Vorbild

```rust
// Source: genossi_service_impl/src/template_storage.rs:10-27
const DEFAULT_TEMPLATES: &[DefaultTemplate] = &[
    DefaultTemplate {
        path: "_layout.typ",
        content: include_bytes!("../../templates/defaults/_layout.typ"),
    },
    DefaultTemplate {
        path: "join_confirmation.typ",
        content: include_bytes!("../../templates/defaults/join_confirmation.typ"),
    },
    // Phase 6 (D-04, D-08, D-10): Teilnehmerlisten-Export PDF template.
    DefaultTemplate {
        path: "teilnehmerliste.typ",
        content: include_bytes!("../../templates/defaults/teilnehmerliste.typ"),
    },
    // Phase 11 (EXPO-01..03): Auszahlungslisten-Export PDF template.
    // NEW — fügt sich am Ende des Arrays ein
    DefaultTemplate {
        path: "auszahlungsliste.typ",
        content: include_bytes!("../../templates/defaults/auszahlungsliste.typ"),
    },
];
```

### Q6 Example — Filename-Bundle-Generierung

```rust
// Source: genossi_service_impl/src/attendance_export.rs:187-194, 231-235
let date_format = time::format_description::parse("[year]-[month]-[day]")
    .expect("static iso-date format");
let date_str = assembly.date.date().format(&date_format)
    .unwrap_or_else(|_| "unknown".to_string());

// ...

Ok(AttendanceExport {
    bytes,
    content_type,
    filename: format!("gv-{}-teilnehmer.{}", date_str, ext),
})
```

**For Phase 11 (ROADMAP SC #2):**

```rust
// Filename schema: auszahlung-{fiscal_year}-{include}.pdf
let include_str = match include {
    ExportInclude::Open => "open",
    ExportInclude::All => "all",
    ExportInclude::Paid => "paid",
};
RepaymentExport {
    bytes,
    content_type: "application/pdf",
    filename: format!("auszahlung-{}-{}.pdf", phase.fiscal_year, include_str),
}
```

### Q7 Example — Permission-Funnel mit Status-Erweiterung

Siehe Pattern 1 oben. Code-Pfad in Phase 6 ist `attendance_export.rs:100-131`. Phase 11 unterscheidet sich nur in der Status-Match-Arm-Anzahl (2 statt 1).

### Q8 Example — OpenAPI-Doc-Struct

```rust
// Source: genossi_rest/src/attendance_export.rs:182-189
#[derive(OpenApi)]
#[openapi(
    paths(export_attendance),
    components(schemas(ExportQuery, ExportIncludeQuery)),
    tags((name = "AttendanceExport",
          description = "Teilnehmerlisten-Export ..."))
)]
pub struct ApiDoc;
```

Und der Merge-Point in `genossi_rest/src/lib.rs:275`:
```rust
(path = "/api/assembly/{assembly_id}/attendance-export", api = attendance_export::ApiDoc),
```

**For Phase 11:** Analog `ApiDoc`-Struct in `genossi_rest/src/repayment_export.rs`, dann in `genossi_rest/src/lib.rs::ApiDoc` Z. 271 (nach `repayment_phase::ApiDoc`):
```rust
(path = "/api/repayment-phase/{phase_id}/export", api = repayment_export::ApiDoc),
```

Wichtig: `RepaymentExport`-Bundle braucht KEIN `ToSchema`-Derive (Body ist binary). `ExportQuery` + `ExportIncludeQuery` benötigen `#[derive(ToSchema, IntoParams, Deserialize, Default)]` (siehe `attendance_export.rs:69-86`).

### Q9 Example — E2E-Setup-Helper-Wiederverwendung

```rust
// Source: genossi_bin/tests/e2e_tests.rs:11043 create_member_with_exit_date
//         genossi_bin/tests/e2e_tests.rs:11109 create_open_repayment_phase

// Phase 11 helper sketch — combine existing helpers:
async fn create_phase_with_entries(
    client: &reqwest::Client,
    server: &TestServer,
    n_members: usize,
    n_with_iban: usize,
) -> (RepaymentPhaseTO, Vec<MemberTO>) {
    let fiscal_year = 2026;
    let mut members = Vec::with_capacity(n_members);
    for i in 0..n_members {
        let mut m = create_member_with_exit_date(
            client, server, 100 + i as i64, fiscal_year, 5).await;
        // First n_with_iban already have IBAN from sample_member();
        // last (n_members - n_with_iban) need IBAN nulled out via PUT.
        if i >= n_with_iban {
            m.bank_account = None;
            m.version = m.version; // bump-version logic — see Phase-7 update pattern
            // ... PUT /api/members/{id}
        }
        members.push(m);
    }
    let phase = create_open_repayment_phase(client, server, fiscal_year, 12000).await;
    (phase, members)
}
```

**Important — Member ohne IBAN:** Default `sample_member()` (Z. 65) hat `bank_account: Some("DE89...")`. Für den NULL-Edge-Case-Test (ROADMAP SC #4) muss ein Member EXPLIZIT mit `bank_account: None` POSTET werden. Member-Update via PUT braucht `version`-Roundtrip — siehe Phase 7 lifecycle test (`e2e_tests.rs:10620`) als Vorbild.

**setup_with_templates() ist Pflicht:** Der PDF-Test braucht das Template auf der Disk; `setup()` allein reicht nicht. Pattern aus Phase 6 (`e2e_tests.rs:10129`).

### Q10 Example — Grep-Gate-Test (kompiliert in den Service-Impl)

```rust
// Source: genossi_service_impl/src/attendance_export.rs:1167-1198
#[test]
fn no_audit_macros_used() {
    // D-17: Export ist Read-Only — kein Audit-Log-Eintrag.
    let src = include_str!("attendance_export.rs");
    let payload: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    // Build the needle strings without writing the literal in this file:
    let create_macro = format!("{}!", "audited_create");
    let update_macro = format!("{}!", "audited_update");
    let delete_macro = format!("{}!", "audited_delete");
    assert!(!payload.contains(&create_macro), "D-17 violated: create-audit macro found");
    assert!(!payload.contains(&update_macro), "D-17 violated: update-audit macro found");
    assert!(!payload.contains(&delete_macro), "D-17 violated: delete-audit macro found");
}
```

**Hinweis (Self-Reference-Trick):** Die Needles werden über `format!("{}!", "audited_create")` runtime-konstruiert, sonst würde das eigene Source-File die Assertions selbst invalidieren. CONTEXT.md sagt zusätzlich "rg shell-grep" — beides ist möglich, aber der inline-Test ist robuster (läuft mit `cargo test`, kein externer Tool-Call).

### Q11 Example — `tracing::info!`-Pattern für read-only-Service

```rust
// Source: genossi_service_impl/src/attendance_export.rs:57-58, 196-204
const EXPORT_TARGET: &str = "attendance_export";

// ... in export():
tracing::info!(
    target: EXPORT_TARGET,
    aid = %assembly_id,
    format = ?format,
    include = ?include,
    rows = rows.len(),
    "exporting attendance"
);
```

**For Phase 11:**

```rust
const EXPORT_TARGET: &str = "repayment_export";

tracing::info!(
    target: EXPORT_TARGET,
    phase_id = %phase_id,
    fiscal_year = phase.fiscal_year,
    format = ?format,
    include = ?include,
    rows = enriched_rows.len(),
    "exporting repayment list"
);
```

### Q12 Example — Verwendungszweck + Betrag im Service pre-computen

```rust
// In RepaymentExportServiceImpl::export, nach den DAO-Reads:

let enriched_rows: Vec<RepaymentExportRow> = entries
    .iter()
    .filter_map(|entry| {
        // Member-Read (N+1; OK für SQLite-In-Memory)
        // Note: tx ist hier schon committed (Phase-6-Pattern); wir lesen
        // die enrichments AUS dem `members`-Map, das vor commit gefüllt
        // wurde — siehe Q5.
        let m = members_by_id.get(&entry.member_id)?;
        if m.deleted.is_some() { return None; }  // D-02

        let amount_cents = (entry.share_count_to_pay_out as i64) * phase.share_value;
        let amount_str = format!("{},{:02}", amount_cents / 100, amount_cents % 100);

        let purpose = format!(
            "Anteilsrückzahlung GJ {} {} {} {}",
            phase.fiscal_year,
            m.member_number,
            m.first_name,
            m.last_name,
        );

        Some(RepaymentExportRow {
            member_number: m.member_number,
            name: format!("{} {}", m.first_name, m.last_name),
            iban: m.bank_account.as_ref().map(|s| s.to_string()).unwrap_or_default(),
            share_count: entry.share_count_to_pay_out,
            amount_str,
            purpose,
        })
    })
    .collect();

// Sortierung nach D-09: (member_number ASC, entry.created ASC)
let mut enriched_rows = enriched_rows;
enriched_rows.sort_by(|a, b| a.member_number.cmp(&b.member_number));
// Note: created-Subsort braucht Entry-Referenz; sortiert die Entries VOR der
// Enrichment-Phase, dann bleibt die Reihenfolge stabil (Rust sort is stable).
```

**Wichtig zur Sort-Reihenfolge:** Da `enriched_rows` von `entries.iter().filter_map` aufgebaut wird und Rust `sort_by` stable ist, sortiert man am besten die `entries` VOR dem Build-Up (zuerst nach `created ASC` als Sub-Sort, dann nach `member_number` als Primary). Pattern:

```rust
let mut entries: Vec<_> = raw_entries.iter().cloned().collect();
entries.sort_by(|a, b| {
    let m_a = members_by_id.get(&a.member_id).map(|m| m.member_number).unwrap_or(i64::MAX);
    let m_b = members_by_id.get(&b.member_id).map(|m| m.member_number).unwrap_or(i64::MAX);
    m_a.cmp(&m_b).then_with(|| a.created.cmp(&b.created))
});
```

### Q13 Example — Path-Param-Whitelist + Query-Default

```rust
// Source: genossi_rest/src/attendance_export.rs:123-128, 135-146
pub async fn export_attendance<RestState: RestStateDef + AttendanceExportRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, format_str)): Path<(Uuid, String)>,
    Query(query): Query<ExportQuery>,
) -> Response {
    // ...
    let format = match format_str.as_str() {
        "csv" => ExportFormat::Csv,
        "pdf" => ExportFormat::Pdf,
        "xlsx" => ExportFormat::Xlsx,
        other => return Err(RestError::BadRequest(format!("unknown export format: {}", other))),
    };
    let include: ExportInclude = query.include.into();
```

**For Phase 11:** Nur `"pdf"`-Arm bleibt. Query-Param:

```rust
#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct ExportQuery {
    #[serde(default)]
    pub include: ExportIncludeQuery,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportIncludeQuery {
    #[default]
    Open,            // D-03: Default ist Open für Banking-Workflow
    All,
    Paid,
}

impl From<ExportIncludeQuery> for ExportInclude {
    fn from(q: ExportIncludeQuery) -> ExportInclude {
        match q {
            ExportIncludeQuery::Open => ExportInclude::Open,
            ExportIncludeQuery::All => ExportInclude::All,
            ExportIncludeQuery::Paid => ExportInclude::Paid,
        }
    }
}
```

### Q14 Example — Status-Gate für RepaymentPhase

```rust
// Source: genossi_dao/src/repayment_phase.rs:9-14
pub enum RepaymentPhaseStatus {
    Preparation,
    Open,
    Closed,
}
```

Beachte: Die Status-Strings sind ENGLISCH (`"Preparation"`, `"Open"`, `"Closed"`), nicht deutsch — siehe Test `repayment_phase.rs:179-191`. Die deutsche Labels (`"Vorbereitung"`, `"Offen"`, `"Abgeschlossen"`) sind nur Frontend-i18n.

**Gate-Logic für Phase 11:** 2 Stati erlaubt (`Open | Closed`); 1 Status rejected (`Preparation`).

```rust
match phase.status {
    RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
    RepaymentPhaseStatus::Preparation => {
        return Err(ServiceError::Conflict(Arc::from("phase_not_exportable")));
    }
}
```

CONTEXT.md verwendet im Trait-Namen `check_admin_and_phase_status` (Plural-Status), bewusst distinkt von Phase-6 `check_admin_and_closed`.

### Q15 Example — DI-Wiring in RestStateImpl::new()

Bestehende Arcs in `RestStateImpl::new()` (Bin lib.rs):

- `pdf_generator: Arc<PdfGenerator>` — line 698, geteilt mit AttendanceExport
- `template_storage: Arc<TemplateStorage>` — line 696
- `repayment_phase_dao: Arc<RepaymentPhaseDao>` — line 766
- `repayment_entry_dao: Arc<RepaymentEntryDao>` — line 767
- `member_dao: Arc<MemberDao>` — line 560
- `permission_service: Arc<PermissionService>` — bereits geteilt durch alle Service-Impls
- `transaction_dao: Arc<TransactionDao>` — geteilt

**Wiring-Block (Vorbild: bin lib.rs:821-830):**

```rust
let repayment_export_service = Arc::new(
    genossi_service_impl::repayment_export::RepaymentExportServiceImpl {
        transaction_dao: transaction_dao.clone(),
        permission_service: permission_service.clone(),
        repayment_phase_dao: repayment_phase_dao.clone(),
        repayment_entry_dao: repayment_entry_dao.clone(),
        member_dao: member_dao.clone(),
        pdf_generator: pdf_generator.clone(),
        template_base: Arc::new(template_storage.base_path().to_path_buf()),
    },
);
```

**RestStateDeps-Impl-Block (Vorbild: bin lib.rs:266-289):**

```rust
pub struct RepaymentExportServiceDependencies;

unsafe impl Send for RepaymentExportServiceDependencies {}
unsafe impl Sync for RepaymentExportServiceDependencies {}

impl genossi_service_impl::repayment_export::RepaymentExportServiceDeps
    for RepaymentExportServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type RepaymentEntryDao = RepaymentEntryDao;
    type MemberDao = MemberDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type RepaymentExportService =
    genossi_service_impl::repayment_export::RepaymentExportServiceImpl<RepaymentExportServiceDependencies>;
```

**Trait-Impl auf RestStateImpl (Vorbild: bin lib.rs:1462-1467):**

```rust
impl genossi_rest::repayment_export::RepaymentExportRestState for RestStateImpl {
    type RepaymentExportService = RepaymentExportService;
    fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService> {
        self.repayment_export_service.clone()
    }
}
```

**Mount in genossi_rest::create_app (Vorbild: lib.rs:640-643):**

```rust
.nest(
    "/api/repayment-phase",
    repayment_export::generate_export_route::<RestState>(),
)
```

Frage: Konflikt mit existierendem `/api/repayment-phase`-Router? **Ja**, der existiert bereits (`lib.rs:613-614`):

```rust
.nest(
    "/api/repayment-phase",
    repayment_phase::generate_route::<RestState>(),
)
```

**Axum-Verhalten:** Zwei `.nest()` auf denselben Prefix werden gemerged; jede Route innerhalb muss einzigartig sein. Die Phase-7-Routes (`/`, `/{id}`, `/{id}/open`, `/{id}/close`) kollidieren NICHT mit Phase-11-Route (`/{phase_id}/export/{format}`), weil die Pfad-Segmente unterschiedlich sind. Trotzdem ist es sauberer, Phase 11 in einer EINZIGEN Router-Funktion zu mounten oder explizit zu testen, dass der Router beide akzeptiert. Empfehlung: Phase 11 mounted als ZWEITES `.nest("/api/repayment-phase", ...)` mit eigener Route — Axum 0.8.3 handhabt das per Merge-Strategie.

Alternative Empfehlung: `generate_export_route()` registriert `/{phase_id}/export/{format}` → Mount unter `/api/repayment-phase` → finale URL `/api/repayment-phase/{phase_id}/export/{format}`. ROADMAP SC #2 verlangt genau diesen Pfad.

**RestStateDef bound erweitern in `create_app` (lib.rs:435-446):**

```rust
pub async fn create_app<
    RestState: RestStateDef
        // ... existing bounds
        + repayment_phase::RepaymentPhaseRestState
        + repayment_entry::RepaymentEntryRestState
        + attendance_export::AttendanceExportRestState
        + repayment_export::RepaymentExportRestState     // NEW
        // ... rest
>
```

## Runtime State Inventory

> Phase 11 ist **kein** Rename/Refactor. Diese Sektion ist hier überflüssig.

**Stored data:** Keine Schema-Änderung; keine Migration für Phase 11 nötig (RepaymentEntry/RepaymentPhase existieren seit Phase 7/8). KEINE neue SQLite-Tabelle.

**Live service config:** Keine Konfig-Änderung.

**OS-registered state:** Keine.

**Secrets/env vars:** Keine.

**Build artifacts:** `templates/defaults/auszahlungsliste.typ` wird per `include_bytes!` in die Binary kompiliert; bei Code-Edit muss neu gebaut werden. Aber kein Stale-Artefakt-Risiko, weil das Template via `provision_defaults()` auf Disk wandert und dort gelesen wird.

## Open Questions

Keine ungeklärten Open Questions auf Research-Ebene. CONTEXT.md hat alle 12 wesentlichen Entscheidungen verbindlich gelockt (D-01 bis D-12). Die 3 Claude's-Discretion-Items (Member-Read-Strategie, Betrag-Rendering-Ort, Path-Whitelist-Style) sind in Q5, Q12 bzw. Q13 mit klarer Empfehlung beantwortet.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build | ✓ (via Nix flake) | 2021 edition / 1.70+ | — |
| SQLite | E2E-Tests (in-memory) | ✓ (libsqlite3 in flake) | bundled via sqlx | — |
| Liberation Sans Fonts | PDF-Render | ✓ (embedded via `include_bytes!` in `pdf_generation.rs:18-23`) | static | — |
| Typst 0.14 + typst-pdf | PDF-Render | ✓ (Cargo.lock pinned) | 0.14 | — |
| TYPST_PACKAGE_CACHE dir | Typst-Package-Downloads | optional (Phase 11 nutzt nur lokales `_layout.typ`, keine externen Packages) | — | `./typst-packages` Default |
| Filesystem (writable for TemplateStorage) | Template-Provisioning | ✓ | — | — |

**Missing dependencies with no fallback:** Keine.

**Missing dependencies with fallback:** Keine.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| (none) | Alle Claims sind code-verifiziert (`include_str!`-Lesungen oder gegrepte Source-Pfade) | — | — |

**Alle Claims sind via Source-Reading verifiziert** — Phase 6 Code, Phase 7-10 Entity-Definitionen, REST-Routing in `lib.rs`, DI-Wiring in `bin/lib.rs`. Keine `[ASSUMED]`-Tags nötig.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Excel-CSV-Export für Anteils-Auszahlungen | Genossi PDF-Export als Banking-Online-Vorlage | Phase 11 (v1.1) | Vorstand kopiert IBAN+Betrag+Verwendungszweck direkt ins Online-Banking-Sammelüberweisungs-Tool |
| Hand-Roll-CSV/XLSX-Renderer pro Domain | Typst-Template + JSON-`sys.inputs`-Injection | Phase 6 etabliert | Konsistenter Render-Pipeline; Schrift+Margins zentral in `_layout.typ` |
| Audit-Macros für ALLE Schreib-Aktionen | Audit-Macros NUR für Member/MemberAction/MemberDocument/Application | Phase 6 D-17 | Read-only-Services (Export) machen keine Audit-Einträge — Hash-Chain bleibt schlank |
| Lokale CSS/XML-Templates | `DEFAULT_TEMPLATES`-Static-Registry mit `include_bytes!` | Phase 6 | Fresh-Install sichert Templates automatisch; User kann sie nach Provisioning editieren |

**Deprecated/outdated:**
- "Audit-Hashchain für Export-Endpoints" — explizit verworfen (REQUIREMENTS.md Z. 85: "Read-only, kein Schreibvorgang — Audit-Belastung ohne Mehrwert")
- "Per-Mitglied-Aggregation im PDF" — D-08 entscheidet sich für Per-Entry-Zeilen (1:1-Mapping zur DB)

## Sources

### Primary (HIGH confidence)

- `genossi_service/src/attendance_export.rs:1-158` — Vorbild für Trait + Domain-Types + Default-Impl + Bundle-Struct + Debug-Impl
- `genossi_service_impl/src/attendance_export.rs:1-1199` — Vorbild für Permission-Funnel, Format-Whitelist im Service, tracing::info!, Grep-Gate-Test, DI-Trait-Pattern
- `genossi_rest/src/attendance_export.rs:1-269` — Vorbild für REST-Handler, map_export_error, Query-Param-Default-Pattern, OpenAPI-ApiDoc, Format-Whitelist im Handler, Router-Funktion, error_handler-Wrapping
- `genossi_rest/src/http_util.rs:43-50` — `content_disposition_attachment` — RFC-6266-Header für Filename mit UTF-8
- `genossi_service_impl/src/pdf_generation.rs:279-336` — `render_attendance_list` Signatur-Vorbild; `build_inputs_attendance` Z. 602-652 für sys.inputs-Dict-Aufbau
- `genossi_service_impl/src/pdf_generation.rs:128-153` — `PdfGenerator::new()` + Font-Loading
- `genossi_service_impl/src/template_storage.rs:5-27` — `DEFAULT_TEMPLATES`-Registry + `provision_defaults`-Mechanismus
- `templates/defaults/teilnehmerliste.typ:1-43` — Vorbild für Repeat-Header, json.decode-Pattern, `_layout.typ`-Import
- `templates/defaults/_layout.typ:1-38` — Letter-Layout mit `font: "Liberation Sans"`, deutsche Sprache
- `genossi_dao/src/repayment_phase.rs:9-92` — `RepaymentPhaseStatus`-Enum (Preparation/Open/Closed), `RepaymentPhaseEntity`-Felder (fiscal_year, share_value)
- `genossi_dao/src/repayment_entry.rs:15-92` — `RepaymentEntryStatus`-Enum (Open/Contacted/PaidOut), `RepaymentEntryEntity`-Felder; `find_by_phase_id` Z. 143-155
- `genossi_dao/src/member.rs:73-196` — `MemberEntity` (incl. `bank_account: Option<Arc<str>>`, `member_number: i64`), `MemberDao::find_by_id` Default-Impl mit soft-delete-Filter
- `genossi_rest/src/lib.rs:189-447` — `RestStateDef`-Trait-Hierarchie, `create_app`-Funktion mit Bounds, ApiDoc-Merge-Block, Router-Mount-Block für `/api/repayment-phase` (Z. 613-614) und attendance_export (Z. 640-643)
- `genossi_bin/src/lib.rs:179-289, 503-958, 1460-1467` — DI-Type-Aliases, RestStateImpl-Felder, RestStateImpl::new()-Aufbau, attendance_export-Wiring (Z. 821-830) als 1:1-Vorbild, Trait-Impl-Block (Z. 1462)
- `genossi_bin/tests/e2e_tests.rs:27-41` — `setup()` mit in-memory SQLite-Pool
- `genossi_bin/tests/e2e_tests.rs:2672-2694` — `setup_with_templates()` mit `provision_defaults`-Call (Pflicht für PDF-Tests)
- `genossi_bin/tests/e2e_tests.rs:10001-10570` — Phase-6 E2E-Tests (Format-Vergleich, Filename-Schema, 403/404/409-Cases)
- `genossi_bin/tests/e2e_tests.rs:10578-11132` — Phase-7/8/9 E2E-Helper (`create_preparation_repayment_phase`, `create_member_with_exit_date`, `create_open_repayment_phase`)
- `genossi_bin/tests/e2e_tests.rs:12018-12100` — Phase-9 `mark_paid_out`-Pattern für PaidOut-Status-Setup im E2E
- `.planning/phases/11-export-pdf-csv/11-CONTEXT.md` — Locked User Decisions D-01..D-12

### Secondary (MEDIUM confidence)

- `.planning/REQUIREMENTS.md:41-46, 110-114` — Export-Requirements EXPO-01..05 (EXPO-04 deferred)
- `.planning/ROADMAP.md:161-173` — Phase 11 Success Criteria
- `.planning/config.json` — `nyquist_validation: false` → Validation-Architecture-Sektion entfällt

### Tertiary (LOW confidence)

- (Keine — alle Quellen sind Source-Code oder lokale Docs)

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — alle Crates in Cargo.lock pinned, alle Pattern-Implementierungen produktiv getestet seit Phase 6 (Mai 2026)
- Architecture: HIGH — 1:1-Replikation von Phase 6 mit minimalen, klar abgegrenzten Anpassungen
- Pitfalls: HIGH — alle 8 Pitfalls aus Phase 6 Code/Tests direkt verifiziert (Grep-Gate-Test existiert produktiv, Permission-Reihenfolge produktiv getestet)
- DI-Wiring: HIGH — Wiring-Code-Pfade direkt gegrepped in `bin/lib.rs`

**Research date:** 2026-05-31
**Valid until:** 2026-06-30 (Phase-6-Pattern ist stable; einzige Variabilität ist neue v1.2-Phase, die CSV nachzieht und additiv ist)
