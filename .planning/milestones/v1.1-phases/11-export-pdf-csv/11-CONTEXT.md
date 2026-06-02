# Phase 11: Export (PDF) - Context

**Gathered:** 2026-05-31
**Status:** Ready for planning

> ⚠ **Scope change vs. ROADMAP/REQUIREMENTS:** CSV-Export wurde während des Discuss-Schritts gestrichen (User-Decision; siehe `<decisions>` D-12). Phase 11 liefert **nur PDF**. EXPO-04 (CSV) wird nach v1.2 deferred. ROADMAP.md, REQUIREMENTS.md und der Phase-Slug `11-export-pdf-csv` müssen vor Planning-Start angepasst werden (siehe `<deferred>`).

<domain>
## Phase Boundary

Phase 11 liefert den PDF-Export der Auszahlungsliste einer `RepaymentPhase`. Vorstand öffnet die Phase im Frontend (Phase 12) und triggert `GET /api/repayment-phase/{id}/export/pdf?include=open|all|paid` — der Server rendert eine Banking-Online-Vorlage als PDF via Typst-Template `auszahlungsliste.typ`. Der Service-Layer ist read-only (kein Audit-Log-Eintrag, kein Schreibvorgang) und Vorstand-only (kein Helper-Branch). Implementierungs-Vorbild ist 1:1 `AttendanceExportServiceImpl` aus Phase 6 — gleiche Permission-Funnel-Logik, gleiches `*-Export`-Bundle-Pattern, gleicher `PdfGenerator`-Aufruf, gleicher `DEFAULT_TEMPLATES`-Registrierungs-Mechanismus.

