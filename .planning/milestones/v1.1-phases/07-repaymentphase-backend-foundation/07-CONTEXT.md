# Phase 7: RepaymentPhase Backend (Foundation) - Context

**Gathered:** 2026-05-29
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 7 liefert das auditpflichtige `RepaymentPhase`-Aggregat als Foundation für den v1.1-Milestone. Die Entität existiert mit Lifecycle `Preparation → Open → Closed`, kann angelegt, gelesen, gelistet, im erlaubten Status korrigiert und über dedizierte Endpoints geöffnet/geschlossen werden. Phase 7 schreibt KEINE RepaymentEntries und triggert KEINE Auto-Befüllung — beides kommt in Phase 8.

**In scope:**
- Migration `repayment_phase`-Tabelle (BLOB-UUID, `fiscal_year INTEGER NOT NULL`, `share_value INTEGER NOT NULL` in Cent, `status TEXT NOT NULL`, `opened_at TEXT NULL`, `closed_at TEXT NULL`, `created`, `deleted`, `version`)
- DAO-Trait + SQLite-Impl + Service-Trait + `*Impl` + REST-Handler
- `Auditable`-Implementierung; `audited_create!` beim Anlegen, `audited_update!` für `share_value`-Korrekturen und Lifecycle-Übergänge
- REST-Endpoints: `POST /api/repayment-phase`, `GET /api/repayment-phase`, `GET /api/repayment-phase/{id}`, `PUT /api/repayment-phase/{id}`, `DELETE /api/repayment-phase/{id}`, `POST /api/repayment-phase/{id}/open`, `POST /api/repayment-phase/{id}/close` — registriert in OpenAPI/Swagger-UI
- E2E-Test (Pattern aus `genossi_bin/tests/e2e_tests.rs`): create → open → update share_value → close; Audit-Chain via `/api/audit/verify` bleibt valide
- Wiring in `genossi_bin/src/lib.rs::RestStateImpl::new()` (neuer Service + DAO + Sub-Router-Registration)

**Out of scope (gehört in Phase 8-12 oder ist explizit nicht gewollt):**
- RepaymentEntry-Entität (Phase 8)
- Auto-Befüllung beim `open` (Phase 8 — PHAS-02 vervollständigt)
- Close-Validation gegen pending RepaymentEntries (Phase 8 — PHAS-03 vervollständigt)
- Mark-paid-out + MemberAction::Verkauf-Cascade (Phase 9)
- Massenmail + Template-Variablen (Phase 10)
- PDF/CSV-Export (Phase 11)
- Frontend / Dioxus-Komponenten (Phase 12)
- Reverse-Transitionen (`Open → Preparation`, `Closed → Open`) — out of scope, immer 409 Conflict

</domain>

<decisions>
## Implementation Decisions

### Status-Naming & Lifecycle-API

- **D-01:** Status-Enum in **Englisch**: `Preparation`, `Open`, `Closed` (analog `AssemblyStatus` in `genossi_dao/src/assembly.rs:10`). DB- und JSON-Repräsentation sind die technischen Strings; Frontend übersetzt via i18n (`genossi-frontend/src/translation.rs`). Pattern-konsistent mit `MemberStatus`, `ApplicationStatus`, `AssemblyStatus`.
- **D-02:** Lifecycle-Transitionen über **dedizierte Action-Endpoints**: `POST /api/repayment-phase/{id}/open` und `POST /api/repayment-phase/{id}/close`. Kein Status-Feld im PUT-Body. Pattern-Anker: `genossi_rest/src/assembly.rs:359-360` (`assembly::generate_route`). PUT bleibt rein für Daten-Updates (siehe Edit-Matrix D-04..D-06).
- **D-03:** Lifecycle-Endpoints akzeptieren **kein Request-Body** und prüfen **kein `version`-Field** (Pattern aus `open_assembly`/`close_assembly`: nur ID). Der Status-Check ist die Concurrency-Defense (Open→Open ist 409). Optimistic Locking via `version`-Body-Field gilt nur für `PUT /api/repayment-phase/{id}`.

### State-Machine-Editier-Regeln

