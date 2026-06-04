# Phase 14: DAO/Domain Foundation - Research

**Researched:** 2026-06-04
**Domain:** Pure-Function H1/H2-Stichtag + DAO/Service/REST-Erweiterungen (read-only foundation)
**Confidence:** HIGH (alle Patterns sind v1.1-codebase-verified)

## Summary

Phase 14 ist read-only Foundation für v1.2 mit klar abgegrenzten 4 Artefakten (Pure-Function, DAO-Methode, Service-Methode, REST-Endpoint). CONTEXT.md hat alle 15 Implementierungs-Entscheidungen bereits getroffen; diese Research validiert die Decisions gegen die v1.1-Codebase und füllt die verbleibenden technischen Lücken: (1) Utoipa-IntoParams-Annotationen für Query-Param, (2) mockall-Default-Impl-Override-Falle, (3) E2E-Setup-Reihenfolge wegen `recalc_dates`-Override, (4) tatsächliches Status-Mapping `PermissionDenied → 401` (nicht 403), (5) Sub-Route-Ordering vor `/{id}`.

**Primary recommendation:** Plan-Reihenfolge wie in CONTEXT.md Claude's-Discretion-Block (Pure-Function → DAO → Service → REST+E2E), mit besonderem Augenmerk auf (a) Mock-Default-Impl-Override-Pattern aus `repayment_phase.rs:976-989`, (b) Sub-Route-Anker VOR `/{id}` (siehe `member.rs:28-40` Reihenfolge, Vorbild `/import` + `/not-reached-by/{job_id}`), (c) E2E `exit_date`-Setup via Austritt-Action POST (nicht direkt im MemberTO — siehe `repayment_letter_e2e.rs:143-202`).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

CONTEXT.md hat 15 Decisions D-14-01 bis D-14-15 fixiert. Sie sind hier per ID referenziert, nicht dupliziert. Der Planner liest CONTEXT.md `<decisions>`-Block direkt.

Kurze Kategorisierung:
- **D-14-01..03** — Pure-Function-Shape (Struct `EffectiveDate { fiscal_year, effective_date }`, neue Datei `membership_adjust.rs`, `pub(crate)`-Sichtbarkeit)
- **D-14-04..07** — H1/H2-Semantik (`month <= 6`, 31.12. → H2 → next year, Schaltjahr-Test, ausführlicher Doc-Comment)
- **D-14-08..12** — DAO/REST/TO (SQL-Override in SQLite, `Arc<[Entity]>`, `/transfer-recipients?exclude_self={uuid}`, ADMIN-only, `MemberSlimTO` mit Slim-Feldern)
- **D-14-13** — Service-Methode auf `MemberService`-Trait, Return `Arc<[Member]>`
- **D-14-14..15** — Test-Strategie (6/2/3/1 Floor pro Layer + Planner-Discretion-für-mehr)

### Claude's Discretion

Aus CONTEXT.md `<decisions>`-Block, "Claude's Discretion"-Abschnitt:
- Default-Impl-Strategie auf DAO-Trait (Default-Impl ODER nur Trait-Signatur — siehe Sektion "Mock-Default-Impl-Override-Falle" unten für Empfehlung)
- `MemberSlimTO`-Field-Set kann erweitert werden, aber keine sensiblen Felder
- Utoipa-Schema mit Status-Codes (siehe Sektion "Utoipa-Annotationen" unten — präzises Mapping)
- Inline-Helper in `compute_effective_date` (z.B. `is_h1`) erlaubt
- Doc-Comment-Sprache Deutsch
- Reihenfolge der Plan-Dateien (4 Pläne empfohlen)

### Deferred Ideas (OUT OF SCOPE)

Siehe CONTEXT.md `<deferred>`-Block. Keine Re-Listung hier.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CANC-02 | H1/H2-Stichtag-Berechnung (Pure-Function `compute_effective_date`) | `compute_dates`-Pattern in `genossi_service_impl/src/member_action.rs:155-177` ist exakter Vorbild — `pub(crate)` freie Funktion. CONTEXT.md D-14-01..07 fixiert Struct-Return + 6 Edge-Case-Tests. |
| TRSF-06 | Empfänger-Search-Endpoint mit DAO + Service + REST | `MemberService::get_all` (`genossi_service/src/member.rs:110`) ist das Trait-Signatur-Vorbild; `get_all_members`-Handler (`genossi_rest/src/member.rs:53`) ist das REST-Vorbild; `MemberSlimTO` ist neuer Slim-Output-Typ (siehe Sektion "MemberSlimTO-Schema" unten). |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| `compute_effective_date` Pure-Function (CANC-02) | Service Layer (`genossi_service_impl`) | — | Pure-Function ohne I/O, gehört zur Business-Domain-Logik; Pattern-Vorbild `compute_dates` (`member_action.rs:155`). REST-Layer ruft sie nicht direkt auf (siehe D-14-03). |
| `RepaymentEntryDao::find_by_member_and_phase` | DAO Layer (Trait in `genossi_dao`, Impl in `genossi_dao_impl_sqlite`) | — | Datenbank-Filter, SQL-Override für Skalierung. Foundation für Phase-16-Sum-Check (PITFALLS Kat 1). |
| `MemberService::list_transfer_recipients` | Service Layer (`genossi_service_impl`) | DAO (`MemberDao`) | Business-Filter `exit_date IS NULL` + Permission-Gate; nutzt vorhandenes `MemberDao::all` (Default-Impl mit `deleted IS NULL`-Filter). |
| `GET /api/members/transfer-recipients` | REST Layer (`genossi_rest`) | Service | HTTP-Routing, Query-Param-Parsing, `MemberSlimTO`-Mapping. Sub-Route in `member::generate_route()`. |
| `MemberSlimTO` (Display-Only-Schema) | REST-Types (`genossi_rest_types`) | — | Klarer API-Vertrag mit reduziertem Schema (kein IBAN/Adresse). |