**In scope:**
- Neues Typst-Template `auszahlungsliste.typ` in `templates/defaults/` mit Registrierung in `DEFAULT_TEMPLATES` (`genossi_service_impl/src/template_storage.rs:10`); Repeat-Header-Tabelle mit den 6 Spalten Mitgliedsnummer, Name, IBAN, share_count_to_pay_out, Betrag (formatierter Euro-String), Verwendungszweck — sortiert nach `member_number ASC`, Sekundär-Sort `created ASC` (bei mehreren Entries pro Member; Planner-Discretion)
- Neuer Service-Layer-Trait `RepaymentExportService` in `genossi_service/src/repayment_export.rs` analog zu `AttendanceExportService` (`attendance_export.rs:76`); Domain-Typen `ExportFormat` (vorerst nur Pdf-Variante) und `ExportInclude` (Open, All, Paid) und `RepaymentExport`-Bundle (`bytes`/`content_type`/`filename` analog `AttendanceExport`)
- Neue Impl `RepaymentExportServiceImpl` in `genossi_service_impl/src/repayment_export.rs` mit Permission-Funnel-Methode `check_admin_and_phase_status` (analog `check_admin_and_closed`, aber Status-Gate: Phase MUSS `Offen` ODER `Abgeschlossen` sein — `Vorbereitung` liefert `ServiceError::Conflict("phase_not_exportable")`)
- `RepaymentExportServiceImpl::export(...)` liest in einer Transaction: `RepaymentPhaseDao::find_by_id` (Phase-Daten für `fiscal_year` + `share_value`) und `RepaymentEntryDao::find_by_phase_id` (alle Einträge der Phase); joint pro Entry den `MemberDao::find_by_id`-Read für Name/Mitgliedsnummer/IBAN (entweder N+1-Queries oder neue `MemberDao::find_by_ids`-Batch-Methode — Planner-Discretion)
- Filter-Anwendung in-memory: `entry.deleted IS NULL AND member.deleted IS NULL` (immer); zusätzlich nach `include`-Parameter (siehe D-01)
- Neuer REST-Handler `genossi_rest/src/repayment_export.rs` analog `attendance_export.rs:122` mit Route `/{phase_id}/export/{format}` montiert auf `/api/repayment-phase` in `lib.rs::create_app`; Format-Whitelist nur `pdf` (alles andere → 400); Query-Param `?include=...` mit Default `open` (siehe D-01)
- Lokales `map_export_error` (analog `attendance_export.rs:59`): `ServiceError::PermissionDenied` → `RestError::Forbidden(403)`
- OpenAPI/Utoipa-Schema-Doku mit Status-Codes 200/400/401/403/404/409
- DI-Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()`: `RepaymentExportServiceImpl`-Instanz mit `PdfGenerator` (existierend, gemeinsam mit `AttendanceExportServiceImpl`) und `template_base` (existierend)
- 6+ E2E-Tests in `genossi_bin/tests/`: PDF-Erfolg (Happy Path), 403 ohne Vorstand-Auth, 400 bei unbekanntem Format, jede `?include`-Variante (`open`/`all`/`paid`), 409 bei `RepaymentPhase` in `Vorbereitung`-Status, 404 bei unbekannter `phase_id`, leere IBAN (Member.bank_account NULL) wird als leere Spalte gerendert
- Grep-Gate-Test: `rg "audited_(create|update|delete)!" genossi_service_impl/src/repayment_export.rs` → `0` Treffer (SC #4 äquivalent, EXPO-05)
- `tracing::info!` mit strukturierten Feldern (`target = "repayment_export"`, `phase_id`, `format`, `include`, `rows`) analog Phase 6 D-18

**Out of scope (deferred, gehört in spätere Phase / explizit verworfen):**
- CSV-Export (EXPO-04) — deferred nach v1.2 (User-Decision D-12); Format-Whitelist im REST blockiert `csv` mit 400
- XLSX-Export — nie im ROADMAP-Scope für v1.1 (anders als Phase 6)
- Frontend-Integration (Tab + Download-Button) — Phase 12 UI-02
- SEPA pain.001 XML-Export — explizit deferred zu v2 (SEPA-01 in REQUIREMENTS.md)
- Audit-Hashchain-Eintrag pro Export-Call — explizit nicht gewollt (EXPO-05; Phase 6 D-17)
- Per-Mitglied-Aggregation (eine Zeile pro IBAN statt pro Entry) — User-Decision D-08 (eine Zeile pro Entry)
- Konfigurierbarer Verwendungszweck-Text pro Phase — User-Decision D-04 (hardcoded Schema)
- Visual-Highlight für fehlende IBAN im PDF — User-Decision D-06 (nur leere Spalte, kein Marker)
- Sekundär-Status-Spalte im PDF — PDF-Spalten-Set ist durch SC #1 + Banking-Use-Case minimal; Status-Sicht gehört ins Frontend (Phase 12)
- Verwendungszweck-SEPA-Sonderzeichen-Sanitization (ä→ae etc.) — Banking-Software macht das normalerweise selbst; Planner kann optional Filter `format_sepa_purpose` in Typst-Template einführen, ist aber nicht REQ

</domain>

<decisions>
## Implementation Decisions

### include-Filter-Semantik

- **D-01:** **`?include=open` = `RepaymentEntryStatus ∈ {Open, Contacted}`** (Recommended-Default). Banking-Vorlage-Use-Case ist "noch nicht ausbezahlt", unabhängig davon ob das Mitglied schon per Mail kontaktiert wurde. Konsistent mit Phase 10 D-04 (Mail-Worker aggregiert ebenfalls `Open + Contacted`). User-Decision.
- **D-02:** **`?include=all` = `Open ∪ Contacted ∪ PaidOut`**, **`?include=paid` = nur `PaidOut`**. Soft-Deleted (`entry.deleted IS NOT NULL` ODER `member.deleted IS NOT NULL`) wird in JEDEM Filter ausgeschlossen — konsistent mit Genossi-Konvention (Phase 8 D-09 etc.). User-Decision.
- **D-03:** **Default-Parameter ist `open`**, wie in ROADMAP SC #2 spezifiziert. Per `#[derive(Default)]` auf einem REST-lokalen `ExportIncludeQuery`-Enum (Pattern aus `attendance_export.rs:80`); Service-Domain-`ExportInclude` hat ebenfalls ein `Default`-Impl mit `Open`. Locked.

### Verwendungszweck-Text (SC #1 PDF-Spalte)