- **D-04:** Edit-Matrix pro Status:
  ```
  Status        | fiscal_year | share_value | Lifecycle-Transition
  ------------- | ----------- | ----------- | ----------------------
  Preparation   | EDIT        | EDIT        | → POST /open  (Open)
  Open          | LOCKED      | EDIT        | → POST /close (Closed)
  Closed        | LOCKED      | LOCKED      | (final, keine Transition)
  ```
  Entspricht **PHAS-04** (`share_value` korrigierbar in Open) und **ROADMAP-Phase-7 Success Criterion #5** (`fiscal_year` read-only nach Open).
- **D-05:** Jeder Versuch ein gelocktes Feld zu setzen oder eine ungültige Lifecycle-Transition zu triggern liefert **409 Conflict** via `ServiceError::Conflict(message)` → `RestError::Conflict`. Pattern-konsistent mit:
  - Phase 6 D-11 (Export gegen falschen GV-Status)
  - PHAS-03 in Phase 8 (close blockt bei pending Entries)
  - PAYO-04 (Phase 9, finaler Status-Toggle)
  - `update_assembly` Version-Mismatch (`genossi_rest/src/assembly.rs:233`)
- **D-06:** **Reverse-Transitionen sind verboten** (Claude's Discretion auf User-Auftrag): `Open → Preparation` und `Closed → Open` liefern 409 Conflict. Begründung:
  1. Pattern-Konsistenz mit Assembly (close ist final)
  2. Vereinfacht State-Machine erheblich
  3. Phase 8 hängt Auto-Befüllung an `open` — Reverse würde Side-Effects in Phase 8/9-Daten erzeugen (RepaymentEntries existieren bereits, MemberActions in Phase 9 bereits gebucht)
  4. Escape-Hatch ist Soft-Delete + Neuanlage (siehe D-08)
- **D-07:** PUT auf ein gelocktes Feld + erlaubtes Feld in derselben Request (z.B. `fiscal_year=2027` und `share_value=120` auf einer Phase im Status `Open`) wird **atomar abgelehnt** (alles oder nichts mit 409). Service-Layer prüft die Differenz gegen den Persistenzstand und antwortet 409 bei jeder verbotenen Mutation. Lockt versehentliche Drift.

### Uniqueness & Soft-Delete

- **D-08:** **Keine DB-Constraint auf `fiscal_year`** — mehrere RepaymentPhases pro Geschäftsjahr sind erlaubt, in beliebigen Statuskombinationen. Realer Use-Case: Q1-Phase für reguläre Austritte + Q4-Phase für spätgemeldete; alte abgeschlossene Phase + neue parallele Vorbereitung. Vorstand verantwortet Logik via UI/Namensgebung (Phase 12).
- **D-09:** **Soft-Delete nur in Status `Preparation` erlaubt** (`DELETE /api/repayment-phase/{id}` setzt `deleted = now()` über `audited_update!`). Versuch in `Open` oder `Closed` → 409 Conflict. Begründung: Sobald `open` geschah, hängen Audit-Einträge und ab Phase 8 RepaymentEntries dran — Löschung würde Audit-Konsistenz brechen.
- **D-10:** **Listing `GET /api/repayment-phase` filtert default `WHERE deleted IS NULL`**, kein `?include_deleted`-Toggle. Konsistent mit Member-/Assembly-Listing. Recovery soft-gelöschter Phasen ist Ausnahmefall via DB-Direktzugriff.

### Detailfelder & Validierung

- **D-11:** `fiscal_year` wird auf **Range `2000..=2100`** validiert (Service-Layer via `ValidationService`, ValidationError mit Field `fiscal_year`). Verhindert Tippfehler (z.B. 226 statt 2026, 22026 statt 2026). Genossenschafts-Use-Case bleibt in dem Range. Realisierbar via `validation.rs`-Pattern.
- **D-12:** `share_value` (INTEGER, Cent) wird **strikt positiv** (>0) validiert. Keine Obergrenze (User-Entscheidung). Service-Layer ValidationError bei `share_value <= 0`. Begründung: Auszahlungs-Wert von 0 oder negativ ist semantisch unsinnig; Obergrenze überlassen wir der Buchhaltung.
- **D-13:** `opened_at` und `closed_at` werden als optionale Timestamps (`Option<PrimitiveDateTime>`) in der Tabelle gespeichert — exakt analog zu `assembly.opened_at` und `assembly.closed_at`. `opened_at` wird gesetzt beim `POST /open`, `closed_at` beim `POST /close`. Beide Felder sind im Audit-Log über `audited_update!` automatisch erfasst (Auditable diff). Nützlich für Phase 11 Filename-Schema und Audit-Lesbarkeit.
- **D-14:** REST-Pfad **Singular**: `/api/repayment-phase` (nicht `/api/repayment-phases`). Konsistent mit ROADMAP-Phase-7-Beschreibung und Genossi-Konvention (`/api/member`, `/api/assembly`, `/api/application`).

### Entity-Skeleton (für Researcher als Anker)

```rust
// genossi_dao/src/repayment_phase.rs (neu)
pub enum RepaymentPhaseStatus { Preparation, Open, Closed }

pub struct RepaymentPhaseEntity {
    pub id: Uuid,                              // BLOB
    pub fiscal_year: i32,                      // INTEGER, validiert 2000..=2100
    pub share_value: i64,                      // INTEGER (Cent), >0
    pub status: RepaymentPhaseStatus,
    pub opened_at: Option<PrimitiveDateTime>,
    pub closed_at: Option<PrimitiveDateTime>,
    pub created: PrimitiveDateTime,
    pub deleted: Option<PrimitiveDateTime>,
    pub version: Uuid,
}

impl Auditable for RepaymentPhaseEntity {
    fn entity_type() -> &'static str { "repayment_phase" }
    fn entity_id(&self) -> Uuid { self.id }
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        // fiscal_year, share_value, status, opened_at, closed_at
        // (NICHT id/version/created/deleted — Konvention aus Auditable-Trait)
    }
}
```

### Claude's Discretion

- **Reverse-Transitionen**: User hat „You decide" gewählt → Claude entscheidet **„nur Vorwärts"** (D-06). Begründung dokumentiert.
- **REST-Body-Schema für PUT**: Nur `fiscal_year`, `share_value`, `version` (Optimistic Locking). Status wird NICHT akzeptiert — Transition läuft ausschließlich über `POST /open`/`POST /close`. Planner kann das im Detail anpassen, wenn `update_member`/`update_assembly`-Body-Schemas dem nicht entsprechen.
- **share_value-Obergrenze**: User hat „keine Obergrenze" gewählt. Falls Planner Schema-Limit (z.B. `i32::MAX`) braucht für DB-Schutz, ist das ok — kein Bruch der User-Decision (`i64::MAX` Cent = ~92 Trillionen Euro, kein praktisches Limit).
- **OpenAPI-Beispielwerte**: Planner darf realistische Defaults wählen (z.B. `fiscal_year: 2026`, `share_value: 12000` für 120,00 €).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap & Anforderungen
- `.planning/ROADMAP.md` §Phase 7 — Goal, 5 Success Criteria, Requirements-Mapping (PHAS-01, PHAS-04, PHAS-05 voll im Scope; PHAS-02/03 Skelett ohne Auto-Befüllung/Close-Validation)
- `.planning/REQUIREMENTS.md` §"RepaymentPhase Lifecycle" — vollständige Spec für PHAS-01..05; §Out-of-Scope für Begründungen (keine SEPA, keine Brief-Automatik, keine Steuerberechnung)
- `.planning/PROJECT.md` §"Current Milestone: v1.1 Anteile-Rückzahlungsphase" — Core-Value, Trigger-Datum (vor GV 2027)

### Pattern-Anker: Assembly-Aggregat (nahezu identischer Vorläufer)
- `genossi_dao/src/assembly.rs:10-46` — `AssemblyStatus` enum + `as_str`/`from_str`/`Default` — direkter Vorlage für `RepaymentPhaseStatus`
- `genossi_dao/src/assembly.rs` (gesamte Datei) — `AssemblyEntity` mit `id`/`status`/`opened_at`/`closed_at`/`created`/`deleted`/`version` plus `Auditable`-Impl — direkte Strukturvorlage
- `migrations/sqlite/20260502000000_create_assembly_table.sql` — Migration-Pattern für audit-pflichtige Lifecycle-Entities (BLOB-PK, INTEGER fiscal_year analog `TEXT date`, indizes auf `status` + `deleted`)
- `genossi_dao_impl_sqlite/src/assembly.rs` — SQLite-DAO-Impl-Vorlage (UPDATE...RETURNING-Pattern für version-Bump, ISO8601-Datetime-Parsing)
- `genossi_service_impl/src/assembly.rs:140-181` — `update_assembly` mit Version-Check; `open_assembly` (Z. 181+); `close_assembly` (Z. 261+) — exakte Vorlage für die drei Service-Methoden
- `genossi_rest/src/assembly.rs:349-361` — `generate_route()` mit Sub-Routes `/`, `/{id}`, `/{id}/open`, `/{id}/close` — REST-Route-Vorlage
- `genossi_rest/src/assembly.rs:233` — OpenAPI `409 Conflict` Response-Doc-Pattern für Lifecycle-Verstöße

### Audit-Infrastruktur
- `genossi_dao/src/auditable.rs` — `Auditable`-Trait-Definition; RepaymentPhase muss implementieren (`entity_type`, `entity_id`, `audit_fields` — Letzteres OHNE id/version/created/deleted)
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!` (beim POST), `audited_update!` (beim PUT, POST /open, POST /close, DELETE)
- `genossi_service_impl/src/audit_log.rs` — Hash-Chain-Berechnung (NICHT manuell re-implementieren — siehe Anti-Pattern in `.planning/codebase/ARCHITECTURE.md`)
- `CLAUDE.md` §"Audit Log System" — 4-Schritt-Checklist für neue auditierte Entities: (1) Auditable-Trait, (2) AuditLogDao-Dependency via `gen_service_impl!`, (3) Audit-Macros statt direkte DAO-Calls, (4) Wiring in `genossi_bin/src/lib.rs`

### Service-Layer-Patterns
- `genossi_service_impl/src/macros.rs` — `gen_service_impl!` Macro für Service-Boilerplate-Reduktion; `MEMBER_SERVICE_PROCESS`-Konstanten-Pattern für `$process`-Parameter der Audit-Macros (RepaymentPhase: `const REPAYMENT_PHASE_SERVICE_PROCESS: &str = "repayment-phase-service"`)
- `genossi_service_impl/src/validation.rs` — `ValidationService` für `fiscal_year`-Range und `share_value`-Positivität; `ValidationFailureItem { field, message }` als ValidationError-Payload
- `genossi_service/src/permission.rs` — `Authentication<Context>` für OIDC-Vorstand-Check; admin-only Endpoint-Pattern

### REST-Layer-Patterns
- `genossi_rest/src/lib.rs` — REST-Router-Registration; OpenAPI-Schema-Composition (`utoipa::OpenApi`); neuer Sub-Router `repayment_phase::generate_route()` muss registriert werden
- `genossi_rest/src/assembly.rs:142,224,279,315` — `#[utoipa::path]`-Annotation-Patterns für POST/PUT/POST(/open)/POST(/close) inkl. Response-Doc und Schema
- `genossi_rest/src/auth_middleware.rs` — `Extension<Context>` Extraction-Pattern

### Binary-Layer (Dependency-Injection)
- `genossi_bin/src/lib.rs` (`RestStateImpl::new()`) — DI-Wiring-Vorlage; neuer `RepaymentPhaseServiceImpl` + DAO; AuditLogDao bereits per `Arc<...>` shared — RepaymentPhaseService bekommt den existierenden Arc

### Testing-Patterns
- `genossi_bin/tests/e2e_tests.rs` — E2E-Pattern mit `start_test_server` + in-memory SQLite + `reqwest`-Client; Vorlage für create→open→update→close-Lifecycle-Test
- `genossi_rest/src/test_server.rs` — `start_test_server` Helper; random ports, isolierte DBs
- `genossi_service_impl/src/assembly.rs:1036-1075` — `test_update_assembly_version_mismatch_returns_conflict` — Vorlage für Version-Mismatch-Tests
- `genossi_rest/src/assembly.rs:405-500` — Existierende Assembly-REST-Tests als Vorlage für POST/PUT/Open/Close Happy-Path

### Architekturelle Constraints
- `.planning/codebase/ARCHITECTURE.md` — Anti-Patterns (Hard Delete, Manual Hash Chain, Service-creates-its-own-Transaction, Inline-RSX); Layer-Verantwortlichkeiten
- `CLAUDE.md` §"Entity Structure" — UUID/BLOB, ISO8601-Timestamps, optimistic locking via `version: Uuid`
- `CLAUDE.md` §"Datetime Handling" — ISO8601-Serde-Custom-Module in `genossi_rest_types/src/lib.rs`
- `.planning/PROJECT.md` §"Constraints" — Tech-Stack (Rust/Axum/SQLx/SQLite), Layered Architecture Pflicht, Component-First Frontend (für Phase 12), Audit-Pflicht für neue auditpflichtige Entities (gilt hier)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`AssemblyEntity` / `AssemblyStatus` / `AssemblyServiceImpl`** — Phase-7-Aggregat hat 1:1 die gleiche Struktur (BLOB-PK, 3-State-Lifecycle, optionale Open/Close-Timestamps, Auditable, optimistic locking). Researcher sollte Assembly als Skeleton verwenden und nur die Domain-spezifischen Felder ersetzen (`name`/`date`/`location` → `fiscal_year`/`share_value`).
- **`Auditable`-Trait + `audited_create!`/`audited_update!`** — komplette Audit-Infrastruktur ist da. `audit_fields()`-Diff erkennt automatisch geänderte Felder pro Update.
- **`ValidationService`** — existiert in `genossi_service_impl/src/validation.rs`; neue Validator-Methoden für `validate_fiscal_year(year: i32)` und `validate_share_value(value: i64)` reihen sich ein.
- **OIDC-Permission-Middleware** — bestehender `Authentication<Context>` über `Extension<Context>` im Handler liefert authentifizierten Vorstand. RepaymentPhase hat keine Helper-Auth (anders als Assembly/Attendance) — vereinfacht den Permission-Check.
- **OpenAPI/Utoipa-Pipeline** — schon hochgezogen; neue Schemas werden in `genossi_rest_types/src/lib.rs` definiert und in `genossi_rest/src/lib.rs::ApiDoc` registriert.

### Established Patterns

- **Layered Implementierungs-Reihenfolge:** DAO-Trait → SQLite-Impl → Service-Trait → Service-Impl → REST-Handler → DI-Wiring → E2E-Test (Backend-First-Konvention).
- **`gen_service_impl!`-Macro** für Service-Impl-Boilerplate (siehe `genossi_service_impl/src/macros.rs`) — reduziert Deps-Struct-Code; AuditLogDao-Dep + UuidService-Dep gehören rein.
- **ISO8601-Datetime via `time::PrimitiveDateTime`** in Entities; SQLite-Storage als TEXT; Custom-Serde-Module in `genossi_rest_types/src/lib.rs` für REST-Layer.
- **DAO-Minimal-Interface** (`create`, `update`, `dump_all`, `find_by_id`) plus Domain-Methoden (`find_by_fiscal_year(year)` evtl. für Phase-8/9-Wiederverwendung, aber nicht Phase-7-Pflicht).
- **REST-Sub-Router** in eigener Datei (`genossi_rest/src/repayment_phase.rs`) mit `generate_route()` und in `lib.rs` per `.merge()` eingebunden.
- **E2E-Test** mit `start_test_server` + in-memory SQLite (Pattern aus `genossi_bin/tests/e2e_tests.rs`), HTTP-Calls via `reqwest`, Roundtrip-Verifikation gegen `/api/audit/verify`.

### Integration Points

- **`genossi_bin/src/lib.rs::RestStateImpl::new()`** — neue DAO (`RepaymentPhaseDaoImpl`) und Service (`RepaymentPhaseServiceImpl`) aufbauen; `Arc::clone(&audit_log_dao)` für Audit-Wiring teilen.
- **`genossi_rest/src/lib.rs`** — `.merge(repayment_phase::generate_route())` ergänzen; OpenAPI-Schema-Registry erweitern (`ApiDoc` derive ergänzt um die neuen Handler).
- **`genossi_service/src/lib.rs`** — `pub mod repayment_phase;` Modul-Deklaration.
- **`genossi_service_impl/src/lib.rs`** — `pub mod repayment_phase;` + selektives Re-Export von `RepaymentPhaseServiceImpl`/`MockRepaymentPhaseServiceImpl`.
- **`genossi_dao/src/lib.rs`** + **`genossi_dao_impl_sqlite/src/lib.rs`** — analog Modul-Deklarationen.
- **`genossi_rest_types/src/lib.rs`** — `RepaymentPhaseTO` (Transfer-Object) + `From<&RepaymentPhaseEntity>` + Utoipa-Schema-Derive.
- **Migration** in `migrations/sqlite/`: nächste Sequenz-Nummer nach `20260506000000_add_code_to_helper_token.sql` (vermutlich `20260529000000_create_repayment_phase_table.sql` o.ä. — Planner setzt finale Datum-Sequenz).

</code_context>

<specifics>
## Specific Ideas

- **Assembly als „kopiere und ersetze"-Vorlage** — Researcher kann das Assembly-Aggregat (Migration + DAO + Service + REST + Tests) als Skeleton 1:1 nutzen und nur Domain-Felder austauschen. Schnellster Weg, niedrigste Risiko-Oberfläche, Pattern-konsistent. Use-Case-Refs in Audit-Diff (`name` → `fiscal_year`) und SQL-Spaltennamen anpassen.
- **`fiscal_year` als `i32`** (nicht `u16` o.ä.) — konsistent mit Genossi-Konvention (DB INTEGER → Rust `i32`). Range-Check via Validator, nicht via Type-Sicherheit.
- **`share_value` als `i64`** (Cent) statt `i32`, um Obergrenzen-Sorgen wegzukommen. INTEGER in SQLite ist 8-Byte. Pattern: Cent-Beträge in Genossi-Codebase noch nicht verbreitet — Phase 7 etabliert das Pattern für Phase 9 (MemberAction::Verkauf-Cascade) und Phase 11 (Export-Berechnung).
- **„Genossenschafts-Realität"** — Vorstand legt typischerweise eine Phase pro GJ an, aber Korrektur-Phasen / Nachzügler-Phasen müssen möglich sein. Daher keine DB-Constraint auf `fiscal_year`. Phase 12 Frontend zeigt die Phasen sortiert nach `fiscal_year DESC, created DESC` und macht so die „aktuellste Phase" auffindbar.

</specifics>

<deferred>
## Deferred Ideas

- **`?include_deleted=true` im Listing-Endpoint** — kein bekannter Use-Case in v1.1; falls Audit-Tooling es später braucht, kann ein eigener `/api/repayment-phase/deleted` Endpoint nachgezogen werden. NICHT in Phase 7.
- **`opened_at`/`closed_at` als Audit-Diff vs. eigene Spalten** — Entscheidung gefallen für eigene Spalten (D-13). Falls die Audit-Hashchain in einem zukünftigen Milestone als alleinige Wahrheits-Quelle für Lifecycle-Zeitstempel ausreicht, könnten die Spalten entfallen — nicht in v1.1.
- **`status_display`-Field auf Deutsch** — User hat das nach Nachhaken zurückgezogen. Falls Mail-Templates (Phase 10) oder PDF-Export (Phase 11) später direkten Zugriff auf DE-Status-Strings brauchen, lösen wir das per Template-Helper, nicht per DB-Spalte.
- **DB-Unique-Constraint auf `(fiscal_year, status)` für `WHERE status='Open'`** — Idee aus der Diskussion zur Doppel-Open-Phase. User hat „komplett frei" gewählt. Falls in der echten Nutzung Doppel-Phasen versehentlich entstehen, kann ein Partial-Unique-Index nachgezogen werden — nicht jetzt.
- **Reverse-Transitionen (`Open → Preparation`)** — explizit verboten (D-06). Falls reale Vorstandsarbeit das wirklich braucht (z.B. Auto-Befüllung in Phase 8 erzeugt fehlerhafte Entries und Vorstand will sauber neu starten), ist Soft-Delete + Neuanlage der Escape-Hatch. Reverse-Endpoint wäre eigene Phase mit Cascade-Logik.

### Reviewed Todos (not folded)
Keine — `gsd-sdk query todo.match-phase 7` lieferte 0 Matches.

</deferred>

---

*Phase: 7-repaymentphase-backend-foundation*
*Context gathered: 2026-05-29*
