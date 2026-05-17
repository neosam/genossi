# Phase 6: Teilnehmerlisten-Export für Generalversammlungen - Research

**Researched:** 2026-05-17
**Domain:** PDF/CSV/XLSX-Export-Generierung aus einem geschlossenen GV-Snapshot, Backend-only neue Logik plus Download-Trigger im Dioxus-WASM-Frontend
**Confidence:** HIGH

## Summary

Phase 6 ist ein **read-only Export-Aggregat** über bereits existierende Datenstrukturen (`assembly_member_snapshot` + `attendance`). Die gesamte Infrastruktur — Typst-PDF-Toolchain mit eingebetteten Liberation-Sans-Fonts, Permission-Funnel im `AttendanceServiceImpl`, RFC-6266-Content-Disposition-Helper, Blob-Download-Pattern im Frontend — ist **bereits im Codebase vorhanden und produktiv erprobt**. Drei Beobachtungen aus der Code-Inspektion bestimmen den Plan-Zuschnitt:

1. **`rust_xlsxwriter = "0.82"` ist bereits eine dev-dependency** in `genossi_bin/Cargo.toml:58` (wird in den Member-Import-E2E-Tests verwendet). Für Phase 6 muss die Dependency lediglich in die produktive `[dependencies]`-Section eines Implementations-Crates wandern — das ist kein "neuer Workspace-Member", sondern eine **Promotion** einer Dev-Dep. **D-02 ist damit faktisch schon halb umgesetzt.** [VERIFIED: `Cargo.lock:4323` zeigt 0.82.0 mit checksum, `genossi_bin/Cargo.toml:58` zeigt dev-dep]
2. **`csv = "1.3"` ist bereits reguläre Dep in `genossi_rest/Cargo.toml:30`** [VERIFIED: grep]. Für Phase 6 muss CSV nur an die richtige Stelle (Service-Layer oder neues Modul) verfügbar gemacht werden — entweder Promotion in Workspace-Deps oder Re-Use der bestehenden REST-Lokal-Dep.
3. **Der bestehende `AttendanceServiceImpl::check_assembly_access` hat einen Admin-Branch ohne Status-Check (D-20 dort)** — der für Phase 6 NICHT direkt taugt, weil D-11 hier einen **expliziten 409-Conflict für Nicht-`Closed`-GVs** verlangt. Der Export-Service braucht entweder einen **eigenen, strikteren Funnel** oder muss `check_assembly_access` aufrufen und anschliessend selbst `assembly.status == Closed` prüfen.

**Primary recommendation:** Backend-Code in **bestehenden Crates erweitern** (kein neues `genossi_export`-Crate). Ein neuer Service `AttendanceExportServiceImpl` in `genossi_service_impl/src/attendance_export.rs` und ein neuer Handler in `genossi_rest/src/attendance_export.rs`. Datenquelle ist `AttendanceDao::list_members_for_assembly` mit `search=None` — keine neue DAO-Methode nötig. Eigener Permission-Funnel (Admin-only, mit `Closed`-Status-Check). Frontend bekommt eine Format-Auswahl-Komponente, die das bestehende `web-sys` Blob-Download-Muster (`api.rs:537-548`) wiederverwendet.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Formate**
- **D-01:** Drei Formate werden parallel angeboten: PDF (Protokoll-Anhang), CSV (Verband / Weiterverarbeitung), XLSX (Excel-natives Format).
- **D-02:** XLSX wird mit `rust_xlsxwriter` erzeugt — Pure-Rust, aktiv gepflegt, keine FFI-Bindings. Crate ist neue Workspace-Dependency.
- **D-03:** CSV ist Semikolon-getrennt und UTF-8 mit BOM kodiert, damit deutsche Excel-Installationen die Datei ohne „Daten → Aus Text"-Wizard direkt öffnen.
- **D-04:** PDF wird über die bestehende Typst-Toolchain (`typst` + `typst-pdf`) erzeugt, parallel zum bereits etablierten Pattern in `genossi_service_impl/src/pdf_generation.rs` (siehe `join_confirmation.typ`, `zahlungsanfrage.typ`).

**Inhalt & Daten**
- **D-05:** Datenquelle ist der `assembly_member_snapshot` plus die `attendance`-Tabelle — historisch stabil. Nachträgliche Member-Mutationen (Namensänderung, Austritt) wirken sich NICHT auf bereits generierte Exporte aus. Konsistenz mit ASSY-02 (Snapshot beim Open der GV) und ASSY-05 (Persistenz nach Schluss).
- **D-06:** Spalten-Whitelist im Export: `member_number`, `first_name`, `last_name`, `salutation`, `title`, `is_present` — exakt die ATTN-01-DSGVO-Whitelist aus `AttendanceMemberRow` (`genossi_dao/src/attendance.rs:45`). `member_id` (UUID) ist im REST nötig für Frontend-PUT/DELETE, im Export-File aber NICHT relevant — Researcher prüft, ob die UUID-Spalte im Export weggelassen werden soll.
- **D-07:** Sortierung ist `member_number ASC` — konsistent mit der Live-Liste seit commit `ed754fc`.
- **D-08:** PDF enthält einen Kopf-Block mit GV-Titel, GV-Datum und Zähler „X von Y anwesend" (X = anwesende, Y = `total` aus `assembly_member_snapshot`). Danach folgt die Tabelle. Keine Unterschriftenspalte, kein Logo, keine Fußzeile mit Export-User/Versionsnummer.

**Auswahl & Filterung**
- **D-09:** Mitglieder-Auswahl ist über Query-Parameter `?include=all|present` steuerbar — `all` liefert alle Snapshot-Mitglieder plus Anwesenheits-Spalte, `present` liefert nur die anwesenden. Default-Wert von `include` und die genaue Darstellung der Anwesenheits-Spalte (z. B. `"ja"`/`"nein"`, `1`/`0`, leer/`✓`) entscheidet der Planner.
- **D-10:** `include=present` ergibt im PDF einen reduzierten Kopf — „X anwesend" statt „X von Y" — weil Y aus der Liste nicht ableitbar ist; Planner kann auch entscheiden, Y trotzdem aus dem Snapshot zu lesen und im Kopf mitzuzeigen.

**Lifecycle & Berechtigung**
- **D-11:** Export ist NUR für GVs im Status `Geschlossen` erlaubt. Aufruf gegen `Vorbereitung` oder `Geöffnet` liefert 409 Conflict (mit klarer Fehlermeldung).
- **D-12:** Post-Close-Korrekturen (ASSY-06) wirken sich auf nachfolgende Exporte aus — der Export reflektiert immer den aktuellen Stand der `attendance`-Tabelle. Dokumentation in OpenAPI sollte das explizit erwähnen.
- **D-13:** Zugriff nur für OIDC-authentifizierten Vorstand (Permission-Check identisch zum bestehenden Assembly-Detail-Endpoint). Helfer haben KEINEN Zugriff auf den Export.

**REST-API**
- **D-14:** Ein Endpoint mit Format via Pfad-Suffix: `GET /api/assembly/{aid}/attendance-export/{format}` mit `format ∈ {csv, pdf, xlsx}`. Query-Parameter `?include=all|present`.
- **D-15:** Content-Disposition-Filename folgt dem Schema `gv-{YYYY-MM-DD}-teilnehmer.{ext}` — Datum aus `assembly.date`, Extension matched Format. Beispiel: `gv-2026-05-15-teilnehmer.pdf`.
- **D-16:** MIME-Types: `application/pdf`, `text/csv; charset=utf-8` mit BOM im Body-Vorspann, `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` für XLSX.

**Audit & Logging**
- **D-17:** KEIN Audit-Hashchain-Eintrag für Export-Aufrufe — Export ist Read-Only auf bereits existierende Daten.
- **D-18:** Server-seitiges `tracing::info!` mit GV-ID, Format, `include`-Parameter und User-ID ist ausreichend für operative Sichtbarkeit.

**UI**
- **D-19:** Download-Button(s) sitzen in der bestehenden Assembly-Detail-Seite des Vorstand-Frontends. Sichtbar/aktivierbar NUR wenn `assembly.status == Closed`. Drei separate Download-Knöpfe (CSV/PDF/XLSX) oder ein einzelner Knopf mit Format-Dropdown — Planner entscheidet.
- **D-20:** Component-First gilt: Wenn der Download-Knopf-Block (Format-Auswahl + Include-Toggle) auf zwei Pages auftauchen sollte, geht er in `genossi-frontend/src/component/`. Im Phase-6-Scope ist nur EINE Page betroffen — Komponentisierung nur falls Wiederverwendung absehbar ist.

### Claude's Discretion

Der Planner / Researcher hat Spielraum bei:
- Default-Wert von `?include` (Empfehlung: `all`).
- Darstellung der Anwesenheits-Spalte pro Format (z. B. `"ja"`/`"nein"` vs. `✓`/leer).
- Ob `member_id` (UUID) im Export-File erscheint oder nicht (Empfehlung: weglassen).
- Ob im PDF Spaltenbreiten/Layout per Typst-Helper gesetzt werden oder die Defaults reichen.
- Ob der UI-Button als Dropdown oder als drei separate Buttons erscheint.
- Ob die Backend-Implementierung als neue Crate `genossi_export` oder im bestehenden `genossi_service_impl`/`genossi_rest` lebt.

### Deferred Ideas (OUT OF SCOPE)

- **Sammelexport mehrerer GVs** (z. B. Jahres-Liste mit allen GVs eines Jahres) — eigene Phase.
- **Automatischer Versand per E-Mail an Verband-Adresse** — könnte später kommen.
- **Unterschriften-Spalte im PDF als Papier-Backup-Liste** — explizit nicht ausgewählt.
- **Logo / Briefkopf** — würde eine Genossenschafts-Konfiguration voraussetzen, die heute noch nicht existiert.
- **XLSX mit Multi-Sheet** — Over-Engineering für Verband.
- **Export-Audit** (wer hat wann exportiert) — bewusst verworfen (D-17).