## Standard Stack

Die v1.1-Codebase ist die Quelle: alles bereits installiert und versioniert in `Cargo.toml`. Keine neuen Dependencies in Phase 14.

### Core (alle bereits in Workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `time` | 0.3 | `Date`, `Month`, `from_calendar_date` für Pure-Function | Genossi-weit für Datum/Zeit-Arithmetik; Schaltjahr-safe |
| `mockall` | 0.13 | Service-Unit-Tests mit MockMemberDao | Genossi-Konvention für Trait-Mocking |
| `async-trait` | — | DAO/Service-Trait-Methoden | Pflicht für `#[async_trait]`-Annotation |
| `axum` | 0.8.3 | REST-Handler + `Query<>`-Extractor | Bereits etabliert |
| `utoipa` | 5.0 | `IntoParams`/`ToSchema`-Derives für OpenAPI | Bereits für alle REST-Endpoints |
| `serde` / `serde_json` | 1.0 | Query/Response-De-Serialisierung | Bereits etabliert |
| `uuid` | 1.6 | UUID-Parsing in Query-Param | Bereits etabliert |
| `sqlx` | 0.8 | SQLite-Override für `find_by_member_and_phase` | Bereits etabliert |
| `tokio` | 1.35+ | `#[tokio::test]` für async Tests | Bereits etabliert |

**Versions-Verifikation:** Alle aus `genossi_dao/Cargo.toml` und `genossi_rest/Cargo.toml` übernommen [VERIFIED: existing workspace `Cargo.toml`]. Keine Neuinstallation, kein `npm view`-Äquivalent für Rust nötig.

## Architecture Patterns

### Pattern 1: Pure-Function als `pub(crate)` mit `#[cfg(test)] mod tests`

**Vorbild:** `genossi_service_impl/src/member_action.rs:155-177` (`compute_dates`)
**Anwendung Phase 14:**

```rust
// genossi_service_impl/src/membership_adjust.rs (neue Datei)
use time::Date;

/// Berechnet den Wirksamkeits-Stichtag nach Verbands-Konvention H1/H2.
///
/// **Konvention** (Verbands-Vorgabe):
/// - H1 (Monat 1-6): Stichtag = 31.12. des laufenden Geschäftsjahres
/// - H2 (Monat 7-12): Stichtag = 31.12. des folgenden Geschäftsjahres
///
/// Grenzwerte (siehe D-14-04..06):
/// - 30.06. zählt zu H1 (`month <= 6`)
/// - 01.07. zählt zu H2
/// - 31.12. zählt zu H2 → Stichtag = 31.12. nächstes Jahr
/// - 29.02. (Schaltjahr) zählt zu H1 → 31.12. desselben Jahres
///
/// Test-Cases in `tests`-Modul decken alle Edge-Cases ab (D-14-06).
pub(crate) fn compute_effective_date(willensbekundung: Date) -> EffectiveDate {
    let fiscal_year = if (willensbekundung.month() as u8) <= 6 {
        willensbekundung.year()
    } else {
        willensbekundung.year() + 1
    };
    let effective_date = Date::from_calendar_date(fiscal_year, time::Month::December, 31)
        .expect("31. Dezember ist in jedem Jahr ein gültiges Datum (kein Schalttag)");
    EffectiveDate { fiscal_year, effective_date }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EffectiveDate {
    pub fiscal_year: i32,
    pub effective_date: Date,
}
```

[VERIFIED: pattern matches `member_action.rs:155-177`]

**Panic-Analyse:** `Date::from_calendar_date(year, December, 31)` panic'ed NUR wenn `year` außerhalb `time::Date`-Range liegt. `time::Date`-Range ist `-9999..=9999` per Default (`MIN..MAX`); `willensbekundung.year() + 1` kann nicht überlaufen für realistische Eingaben. `December` und `31` sind statisch valid. **`.expect(...)` ist akzeptabel** und reflektiert die Invariante. [VERIFIED: `time::Date::from_calendar_date` docs — `time` 0.3 erfordert nur valid Year, valid Day-of-Month]

### Pattern 2: DAO-Methode mit Default-Impl + SQLite-Override

**Vorbild:** `genossi_dao/src/repayment_entry.rs:138-150` (`find_by_phase_id`)
**Anwendung Phase 14:** Erweitere Trait-Definition direkt:

```rust
// genossi_dao/src/repayment_entry.rs (Ergänzung)
async fn find_by_member_and_phase(
    &self,
    member_id: Uuid,
    phase_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
    let all_entities = self.dump_all(tx).await?;
    let filtered: Vec<RepaymentEntryEntity> = all_entities
        .iter()
        .filter(|e| e.member_id == member_id
                 && e.phase_id == phase_id
                 && e.deleted.is_none())
        .cloned()
        .collect();
    Ok(filtered.into())
}
```

SQLite-Override in `genossi_dao_impl_sqlite/src/repayment_entry.rs` analog zu D-14-08:

```rust
async fn find_by_member_and_phase(
    &self,
    member_id: Uuid,
    phase_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
    let member_blob = member_id.as_bytes().to_vec();
    let phase_blob = phase_id.as_bytes().to_vec();
    let rows = sqlx::query_as::<_, RepaymentEntryDb>(
        "SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, \
         deleted, version FROM repayment_entry \
         WHERE member_id = ? AND phase_id = ? AND deleted IS NULL \
         ORDER BY created ASC, id ASC",
    )
    .bind(member_blob)
    .bind(phase_blob)
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    rows.iter()
        .map(RepaymentEntryEntity::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into())
}
```

[VERIFIED: SQL-Pattern matches `dump_all` in `genossi_dao_impl_sqlite/src/repayment_entry.rs:71-91`]

**`ORDER BY created ASC, id ASC`** für Determinismus übernommen aus `dump_all`-Pattern (D-08 Plan 08-02, STATE.md). Wichtig wenn Tests mehrere Entries gleichzeitig anlegen.

### Pattern 3: REST-Handler mit `Query<>`-Extractor + `IntoParams`-Schema

**Vorbild:** `genossi_rest/src/repayment_entry.rs:59-141` (`ListEntriesQuery` + `list_repayment_entries`)

```rust
// genossi_rest/src/member.rs (Ergänzung)
use axum::extract::Query;
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct TransferRecipientsQuery {
    /// UUID des aktuellen Mitglieds — wird aus der Ergebnis-Liste ausgefiltert (Self-Transfer-Block).
    pub exclude_self: Uuid,  // serde lehnt invalide UUIDs automatisch ab → 422 von axum
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "/transfer-recipients",
    params(TransferRecipientsQuery),
    responses(
        (status = 200, description = "Aktive Transfer-Empfänger (ohne self)", body = [MemberSlimTO]),
        (status = 400, description = "Invalid exclude_self UUID format"),
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_transfer_recipients<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<TransferRecipientsQuery>,
) -> Response {
    error_handler(
        (async {
            let members: Vec<MemberSlimTO> = rest_state
                .member_service()
                .list_transfer_recipients(
                    query.exclude_self,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(MemberSlimTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&members)?))
                .unwrap())
        })
        .await,
    )
}
```

[VERIFIED: Pattern aus `repayment_entry.rs:59-141`]

### Pattern 4: Sub-Route MUSS vor `/{id}` registriert werden

**KRITISCHER PITFALL** (siehe Sektion "Common Pitfalls" Pitfall 1):
Axum matched in Deklarations-Reihenfolge. `/transfer-recipients` würde sonst auf `/{id}` mit Wert `"transfer-recipients"` matchen und einen UUID-Parse-Fehler liefern (400).

**Vorbild der Reihenfolge** (`genossi_rest/src/member.rs:28-40`):
```rust
pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_members::<RestState>))
        .route("/transfer-recipients", get(get_transfer_recipients::<RestState>))  // ← NEU, MUSS hier
        .route("/import", post(import_members::<RestState>))                       // bereits literal-first
        .route("/not-reached-by/{job_id}", get(get_members_not_reached_by::<RestState>))
        .route("/{id}", get(get_member::<RestState>))           // ← literal-Routes davor!
        .route("/", post(create_member::<RestState>))
        .route("/{id}", put(update_member::<RestState>))
        .route("/{id}", delete(delete_member::<RestState>))
}
```

[VERIFIED: STATE.md Plan-08-05-Entry für `/batch-status`-Pattern]

### Anti-Patterns to Avoid

- **`/{id}` vor `/transfer-recipients`** — siehe Pattern 4. Axum frisst Literal-String als UUID-Parse-Fehler.
- **`MemberSlimTO` aus `MemberTO::from` ableiten** — verhindere Datenleck (IBAN, Adresse). Stattdessen `impl From<&Member> for MemberSlimTO` direkt aus Service-Entity. Vorbild: `AttendanceMemberTO` (`genossi_rest_types/src/lib.rs:2189-2197`) hat explizites Verbot von `From<&MemberTO>` im Doc-Comment.
- **`compute_effective_date` in REST-Layer aufrufen** — verboten durch D-14-03 (`pub(crate)`). Validation in Phase 15 PERM-02 ist Service-internal.
- **`pub` statt `pub(crate)` auf Pure-Function** — Re-Export an REST/Frontend ist deferred (CONTEXT.md `<deferred>` "compute_effective_date als pub-Re-Export").

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| H1/H2-Berechnung | Eigene `Date`-Arithmetik mit `chrono::NaiveDate` | `time::Date::from_calendar_date(year, Month::December, 31)` | `time` ist bereits Workspace-Standard; `chrono` würde neuen Dependency-Knoten einführen |
| Pure-Function-Doc | Externes Markdown-File mit H1/H2-Regel | `///`-Doc-Comment auf der Funktion selbst (D-14-07) | IDE-Hover zeigt die Regel; rustdoc generiert HTML; verändert sich mit dem Code |
| Query-Param-Parsing | Manuelles `.split('?')` aus URL | `axum::extract::Query<T>` + `Deserialize` | Axum-Built-in; Utoipa generiert OpenAPI-Schema aus `IntoParams`-Derive |
| Default-Impl-Test im Trait-Modul | Eigenen Mock hand-rollen | Standard `crate::MockTransaction` aus `genossi_dao` | bereits vorhanden über `#[automock(type Transaction = crate::MockTransaction;)]` |
| UUID-Validation in REST-Handler | `if !is_valid_uuid(s) { 400 }` | `Query<TransferRecipientsQuery>` mit Feld `exclude_self: Uuid` | serde deserialisiert auto und liefert 422 bei invalid (axum-Default für deserialize-Fehler) |