- **D-04:** **Verwendungszweck = `Anteilsrückzahlung GJ {fiscal_year} {member_number} {first_name} {last_name}`** (hardcoded Schema im Typst-Template ODER im Service-Pre-Computing — Planner-Discretion). Beispiel: `"Anteilsrückzahlung GJ 2026 1234 Max Mustermann"` (~47 Zeichen, weit unter SEPA-140-Zeichen-Limit auch bei längeren Namen). User-Decision (Custom-Answer).
- **D-05:** **Keine SEPA-Zeichensatz-Sanitization** (Sonderzeichen ä/ö/ü/ß bleiben drin). Banking-Software des Vorstands ersetzt automatisch; Genossi liefert lesbares Original. Planner darf optional einen Filter einführen, falls auf User-Wunsch nachjustiert.

### IBAN-NULL-Edge-Case

- **D-06:** **Fehlende IBAN (`Member.bank_account IS NULL`) → leere IBAN-Spalte im PDF, alle anderen Spalten gefüllt.** Export blockiert nie. Vorstand sieht im PDF/Frontend, welche Mitglieder ohne IBAN sind, und kann nachpflegen. Kein Skip, kein Visual-Highlight, kein 409. User-Decision.
- **D-07:** **Empty-String vs. `Option<String>`:** Im PDF-Template wird `member.bank_account.unwrap_or_default()` bzw. äquivalent ein leerer String gerendert. Analog Phase 6 (Salutation/Title-Pattern in `attendance_export.rs:276`).

### Aggregation pro Mitglied vs. pro Entry

- **D-08:** **Eine Zeile pro `RepaymentEntry`.** 1:1-Mapping zur DB. Wenn ein Mitglied zwei Entries hat (z.B. Teilabtretung + Vollrückzahlung), erscheinen zwei Zeilen im PDF. Banking-Vorstand sieht zwei separate Überweisungen an dieselbe IBAN — explizit gewollt für audit-konsistente Spur. Konsistent mit Phase 8 D-04 (mehrere Entries pro Member explizit erlaubt). User-Decision.
- **D-09:** **Sortierung:** Primär nach `member.member_number ASC`, sekundär nach `entry.created ASC` (deterministischer Sub-Sort bei Mehrfach-Entries pro Member). SC #3 (CSV-Sortierung) entfällt mit D-12. Planner-Discretion auf den Sub-Sort.

### Phase-Status-Gate (locked durch EXPO-01 + SC #2)

- **D-10:** **Export erlaubt für `RepaymentPhaseStatus ∈ {Offen, Abgeschlossen}`.** `Vorbereitung` → `ServiceError::Conflict("phase_not_exportable")` → 409. Permission-Funnel-Methode heißt `check_admin_and_phase_status` (distinkt von Phase 6 `check_admin_and_closed`, weil hier ZWEI Status erlaubt sind). Permission-Check (admin) läuft VOR Status-Check, damit non-admin keinen Status-Leak bekommt (Phase 6 D-11/D-13-Pattern).

### Permission + Audit (locked durch EXPO-05 + Phase 6 D-13/D-17)

- **D-11:** **Vorstand-only via `PermissionService::check_permission("admin", ...)`**; `Helper`-Auth liefert `RestError::Forbidden(403)` (nicht 401). Kein Helper-Branch. Lokales `map_export_error` mappt `ServiceError::PermissionDenied → RestError::Forbidden(...)` (Phase 6 D-13). **Null `audited_*!`-Calls** im Service-Impl (Grep-Gate-Test im E2E-Setup). Audit-Log bleibt unverändert; `/api/audit/verify` muss nach Export-Calls weiterhin valide sein (E2E-Test deckt das ab).

### Scope-Change: CSV-Streichung

- **D-12:** **CSV-Export (EXPO-04) wird komplett aus Phase 11 entfernt** (User-Decision: "Lass CSV erst mal weg"). Konsequenzen für ROADMAP/REQUIREMENTS:
  - REQUIREMENTS.md: EXPO-04 aus Phase-11-Mapping entfernen; in "v2 deferred" verschieben (mit Notiz: "ausgesetzt; Buchhaltung kann PDF-Werte abtippen oder Frontend-View nutzen, bis konkreter Bedarf signalisiert wird")
  - ROADMAP.md: Phase 11 umbenennen von "Export (PDF + CSV)" zu "Export (PDF)"; SC #3 (CSV-Sortierung/Roundtrip) streichen; SC #5 reduzieren auf PDF-only-E2E-Tests; Plan-Count-Schätzung reduziert sich um ~25%
  - Phase-Slug bleibt `11-export-pdf-csv` (Pfad-Stabilität in `.planning/`); Renaming-Aufwand ist nicht der Anpassung wert
  - Format-Whitelist im REST: nur `pdf` (alles andere → 400). REST-Test deckt `csv` → 400 ab
  - Sollte CSV in v1.2 doch noch reinkommen: Re-Add ist additiv (neue Format-Variante + neuer `render_csv`-Free-Function-Helper analog `attendance_export.rs:249` + neuer E2E-Test), bricht keine bestehenden Verträge