## Project Constraints (from CLAUDE.md)

- **Tech-Stack-Beschränkung:** Rust + Axum + SQLx + SQLite Backend, Dioxus WASM Frontend — keine Sprach- oder DB-Wechsel.
- **Layered Architecture:** DAO → Service → REST-Reihenfolge ist Pflicht. Neue Entitäten implementieren bestehende Trait-Patterns (`gen_service_impl!`, `automock` mit `MockTransaction`).
- **DSGVO-SELECT-Whitelist:** DAO-Queries listen Spalten explizit auf — NIEMALS `SELECT m.*`. Der Export wiederverwendet die bereits validierte 7-Spalten-Whitelist aus `AttendanceMemberRow`.
- **Component-First Frontend:** Wenn UI auf zwei Pages erscheint, wandert sie in `genossi-frontend/src/component/`. Pages enthalten kein inline-RSX für wiederverwendbare Bausteine.
- **Audit-Linie:** Auditierte Entitäten (Member, MemberAction, MemberDocument, Application, Assembly, HelperToken) **müssen** `audited_*!`-Macros verwenden. Phase-6-Export schreibt keine Daten → KEINE neue auditierte Entität, KEINE `Auditable`-Impl.
- **Forbidden:** `#[derive(Default)]` für DAO-Entities wegen Audit-Hashchain-Risiko; impliziter `unwrap_or_default()` in `audit_fields()` (siehe `assembly.rs:67-83`).
- **OIDC-Provider ist Nextcloud** (nicht WordPress — ältere Specs sind stale).
- **`/api/...` ist immer prefix** für alle Backend-Endpoints, nicht nur die neuen.

## Phase Requirements

Keine REQ-IDs in ROADMAP gemappt (`phase_req_ids = null`). CONTEXT.md `<decisions>` D-01..D-20 sind die verbindliche Anforderungsquelle. Indirekt baut Phase 6 auf folgenden Anforderungen auf:

| Implizite Predecessor-Req | Bedeutung für Phase 6 |
|---------------------------|------------------------|
| ASSY-02 (Snapshot beim Öffnen) | Datengrundlage `assembly_member_snapshot`, garantiert stabiles "Y" für den Header |
| ASSY-05 (Persistenz nach Schluss) | `Closed`-GVs behalten ihre Daten — Export-Aufruf ist auch Wochen später möglich |
| ASSY-06 (Post-Close-Korrekturen) | `attendance`-Tabelle kann auch nach `Closed` aktualisiert werden — Export reflektiert aktuellen Stand (D-12) |
| ATTN-01 (DSGVO-Whitelist 7 Felder) | Identische Whitelist für Export — kein PII-Leak |
| ATTN-05 (Anwesenheit nicht auditiert) | Konsistent: Export auch nicht auditiert (D-17) |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| GV-Status-Validierung (Closed-only) | API/Backend (Service) | — | Statusprüfung gehört in den Permission-Funnel des Services, NICHT ins Frontend. Frontend-Visibility-Gate ist nur UX-Lack, keine Sicherheit. |
| Datenabfrage Snapshot+Attendance | API/Backend (DAO) | — | Bestehende `list_members_for_assembly` (`genossi_dao_impl_sqlite/src/attendance.rs:107-162`) deckt alles ab. |
| Permission-Check (Admin-only) | API/Backend (Service) | — | Identisch zum bestehenden Pattern in `attendance.rs::check_assembly_access`, ohne Helper-Branch. |
| `include=present` Filter | API/Backend (Service) | — | In-Memory-Filter über `is_present` nach DAO-Call (Quasi-No-Op, max ~500 Mitglieder). |
| PDF-Generierung (Typst) | API/Backend (Service) | — | Bestehende `PdfGenerator`-Toolchain mit eingebetteten Fonts. |
| CSV-Generierung (UTF-8+BOM+;) | API/Backend (Service) | — | `csv` crate 1.3 + manuelles BOM-Prefix; kein Dependency-Neuzugang. |
| XLSX-Generierung | API/Backend (Service) | — | `rust_xlsxwriter` 0.82 ist bereits in `Cargo.lock` als dev-dep; promotion auf produktiv. |
| Content-Disposition-Header | API/Backend (REST) | — | Bestehender `content_disposition_attachment`-Helper (`http_util.rs:43-50`). |
| Download-Trigger | Browser/Client (WASM) | — | Bestehendes Pattern in `api.rs:537-548` (fetch → blob → `Url::create_object_url_with_blob` → `<a download>`). |
| Format-Auswahl-UI | Browser/Client (Page/Component) | — | Sichtbarkeit gebunden an `assembly.status == Closed` aus dem bestehenden `AssemblyDetails`-Page-Signal. |

## Standard Stack

### Core (bereits im Workspace, Promotion nötig)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rust_xlsxwriter` | 0.82 (bereits in Cargo.lock) bzw. 0.94/0.95 (aktuell, Stand Mai 2026) | XLSX-Generierung als `Vec<u8>` via `Workbook::save_to_buffer()` | Pure Rust, keine C-FFI, aktiver Maintainer (jmcnamara, Author auch der Python-XlsxWriter-Lib), MIT/Apache-2.0-License, einzige Default-Dep ist `zip` [VERIFIED: docs.rs/crates.io, see Sources]. **Bereits in `genossi_bin/Cargo.toml:58` als dev-dep für Member-Import-Tests.** |
| `csv` | 1.3 | CSV-Generierung mit konfigurierbarem Delimiter | Bereits Dep in `genossi_rest/Cargo.toml:30` und `genossi_bin/Cargo.toml:60` (dev). `WriterBuilder::new().delimiter(b';').from_writer(Vec::new())` ist der Standard-Pattern [VERIFIED: docs.rs/csv]. |
| `typst` + `typst-pdf` | 0.14 (Workspace-Default) | PDF-Generierung aus Templates | Bestehende Toolchain in `genossi_service_impl/src/pdf_generation.rs`, Liberation-Sans-Fonts eingebettet (`fonts/LiberationSans-*.ttf`), Templates leben in `templates/`. |

### Supporting (bereits im Workspace, kein Zukauf)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde_json` | 1.0 (Workspace) | JSON-Payload für Typst-`sys.inputs` | Pattern aus `pdf_generation.rs::build_inputs`: gesamtes Daten-Objekt als JSON-String, im Typst-Template via `json.decode(sys.inputs.at("..."))`. |
| `time` | 0.3 | Datum-Formatierung `YYYY-MM-DD` für Filename | Bestehende Workspace-Dep, `assembly.date.date()` + `format_description::parse("[year]-[month]-[day]")`. |
| `tracing` | 0.1 | `#[instrument(skip(rest_state))]` + `info!`-Log mit GV-ID + Format | D-18 fordert explizit `tracing::info!`. |
| `axum` | 0.8.3 | HTTP-Handler mit `Path<(Uuid, String)>` + `Query<ExportQuery>` | Pattern aus `attendance.rs:128-181`. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `rust_xlsxwriter` | `xlsxwriter` (C-FFI-Wrapper über `libxlsxwriter`) | C-Dep verkompliziert Builds (Nix-Toolchain!), MSRV-Risiken, kein Vorteil für simple Tabellen. **VERWORFEN durch D-02.** [CITED: docs.rs/crates.io/xlsxwriter] |
| `rust_xlsxwriter` | `umya-spreadsheet` (Pure Rust, read+write) | Schwerer (lädt komplettes Workbook in-memory), 3-4× langsamer beim Lesen, und für Write-Only-Use-Cases unnötig komplex. [VERIFIED: WebSearch, umaranis.com] |
| `csv` 1.3 mit manuellem `Vec<u8>`-BOM | `csv` + `Cursor<Vec<u8>>` + explizites `write_all(&[0xEF,0xBB,0xBF])` davor | Funktional identisch; das manuelle BOM-Prefix ist Standard-Pattern für Excel-Kompatibilität [CITED: WebSearch zu UTF-8-BOM+Excel-DE]. |
| Typst-PDF | `printpdf` / `wkhtmltopdf` / Headless-Chromium | Würde komplette neue Toolchain einführen. Typst ist bereits Standard im Projekt — `pdf_generation.rs` ist produktiv erprobt. |
| Eigene Crate `genossi_export` | Code in `genossi_service_impl` + `genossi_rest` | Neue Crate erhöht Build-Time (cold compile +2-3s) ohne Re-Use-Vorteil (keine externe Verwendung absehbar). Bestehende Layered-Architektur reicht. **Empfehlung: KEIN neues Crate.** |

**Versions-Verifikation Stand 2026-05-17:**
- `rust_xlsxwriter`: **0.94.0 (Februar 2026)** ist laut WebSearch die aktuelle Stable; **0.95.0 (9. Mai 2026)** ist die allerneueste laut lib.rs. Im Workspace liegt 0.82.0 — **die im Workspace eingefrorene Version genügt für Phase 6**, aber der Planner sollte bewerten, ob ein Bump auf 0.94/0.95 sinnvoll ist (Risk-Free, keine Breaking Changes erwartet bei Patch-API `save_to_buffer`). [VERIFIED: WebSearch + WebFetch lib.rs]
- `csv`: 1.3.x ist aktuelle stabile Version, im Workspace vorhanden.

**Installation (Promotion + neue Dep für Service-Impl):**