**Key insight:** Phase 14 ist erweiternd zu existierenden Patterns; alle Hand-Roll-Versuchungen haben bereits etablierte Codebase-Lösungen.

## Runtime State Inventory

Nicht-anwendbar — Phase 14 ist eine reine Code-Erweiterung (keine Renames, keine Migrations, keine externen Services).

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Keine — Phase 14 erweitert nur Read-Queries | — |
| Live service config | Keine — keine externen Services | — |
| OS-registered state | Keine — keine Worker, kein Scheduler-Eintrag | — |
| Secrets/env vars | Keine | — |
| Build artifacts | Keine — kein neuer Crate, keine Migration | — |

## Common Pitfalls

### Pitfall 1: Sub-Route-Ordering (`/transfer-recipients` muss vor `/{id}`)
**What goes wrong:** GET `/api/members/transfer-recipients?exclude_self=...` → axum interpretiert `transfer-recipients` als UUID-Path-Parameter → UUID-Parse-Error → HTTP 400 mit kryptischer Fehlermeldung.
**Why it happens:** Axum matched Routes in **Deklarations-Reihenfolge**; statische Literale haben keinen automatischen Vorrang.
**How to avoid:** Sub-Route in `member::generate_route()` direkt nach `/` deklarieren, **vor jedem `/{id}`-Eintrag**. Inline-Doc-Comment im `generate_route` fixieren (Pattern-Vorlage aus STATE.md Plan-08-05-Eintrag).
**Warning signs:** E2E-Test mit URL `/api/members/transfer-recipients` liefert 400 statt 200 mit JSON-Array. Lokaler `cargo run` + curl-Test bestätigt das Symptom.