### Claude's Discretion

- **Member-Read-Strategie:** N+1-Reads via `MemberDao::find_by_id` pro Entry vs. neue Batch-Methode `MemberDao::find_by_ids(ids: &[Uuid])`. Bei einer Phase mit ~50-100 Entries ist N+1 in der Praxis OK (gleicher Tx, lokales SQLite); Batch-Methode wäre cleaner. Planner entscheidet beim Plan-Schritt.
- **Betrag-Rendering im Typst-Template:** Worker oder Service pre-computed Strings `"60,00"` (deutsche Lokalisierung, 2 Nachkommastellen, kein Tausenderpunkt) per `format!("{},{:02}", cents / 100, cents % 100)` — gleicher Stil wie Phase 10 D-04. Alternative: minijinja-Filter im Typst-Template. Empfehlung: Pre-Computing im Service (Typst hat keine native Locale-Formatierung).
- **`format`-Path-Param-Whitelist:** ROADMAP-Pfad ist `/{format}`. Mit D-12 ist nur `pdf` erlaubt. Planner darf entscheiden, ob ein eigenes Path-Segment-Whitelist-Enum nötig ist oder ob das `RestError::BadRequest(...)` für non-`pdf` reicht. Empfehlung: gleiches Pattern wie Phase 6 D-14 (`match format_str.as_str() { "pdf" => ..., other => 400 }`).
- **Permission-Privilege-String:** Phase 6 verwendet `"admin"` (Konstante `ADMIN_PRIVILEGE`). Planner sollte denselben String verwenden — kein neues `repayment.export`-Privilege, konsistent mit allen anderen Vorstand-Endpoints.
- **E2E-Test-Datenaufbau:** Phase 9 + 10 haben bereits Repayment-Phase-+-Entry-Setup-Helper aufgebaut (siehe `genossi_bin/tests/`); wiederverwenden, nicht neu schreiben.
- **Typst-Template-Layout:** Repeat-Header (analog `teilnehmerliste.typ`). 6 Spalten; auto/1fr/1fr/auto/auto/1fr-Verteilung sinnvoll (IBAN braucht ~25 Zeichen, Verwendungszweck ~50 Zeichen). Header-Felder fett. Optional: Summenzeile am Ende mit Anzahl Einträge + Gesamtbetrag — nicht REQ, aber Banking-Vorstand nimmt das gerne mit. Planner darf entscheiden.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 11 Roadmap & Requirements
- `.planning/ROADMAP.md` §"Phase 11: Export (PDF + CSV)" — Goal, SC #1–#5 (HINWEIS: SC #3/#5 müssen für D-12 nachgeführt werden vor Planning)
- `.planning/REQUIREMENTS.md` §"Export" — EXPO-01..05 (EXPO-04 muss für D-12 nach deferred verschoben werden)
- `.planning/PROJECT.md` — v1.1-Milestone-Goal, lockt Banking-Vorlage-Use-Case

### Phase 6 (1:1-Vorbild für Service- und REST-Patterns)
- `genossi_service/src/attendance_export.rs` — Service-Trait + Domain-Typen-Vorbild (`ExportFormat`, `ExportInclude`, Export-Bundle-Struct)
- `genossi_service_impl/src/attendance_export.rs` — Impl-Vorbild (Permission-Funnel `check_admin_and_closed`, Format-Writer-Free-Functions, `pdf_generator.render_attendance_list`-Call, kein Audit, `tracing::info!` statt Audit)
- `genossi_rest/src/attendance_export.rs` — REST-Handler-Vorbild (lokales `map_export_error` mit 403, Format-Path-Whitelist, `ExportQuery`/`ExportIncludeQuery` mit `#[derive(Default)]`, OpenAPI-Schema)
- `templates/defaults/teilnehmerliste.typ` — Typst-Template-Vorbild (Repeat-Header-Tabelle, `json.decode(sys.inputs.at("..."))`, `_layout.typ`-Import)
- `.planning/milestones/v1.0-phases/phase-06-*/` (falls archiviert) — historisches CONTEXT/RESEARCH/PLAN für Phase 6 falls Planner Detail-Backstory braucht