```toml
# In Cargo.toml [workspace.dependencies]:
rust_xlsxwriter = "0.82"   # ODER bumpen auf "0.94" — Planner-Entscheidung
csv = "1.3"

# In genossi_service_impl/Cargo.toml [dependencies]:
rust_xlsxwriter = { workspace = true }
csv = { workspace = true }
```

`genossi_bin/Cargo.toml:58` (dev-dep) kann anschließend optional auf `{ workspace = true }` migriert werden.

## Architecture Patterns

### System Architecture Diagram

```
                  [Vorstand-Browser]
                          |
                          | 1) Klick "PDF exportieren" (assembly.status==Closed)
                          v
                  [AssemblyDetailsPage]
                          |
                          | 2) api::export_attendance(config, aid, "pdf", include)
                          v
                  [api.rs::export_attendance]  -- fetch + .blob() pattern
                          |
                          | 3) GET /api/assembly/{aid}/attendance-export/pdf?include=all
                          v
                  [auth_middleware] -- OIDC-cookie → Context
                          |
                          v
            [AttendanceExportHandler]  (genossi_rest/src/attendance_export.rs)
                          |
                          | 4) error_handler(async{ ... }.await)
                          v
            [AttendanceExportServiceImpl]  (genossi_service_impl/src/attendance_export.rs)
              |
              | 5) check_admin_and_closed(aid, ctx, tx)
              |     - find_by_id(aid) -> EntityNotFound / Closed-check
              |     - permission_service.check_permission("admin", ctx)
              v
              |
              | 6) attendance_dao.list_members_for_assembly(aid, None, tx)
              v
              |
              | 7) (optional) filter rows where is_present==true (include=present)
              v
              |
              | 8) match format { "pdf" => render_pdf(rows, assembly, total),
              |                  "csv" => render_csv(rows, include_total),
              |                  "xlsx" => render_xlsx(rows, include_total) }
              v
              |
              | 9) (Vec<u8>, "application/pdf"|"text/csv"|"appl/vnd..xlsx")
              v
            [AttendanceExportHandler]
              |
              | 10) Content-Disposition: attachment; filename="gv-2026-05-15-teilnehmer.pdf"
              |     Content-Type: <mime>
              |     Body::from(bytes)
              v
                  [Browser-Blob]
                          |
                          | 11) Url::create_object_url_with_blob() → <a download="...">click
                          v
                  [User-Download]
```

### Recommended Project Structure

Keine neuen Crates — minimal-invasive Erweiterung der bestehenden Struktur:

```
genossi_service/src/
└── attendance_export.rs       # NEU: trait AttendanceExportService { fn export(...) }

genossi_service_impl/src/
├── attendance_export.rs       # NEU: AttendanceExportServiceImpl, gen_service_impl!
├── attendance_export/
│   ├── csv_writer.rs          # NEU: optional submodule für CSV-BOM-Pattern (Tests separat)
│   ├── xlsx_writer.rs         # NEU: rust_xlsxwriter logic
│   └── pdf_writer.rs          # NEU: build_inputs für teilnehmerliste.typ
└── pdf_generation.rs          # bestehend: erweitern um `render_attendance_list(...)` ODER neue Generic-Methode

templates/
└── teilnehmerliste.typ        # NEU: Typst-Template mit table.header(repeat:true)

genossi_rest/src/
├── attendance_export.rs       # NEU: handler + generate_route()
└── lib.rs                     # bestehend: pub mod + Router-Registration

genossi_bin/src/lib.rs         # bestehend: DI-Wiring AttendanceExportServiceDependencies

genossi-frontend/src/
├── api.rs                     # erweitern: pub async fn export_attendance(...)
├── page/assembly_details.rs   # erweitern: Export-Block für status==Closed
└── component/                 # OPTIONAL: ExportFormatPicker wenn auf zweite Page wandert
```

### Pattern 1: Permission-Funnel mit Status-Check (Service-Layer)

**What:** Phase-6-spezifischer Funnel — strikter als der bestehende `AttendanceServiceImpl::check_assembly_access`. Kein Helper-Branch, plus expliziter `Closed`-Check.

**When to use:** Als allererster DAO-touchender Schritt jeder Export-Methode.

**Example:**

```rust
// Inspired by genossi_service_impl/src/attendance.rs:79-115
// Source: genossi_service_impl/src/attendance.rs (production code, verified pattern)

const EXPORT_PROCESS: &str = "attendance-export-service";

async fn check_admin_and_closed(
    &self,
    assembly_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<AssemblyEntity, ServiceError> {
    // 1) Load assembly (handles 404)
    let assembly = self
        .assembly_dao
        .find_by_id(assembly_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(assembly_id))?;

    // 2) Admin-only — KEIN Helper-Branch (D-13)
    // Authentication::Full bypasst für E2E-Tests (Pattern aus attendance.rs)
    match &context {
        Authentication::Full => {}
        Authentication::Context(_) => {
            self.permission_service
                .check_permission("admin", context.clone())
                .await?;
        }
    }

    // 3) Status-Gate (D-11) — D-12: post-close updates erlaubt, deshalb Closed reicht
    if assembly.status != AssemblyStatus::Closed {
        return Err(ServiceError::Conflict(Arc::from("assembly_not_closed")));
    }

    Ok(assembly)
}
```

### Pattern 2: Typst-Template mit `table.header(repeat: true)` für lange Listen

**What:** Tabellen-Pattern für Listen >1 Seite mit wiederholtem Header. [VERIFIED: WebFetch typst docs `table` + WebSearch forum]

**When to use:** Bei N>40 Zeilen (eine A4-Seite mit Liberation Sans 11pt fasst grob 40-50 Zeilen) — Typst paginiert automatisch.

**Example (neues Template `templates/teilnehmerliste.typ`):**

```typst
// Source: typst.app/docs/reference/model/table/ + forum.typst.app/t/2374
// Available via sys.inputs:
//   meta.title (str)            -- GV-Name
//   meta.date  (str)            -- "15.05.2026"
//   meta.present (int)          -- X
//   meta.total (int or none)    -- Y, none bei include=present
//   rows (array of dict)        -- [{"member_number":N, "first_name":..., ...}]

#import "_layout.typ": letter

#let meta = json.decode(sys.inputs.at("meta"))
#let rows = json.decode(sys.inputs.at("rows"))

#show: letter.with(title: meta.title, date: meta.date)

#text(size: 12pt)[
  #if meta.at("total", default: none) != none [
    *#meta.present von #meta.total anwesend*
  ] else [
    *#meta.present anwesend*
  ]
]

#v(0.5cm)

#table(
  columns: (auto, 1fr, 1fr, auto, auto, auto),
  align: (right, left, left, left, left, center),
  stroke: 0.5pt,
  table.header(
    repeat: true,
    [*Nr.*], [*Nachname*], [*Vorname*], [*Anrede*], [*Titel*], [*anwesend*],
  ),
  ..rows.map(r => (
    [#r.member_number],
    [#r.last_name],
    [#r.first_name],
    [#r.at("salutation", default: "")],
    [#r.at("title", default: "")],
    if r.is_present [✓] else [],
  )).flatten()
)
```

### Pattern 3: CSV mit BOM + Semikolon (Service-Layer)

**What:** Manuelles `\xEF\xBB\xBF`-Prefix in `Vec<u8>`, dann `csv::WriterBuilder` mit Semikolon-Delimiter.

**Example:**

```rust
// Source: WebFetch docs.rs/csv/struct.WriterBuilder + community CSV-BOM-Excel pattern
use csv::WriterBuilder;
use std::io::Write;

fn render_csv(rows: &[AttendanceMemberRow], total: Option<u64>) -> Result<Vec<u8>, ServiceError> {
    // BOM für Excel-DE Auto-Detect (D-03)
    let mut buf: Vec<u8> = vec![0xEF, 0xBB, 0xBF];

    // Optional: Kopf-Zeile "X von Y anwesend" als erste CSV-Zeile, oder
    // nur Header-Row und Stats weglassen. Empfehlung: Stats NICHT in CSV
    // (Konsumenten erwarten saubere Header-Row-Struktur). Wer Stats braucht,
    // hat die PDF-Variante.

    let mut wtr = WriterBuilder::new()
        .delimiter(b';')
        .from_writer(buf);  // Wrappe direkt das BOM-Prefix-Vec

    wtr.write_record(&[
        "Mitgliedsnummer", "Nachname", "Vorname",
        "Anrede", "Titel", "anwesend",
    ])
    .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;

    for r in rows {
        wtr.write_record(&[
            r.member_number.to_string(),
            r.last_name.to_string(),
            r.first_name.to_string(),
            r.salutation.as_deref().unwrap_or("").to_string(),
            r.title.as_deref().unwrap_or("").to_string(),
            if r.is_present { "ja" } else { "nein" }.to_string(),
        ])
        .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;
    }

    wtr.into_inner()
        .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))
}
```

### Pattern 4: XLSX als `Vec<u8>` via `save_to_buffer`

**What:** `Workbook::save_to_buffer() -> Result<Vec<u8>, XlsxError>` ist der Standard-Pattern für HTTP-Streaming. [VERIFIED: WebFetch docs.rs/rust_xlsxwriter `save_to_buffer`]

**Example:**

```rust
// Source: docs.rs/rust_xlsxwriter/workbook/struct.Workbook.html#method.save_to_buffer
use rust_xlsxwriter::{Format, Workbook};

fn render_xlsx(rows: &[AttendanceMemberRow], _total: Option<u64>) -> Result<Vec<u8>, ServiceError> {
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Teilnehmer")
        .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;

    // Header-Row
    let headers = ["Mitgliedsnummer", "Nachname", "Vorname", "Anrede", "Titel", "anwesend"];
    for (col, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *h, &bold)
            .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;
    }

    // Daten-Rows
    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        sheet.write_number(row, 0, r.member_number as f64)
            .and_then(|s| s.write_string(row, 1, &r.last_name))
            .and_then(|s| s.write_string(row, 2, &r.first_name))
            .and_then(|s| s.write_string(row, 3, r.salutation.as_deref().unwrap_or("")))
            .and_then(|s| s.write_string(row, 4, r.title.as_deref().unwrap_or("")))
            .and_then(|s| s.write_string(row, 5, if r.is_present { "ja" } else { "nein" }))
            .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))?;
    }

    workbook.save_to_buffer()
        .map_err(|e| ServiceError::InternalError(Arc::from(e.to_string())))
}
```