### Pitfall 2: mockall überschreibt Trait-Default-Impl
**What goes wrong:** Service-Unit-Test instanziert `MockRepaymentEntryDao`; ruft `dao.find_by_member_and_phase(...)` auf; Test panic'ed mit "no matching expectation" — obwohl die Trait-Default-Impl `dump_all().filter(...)` macht.
**Why it happens:** `#[automock]` und `mock!`-Makros generieren Mocks, die ALLE Trait-Methoden überschreiben — auch die mit Default-Impl. Default-Impl wird im Mock IGNORIERT. Identifiziert in STATE.md Plan-03-Lektion und Phase-7-Plan-03-Update.
**How to avoid:**
- Service-Tests, die das DAO mocken: `dao.expect_find_by_member_and_phase().returning(|_, _, _| Ok(Arc::from(vec![])))` explizit setzen.
- Bei `mock!`-Verwendung (custom Mock-Definition wie `TestRepaymentEntryDao` in `repayment_phase.rs:704`) MUSS die neue Methode in der Mock-Definition gelistet werden (sonst E0046 Missing-Trait-Method).
- Empfehlung: **Default-Impl im Trait bereitstellen** (D-14-08 Claude's Discretion erlaubt das), damit reale SQLite-Tests + Service-Default-Tests beide funktionieren, und Mock-Tests müssen `.expect_find_by_member_and_phase()` explizit setzen.

[VERIFIED: `repayment_phase.rs:976-989` doc-comment beschreibt exakt dieses Pattern]

### Pitfall 3: `MemberTO.exit_date` wird im Member-Create ignoriert (E2E-Setup)
**What goes wrong:** E2E-Test setzt `MemberTO { exit_date: Some(...) }` direkt im POST `/api/members` → Member wird angelegt, aber `exit_date` ist `None`.
**Why it happens:** `MemberServiceImpl::create` ruft `recalc_dates` (`member.rs:33-56`), das `exit_date` ausschließlich aus `MemberAction::Austritt`-Actions ableitet (`compute_dates` in `member_action.rs:155-177`). Der vom Client gesendete Wert wird überschrieben.
**How to avoid:** 3-Schritt-Setup im E2E (Vorbild `repayment_letter_e2e.rs:143-202`):
  1. POST `/api/members` mit `exit_date: None`
  2. POST `/api/members/{id}/actions` mit `MemberActionTO { action_type: Austritt, effective_date: Some(...), shares_change: 0 }`
  3. GET `/api/members/{id}` — `recalc_dates` hat `exit_date` jetzt korrekt gesetzt
**Warning signs:** E2E-Test mit `members.iter().filter(|m| m.exit_date.is_none())` liefert ALLE Members zurück, obwohl 1 gekündigt sein sollte → Setup hat `exit_date` nicht persistiert.

[VERIFIED: STATE.md Plan-08-06-Eintrag dokumentiert dasselbe Pattern]

### Pitfall 4: `PermissionDenied` mapped auf **401**, nicht 403
**What goes wrong:** E2E-Test asserted `StatusCode::FORBIDDEN` (403) für non-admin-Aufruf → Test schlägt fehl, weil Server 401 zurückgibt.
**Why it happens:** Global `From<ServiceError> for RestError` in `genossi_rest/src/lib.rs:107-117` mapped `ServiceError::PermissionDenied → RestError::Unauthorized → HTTP 401`. Es gibt KEIN globales 403-Mapping. Lokales Override in `attendance.rs` ist die einzige Ausnahme (Helper-spezifisch). Phase 14 ist Vorstand-only ohne Helper-Differenzierung → **kein Override nötig, Status ist 401**.
**How to avoid:**
- Utoipa-Annotation: `(status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle")` — keine 403-Response listen.
- E2E-Test asserted `StatusCode::UNAUTHORIZED` (401), nicht `FORBIDDEN`.
- CONTEXT.md "Claude's Discretion" listet 403 als Option — das ist eine **Annahme** und wird hier durch Codebase-Verifikation korrigiert.

[VERIFIED: `genossi_rest/src/lib.rs:107-117` und v1.1 Plan-07-04 Eintrag in STATE.md ("globales `From<ServiceError>` reicht für Phase 7 — ValidationError → 400, PermissionDenied → 401")]

### Pitfall 5: `EffectiveDate.year()` vs. Pure-Function-Return-Type
**What goes wrong:** Planner deklariert Tuple `(i32, Date)` → Konflikt mit D-14-01 (Struct-Return).
**Why it happens:** ARCHITECTURE.md §3 zeigt eine ältere Variante mit Tuple-Return (`-> time::Date`); CONTEXT.md hat D-14-01 explizit auf Struct umgestellt.
**How to avoid:** **CONTEXT.md ist authoritativ.** ARCHITECTURE.md ist Milestone-Research, vor Phase-14-Discuss erstellt. D-14-01 ist die finale Decision: `EffectiveDate { fiscal_year, effective_date }` mit Named Fields.

### Pitfall 6: `compute_effective_date` und invalide `time::Date`-Eingaben
**What goes wrong:** Planner sorgt sich um Panic bei "invalid month".
**Why it happens:** `time::Date` ist eine **type-safe** Wrapper — sie KANN nicht mit invalidem Monat konstruiert werden. Der Konstruktor `Date::from_calendar_date(year, month, day)` liefert `Result<Date, ComponentRange>` und fängt invalide Werte zur Konstruktions-Zeit.
**How to avoid:** Akzeptiere `Date` (NICHT `i32, u8, u8`) als Input. Damit ist der Compiler dein Validator. Das `.expect(...)` auf `Date::from_calendar_date(year, December, 31)` ist sicher, weil 31. Dezember in jedem Jahr existiert (kein Schalttag). Bei extremen Edge-Cases (`willensbekundung.year() == i32::MAX`) würde `year + 1` overflowen — aber `time::Date` limitiert die Year-Range auf `±9999` (deutlich unter `i32::MAX`). [VERIFIED: `time` 0.3 docs — `Date::MIN..=Date::MAX` deckt Year `-9999..=9999` ab]

## Utoipa-Annotationen (Klärung von CONTEXT.md "Claude's Discretion")

**Status-Codes für `GET /api/members/transfer-recipients`:**

| Status | When | Body |
|--------|------|------|
| 200 | OK + Liste (kann leer sein) | `[MemberSlimTO]` |
| 400 | UUID-Format-Fehler in `exclude_self` (serde-Deserialize-Fehler, axum-Default) | Standard axum-Fehler-Body — kein eigenes Schema |
| 401 | Kein gültiger Auth-Context ODER kein admin-Privilege (siehe Pitfall 4) | Standard axum-Fehler-Body |
| 500 | DAO/Service-Fehler (DatabaseError, InternalError) | Standard axum-Fehler-Body |

**Explizit NICHT gelistet:**
- **404** — Endpoint hat keine Path-ID; eine fehlende Mitglieder-Liste ist 200 + `[]`.
- **403** — Existiert nicht im globalen Mapping (siehe Pitfall 4); CONTEXT.md "Claude's Discretion"-Annahme ist falsch.

**Schema-Registrierung:** `MemberSlimTO` muss in `ApiDoc::components(schemas(...))` (`member.rs:342`) hinzugefügt werden, sonst fehlt das Schema im Swagger-UI.

```rust
// member.rs:342 — Ergänzung
components(schemas(
    MemberTO,
    MemberSlimTO,  // ← NEU
    genossi_rest_types::SalutationTO,
    genossi_rest_types::MemberStatusTO,
    MemberImportResultTO,
    genossi_rest_types::MemberImportErrorTO,
    MemberImportUpload,
))
```

`OpenApi`-Derive: `get_transfer_recipients` zu `paths(...)`-Liste hinzufügen.

## MemberSlimTO-Schema (Klärung von D-14-12)

```rust
// genossi_rest_types/src/lib.rs (Ergänzung)
use crate::SalutationTO;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MemberSlimTO {
    pub id: Uuid,
    pub member_number: i64,
    pub salutation: Option<SalutationTO>,
    pub title: Option<String>,
    pub first_name: String,
    pub last_name: String,
}

impl From<&genossi_service::member::Member> for MemberSlimTO {
    fn from(m: &genossi_service::member::Member) -> Self {
        Self {
            id: m.id,
            member_number: m.member_number,
            salutation: m.salutation.as_ref().map(SalutationTO::from),
            title: m.title.as_ref().map(|t| t.to_string()),
            first_name: m.first_name.to_string(),
            last_name: m.last_name.to_string(),
        }
    }
}
```

**Wichtige Detail-Anpassungen gegenüber CONTEXT.md D-14-12:**
- `member_number` ist `i64` (nicht `i32`) — gemäß `genossi_service::member::Member::member_number` (`genossi_service/src/member.rs:14`).
- `title`, `first_name`, `last_name` als `String` (nicht `Arc<str>`) — REST-TOs nutzen `String` für serde-Kompatibilität; Service-Layer hat `Arc<str>`.
- Frontend-Display-Reihenfolge (D-14-Specifics): Mitgliedsnummer → Anrede → Titel → Vorname → Nachname. Feld-Reihenfolge im Struct spiegelt das.

**KEINE sensiblen Felder:** kein `email`, `bank_account`, `street`, `current_shares`, `current_balance`. Erweiterung in Phase 18 möglich (CONTEXT.md `<deferred>` "current_shares-Anzeige").

[VERIFIED: `AttendanceMemberTO` (`genossi_rest_types/src/lib.rs:2189-2197`) ist Vorbild für Slim-TO mit PII-Guard]

## Code Examples (Service-Layer + E2E)

### Service-Methode `list_transfer_recipients`

```rust
// genossi_service_impl/src/member.rs (Ergänzung an MemberServiceImpl)
async fn list_transfer_recipients(
    &self,
    exclude_member_id: Uuid,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<Arc<[Member]>, ServiceError> {
    let tx = self.transaction_dao.use_transaction(tx).await?;

    // Permission-Funnel: ADMIN_PRIVILEGE (Vorstand-only)
    self.permission_service
        .check_permission(ADMIN_PRIVILEGE, context)
        .await?;

    let members: Arc<[Member]> = self
        .member_dao
        .all(tx.clone())  // Default-Impl filtert bereits deleted IS NULL
        .await?
        .iter()
        .filter(|e| e.exit_date.is_none() && e.id != exclude_member_id)
        .map(Member::from)
        .collect();

    self.transaction_dao.commit(tx).await?;
    Ok(members)
}
```

**Pattern-Anker:** Übernommen aus `MemberServiceImpl::get_all` (`genossi_service_impl/src/member.rs:90-111`). Permission-Funnel-Reihenfolge: `use_transaction` → `check_permission` → DAO-Call → `commit` (Vorbild aus `repayment_phase.rs:99-108`).

**ADMIN_PRIVILEGE-Import:** `genossi_service_impl/src/member.rs` deklariert bereits lokale Konstanten `VIEW_MEMBERS_PRIVILEGE` und `MANAGE_MEMBERS_PRIVILEGE` (Zeile 18-19). Für Phase 14 ist `ADMIN_PRIVILEGE` aus `genossi_service::permission` zu importieren (`use genossi_service::permission::ADMIN_PRIVILEGE`) ODER als zusätzliche lokale Konstante zu deklarieren (`const ADMIN_PRIVILEGE: &str = "admin";` analog zu `repayment_phase.rs:50`). **Empfehlung:** Import aus `genossi_service::permission`, weil dort die kanonische Definition liegt (Zeile 28); lokale Re-Deklaration ist Konvention nur in v1.1-Service-Impls, nicht zwingend.

### E2E-Test-Setup (3 Members, 1 aktiv, 1 gekündigt, 1 self)

```rust
// genossi_bin/tests/transfer_recipients_e2e.rs (neue Datei)
// Pattern aus repayment_letter_e2e.rs:104-202

async fn create_active_member(client: &reqwest::Client, server: &TestServer, n: i64) -> MemberTO {
    let m = MemberTO {
        id: None,
        member_number: n,
        first_name: format!("Aktiv{}", n),
        last_name: "Mitglied".to_string(),
        salutation: Some(SalutationTO::Herr),
        title: None,
        // ... pflicht-felder mit defaults
        join_date: time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
        shares_at_joining: 1,
        current_shares: 1,
        exit_date: None,
        // ...
    };
    let r = client.post(server.url("/api/members")).json(&m).send().await.unwrap();
    r.json().await.unwrap()
}

async fn create_cancelled_member(client: &reqwest::Client, server: &TestServer, n: i64) -> MemberTO {
    let m = create_active_member(client, server, n).await;
    // 3-Schritt-Setup für exit_date (Pitfall 3)
    let exit_date = time::Date::from_calendar_date(2026, time::Month::June, 15).unwrap();
    let austritt = MemberActionTO {
        id: None,
        member_id: m.id.unwrap(),
        action_type: ActionTypeTO::Austritt,
        date: exit_date,
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(exit_date),
        comment: None,
        created: None,
        deleted: None,
        version: None,
    };
    client.post(server.url(&format!("/api/members/{}/actions", m.id.unwrap())))
          .json(&austritt).send().await.unwrap();
    // Re-load: recalc_dates hat exit_date gesetzt
    client.get(server.url(&format!("/api/members/{}", m.id.unwrap())))
          .send().await.unwrap().json().await.unwrap()
}

#[tokio::test]
async fn test_transfer_recipients_filters_self_and_cancelled() {
    let server = setup_server().await;
    let client = reqwest::Client::new();

    let m_active = create_active_member(&client, &server, 1).await;
    let m_cancelled = create_cancelled_member(&client, &server, 2).await;
    let m_self = create_active_member(&client, &server, 3).await;
    let self_id = m_self.id.unwrap();

    assert!(m_cancelled.exit_date.is_some(), "cancelled member must have exit_date");

    let resp = client
        .get(server.url(&format!("/api/members/transfer-recipients?exclude_self={}", self_id)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let recipients: Vec<MemberSlimTO> = resp.json().await.unwrap();
    assert_eq!(recipients.len(), 1, "only m_active should remain");
    assert_eq!(recipients[0].id, m_active.id.unwrap());
    // m_cancelled rausgefiltert (exit_date IS NOT NULL)
    // m_self rausgefiltert (exclude_self)
}
```

**Auth-Setup:** v1.1-Test-Server-Pattern (`genossi_rest/src/test_server.rs:18-58`) bootet via `create_app(rest_state)`. Permission-Mode hängt vom build davon ab — Workspace-default ohne `--features oidc` ist mock_auth, das Vorstand-Privileg erfüllt. Bestätigt durch erfolgreiche admin-Aufrufe in `repayment_letter_e2e.rs`.

## State of the Art

Keine relevanten technischen State-of-the-Art-Änderungen für diese Phase. Pure-Function, DAO-Filter, REST-Sub-Route sind stabile Patterns; das Genossi-eigene v1.1-Pattern ist die Referenz.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `time::Date`-Year-Range erlaubt `year() + 1` ohne Overflow für alle realistischen Eingaben | Pattern 1 (Pure-Function) | Niedrig — `time::Date` limitiert auf `±9999`; mit Default-Calendar-Date sind Eingaben automatisch im Range. Verifiziert über `time` 0.3 docs. |
| A2 | Workspace-Build ohne `--features oidc` aktiviert mock_auth in E2E-Tests | E2E-Setup | Niedrig — Verifizierung über erfolgreiche Aufrufe in `repayment_letter_e2e.rs` und e2e_tests.rs |
| A3 | `ADMIN_PRIVILEGE`-Konstante kann via `use genossi_service::permission::ADMIN_PRIVILEGE` importiert werden | Service-Methode | Niedrig — verifiziert in `genossi_service/src/permission.rs:28`; ist `pub const` |

**Keine ungeprüften Assumptions, die User-Bestätigung erfordern.** Alle CONTEXT.md-D-14-XX-Decisions sind durch User in `/gsd-discuss-phase` gesetzt; diese Research validiert sie gegen die Codebase.

## Open Questions

1. **Soll `ADMIN_PRIVILEGE` als Konstante in `member.rs` (Service-Impl) re-deklariert oder aus `permission`-Modul importiert werden?**
   - What we know: v1.1-Service-Impls re-deklarieren lokal (`repayment_phase.rs:50`, `member.rs:17-19` für andere Privilegien).
   - What's unclear: Konvention für ADMIN.
   - Recommendation: Bei minimal-invasivem Code-Pfad — Import via `use genossi_service::permission::ADMIN_PRIVILEGE`. Wenn Konvention strikt verlangt — lokal redeklarieren. **Planner darf entscheiden.**

2. **Sollen die 6 Pure-Function-Tests im selben Modul (`#[cfg(test)] mod tests`) oder in einem separaten Test-File leben?**
   - What we know: D-14-14 sagt "in `membership_adjust.rs::tests`", was Inline-`mod tests` impliziert.
   - What's unclear: Bei späteren Phasen wächst die Datei — eventuell auslagern.
   - Recommendation: **Inline** für Phase 14 — folgt `compute_dates`-Vorbild. Auslagerung ist Phase-15+-Refactor falls nötig.

3. **Default-Impl im `RepaymentEntryDao`-Trait JA/NEIN?**
   - What we know: D-14-08 Claude's Discretion lässt das offen. SQLite-Impl überschreibt sowieso.
   - What's unclear: Reine Mock-Strategie — mit oder ohne Default-Impl?
   - Recommendation: **MIT Default-Impl** (analog `find_by_phase_id`-Pattern). Begründung: (a) Test im DAO-Trait-Modul kann das Default-Verhalten gegen MockTransaction verifizieren; (b) Konsistenz mit Bestand; (c) Mocks brauchen ohnehin `.expect_find_by_member_and_phase()` (siehe Pitfall 2) — Default-Impl ändert daran nichts. Der Default-Impl-Test in `genossi_dao/src/repayment_entry.rs::tests` wird via `MockTransaction` ausgeführt und zeigt `dump_all → filter`-Logik.

## Environment Availability

Skipped — Phase 14 ist eine reine In-Repo-Code-Erweiterung. Bestehende Workspace-Tools (cargo, sqlx, time, axum, utoipa) sind alle bereits verfügbar.

## Project Constraints (from CLAUDE.md)

| Constraint | How Phase 14 honors it |
|------------|------------------------|
| Layered DAO/Service/REST | 4 Artefakte in den jeweiligen Crates: DAO-Trait + SQLite-Impl, Service-Trait + Impl, REST-Handler |
| Audit-Pflicht für Member-Writes | **Nicht anwendbar** — Phase 14 ist read-only; keine `audited_create!/update!`-Aufrufe |
| Component-First Frontend | **Nicht anwendbar** — Phase 14 hat keine Frontend-Komponente |
| `cargo fmt` + `cargo clippy --all-targets --all-features` | Plan muss Format/Lint im Verification-Schritt vorsehen |
| `Arc<[T]>` für Listen | DAO und Service liefern `Arc<[RepaymentEntryEntity]>` bzw. `Arc<[Member]>` |
| Soft-Delete-Filter `deleted IS NULL` | SQL-Override im SQLite-Impl enthält `AND deleted IS NULL` (analog `dump_all`); Default-Impl via `e.deleted.is_none()` |
| ISO8601-Datetime | **Nicht direkt anwendbar** — Phase 14 hat keine Datum-Felder im REST-Schema (`MemberSlimTO` listet kein Datum) |
| GSD-Workflow vor Edit/Write | Plan-Erstellung durch `/gsd-plan-phase` weiterhin pflicht |
| `jj` statt `git` (Memory-Item) | Commit-Befehle im Plan via `jj commit -m …`, NICHT `git commit` |
| Tests für Changes (User-CLAUDE.md) | Floor 6/2/3/1 + Planner-Discretion oben (D-14-14, D-14-15) |

## Sources

### Primary (HIGH confidence)
- `genossi_dao/src/repayment_entry.rs:1-292` — Trait-Definition + Default-Impl-Vorbild (`find_by_phase_id`)
- `genossi_dao_impl_sqlite/src/repayment_entry.rs:1-418` — SQLite-Impl-Pattern + In-Memory-Test-Setup
- `genossi_service/src/member.rs:104-143` — `MemberService`-Trait-Signatur
- `genossi_service_impl/src/member.rs:1-120` — `MemberServiceImpl` + Permission-Funnel-Pattern
- `genossi_service_impl/src/member_action.rs:155-177` — `compute_dates` Pure-Function-Vorbild
- `genossi_service_impl/src/repayment_phase.rs:976-989` — mockall-Default-Impl-Override-Falle dokumentiert
- `genossi_service/src/permission.rs:28` — `ADMIN_PRIVILEGE = "admin"`
- `genossi_rest/src/member.rs:1-346` — REST-Handler-Pattern + Router + OpenAPI
- `genossi_rest/src/repayment_entry.rs:59-141` — `IntoParams`/`Query<>`-Pattern
- `genossi_rest/src/lib.rs:104-117` — Global `From<ServiceError> for RestError` (PermissionDenied → 401)
- `genossi_rest_types/src/lib.rs:1-150, 2189-2360` — TO-Pattern + `AttendanceMemberTO`-Slim-Vorbild
- `genossi_bin/tests/repayment_letter_e2e.rs:104-262` — E2E-Helper-Pattern für Members
- `.planning/STATE.md` (Plan-03-Lektion, Plan-08-05-Eintrag, Plan-08-06-Eintrag, Plan-07-04-Eintrag) — mockall-Falle, Sub-Route-Ordering, exit_date-Setup, PermissionDenied-401
- `.planning/phases/14-dao-domain-foundation/14-CONTEXT.md` — 15 Decisions D-14-01..15

### Secondary (MEDIUM confidence)
- `.planning/research/ARCHITECTURE.md` §3 — H1/H2-Pure-Function-Skeleton (vor D-14-01 Tuple-Variante; ersetzt durch CONTEXT.md Struct-Variante)
- `.planning/research/PITFALLS.md` Kat 4 + Kat 7 — H1/H2 Edge-Cases + Empfänger-Search-Soft-Delete
- `time` 0.3 docs — `Date::from_calendar_date` Range/Errors

### Tertiary (LOW confidence)
- Keine LOW-confidence-Claims; alle Patterns sind Codebase-grounded.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — alle Crates bereits im Workspace, Versionen verifiziert
- Architecture: HIGH — alle Patterns durch v1.1-File-Line-Verweise abgedeckt
- Pitfalls: HIGH — alle 6 Pitfalls codebase-grounded (file:line references); 4 davon haben STATE.md-Lektion-Einträge als zusätzliche Bestätigung

**Research date:** 2026-06-04
**Valid until:** unlimited within v1.2 milestone — Phase 14 ist read-only Foundation; nur strukturelle Code-Änderungen in Phase 15-17 könnten Pattern-Updates erfordern (z.B. wenn Phase 15 Pure-Function um `pub`-Re-Export erweitert).