### Phase 7/8/9/10 (Repayment-Entitäten und -Daten)
- `genossi_dao/src/repayment_phase.rs` — `RepaymentPhaseEntity` (`fiscal_year`, `share_value`, `status`)
- `genossi_dao/src/repayment_entry.rs` — `RepaymentEntryEntity` (`member_id`, `phase_id`, `share_count_to_pay_out`, `status`)
- `.planning/phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — Phase-7 Edit-Matrix + Singular-`/repayment-phase`-Konvention (D-14)
- `.planning/phases/09-auszahlungs-buchung-atomisch-auditiert/09-CONTEXT.md` — Phase-9 Status-Lifecycle (`mark_paid_out` PaidOut-Erzeugung)
- `.planning/phases/10-massenmail-anbindung-template-variablen/10-CONTEXT.md` — Phase-10 D-04 Euro-Format-Rendering (deutsche Lokalisierung `"60,00"`) — Service-Pre-Computing-Pattern für `auszahlungsliste.typ` wiederverwenden

### Member-Schema (IBAN-Feld + Whitelist)
- `genossi_dao/src/member.rs` — `MemberEntity { bank_account: Option<Arc<str>> }` (= IBAN-Feld, NUR der Feldname weicht ab); `member_number: i64` ist die externe ID
- (Frühere PII-Whitelist aus Phase 3 für Helper-Sicht ist HIER NICHT relevant — Phase 11 ist Vorstand-only, voller Member-Read erlaubt)

### Reusable Infrastructure
- `genossi_service_impl/src/pdf_generation.rs:279` (`PdfGenerator::render_attendance_list`) — Template-Render-Funktion, Vorbild für neue `render_repayment_list`-Methode ODER Generalisierung via Typst-`sys.inputs`-Schema
- `genossi_service_impl/src/template_storage.rs:10` (`DEFAULT_TEMPLATES`) — Registrierungs-Mechanismus für `auszahlungsliste.typ`; muss um neuen `DefaultTemplate`-Eintrag erweitert werden, sonst läuft Fresh-Install in `template not found`-Fehler
- `templates/defaults/_layout.typ` — gemeinsames Letter-Layout (wiederverwenden, nicht duplizieren)
- `genossi_rest/src/lib.rs` — `create_app`-Router-Registrierung (neue Route nach gleichem Pattern wie `attendance_export::generate_export_route`); `error_handler` für `RestError`-Mapping; `extract_auth_context`-Helper

### Anti-Patterns / Lessons Learned
- `CLAUDE.md` (root + genossi-frontend/) — Component-First-Prinzip relevant für Phase 12 (nicht hier), aber: für Backend-Tests gilt das No-Mocking-DB-Prinzip aus Phase 9/10 (E2E-Tests gegen In-Memory-SQLite, nicht Mocks)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`AttendanceExportServiceImpl`** (`genossi_service_impl/src/attendance_export.rs`, 1199 LOC) — Komplettes 1:1-Vorbild für Trait, Impl, Permission-Funnel, PDF-Render-Call und Test-Suite (Mocks: `MockTestTxDao`, `MockTestAssemblyDao` etc.). Phase 11 wird ~70% kürzer (kein XLSX, kein CSV, kein PII-Whitelist, kein Helper-Branch in der Logik), aber alle Pattern-Strukturen sind 1:1 wiederverwendbar.
- **`PdfGenerator`** (`genossi_service_impl/src/pdf_generation.rs`) — Existierender Renderer mit Typst-World, Font-Cache, Package-Cache. Phase 11 fügt eine neue Methode `render_repayment_list` hinzu (oder generalisiert eine bestehende — Planner-Discretion). Constructor schon via Arc geteilt, Wiring im `genossi_bin` ist trivial.
- **`DEFAULT_TEMPLATES`** (`genossi_service_impl/src/template_storage.rs:10`) — Statisches Array; neuer Eintrag für `auszahlungsliste.typ` (`include_bytes!("../../templates/defaults/auszahlungsliste.typ")`) muss ergänzt werden.
- **`AttendanceExport`-Bundle-Struct** (`genossi_service/src/attendance_export.rs:58`) — Vorbild für `RepaymentExport { bytes, content_type, filename }`; das `Debug`-Impl mit `bytes_len` statt Bytes-Dump direkt übernehmen, sonst Hex-Spam in Tests.
- **`http_util::content_disposition_attachment`** (`genossi_rest/src/...`) — Existierender Helper für `Content-Disposition`-Header; sicher gegen Filename-Injection.
- **`extract_auth_context`** (`genossi_rest/src/lib.rs`) — Existing Auth-Extraktor; gleicher Aufruf wie in Phase 6 `attendance_export.rs:131`.

### Established Patterns
- **Permission-Funnel-Methode** auf dem Impl-Struct (nicht in einer separaten Klasse); private async fn vor dem `impl Service`-Block. Order: 1) load-by-id (404), 2) admin-check (403), 3) status-check (409). Phase 6 D-11/D-13.
- **`map_export_error`** lokal pro REST-Modul, NICHT global: PermissionDenied → Forbidden(403), alle anderen Varianten delegieren ans globale `From<ServiceError>`. Phase 6 D-13.
- **Read-only-Service ohne Audit:** Grep-Gate-Test `rg "audited_(create|update|delete)!"` muss `0` Treffer im Service-Impl liefern. `tracing::info!` mit `target = "<service_name>"` ersetzt den Audit-Eintrag. Phase 6 D-17/D-18.
- **Format-Whitelist-Match** im REST-Handler (NICHT im Service): `match format_str.as_str() { "pdf" => ..., other => 400 }`. Phase 6 D-14. Service erhält bereits ein geparstes `ExportFormat`-Enum.
- **Query-Param-Default via `#[derive(Default)]`**: REST-lokales `ExportIncludeQuery`-Enum mit `#[default]`-Variante; `From<ExportIncludeQuery> for ExportInclude` mappt 1:1. Phase 6 D-09.
- **Filename-Schema im Service-Bundle** (nicht im Handler): Filename wird vom Service erzeugt und im Bundle zurückgegeben — Server-generated, kein User-Input-Pfad zur Content-Disposition. Phase 6 D-15.
- **`assert!(res.is_ok(), "{:?}", res)`** in Unit-Tests + manueller `Debug`-Impl auf Bundle, der nur `bytes_len` druckt (Phase 6 D-19; siehe `attendance_export.rs:64`).
- **`tx.clone()` durchziehen + `commit(tx)` am Ende** (read-only, aber Pattern-konsistent). Phase 6.
- **DI-Wiring in `RestStateImpl::new()`**: Neue Service-Instanz mit `template_base` und `pdf_generator` aus dem schon existierenden Setup für `AttendanceExportServiceImpl` ableiten — kein zweiter `PdfGenerator`-Bau nötig.