### Pattern 5: REST-Handler mit Content-Disposition

**What:** Bestehender Helper `content_disposition_attachment` aus `genossi_rest/src/http_util.rs:43-50` setzt RFC-6266-konformen Header mit ASCII+UTF-8-Fallback.

**Example:**

```rust
// Source: genossi_rest/src/member_document.rs:256-263 (verified, production)
// Hat bereits den Pattern für PDF-Download — direkt übertragbar.

let date_str = assembly.date.date()
    .format(&time::format_description::parse("[year]-[month]-[day]").unwrap())
    .unwrap_or_else(|_| "unknown".into());
let filename = format!("gv-{}-teilnehmer.{}", date_str, extension);
let cd = crate::http_util::content_disposition_attachment(&filename);

Ok(Response::builder()
    .status(200)
    .header("Content-Type", mime_type)
    .header("Content-Disposition", &cd)
    .body(Body::from(bytes))
    .unwrap())
```

### Pattern 6: Frontend-Blob-Download via `web-sys`

**What:** Existiert bereits in `genossi-frontend/src/api.rs:497-548` (Pattern aus Template-PDF-Preview). 1:1 wiederverwendbar.

**Example (neu in `api.rs`):**

```rust
// Source: genossi-frontend/src/api.rs:497-548 (verified, production)
// Pattern liefert eine blob: URL, die das aufrufende Component an ein <a download="..."> hängt.

pub async fn export_attendance_url(
    config: &Config,
    assembly_id: Uuid,
    format: &str,       // "csv" | "pdf" | "xlsx"
    include: &str,      // "all" | "present"
) -> Result<String, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!(
        "{}/api/assembly/{assembly_id}/attendance-export/{format}?include={include}",
        config.backend
    );
    info!("Exporting attendance: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("GET");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    let resp: web_sys::Response = resp_value.dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;
    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    let blob: web_sys::Blob = blob.dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;
    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    Ok(blob_url)
}
```

**Component-Trigger** (im Page/Component):

```rust
// Anchor-Trick: programmatisch <a download> erzeugen und klicken
// (verbreitetes WASM-Pattern, weil `download`-Attribut auf navigations-induzierte
//  Antworten nicht zieht, wenn Content-Disposition vom Server kommt UND die URL
//  cross-origin oder eine blob:-URL ist).

spawn(async move {
    let cfg = CONFIG.read().clone();
    match api::export_attendance_url(&cfg, aid, "pdf", "all").await {
        Ok(blob_url) => {
            let document = web_sys::window().unwrap().document().unwrap();
            let a: web_sys::HtmlAnchorElement = document
                .create_element("a").unwrap()
                .dyn_into().unwrap();
            a.set_href(&blob_url);
            // Browser-suggested filename — wird vom Server-Content-Disposition
            // ÜBERSCHRIEBEN, falls vorhanden; hier nur Fallback.
            a.set_download(&format!("gv-{date}-teilnehmer.pdf"));
            a.click();
            // Cleanup
            let _ = web_sys::Url::revoke_object_url(&blob_url);
        }
        Err(e) => on_error.call(e.message),
    }
});
```

**Alternative — direkter `<a href>`-Link:** Funktioniert nur, wenn:
1. Die Session-Cookie cross-request-stable ist (im OIDC-Setup ja, weil tower-cookies+sessions).
2. Kein CSRF-Token im Request-Header gebraucht wird (im aktuellen Genossi-Setup: keine CSRF-Header).
3. Der Server `Content-Disposition: attachment` setzt, sonst öffnet das PDF inline.

→ Im konkreten Setup (Same-Origin, Session-Cookie, Server-Content-Disposition) wäre `<a href="/api/assembly/{aid}/attendance-export/pdf?include=all" download>` **die einfachste Variante** und käme ohne JS-Boilerplate aus. **Planner-Empfehlung: Direkt-Anker testen** — wenn Browser-Verhalten konsistent, dann Blob-Pattern überspringen.

### Anti-Patterns to Avoid

- **`SELECT m.*` im Export-DAO oder Erweitern von `AttendanceMemberRow` um PII-Felder:** Würde die 7-Spalten-DSGVO-Whitelist (`AttendanceMemberRow`-Doc-Kommentar: "**PII-Leak-Guard:** SELECT-Whitelist of exactly 7 columns") umgehen. Export wiederverwendet **ohne Modifikation** die bestehende DAO-Methode.
- **`audited_*!`-Macros im Export-Service:** D-17 + ATTN-05 + CLAUDE.md §"Audit Log System" — Export ist Read-Only, kein Audit. Hinzufügen würde die Hash-Chain mit non-mutating Events füllen und die "nur create/update/delete sind auditiert"-Linie brechen.
- **Bypass des Status-Gates im Frontend ohne Backend-Check:** D-11 muss **server-seitig** erzwungen werden. Frontend-`if status == Closed`-Visibility ist nur UX, kein Sicherheits-Gate.
- **Hand-rolled XLSX (ZIP+XML zusammenbauen):** Vorhandenes `rust_xlsxwriter` löst das Problem. Eigenes Zusammenbauen würde Edge-Cases (Format-Strings, Excel-Quirks bei deutschen Umlauten in Strings) öffnen, die der Maintainer von `rust_xlsxwriter` (gleicher Author wie Python-XlsxWriter, 10+ Jahre Domain-Wissen) bereits gelöst hat.
- **Inline-RSX-Duplikate im Frontend für den Export-Button:** Wenn der Block tatsächlich auf mehr als einer Seite landet, Komponente extrahieren (D-20 + CLAUDE.md §Component-First). Aktuell vermutlich nur EINE Seite — dann inline OK.
- **`#[derive(Default)]` auf einer neuen Export-Entity:** Es entstehen keine neuen DAO-Entities in Phase 6. Falls doch (z. B. ein `ExportRequestTO`), nicht `Default` derivieren — Genossi-Konvention wegen Audit-Konsistenz.
- **`unwrap_or_default()` in Audit-Feldern:** Hier irrelevant (Phase 6 schreibt nichts), aber als Anti-Pattern dokumentiert in `assembly.rs:67-83` mit "WR-08: do NOT use `unwrap_or_default()` here — a silent empty string in the audit log is forensically useless".

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Excel-XLSX-Datei erzeugen | Eigener ZIP+XML-Generator | `rust_xlsxwriter::Workbook::save_to_buffer()` | XLSX = ZIP-Container mit ~10 XML-Dateien (Workbook.xml, Worksheets, SharedStrings, Styles, ContentTypes, Relationships). Hand-Roll erzeugt mit hoher Wahrscheinlichkeit Excel-Warnungen ("File needs to be recovered"). |
| CSV-Escaping (Quote, Komma in Feldern, CRLF) | Eigene `format!()`-Logik | `csv` crate 1.3 (`WriterBuilder`) | RFC-4180-konformes Escaping; deutsche Namen mit "von der Heyden, Kurt" und Newlines in Adressen sind klassische Bug-Quellen. |
| Content-Disposition-Filename mit Umlauten | Eigene `format!`-String-Konkatenation | Bestehender `http_util::content_disposition_attachment` (`genossi_rest/src/http_util.rs:43-50`) | RFC-6266 fordert ASCII-Fallback **und** `filename*=UTF-8''`-Variante — Helper erledigt beides. |
| PDF-Generierung | Eigener PDF-Writer mit `printpdf` | Bestehender `PdfGenerator` mit Typst | Toolchain inkl. eingebetteter Fonts und Package-Cache ist produktiv erprobt (`pdf_generation.rs`, Tests 100% durch). |
| Permission-Check für Vorstand-only | Eigene `if user.is_admin {}`-Logik | Bestehender `PermissionService::check_permission("admin", ctx)` | Pattern aus `attendance.rs:109-111`; OIDC-Integration + Mock-Auth-Toggle bereits gelöst. |
| GV-Status-Lookup | Eigene SQL-Query | Bestehender `AssemblyDao::find_by_id` | Plus konsistentes Soft-Delete-Filtering. |
| Blob-Download im Browser | DataURI mit base64-encoded Body | Bestehender `web-sys::Url::create_object_url_with_blob`-Pattern aus `api.rs:537-548` | Base64-DataURIs überschreiten oft Browser-Limits bei 1000-Mitglieder-Excel-Dateien. Blob-URL ist Standard. |
| RFC-3161-Timestamp für PDF | Eigenes ASN.1-Encoding | NICHT BENÖTIGT (D-08 + D-17 — kein Audit-Timestamp im Export) | Phase-6-Exports sind reine Ableitungen aus auditierten Datenquellen; das Quell-Audit reicht für Nachvollziehbarkeit. |

**Key insight:** Phase 6 hat einen ungewöhnlich hohen "Don't hand-roll"-Faktor, weil **alle erforderlichen Teile bereits im Codebase liegen oder als reife Crates vorhanden sind**. Die Plan-Risiko-Komponente liegt nicht in der Technologie, sondern in (a) korrekter Verdrahtung von Permission-Funnel + Status-Gate (sicherheitskritisch) und (b) der **`rust_xlsxwriter` dev-dep → produktiv-dep Promotion**, die nicht vergessen werden darf.

## Common Pitfalls

### Pitfall 1: Status-Gate nur im Frontend, nicht im Backend

**What goes wrong:** Vorstand klickt UI-Button, der nur sichtbar ist wenn `assembly.status == Closed`. Backend liefert den Export aber auch für `Open`-GVs aus.

**Why it happens:** Frontend-Visibility-Logic in Pages ist verlockend einfach (`if status == Closed { rsx!{...} }`) und versteckt den Backend-Bug, bis ein User mit cURL den Endpoint direkt aufruft.

**How to avoid:** D-11 server-seitig durchsetzen, **bevor** ein Byte gerendert wird. Pattern wie in `helper_token.rs:376`:

```rust
if assembly.status != AssemblyStatus::Closed {
    return Err(ServiceError::Conflict(Arc::from("assembly_not_closed")));
}
```

**Warning signs:** Tests fehlen für `Open`/`Preparation`-Aufrufe gegen den Export-Endpoint mit erwarteter 409.

### Pitfall 2: BOM doppelt eingefügt durch `csv` crate

**What goes wrong:** Manuelles BOM-Prefix-`Vec` an `WriterBuilder::from_writer(...)` gibt — aber spätere `csv`-Schreibvorgänge schreiben sauber davor weiter; ODER man wickelt den CSV-Output noch mal in eine BOM-präfixierte Vec ein und schreibt das BOM doppelt.

**Why it happens:** `csv` crate hat **keine eingebaute BOM-Unterstützung** [CITED: docs.rs/csv]. Es ist Aufgabe des Aufrufers — und das passiert leicht falsch.

**How to avoid:** Einen einzigen `Vec<u8>` mit BOM initialisieren, in `WriterBuilder::from_writer(buf)` reingeben, und `wtr.into_inner()` zurückgeben. Test: erste 3 Bytes sind `[0xEF, 0xBB, 0xBF]` und nicht 6.

**Warning signs:** Excel zeigt `ï»¿Mitgliedsnummer` in der ersten Zelle (BOM falsch interpretiert) oder zwei BOMs in den ersten 6 Bytes.

### Pitfall 3: Typst-Template-Lookup-Pfad falsch konfiguriert

**What goes wrong:** Neue Datei `templates/teilnehmerliste.typ` wird vom `PdfGenerator` nicht gefunden, weil `template_base` auf einen anderen Pfad zeigt.

**Why it happens:** `PdfGenerator::render` nimmt `template_base: &Path` als Parameter; dieser wird vom Caller bestimmt (z. B. `rest_state.template_storage().base_path()` in `member_document.rs:359`). Es gibt zwei mögliche Roots: `./templates/` (Default-Layout für Member) und das `TemplateStorage`-konfigurierbare Verzeichnis.

**How to avoid:** Das bestehende `template_storage().base_path()`-Pattern nutzen, und das neue Template **in der defaults-Hierarchie** ablegen (`templates/defaults/teilnehmerliste.typ` + ggf. nicht-defaults-Override, je nach Konvention). E2E-Test verifiziert, dass `PdfGenerator::render` auf `teilnehmerliste.typ` ein `%PDF-`-Byte-Prefix zurückgibt.

**Warning signs:** `TemplateError::NotFound` im Service-Layer, mappt zu RestError::NotFound (was als 404 in der API rausgeht und vom Frontend als "Nicht gefunden" angezeigt wird — verwirrend, wenn die GV definitiv existiert).

### Pitfall 4: `PdfGenerator::render` ist auf `Member` typisiert — neue Methode nötig

**What goes wrong:** Die bestehende `render`-Methode nimmt `&Member`. `render_application` nimmt `&Application`. Beide sind type-spezifisch. Für die Teilnehmerliste gibt es keine passende Methode.

**Why it happens:** Das Pattern in `pdf_generation.rs:153-204` und `:207-254` ist nicht generic — jede neue Entität braucht eine eigene `render_xxx` + `build_inputs_xxx`.

**How to avoid:** **Neue Methode** `render_attendance_list(&self, template_path, template_base, assembly, rows, total)` hinzufügen, ODER eine generic `render_with_inputs(&self, template_path, template_base, inputs: Dict)` einführen, die alle drei Use-Cases bedient. Empfehlung: **generic Methode**, weil das den dritten Boilerplate-Block (1080 → +60 Zeilen) vermeidet und für künftige Phasen (z. B. Zahlungsanfrage-Erweiterungen) wiederverwendbar ist.

**Warning signs:** `render` und `render_application` beim Hinzufügen von `render_attendance_list` werden Copy-Paste-erweitert — Refactor-Schuld.

### Pitfall 5: `rust_xlsxwriter` chained Builder-API verwirrt mit Lifetime

**What goes wrong:** `worksheet.write_*` gibt `&mut Worksheet` zurück; bei Chains wie `sheet.write_string(...).and_then(|s| s.write_number(...))` greift man auf borrowed `s`. Bei vielen Spalten wird der Code unleserlich.

**Why it happens:** Der API ist auf Method-Chaining ausgelegt, aber `?`-Propagation funktioniert direkt; man muss nicht `and_then` verwenden.

**How to avoid:** Pro Zeile `?`-Style mit Variablen verwenden, nicht `and_then`:

```rust
sheet.write_number(row, 0, r.member_number as f64)?;
sheet.write_string(row, 1, &r.last_name)?;
sheet.write_string(row, 2, &r.first_name)?;
// etc.
```

**Warning signs:** Verschachtelte `and_then`-Closures, in denen der ?-Operator nicht funktioniert (weil der Closure-Body `Result` zurückgeben muss).

### Pitfall 6: `assembly.date` ist `PrimitiveDateTime`, nicht `Date` — Datums-Formatierung

**What goes wrong:** Filename `gv-{YYYY-MM-DD}-teilnehmer.pdf` benötigt einen reinen Date-String. `assembly.date` ist aber `PrimitiveDateTime` (siehe `assembly.rs:48`).

**Why it happens:** Genossi speichert GV-Datum mit Time-Komponente (vermutlich für Future-Zeit-Display "GV 15.05.2026 19:00 Uhr"), aber für den Filename ist die Zeit irrelevant.

**How to avoid:** `assembly.date.date()` → `time::Date`, dann mit `format_description::parse("[year]-[month]-[day]")` formatieren. Pattern existiert in `member_document.rs` und `pdf_generation.rs:333,460`:

```rust
let fmt = time::format_description::parse("[year]-[month]-[day]")
    .expect("valid format");
let date_str = assembly.date.date().format(&fmt)
    .unwrap_or_else(|_| "unknown".into());
```

**Warning signs:** Filename enthält Time-Komponente (`gv-2026-05-15T19:00:00-...`) oder Filename ist leer/Default.

### Pitfall 7: `web-sys::Url` Object-URL-Leak im Frontend

**What goes wrong:** Jeder `create_object_url_with_blob` belegt Browser-Memory, bis der Tab geschlossen oder `revoke_object_url` aufgerufen wird. Bei häufigem Export-Klick → Memory-Leak.

**Why it happens:** Browser garantieren erst beim Document-Lifecycle-Ende Cleanup. WASM-Apps mit langer Lebensdauer (SPA-Routing) leaken pro Download eine Blob-URL.

**How to avoid:** Nach dem `.click()` direkt `web_sys::Url::revoke_object_url(&blob_url)` aufrufen. Snippet in Pattern 6 enthalten.

**Warning signs:** Performance-Test: 50× Export klicken → Browser-Memory wächst monoton.

### Pitfall 8: `rust_xlsxwriter::Workbook` ist NICHT `Send`

**What goes wrong:** `Workbook` und `Worksheet` halten internen Mutable State (Rc?), kreuzen Tokio-`await`-Grenzen nicht. Wenn man die XLSX-Erzeugung in eine async-Funktion packt, die `await`s zwischen Workbook-Ops macht → Compile-Error.

**Why it happens:** XLSX-Schreibung ist inhärent sequentiell + CPU-bound. Kein Vorteil durch async.

**How to avoid:** Den gesamten XLSX-Block in einem synchronen Funktionsblock kapseln. Wenn der Service async ist (was er ist, weil Trait `AttendanceExportService` mit async trait): den synchronen Render in eine `tokio::task::spawn_blocking` packen, ODER (einfacher) den gesamten render-Block synchron durchziehen, ohne `.await`. Die DAO-Calls sind ja vor dem Render abgeschlossen.

**Warning signs:** Compile-Fehler "future is not Send" mit Hinweis auf `Workbook`.

## Runtime State Inventory

Phase 6 ist **kein Rename/Refactor/Migration**, sondern reines Feature-Add. Diese Sektion wäre eigentlich entbehrlich, aber zur Vollständigkeit:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — verified by grep. Phase 6 schreibt nichts in DB, nichts in ChromaDB, nichts in Caches. | None |
| Live service config | Keine externen Services involviert (kein n8n, kein Datadog für genossi3). | None |
| OS-registered state | Keine pm2/systemd/Task-Scheduler-Einträge berührt. | None |
| Secrets/env vars | Keine neuen Env-Vars nötig — bestehender OIDC-Pfad reicht. `TYPST_PACKAGE_CACHE` (siehe `pdf_generation.rs:29`) ist bereits konfigurierbar; das neue Template benötigt keinen Package-Import. | None |
| Build artifacts | `rust_xlsxwriter` Promotion von dev-dep zu reguläre Dep bewirkt: Beim ersten `cargo build` wird die Crate zusätzlich in den Release-Build aufgenommen (ca. +800 KB Binary, basierend auf `zip` dep). | Nach Promotion einmal `cargo build --release` für Verifikation. |