### Integration Points
- **`genossi_rest/src/lib.rs::create_app`**: Neue Route registrieren analog `attendance_export::generate_export_route()`. Mountpoint: `/api/repayment-phase` (Singular per Phase-7-D-14-Konvention). Wenn schon ein `/api/repayment-phase`-Router existiert (Phase 7+8+9), neue Sub-Route `/{phase_id}/export/{format}` hinzufügen, sonst neuer Sub-Router.
- **OpenAPI-Doc-Merge**: Neuer `ApiDoc`-Struct in `repayment_export.rs` (Pattern Phase 6 D-22) — wird in `genossi_rest/src/lib.rs::merged_openapi` aggregiert; `RepaymentExport`-Bundle braucht KEIN `ToSchema` (Body ist binary), aber `ExportQuery`/`ExportIncludeQuery` schon.
- **DI in `genossi_bin/src/lib.rs::RestStateImpl::new()`**: Neue Dependencies: `RepaymentPhaseDao`, `RepaymentEntryDao`, `MemberDao` (alle existieren bereits in `RestStateImpl`-Wiring durch Phase 7-10), `PermissionService`, `TransactionDao`, `PdfGenerator`, `template_base`.
- **E2E-Test-Setup**: `test_server::start_test_server` + bestehende Repayment-Phase-Setup-Helper aus `e2e_tests.rs` (Phase 9 + 10 haben das schon aufgebaut).