**Canonical question:** *Nach Phase-6-Implementierung — was hat sich in der Laufzeit-Welt verändert?* Antwort: Nichts außer einer um ein paar Endpoints reicheren REST-API und einem etwas größeren Binary. Keine Migrations, keine neuen Cron-Jobs, keine neuen Worker, keine neuen Tabellen.

## Code Examples

### Beispiel: Vollständiger Service-Method-Skelett

```rust
// Source: composite aus genossi_service_impl/src/attendance.rs (Service-Pattern),
// member_document.rs (Render-Pattern), und CONTEXT.md D-01..D-20.

use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::attendance::{AttendanceDao, AttendanceMemberRow};
use genossi_dao::TransactionDao;
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::ServiceError;

use crate::gen_service_impl;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat { Csv, Pdf, Xlsx }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportInclude { All, Present }

pub struct ExportResult {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
    pub filename: String,
}

gen_service_impl! {
    struct AttendanceExportServiceImpl: AttendanceExportService = AttendanceExportServiceDeps {
        AttendanceDao: AttendanceDao<Transaction = Self::Transaction> = attendance_dao,
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        // Nicht generic, weil PdfGenerator stateless ist:
        // PdfGenerator wird per Arc reingegeben, kein DAO-Trait.
    }
}

#[async_trait]
impl<Deps: AttendanceExportServiceDeps> AttendanceExportService for AttendanceExportServiceImpl<Deps> {
    type Context = Deps::Context;
    type Transaction = Deps::Transaction;

    async fn export(
        &self,
        assembly_id: Uuid,
        format: ExportFormat,
        include: ExportInclude,
        context: Authentication<Self::Context>,
    ) -> Result<ExportResult, ServiceError> {
        let tx = self.transaction_dao.use_transaction(None).await?;

        // D-13: Admin-Only Funnel + D-11: Closed-Status-Gate.
        let assembly = self.check_admin_and_closed(assembly_id, context, tx.clone()).await?;

        // D-05: Datenquelle ist die bestehende Attendance-DAO-Methode.
        let mut rows = self.attendance_dao
            .list_members_for_assembly(assembly_id, None, tx.clone())
            .await?
            .to_vec();

        // D-09: Filter nach include.
        if matches!(include, ExportInclude::Present) {
            rows.retain(|r| r.is_present);
        }

        self.transaction_dao.commit(tx).await?;

        // Stats für PDF-Header (D-08, D-10)
        let present = rows.iter().filter(|r| r.is_present).count() as u64;
        let total = match include {
            ExportInclude::All => Some(rows.len() as u64),
            ExportInclude::Present => None, // D-10: kein Y bei present-only
        };

        // D-15: Filename-Schema gv-{YYYY-MM-DD}-teilnehmer.{ext}
        let fmt = time::format_description::parse("[year]-[month]-[day]")
            .expect("static format");
        let date_str = assembly.date.date().format(&fmt)
            .unwrap_or_else(|_| "unknown".into());

        let (bytes, mime, ext) = match format {
            ExportFormat::Csv => (
                render_csv(&rows)?,
                "text/csv; charset=utf-8",
                "csv",
            ),
            ExportFormat::Xlsx => (
                render_xlsx(&rows)?,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "xlsx",
            ),
            ExportFormat::Pdf => (
                self.render_pdf(&assembly, &rows, present, total)?,
                "application/pdf",
                "pdf",
            ),
        };

        Ok(ExportResult {
            bytes,
            mime,
            filename: format!("gv-{}-teilnehmer.{}", date_str, ext),
        })
    }
}
```

### Beispiel: REST-Handler

```rust
// Source: composite aus genossi_rest/src/attendance.rs + member_document.rs

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Attendance Export",
    path = "/api/assembly/{assembly_id}/attendance-export/{format}",
    params(
        ("assembly_id" = Uuid, Path, description = "Assembly ID (must be in status Closed)"),
        ("format" = String, Path, description = "csv | pdf | xlsx"),
        ExportQuery,
    ),
    responses(
        (status = 200, description = "Export file (binary)", content_type = "application/octet-stream"),
        (status = 400, description = "Unknown format or invalid include parameter"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden — not admin"),
        (status = 404, description = "Assembly not found"),
        (status = 409, description = "Assembly is not in status Closed"),
    ),
)]
pub async fn export_attendance<RestState: RestStateDef + AttendanceExportRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((assembly_id, format_str)): Path<(Uuid, String)>,
    Query(query): Query<ExportQuery>,
) -> Response {
    error_handler((async {
        let auth = crate::extract_auth_context(Some(context))?;

        let format = match format_str.as_str() {
            "csv" => ExportFormat::Csv,
            "pdf" => ExportFormat::Pdf,
            "xlsx" => ExportFormat::Xlsx,
            other => return Err(RestError::BadRequest(format!("Unknown format: {}", other))),
        };
        let include = query.include.unwrap_or(ExportInclude::All);

        tracing::info!(
            assembly_id = %assembly_id,
            format = ?format,
            include = ?include,
            "Exporting attendance list" // D-18
        );

        let result = rest_state
            .attendance_export_service()
            .export(assembly_id, format, include, auth)
            .await?;

        let cd = crate::http_util::content_disposition_attachment(&result.filename);

        Ok(Response::builder()
            .status(200)
            .header("Content-Type", result.mime)
            .header("Content-Disposition", &cd)
            .body(Body::from(result.bytes))
            .unwrap())
    }).await)
}
```

### Beispiel: E2E-Test (PDF-Magic-Bytes)

```rust
// Pattern aus genossi_bin/tests/e2e_tests.rs (existing setup() helper)
// Magic-Bytes:
//   PDF:  %PDF- (bytes 0x25 0x50 0x44 0x46 0x2D)
//   XLSX: PK\x03\x04 (ZIP-Container, 0x50 0x4B 0x03 0x04)
//   CSV:  BOM präfix 0xEF 0xBB 0xBF (D-03)

#[tokio::test]
async fn test_export_pdf_closed_assembly_returns_pdf_bytes() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // 1) Assembly anlegen, öffnen, schließen (Pattern aus bestehenden Tests)
    let aid = create_open_close_assembly_with_members(&client, &server).await;

    // 2) Export aufrufen
    let resp = client
        .get(server.url(&format!("/api/assembly/{aid}/attendance-export/pdf?include=all")))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "application/pdf");
    assert!(
        resp.headers()["content-disposition"]
            .to_str().unwrap()
            .contains(r#"filename=""#)
    );

    // 3) Magic-Bytes prüfen
    let bytes = resp.bytes().await.unwrap();
    assert!(bytes.starts_with(b"%PDF-"), "expected PDF magic bytes, got {:?}", &bytes[..8]);
}

#[tokio::test]
async fn test_export_csv_starts_with_bom_and_uses_semicolon() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let aid = create_open_close_assembly_with_members(&client, &server).await;

    let resp = client
        .get(server.url(&format!("/api/assembly/{aid}/attendance-export/csv?include=all")))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.bytes().await.unwrap();
    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF], "expected UTF-8 BOM at start");

    // Skip BOM, parse rest as UTF-8 string, check semicolon delimiter
    let body = std::str::from_utf8(&bytes[3..]).unwrap();
    let first_line = body.lines().next().unwrap();
    assert!(first_line.contains(';'), "expected semicolon delimiter, got: {}", first_line);
    assert!(!first_line.contains(','), "expected NO comma delimiter");
}

#[tokio::test]
async fn test_export_xlsx_returns_zip_container() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let aid = create_open_close_assembly_with_members(&client, &server).await;

    let resp = client
        .get(server.url(&format!("/api/assembly/{aid}/attendance-export/xlsx?include=all")))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.bytes().await.unwrap();
    // XLSX is a ZIP container — magic bytes PK\x03\x04
    assert_eq!(&bytes[..4], b"PK\x03\x04", "expected ZIP magic bytes (XLSX container)");
}

#[tokio::test]
async fn test_export_open_assembly_returns_409_conflict() {
    let server = setup().await;
    let client = reqwest::Client::new();
    // Helper: assembly opened, but NOT closed
    let aid = create_open_assembly_with_members(&client, &server).await;

    let resp = client
        .get(server.url(&format!("/api/assembly/{aid}/attendance-export/pdf?include=all")))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body = resp.text().await.unwrap();
    assert!(body.contains("assembly_not_closed"));
}

#[tokio::test]
async fn test_export_unknown_format_returns_400() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let aid = create_open_close_assembly_with_members(&client, &server).await;

    let resp = client
        .get(server.url(&format!("/api/assembly/{aid}/attendance-export/json?include=all")))
        .send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
```

**Test-Infrastruktur:**
- `setup()` aus `genossi_bin/tests/e2e_tests.rs:24-38` — startet in-memory SQLite + Test-Server. Direkt wiederverwendbar.
- `start_test_server` aus `genossi_rest/src/test_server.rs` — bindet auf zufälligen Port, droppt am Test-Ende. Trait-Bounds in `test_server.rs:18-26` müssen evtl. um `AttendanceExportRestState` erweitert werden.
- Mock-Auth-Mode (Feature `mock_auth`) ist Default → `Authentication::Context(MockContext)` reicht für Permission-Test.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `xlsxwriter` (libxlsxwriter C-FFI) | `rust_xlsxwriter` (Pure Rust, gleicher Author) | Verbreitet seit ~2022; in 2026 produktiv erprobt mit 1.6 Mio Downloads | Keine C-Toolchain in Nix-Build nötig, MSRV stabil, gleiche API-Familie wie Python `XlsxWriter` |
| Hand-crafted PDF mit `printpdf` | Typst (`typst`/`typst-pdf`) | Im Genossi-Codebase eingeführt mit `pdf_generation.rs` (siehe Phase 0/1) | Templates sind menschlich lesbar (`.typ`-Dateien), Vorstand kann sie editieren (siehe Template-Editor in `genossi_service_impl`) |
| WASM-Frontend mit DataURI-base64 | Blob-URL + `Url::create_object_url_with_blob` + Anchor-Click | Standard seit IE10 (2013), in 2026 universell unterstützt | Skaliert für große Dateien (10 MB+); base64-DataURIs sind in vielen Browsern auf wenige MB begrenzt |
| `csv` v0.x (BurntSushi initial) | `csv` 1.3 (stabilisierte API) | API frozen seit ~2018 | Keine Breaking Changes; sicher in Workspace zu nutzen |
| `Workbook::save()` (File-System) | `Workbook::save_to_buffer()` → `Vec<u8>` | Standard für HTTP-Streaming | Kein temporäres File nötig, schneller, kein Cleanup-Code |

**Deprecated/outdated:**
- Hand-rolling XLSX als ZIP-mit-XML — gibt es im Rust-Ökosystem noch in Form von "OOXML"-Tutorials, ist aber durch `rust_xlsxwriter` obsolet.
- `Workbook::save(...)` ins Filesystem schreiben und danach lesen — funktioniert, ist aber für Web-APIs unnötig. Nutze `save_to_buffer`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `rust_xlsxwriter` MSRV ist mit Genossi-Toolchain kompatibel (Rust 2021, kein nightly-Feature) | Standard Stack | Compile-Fehler beim Promotion; Workaround: Version 0.82 ist sicher (bereits gebaut in `Cargo.lock`), nur Upgrade auf 0.94/0.95 nicht-blind durchführen. |
| A2 | Genossi-Genossenschaft hat <500 aktive Mitglieder (Pagination im Export nicht nötig) | Pattern 2 / 4 | Bei 5000+ Mitgliedern wird die PDF-Erzeugung 5-10s dauern — User-Feedback nötig, evtl. Background-Job. Validierung: `SELECT COUNT(*) FROM member WHERE deleted IS NULL` in der konkreten Genossi-Instanz. |
| A3 | OIDC-Cookie wird vom Browser bei `<a href="">`-Navigation automatisch mitgeschickt (Same-Origin-Setup) | Pattern 6 | Wenn Cookie-`SameSite=Strict` oder Cross-Origin: direkter Anker-Link würde 401 liefern, dann nur Blob-Pattern bleibt. Test: einen Auth-Flow durchspielen + manuell auf `/api/assembly/.../export/pdf` navigieren. |
| A4 | `text/csv; charset=utf-8` ist ein akzeptabler MIME für deutsche Excel-Installationen | Pattern 5 | Excel öffnet auch `application/octet-stream` mit `.csv`-Extension. Falls Excel-DE den MIME ignoriert und nach Sniffing geht: kein Risiko. |
| A5 | Neue Methode `render_attendance_list` in `PdfGenerator` ist die saubere Erweiterung (gegenüber Generic `render_with_inputs`) | Pitfall 4 | Generic-Variante ist eleganter, aber refactor-aufwendiger (bestehende `render`/`render_application` müssten umgebaut werden). Pragmatisch: copy-paste-Variant ist 50 Zeilen Boilerplate, dafür risk-free. |
| A6 | Status-Wert `AssemblyStatus::Closed` ist der einzige für D-11 zugelassene Wert (kein `Cancelled`/`PostClosed` o.ä.) | Pattern 1 | Enum hat aktuell nur `Preparation`, `Open`, `Closed` (siehe `assembly.rs:10-14`) — verifiziert. Wenn das Enum erweitert wird, muss der Funnel angepasst werden. |
| A7 | `csv` 1.3 ohne Feature-Flags ist ausreichend (kein `serde`-Feature nötig, weil wir keine Structs serialisieren) | Standard Stack | Stimmt — wir schreiben Slices von `&str`. |

**Wichtig:** A1, A2 und A3 sollten **vor Plan-Wave-Start** mit dem User abgestimmt werden:
- A1: Soll `rust_xlsxwriter` auf 0.94/0.95 gebumpt werden, oder bei 0.82 bleiben?
- A2: Wie viele Mitglieder hat die produktive Genossenschaft heute?
- A3: Im OIDC-Mode — funktioniert ein direkter `<a href="/api/...">`-Klick und löst einen authentifizierten Download aus?

## Open Questions

1. **`rust_xlsxwriter` Version: 0.82 (bereits in Lock) vs 0.94/0.95 (aktuelle Stable)?**
   - What we know: 0.82 ist bereits in `Cargo.lock`, also gebaut + binär OK. 0.94/0.95 sind aktuelle Stable mit gleicher API-Form.
   - What's unclear: Gibt es seit 0.82 API-Changes, die die Service-Implementierung beeinflussen? (Schwankungen sind oft Format/Style-Erweiterungen — die Core-API `Workbook::new()` / `add_worksheet()` / `write_string()` / `save_to_buffer()` ist stabil seit ~0.40.)
   - Recommendation: **Bei 0.82 bleiben**, weil die Crate bereits in Lock ist und kein Bump-Anlass besteht. Späterer Bump (z. B. bei Sicherheits-Advisory in `zip` 2.4.2) ist trivial.

2. **`render_attendance_list` vs Generic `render_with_inputs` in `PdfGenerator`?**
   - What we know: Bestehender Pattern ist type-spezifisch (`render` für Member, `render_application` für Application).
   - What's unclear: Lohnt sich der Refactor zu generic genau jetzt?
   - Recommendation: **Pragmatisch type-spezifische Methode hinzufügen** (`render_attendance_list`), Generic-Refactor in eine spätere Phase verschieben. Erspart Risiko an bestehenden produktiven Endpoints.

3. **Default-Wert für `?include`-Query-Parameter?**
   - What we know: D-09 erlaubt Planner-Entscheidung. Empfehlung im CONTEXT: `all`.
   - What's unclear: Erwartung des Verbands — wollen die "alle Mitglieder mit Anwesenheits-Markierung" oder "nur die Anwesenden"?
   - Recommendation: **Default `all`** — der Verband bekommt damit eine vollständige Mitgliederliste mit Anwesenheits-Spalte; gleichzeitig ist `?include=present` ein Opt-In für die kompakte Variante.

4. **Anwesenheits-Spalte: `"ja"`/`"nein"` (CSV/XLSX) und `✓`/leer (PDF) — konsistent?**
   - What we know: D-09 lässt das offen. Excel/CSV-DE-User erwarten Textwerte.
   - What's unclear: Sortierbarkeit in Excel — `"ja"`/`"nein"` sortiert alphabetisch ("ja" vor "nein"), `1`/`0` numerisch (passender). Verband-Konvention?
   - Recommendation: **Strings `"ja"`/`"nein"` in CSV+XLSX**, Glyph `✓` in PDF. Konsistent mit Excel-DE-Konvention.

5. **Frontend-Download: direkter `<a href>` vs `web-sys`-Blob-Pattern?**
   - What we know: Bestehendes Pattern in `api.rs` nutzt Blob für PDFs. Aber bei einem reinen Auth-Cookie-Setup ist `<a href>` einfacher.
   - What's unclear: Ob der `<a download>` korrekt mit dem Cookie funktioniert (Test nötig).
   - Recommendation: **Direkten Anker-Link versuchen** (3 Zeilen Code, kein wasm-bindgen), Fallback auf Blob falls problematisch.

## Environment Availability

Phase 6 introduziert keine neuen System-Dependencies — Tools sind alle Cargo-managed:

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain (≥1.70) | Build aller Crates | ✓ | wie bisher | — |
| `cargo` | Build | ✓ | wie bisher | — |
| `rust_xlsxwriter` 0.82 | XLSX-Export | ✓ (in Cargo.lock) | 0.82.0 | Bei Compile-Problem: Pin auf bestehende Version, kein Upgrade |
| `csv` 1.3 | CSV-Export | ✓ (in mehreren `[dependencies]`) | 1.3.x | — |
| `typst` + `typst-pdf` 0.14 | PDF-Export | ✓ (Workspace-Default) | 0.14 | — |
| Liberation-Sans-Fonts | PDF-Text | ✓ (embedded via `include_bytes!` in `pdf_generation.rs:15-20`) | — | — |
| SQLite + bestehende Migrations | DB-Zugriff | ✓ | wie bisher | — |
| `sqlx-cli` (für Tests) | Migrationen | ✓ | wie bisher | — |
| Dioxus + `dx serve` | Frontend-Build | ✓ | 0.6.3 | — |
| Nix-Toolchain (optional, dev) | Reproduzierbare Dev-Env | ✓ | wie bisher (flake.nix vorhanden) | — |

**Keine fehlenden Dependencies — alles bereits verfügbar.**

## Security Domain

> `security_enforcement` ist nicht explizit gesetzt in `.planning/config.json`. Default-Behandlung: Sicherheits-Aspekte explizit machen.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture | yes | Layered Architecture (DAO/Service/REST/Frontend) ist Genossi-Konvention |
| V2 Authentication | yes | OIDC via Nextcloud (bestehender Pfad in `auth_middleware.rs`) — Export-Endpoint nutzt denselben Middleware-Pfad |
| V3 Session Management | yes | tower-sessions / tower-cookies bestehend; Export-Aufruf prüft Session via Middleware |
| V4 Access Control | yes | `PermissionService::check_permission("admin", ctx)` — bestehender Mechanismus, ergänzt um D-11 Status-Gate |
| V5 Input Validation | yes | Path-Parameter `format` muss Whitelist sein (`csv` / `pdf` / `xlsx`); `?include`-Query-Parameter ebenfalls Whitelist (`all` / `present`); Pattern aus `helper_token.rs` für 400 bei unbekannten Werten |
| V6 Cryptography | no | Keine Krypto-Logik in Phase 6 — Export ist Plain-Text-Datei |
| V7 Error Handling | yes | Konsistentes ServiceError→RestError-Mapping; **NICHT** Stack-Traces o.ä. ans Frontend leaken |
| V8 Data Protection | yes | DSGVO-Whitelist im DAO; siehe Pitfall "PII-Leak via Whitelist-Erweiterung" |
| V9 Communication | yes | HTTPS via Reverse-Proxy (bestehend); Content-Type sauber gesetzt (kein `*/*`) |
| V10 Malicious Code | n/a | Keine User-Input-Verarbeitung (Filename ist server-generiert) |
| V11 Business Logic | yes | D-11 Status-Gate; D-13 Admin-only |
| V12 Files & Resources | yes | Filename-Schema deterministisch (kein User-Input → kein Path-Traversal-Risiko) |
| V13 API & Web Services | yes | OpenAPI-Doku via Utoipa; Path-Parameter validated |
| V14 Configuration | n/a | Keine neuen Config-Werte |