</code_context>

<specifics>
## Specific Ideas

- **Verwendungszweck-Wortlaut wörtlich:** `"Anteilsrückzahlung GJ {fiscal_year} {member_number} {first_name} {last_name}"` — exakt diese Reihenfolge, keine Kommas/Bindestriche, Leerzeichen-getrennt (User-Custom-Answer in D-04).
- **Banking-Vorlage = primärer PDF-Use-Case** — der Vorstand öffnet das PDF, kopiert IBAN+Betrag+Verwendungszweck in die Online-Banking-Sammelüberweisung. PDF muss für diesen Workflow optimiert sein (klare Spalten, keine winzigen Schriftgrößen, IBAN gut lesbar).
- **`?include=open` als Default** für Banking-Workflow — Vorstand sieht erstmal nur die offenen Posten; "all" und "paid" sind sekundäre Sichten (z.B. nach Phasen-Abschluss zur Buchhaltungs-Kontrolle).
- **Phase 6 als 1:1-Vorbild** — keine Erfindung, nur die Repayment-Anpassung der Felder/Spalten/Filter.

</specifics>

<deferred>
## Deferred Ideas

### CSV-Export (EXPO-04) — nach v1.2
- **Was:** CSV-Export der Auszahlungsliste mit Semikolon-Separator + UTF-8-BOM für Buchhaltung
- **Warum deferred:** User-Decision während Discuss-Phase ("Lass CSV erst mal weg"); Buchhaltung hat aktuell keinen konkreten Spec-Bedarf, das PDF-Format reicht für den Banking-Workflow. EXPO-04 wandert zurück in REQUIREMENTS.md `## v2 Requirements (deferred)`.
- **Re-Add ist additiv:** Neue `ExportFormat::Csv`-Variante + neuer `render_csv`-Free-Function-Helper analog `attendance_export.rs:249` + Format-Whitelist um `csv` erweitern + 2-3 neue E2E-Tests. Bricht keine bestehenden Verträge. Spalten-Set bleibt offen (PDF-Mirror? + entry_id + status? eigene Buchhaltungs-Form?) — beim Re-Add zu klären.
- **Routing:** Vor Planning-Start: REQUIREMENTS.md anpassen (EXPO-04 → deferred), ROADMAP.md Phase 11 umbenennen "Export (PDF + CSV)" → "Export (PDF)", SC #3 streichen, SC #5 reduzieren auf PDF-only-Tests.

### XLSX-Export — nie im v1.1-Scope
- Phase 6 hatte XLSX (für GV-Teilnehmerlisten). Für Auszahlungs-Phase nicht angefragt; PDF (Banking) + CSV (Buchhaltung, deferred) sind die genannten Formate. Re-Add bei Bedarf analog wie oben.

### Visual-Warnzeile / IBAN-Marker im PDF
- User-Decision D-06 ist "stiller Skip" für IBAN-NULL. Wenn UAT zeigt, dass Vorstand IBAN-Lücken übersieht, kann Phase 12 (Frontend) ein "IBAN fehlt"-Badge im Tabellen-Tab ergänzen — ist dort einfacher zu rendern als im PDF.

### SEPA pain.001 XML-Export
- SEPA-01 in REQUIREMENTS.md `## v2 Requirements (deferred)`. Direkter Banking-Upload statt manuellem Copy-Paste aus dem PDF. Out-of-Scope für v1.1 per Project-Constraint.

### Konfigurierbarer Verwendungszweck-Text pro Phase
- D-04 lockt das Schema hardcoded. Wenn später (z.B. ab v1.2) andere Genossenschaften das Tool nutzen und ihr eigenes Wording wollen, neue Spalte `RepaymentPhase.payout_purpose_template TEXT` (mit Default und Edit-Matrix). Nicht jetzt.

### Summenzeile im PDF (Anzahl Einträge + Gesamtbetrag)
- Claude's-Discretion-Item; Banking-Vorstand-Nice-to-Have. Planner darf optional aufnehmen, ist kein REQ.

</deferred>

---

*Phase: 11-Export-PDF*
*Context gathered: 2026-05-31*