### Known Threat Patterns for genossi3 stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| PII-Leak durch erweiterte Spalten-Whitelist (z. B. zukünftiges `email`-Feld in `MemberEntity` wandert in `AttendanceMemberRow`) | Information Disclosure | DAO-Query ist explizit `SELECT m.id, m.member_number, m.first_name, ...` (siehe `attendance.rs:124-127`) — Erweiterung erfordert bewusste Code-Änderung in der Query. **TODO:** Test, der die 7-Feld-Whitelist im `AttendanceMemberRow`-Struct prüft (existiert bereits: `attendance.rs:155-171`). |
| Status-Bypass: User ruft Export-Endpoint mit `Open`-GV auf und bekommt vorzeitig die Liste | Tampering / Elevation of Privilege | D-11 server-seitig durchsetzen via `check_admin_and_closed`-Funnel; E2E-Test mit `Open`-Status erwartet 409 |
| Helfer-Token-Bypass: Helfer-Token in Header wird vom Funnel ignoriert, aber Helper-Branch fehlt — was passiert? | Elevation of Privilege | Der Funnel ruft `permission_service.check_permission("admin", ctx)` — wenn Helfer-Token keine `admin`-Privilege hat (was beabsichtigt ist), liefert das `PermissionDenied` → 403. Test: Helfer-Session → 403. |
| Cross-Site-Request-Forgery beim Export | Tampering | GET-Requests sind CSRF-resistent **wenn** der Server keine state-changing Operationen auf GET ausführt. Export ist read-only ✓. SameSite-Cookie-Policy via tower-sessions ist Default `Lax` (im Code zu verifizieren). |
| Filename-Injection (Path-Traversal via Filename) | Tampering | Filename wird server-seitig aus `assembly.date` deterministisch gebildet; kein User-Input fließt in den Filename. `content_disposition_attachment`-Helper sanitisiert zusätzlich (`http_util.rs:53-63`). |
| Resource-Exhaustion via riesigen XLSX (1 Mio Zellen) | Denial of Service | Datenbasis ist `assembly_member_snapshot` — gebunden an Mitgliederzahl der Genossi (heute <500). Kein User-controllierter Multiplier. Mitigation: keine speziellen Maßnahmen nötig in Phase 6. |
| Audit-Bypass: Export wird nicht auditiert, Vorstand exfiltriert Liste unbemerkt | Repudiation | **Bewusste Akzeptanz (D-17)** — Genossi-Policy: nur create/update/delete auditieren. Wenn ein "Export-Audit-Log" später gefordert wird, ist das eine eigene Phase. `tracing::info!` (D-18) liefert wenigstens operative Sichtbarkeit. |

## Sources

### Primary (HIGH confidence — Codebase-Inspektion, direkt verifiziert)

- `genossi_dao/src/attendance.rs:40-53,55-128` — `AttendanceMemberRow` 7-Feld-Whitelist + `AttendanceDao::list_members_for_assembly`
- `genossi_dao_impl_sqlite/src/attendance.rs:107-162` — SQLite-Query mit SELECT-Whitelist und JOIN snapshot+attendance
- `genossi_service_impl/src/attendance.rs:79-115` — `check_assembly_access`-Funnel als Vorbild für den Export-Funnel
- `genossi_service_impl/src/pdf_generation.rs:1-1080` — `PdfGenerator`-Toolchain, `TemplateWorld`, `build_inputs`-Pattern, Test-Pattern mit `%PDF-`-Magic-Bytes
- `genossi_rest/src/attendance.rs:42-235` — Handler-Pattern + `map_attendance_error` + Router-Wiring
- `genossi_rest/src/member_document.rs:232-267` — Download-Pattern mit `Content-Disposition` + `application/pdf`
- `genossi_rest/src/http_util.rs:1-80` — `content_disposition_attachment` und `sanitize_filename_component` Helper
- `genossi_rest/src/lib.rs:77-178` — `RestError` enum + `error_handler` mit 409-Mapping
- `genossi_rest/src/test_server.rs:1-61` — `start_test_server`-Helper für E2E-Tests
- `genossi_rest_types/src/lib.rs:1700-1751` — `AttendanceMemberTO` + 7-Feld-Whitelist-Test
- `genossi_dao/src/assembly.rs:10-56,96-138` — `AssemblyStatus` Enum + `AssemblyDao` Trait
- `genossi_dao/src/assembly_member_snapshot.rs:1-45` — `AssemblyMemberSnapshotDao` mit `count_by_assembly_id`
- `genossi-frontend/src/api.rs:497-548` — Bestehendes Blob-Download-Pattern für PDF-Templates
- `genossi-frontend/src/page/assembly_details.rs:1-200` — Page-Struktur mit `assembly.status`-Sichtbarkeits-Gate
- `genossi-frontend/CLAUDE.md` — Component-First-Prinzip
- `templates/join_confirmation.typ` + `templates/_layout.typ` — Typst-Template-Stil-Anker
- `Cargo.toml` + `genossi_bin/Cargo.toml:58` + `Cargo.lock:4323` — Verifikation, dass `rust_xlsxwriter = "0.82"` bereits als dev-dep vorhanden ist
- `genossi_rest/Cargo.toml:30` + `genossi_bin/Cargo.toml:60` — `csv = "1.3"` ist bereits Dep
- `.planning/REQUIREMENTS.md` — ATTN-01, ASSY-02, ASSY-05, ASSY-06, ATTN-05 als Predecessor-Anforderungen
- `CLAUDE.md` — Layered Architecture, Component-First, Audit Log System Regeln

### Secondary (MEDIUM confidence — Offizielle Docs via WebFetch)

- `https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/workbook/struct.Workbook.html#method.save_to_buffer` — `Workbook::save_to_buffer() -> Result<Vec<u8>, XlsxError>` Signatur
- `https://github.com/jmcnamara/rust_xlsxwriter` — Maintainer, MIT/Apache-2.0, Default-Feature-Set (nur `zip` als Dep), 1.6 Mio Downloads
- `https://docs.rs/csv/latest/csv/struct.WriterBuilder.html` — `WriterBuilder::new().delimiter(b';').from_writer(Vec::new())` + `into_inner()` Pattern
- `https://typst.app/docs/reference/model/table/` — `table.header(repeat: true)` für seitenübergreifende Tabellen + `..arr.flatten()` Spread-Pattern
- `https://forum.typst.app/t/how-can-i-break-a-very-long-table-to-the-next-page/2374` — Tabellen-Pagination ist automatisch (ohne `figure()`-Wrapper)
- `https://lib.rs/crates/rust_xlsxwriter` — Aktuelle Version 0.95.0 (9. Mai 2026), MIT/Apache-2.0, einzige Default-Dep ist `zip 7.2`
- `https://crates.io/crates/rust_xlsxwriter` — Versionsverlauf, Maintenance-Indikator

### Tertiary (Cross-verified — WebSearch-Befunde mit Offiziellen Docs validiert)

- `https://umaranis.com/2026/05/04/reading-excel-files-in-rust-calamine-vs-umya-spreadsheet/` — Vergleich `rust_xlsxwriter` vs `umya-spreadsheet` (Write-Only vs Read+Write+Modify), Performance-Daten
- WebSearch zu `UTF-8 BOM EF BB BF Excel CSV German semicolon` — bestätigt: 3-Byte-BOM-Prefix vor CSV-Body ist Standard-Pattern für Excel-DE Auto-Detect
- WebSearch zu `dioxus wasm web-sys blob download` — bestätigt: `Url::create_object_url_with_blob` + Anchor-Click ist das verbreitete WASM-Pattern

## Metadata

**Confidence breakdown:**
- Standard Stack: **HIGH** — alle Libraries via offizielle Docs/Codebase verifiziert, `rust_xlsxwriter` sogar bereits in Lock-File
- Architecture Patterns: **HIGH** — bestehende Genossi-Patterns 1:1 wiederverwendet (Permission-Funnel, Handler-Skeleton, Blob-Download, Content-Disposition)
- Pitfalls: **MEDIUM-HIGH** — meisten aus konkretem Code-Review abgeleitet; Pitfall 8 (`Workbook` not `Send`) ist generischer Async-Rust-Issue mit niedrigerer Codebase-Verifikation
- Code Examples: **HIGH** — Service-Skelett, Handler, Tests alle direkt vom bestehenden `attendance`-Pattern abgeleitet, das produktiv läuft
- Security Domain: **MEDIUM** — bekanntes Threat-Modell, aber Phase 6 hat geringere Angriffsfläche als Phase 2/3

**Research date:** 2026-05-17
**Valid until:** 2026-06-17 (30 Tage — Tech-Stack ist stabil, Typst und `rust_xlsxwriter` haben keine Breaking-Change-Roadmap)
