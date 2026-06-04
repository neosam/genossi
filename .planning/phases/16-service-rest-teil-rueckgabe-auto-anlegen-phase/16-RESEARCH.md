# Phase 16: Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase - Research

**Researched:** 2026-06-04
**Domain:** Rust backend (Service + REST extension on existing genossi `MembershipAdjustService`)
**Confidence:** HIGH (codebase audit; all upstream APIs verified against source)

## Summary

Phase 16 erweitert den in Phase 15 etablierten `MembershipAdjustService` um die Teil-Rückgabe-Operation. **Sehr viel weniger neue Bausteine als das CONTEXT suggeriert** — die kritischen Foundation-Stücke (`RepaymentEntryDao::find_by_member_and_phase`, `RepaymentEntryStatus`-Enum mit drei Varianten, Auto-Fill-Loop in `open_repayment_phase`, `audited_create!`-Macro, `RepaymentEntryEntity::Auditable`, ISO8601-Date-Serde, Sub-Route-Pattern, REST-Handler-File) existieren bereits.

**Drei Stellen brauchen echte Architektur-Entscheidung durch den Planner:**

1. **`RepaymentPhaseService::create_repayment_phase` akzeptiert KEIN `tx`-Parameter** ([VERIFIED: genossi_service/src/repayment_phase.rs:111-115](file:///home/neosam/programming/rust/projects/genossi3/genossi_service/src/repayment_phase.rs)). Die Methode commitet ihre eigene Tx intern. Das bricht das D-16-04 "Single-Tx atomar"-Versprechen. Planner muss entscheiden zwischen (a) Trait-Erweiterung um tx-akzeptierende Variante, (b) Inlining der Create-Logik im `MembershipAdjustServiceImpl`, oder (c) Auto-Create in **separater** Tx VOR der Entry-Create-Tx (semantisch lockerer, aber CONTEXT D-16-04 explizit dagegen).

2. **Kein `find_by_fiscal_year` auf `RepaymentPhaseDao`-Trait oder SQLite-Impl.** Aber: SQLite `dump_all` ordnet bereits `ORDER BY fiscal_year DESC, created DESC` ([VERIFIED: genossi_dao_impl_sqlite/src/repayment_phase.rs:81-88](file:///home/neosam/programming/rust/projects/genossi3/genossi_dao_impl_sqlite/src/repayment_phase.rs)). Damit ist `dump_all().iter().find(|p| p.fiscal_year == target)` bzw. `.first()` für share_value-Lookup trivial — keine neue DAO-Query nötig (D-16-05).

3. **Existing Auto-Fill-Loop hat KEIN Skip-Pattern** ([VERIFIED: genossi_service_impl/src/repayment_phase.rs:368-394](file:///home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/repayment_phase.rs)). Insertion-Point für D-16-03 ist exakt zwischen Z. 368 (`for member in targets {`) und Z. 386 (`crate::audited_create!`).

**Primary recommendation:** Wegen der Single-Tx-Anforderung (D-16-04) und der Tatsache, dass `create_repayment_phase` nur 33 LOC ohne tx-Parameter ist, ist Inlining (Option b) am sichersten — UND die existierende Service-Methode bleibt unverändert für Phase 15/17. Der Audit-Process-String für die Auto-Created-Phase sollte dann der existing `"repayment-phase.create"` String bleiben (semantisch korrekt: "Phase wurde angelegt").

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Service-Trait-Erweiterung (`partial_repayment`) | Service Layer (trait) | — | D-16-17 inkrementelles Trait-Wachstum, Vorbild Phase 15 cancel/increase_shares |
| Service-Impl (Permission-Funnel, Validierung, Tx-Coordination, audited_create!) | Service Layer (impl) | — | D-15-01..03 etablierte Konvention; Foundation für Multi-DAO-Tx |
| Auto-Anlegen-Phase (Delegation oder Inlining) | Service Layer (impl) | — | D-16-02 Service-Delegation bevorzugt; falls Tx-Sharing blockiert: Inlining |
| Sum-Check (find_by_member_and_phase + filter + sum) | Service Layer (impl) | — | D-16-08 Service-Layer-Sum; DAO liefert nur die Rohliste |
| Auto-Fill-Skip-Pattern in `open_repayment_phase` | Service Layer (impl) | — | D-16-03; Insertion-Point Z. 368-386 |
| Range-Validation (pure function) | Service Layer (impl, `pub(crate)` helper) | — | D-15-05..08-Konvention; testbar ohne DAO-Mocks |
| Audit-Logging | Service Layer (audited_create! macro) | DAO Layer (Auditable trait already implemented) | AUDT-01; Macro-Pattern etabliert |
| REST-Endpoint (POST /api/members/{id}/partial-repayment) | REST Layer | — | D-16-14; in `membership_adjust.rs` erweitern |
| Request/Response-DTOs | Rest-Types | — | D-16-15..16; ISO8601-Date-Serde wiederverwenden |
| Sub-Route-Registration (vor /{id}) | REST Layer (`member.rs::generate_route`) | — | D-14-08-Lesson |
| DI-Wiring (RepaymentPhaseService + RepaymentEntryDao zu MembershipAdjustServiceDeps) | Binary Layer (`genossi_bin::lib.rs::RestStateImpl::new`) | — | Bestehender Slot wird erweitert |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PART-01 | Vorstand kann Teil-Rückgabe mit `n` (1..current_shares) + Willensbekundungs-Datum auslösen | REST-Handler-Datei existiert (`genossi_rest/src/membership_adjust.rs`), Sub-Route-Pattern etabliert, Request-DTO-Konvention dokumentiert |
| PART-02 | System berechnet H1/H2-Stichtag (Ziel-fiscal_year) analog Kündigung | `compute_effective_date` Pure-Function existiert in `genossi_service_impl/src/membership_adjust.rs:255-267`, wiederverwendbar 1:1 |
| PART-03 | System erzeugt `RepaymentEntry` in Ziel-Phase mit `share_count_to_pay_out = n`, Status `Open` | `RepaymentEntryEntity` und `RepaymentEntryStatus::Open` existieren; `RepaymentEntryEntity` ist auditable; `audited_create!`-Macro etabliert |
| PART-04 | Sum-Check `sum(open_entries.share_count) + n <= member.current_shares` | `RepaymentEntryDao::find_by_member_and_phase` existiert bereits auf Trait UND SQLite-Impl mit SQL-WHERE-Filter (`genossi_dao_impl_sqlite/src/repayment_entry.rs:185-217`); Service summiert in Code |
| PART-05 | Ziel-RepaymentPhase wird automatisch angelegt (Variante B per D-16-01) | `RepaymentPhaseService::create_repayment_phase` existiert — aber OHNE tx-Param (siehe Open Question #1); Auto-Fill-Skip-Insertion-Point bei `repayment_phase.rs:368` identifiziert |
| PART-06 | System erzeugt KEINE MemberAction und reduziert NICHT `current_shares` direkt | Service-Impl ruft KEIN `recalc_dates`/`recalc_migrated` (PITFALLS-Kat-10); `member_dao.update` wird NICHT verwendet |

## Project Constraints (from CLAUDE.md)

- **Tech-Stack-Lock:** Rust + Axum + SQLx + SQLite Backend — keine Sprach- oder DB-Wechsel.
- **Layered-Architektur:** DAO-Trait → Service-Trait → REST-Handler; neue DAO-Methoden auf Trait UND SQLite-Impl.
- **Audit-Pflicht:** Member, MemberAction, MemberDocument, Application **MUSS** `audited_create!`/`update!`/`delete!` Macros verwenden — direkter `dao.create/update`-Call ist Grep-Gate-Verstoß (AUDT-01). RepaymentEntry implementiert `Auditable` bereits (genossi_dao/src/repayment_entry.rs:65-87) und MUSS Macros verwenden.
- **Soft-Delete:** Kein DELETE; `deleted: Option<PrimitiveDateTime>` setzen; alle Queries müssen `WHERE deleted IS NULL` filtern (Default-Impl macht das).
- **Optimistic-Locking:** `version: Uuid` — DAO `update` setzt neue Version intern; Service darf `entity.version` NICHT vor `audited_update!` bumpen (siehe Phase 15 D-15-Note in membership_adjust.rs:216-223).
- **jj statt git:** Repo ist Jujutsu (`.jj/` + `.git/`); Commits via `gsd-sdk query commit` (orchestrator-managed).
- **GSD-Workflow-Enforcement:** Edit/Write nur via `/gsd-execute-phase` o.ä.
- **Tests Pflicht:** User's Global-Memory: "Always make sure you have tests for the changes."

## User Constraints (from CONTEXT.md)

### Locked Decisions

(Copied verbatim from 16-CONTEXT.md `<decisions>`.)

- **D-16-01:** Variante B — Auto-Create RepaymentPhase in Status `Open` + Auto-Fill-Skip-Pattern; D-11.1-Phase-Status-Guard bleibt unangetastet.
- **D-16-02:** Phase-Auto-Create via existing `RepaymentPhaseService::create_repayment_phase` (Delegation, nicht direkter DAO+audited_create!). `MembershipAdjustServiceImpl` bekommt neue Dependency `repayment_phase_service`.
- **D-16-03:** Auto-Fill-Skip-Lookup per-Member im Loop (nicht Bulk-Prefetch). In `open_repayment_phase`-Auto-Fill-Loop (Z. 319-395, Skip-Check-Insertion-Point bei Z. 368-386) wird pro iterierten Member ein `find_by_member_and_phase`-Call gemacht; non-empty → `continue`.
- **D-16-04:** Single-Tx atomar für Phase-Auto-Create + Entry-Create. Beide Operationen teilen denselben `tx`-Handle.
- **D-16-05:** `share_value` aus letzter existierender `RepaymentPhase` übernehmen (unabhängig vom Status). Planner-Discretion: `dump_all()` + Sort vs. neue targeted Query.
- **D-16-06:** Fallback bei keiner Vorgänger-RepaymentPhase: hardcoded Default → HTTP 200 (kein 409).
- **D-16-07:** `DEFAULT_SHARE_VALUE_CENT: i64 = 10000` (= 100 EUR pro Anteil).
- **D-16-08:** Service-Layer-Sum-Check mit `find_by_member_and_phase`. Filter `status != PaidOut`, summieren, prüfen.
- **D-16-09:** Status-Filter `status != PaidOut` (nicht nur `status == Open`).
- **D-16-10:** Gekündigtes Mitglied (`exit_date IS NOT NULL`) → HTTP 409 Conflict, blocken (NICHT 400 wie bei UPGD-04).
- **D-16-11:** Voll-Rückgabe (`shares == current_shares`) → HTTP 400 Bad Request, blocken mit Verweis auf cancel_membership.
- **D-16-12:** Range-Validation `1 <= shares < current_shares` (strikt), VOR Sum-Check und VOR Auto-Anlegen.
- **D-16-13:** Audit-Process-String `"member-adjust.partial-repayment"` (`const PARTIAL_REPAYMENT_PROCESS`).
- **D-16-14:** REST-Endpoint `POST /api/members/{id}/partial-repayment` MUSS vor `/{id}` registriert werden (D-14-08).
- **D-16-15:** Request-DTO `PartialRepaymentRequestTO { willensbekundung_date: Date, shares: i32 }` (siehe Open Question #3 zu i32 vs. i64) in `genossi_rest_types/src/lib.rs`.
- **D-16-16:** Response-Body `{ entry: RepaymentEntryTO, member: MemberTO, phase: Option<RepaymentPhaseTO> }`. `phase` nur befüllt bei Auto-Anlegen.
- **D-16-17:** Trait wächst inkrementell um `partial_repayment` (drittes Methode nach cancel_membership + increase_shares).
- **D-16-18:** `validate_willensbekundung_date` wird wiederverwendet (Phase 15 D-15-05..08).
- **D-16-19:** KEINE direkte MemberAction-Erzeugung, KEINE `Member.current_shares`-Mutation, KEIN `recalc_dates`/`recalc_migrated`.

### Claude's Discretion

- `find_by_member_and_phase`-DAO-Methode-Lokation: bereits auf `RepaymentEntryDao` mit SQL-Override realisiert.
- `share_value`-Lookup-Mechanik: `dump_all` reicht (SQLite ordnet bereits `ORDER BY fiscal_year DESC`).
- Handler-Datei-Placement: `genossi_rest/src/membership_adjust.rs` erweitern.
- Plan-File-Aufteilung: Planner-Discretion (CONTEXT empfiehlt 4 Plans).
- Response-DTO-Naming: benannt für OpenAPI bevorzugt.
- Auto-Anlegen-Reihenfolge: `match`/`if let` ODER Helper `ensure_repayment_phase`.
- Permission-Doppel-Check: OK, nicht umgehen.
- E2E-Tests: Roadmap-Test #5 durch zwei Variante-B-Tests ersetzen.

### Deferred Ideas (OUT OF SCOPE)

- `transfer_shares` → Phase 17
- AUDT-02 (shared Process-String für Übertrag-Pair) → Phase 17
- Frontend-Modal → Phase 18
- Variante A/C bewusst verworfen
- Bulk-Prefetch für Skip-Pattern
- Targeted `sum_open_shares_by_member_and_phase`-DAO-Query
- Audit-Macro-Erweiterung `audited_update_with!`
- `share_value`-Default als Config-Setting (YAGNI)
- Pessimistic-Lock auf Member während v1.2-Dialog
- `current_shares == 0` defensiver Check (durch Range-Validation abgedeckt)

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| async-trait | 0.1 | Service/DAO trait async methods | [VERIFIED: project Cargo.toml] etabliertes Pattern in allen Service-Traits |
| mockall | 0.13 | Trait mocks für Service-Unit-Tests | [VERIFIED] Phase 15 unit tests use `mock! { ... }` macro per-file |
| tokio | 1.35+ | Async runtime | [VERIFIED] Workspace-default |
| sqlx | 0.8 | DAO impl | [VERIFIED] Workspace-default |
| time | 0.3 | `Date`, `PrimitiveDateTime` | [VERIFIED] etabliert; H1/H2-Berechnung in `compute_effective_date` |
| uuid | 1.6 | Entity-IDs | [VERIFIED] etabliert |
| axum | 0.8.3 | REST-Handler | [VERIFIED] etabliert |
| utoipa | 5.0 | OpenAPI-Doku | [VERIFIED] etabliert |
| serde / serde_json | 1.0 | (De)Serialization | [VERIFIED] etabliert |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| reqwest | 0.11/0.12 | E2E HTTP-Client | E2E-Tests in `genossi_bin/tests/` |
| tracing | 0.1 | Structured logging | `#[instrument(skip(rest_state))]` auf Handlern |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `dump_all` + Filter für share_value-Lookup | targeted `find_latest` Query auf SQLite | Targeted ist effizienter (<10 Phasen → irrelevant); dump_all ordnet bereits `fiscal_year DESC, created DESC` |
| Delegated Phase-Auto-Create via Service | Inlined DAO+audited_create! im MembershipAdjustService | Service-Delegation ist sauberer, **aber** verlangt Trait-Erweiterung wegen fehlendem tx-Parameter (Open Question #1) |

**Installation:** Keine neuen Dependencies — alle nötigen Crates sind im Workspace.

**Version verification:** Keine Version-Bumps in Phase 16 — nur Erweiterungen bestehender Module.

## Architecture Patterns

### System Architecture Diagram

```
                       POST /api/members/{id}/partial-repayment
                                      │
                                      ▼
              ┌─────────────────────────────────────────────┐
              │   genossi_rest/src/membership_adjust.rs     │
              │   (extend with partial_repayment handler)   │
              │   - Path<Uuid>, Json<PartialRepaymentReqTO> │
              │   - error_handler wraps async block          │
              │   - Build response { entry, member, phase } │
              └────────────────┬────────────────────────────┘
                               │ rest_state.membership_adjust_service()
                               ▼
              ┌─────────────────────────────────────────────┐
              │   genossi_service_impl/membership_adjust.rs │
              │   (extend with partial_repayment impl)      │
              │                                              │
              │   1. use_transaction(tx)                    │
              │   2. permission_service.check(ADMIN)         │
              │   3. member = member_dao.find_by_id          │
              │   4. exit_date.is_some() → HTTP 409 (D-16-10)│
              │   5. validate_partial_repayment_shares(...) │
              │   6. validate_willensbekundung_date(...)    │
              │   7. effective = compute_effective_date(...)│
              │   8. ensure_repayment_phase(fiscal_year)    │
              │      ├─ find existing via dump_all+filter   │
              │      └─ if None: auto-create with share_value│
              │                  from latest phase or 10000 │
              │   9. existing = find_by_member_and_phase    │
              │  10. Sum-Check (filter != PaidOut, sum)    │
              │  11. audited_create!(RepaymentEntry,         │
              │      PARTIAL_REPAYMENT_PROCESS)              │
              │  12. transaction_dao.commit                  │
              │  13. Return (entry, member, Some/None phase)│
              └────────┬────────────────────────────────────┘
                       │
            ┌──────────┼─────────────┐
            ▼          ▼              ▼
       member_dao  repayment_      repayment_
                   phase_dao       entry_dao
                   (find_by_id /   (find_by_
                   create via      member_and_
                   service or      phase /
                   inline)         create via
                                   audited_create!)
            │          │              │
            └──────────┼──────────────┘
                       ▼
              ┌─────────────────────────────────────────────┐
              │   audit_log_dao (Hash-Chain SHA256)         │
              │   - audited_create! writes entries via      │
              │     get_latest_hash + build_create_entries  │
              └─────────────────────────────────────────────┘

                       ┃ (separate code path)
                       ▼
              ┌─────────────────────────────────────────────┐
              │   genossi_service_impl/repayment_phase.rs   │
              │   open_repayment_phase auto-fill loop       │
              │   (Z. 368-386 — INSERTION POINT)            │
              │                                              │
              │   for member in targets {                    │
              │       // NEW (D-16-03):                      │
              │       let existing = repayment_entry_dao    │
              │           .find_by_member_and_phase(        │
              │               member.id, phase_id, tx)?;   │
              │       if !existing.is_empty() { continue; } │
              │                                              │
              │       // existing logic Z. 369-393:          │
              │       audited_create!(RepaymentEntry, ...)  │
              │   }                                          │
              └─────────────────────────────────────────────┘
```

### Recommended Project Structure (Files to touch)

```
genossi_dao/src/
└── repayment_entry.rs                  # READ-ONLY: find_by_member_and_phase already exists (Z. 162-175)

genossi_dao_impl_sqlite/src/
└── repayment_entry.rs                  # READ-ONLY: SQL-Override already exists (Z. 185-217)

genossi_service/src/
└── membership_adjust.rs                # EXTEND: add partial_repayment to trait (Z. 22-44)

genossi_service_impl/src/
├── membership_adjust.rs                # EXTEND: add partial_repayment impl + PARTIAL_REPAYMENT_PROCESS const + DEFAULT_SHARE_VALUE_CENT const + validate_partial_repayment_shares helper + service_tests
└── repayment_phase.rs                  # EXTEND: insert Skip-Pattern at Z. 368-386 in open_repayment_phase

genossi_rest_types/src/
└── lib.rs                              # ADD: PartialRepaymentRequestTO + PartialRepaymentResponseTO (after Z. 544 MembershipAdjustResponseTO)

genossi_rest/src/
├── membership_adjust.rs                # EXTEND: add partial_repayment handler + register in ApiDoc
└── member.rs                           # EXTEND: add /{id}/partial-repayment route in generate_route (Z. 64-71 pattern)

genossi_bin/src/
└── lib.rs                              # EXTEND: add repayment_phase_service + repayment_entry_dao to MembershipAdjustServiceDependencies (Z. 484-504) AND to MembershipAdjustServiceImpl-construction (Z. 724-733)

genossi_bin/tests/
└── membership_adjust_e2e.rs            # EXTEND: add 8 E2E tests (or new file partial_repayment_e2e.rs — Planner-Discretion)
```

### Pattern 1: Service-Methode wiederverwendet Phase-15-Skelett

**What:** Permission-Funnel + Tx-Lifecycle + Datum-Validierung + Member-State-Check folgen exakt dem `increase_shares`-Aufbau.

**When to use:** Für `partial_repayment` 1:1 wiederverwenden.

**Example (verbatim aus genossi_service_impl/src/membership_adjust.rs:142-185):**
```rust
async fn increase_shares(...) -> Result<(MemberAction, Member), ServiceError> {
    let tx = self.transaction_dao.use_transaction(tx).await?;

    let user_id = self
        .permission_service
        .current_user_id(context.clone())
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());

    self.permission_service
        .check_permission(ADMIN_PRIVILEGE, context)
        .await?;

    if shares <= 0 { return Err(ServiceError::ValidationError(vec![...])); }

    let today = time::OffsetDateTime::now_utc().date();
    let validation_errors = validate_willensbekundung_date(willensbekundung_date, today);
    if !validation_errors.is_empty() { return Err(ServiceError::ValidationError(validation_errors)); }

    let member_entity = self.member_dao.find_by_id(member_id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(member_id))?;

    if member_entity.exit_date.is_some() { return Err(ServiceError::ValidationError(...)); }
    // ... entity construction + audited_create! + audited_update! + commit
}
```

**Phase 16 abweichend von Phase 15:**
- Exit-Date-Block ist `Conflict` statt `ValidationError` (D-16-10, HTTP 409 statt 400)
- Range-Validation ist Pure-Helper-Function `validate_partial_repayment_shares` (testbar)
- KEIN `audited_update!(member_dao, ...)` (PART-06: current_shares unverändert)
- ZUSÄTZLICH: `ensure_repayment_phase` + `find_by_member_and_phase`-Sum-Check + `audited_create!(repayment_entry_dao, ...)`
- Return-Type ist `(RepaymentEntry, Member, Option<RepaymentPhase>)` statt `(MemberAction, Member)`

### Pattern 2: audited_create!-Macro-Aufruf

**What:** Atomare DAO-create + Audit-Log-Insert via Macro-Expansion.

**Example (verbatim aus audit_macros.rs):**
```rust
crate::audited_create!(
    self,
    self.repayment_entry_dao,       // DAO trait-implementor
    &entry_entity,                  // &RepaymentEntryEntity
    PARTIAL_REPAYMENT_PROCESS,      // process string
    &user_id,                       // user_id: &str
    tx                              // tx: Self::Transaction (Clone-able)
);
```

**Wichtig:** Das Macro expandiert zu `$dao.create($entity, $process, $tx.clone()).await?` + `get_latest_hash + build_create_entries + create_entries`. `self.audit_log_dao` und `self.uuid_service` werden implizit verwendet — beide müssen im Dep-Struct existieren (sind sie schon, Phase 15).

### Pattern 3: Sub-Route-Registration vor /{id}

**What:** Literal Sub-Routes mit Path-Parameter vor Catch-All-Routes in axum.

**Example (verbatim aus genossi_rest/src/member.rs:64-76):**
```rust
.route("/{id}/cancel", post(crate::membership_adjust::cancel_membership::<RestState>))
.route("/{id}/increase-shares", post(crate::membership_adjust::increase_shares::<RestState>))
// NEW Phase 16:
.route("/{id}/partial-repayment", post(crate::membership_adjust::partial_repayment::<RestState>))
// Path-parameter routes LAST.
.route("/{id}", get(get_member::<RestState>))
```

**Hinweis:** axum 0.8 matched literale Segmente vor Path-Param-Segmenten innerhalb desselben nested Routers; technisch kann `/{id}/partial-repayment` an beliebiger Position relativ zu `/{id}` stehen. **Defensive Konvention** (D-14-08): vor `/{id}`.

### Anti-Patterns to Avoid

- **Direkter `repayment_entry_dao.create(...)`-Call:** Verletzt AUDT-01 (Grep-Gate). IMMER `audited_create!`.
- **Eigene Transaktion in `partial_repayment` öffnen UND eine zweite über `create_repayment_phase`:** Bricht D-16-04. Siehe Open Question #1.
- **`entity.version` vor `audited_update!` bumpen:** DAO setzt neue Version intern; falsch geriebener Bump führt zu `Version mismatch` (Phase 15 Rule-1-Fix; siehe membership_adjust.rs:216-223).
- **`recalc_dates`/`recalc_migrated` aufrufen:** PART-06 + PITFALLS-Kat-10 — Teil-Rückgabe erzeugt KEINE MemberAction.
- **Phase-Auto-Create AUSSERHALB der Service-Tx:** D-16-04 verletzt, Tx-Atomicity weg.
- **Phase-Auto-Create mit `share_value = 0`:** `validate_phase_fields` in repayment_phase.rs:69-87 lehnt `share_value <= 0` ab; Fallback MUSS DEFAULT_SHARE_VALUE_CENT sein.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Audit-Logging | Manuelles `audit_log_dao.create_entries(...)` | `audited_create!`-Macro | Konsistente Hash-Chain, kein Race auf get_latest_hash, Grep-Gate-konform |
| H1/H2-Berechnung | Eigene Date-Math | `compute_effective_date(willensbekundung)` (`membership_adjust.rs:255-267`) | Schaltjahr-Edge-Cases bereits getestet (D-14-04..06) |
| Willensbekundung-Datum-Validation | Inline Date-Bounds | `validate_willensbekundung_date(date, today)` (`membership_adjust.rs:284-301`) | Pure-Function, getestet für Vorjahr/aktuelles/nächstes/übernächstes Jahr |
| Member-by-Phase-Filter | dump_all + Loop | `RepaymentEntryDao::find_by_member_and_phase` (`repayment_entry.rs:162-175` Trait, `:185-217` SQLite-Impl) | SQL-WHERE-Override mit korrektem `deleted IS NULL`-Filter und stabilem `ORDER BY created ASC, id ASC` |
| Phase-Lookup-by-fiscal_year | Eigene SQL-Query | `repayment_phase_dao.dump_all(tx)` + `iter().find(|p| p.fiscal_year == target && p.deleted.is_none())` | dump_all ordnet `ORDER BY fiscal_year DESC, created DESC` — passend für "latest first"; <10 Phasen ist irrelevant für Performance |
| Tx-Lifecycle | Manual `pool.begin()` | `self.transaction_dao.use_transaction(tx).await?` | Vorhanden in `TransactionDao`; Service-Pattern in allen v1.2-Methoden |

**Key insight:** Phase 16 fügt im Backend **kein neues Engineering** hinzu — alle Bausteine existieren. Die Arbeit besteht aus Komposition + einer Architektur-Entscheidung (Open Question #1).

## Runtime State Inventory

Phase 16 ist **kein Rename/Refactor/Migration**. Trotzdem auf der Sicherheitsseite:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — keine Datenfelder werden umbenannt | none |
| Live service config | None | none |
| OS-registered state | None | none |
| Secrets/env vars | None | none |
| Build artifacts | None — pure feature addition | none |

## Common Pitfalls

### Pitfall 1: `RepaymentPhaseService::create_repayment_phase` öffnet eigene Transaktion (BLOCKER für D-16-04)

**What goes wrong:** Service-Delegation per D-16-02 ruft `repayment_phase_service.create_repayment_phase(submission, context)` auf, aber die Methode ([VERIFIED: genossi_service_impl/src/repayment_phase.rs:94-165](file:///home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/repayment_phase.rs)) startet intern eine eigene Tx (`use_transaction(None)` Z. 99) und commitet sie am Ende (Z. 163). Sobald wir danach im `partial_repayment` einen `audited_create!(RepaymentEntry, ..., tx_outer)` machen, sind das zwei separate Transaktionen — bei einem Fail der zweiten Tx bleibt die auto-erzeugte Phase persistiert.

**Why it happens:** `RepaymentPhaseService`-Trait wurde in Phase 7 entworfen, bevor es einen externen Aufrufer gab, der Tx-Sharing braucht.

**How to avoid:** Drei Optionen (Planner-Entscheidung):
- **(a) Trait-Erweiterung:** Add tx-akzeptierende Methode `create_repayment_phase_with_tx(submission, context, tx)` ODER ändere bestehende Signatur auf `Option<Self::Transaction>` (BREAKING für andere Caller).
- **(b) Inlining im MembershipAdjustServiceImpl:** Repliziere die 33 LOC von `create_repayment_phase` (Z. 94-165) inline; Audit-Process-String bleibt `REPAYMENT_PHASE_PROCESS_CREATE = "repayment-phase.create"`. Bricht Service-Boundary, ist aber das geringste Risiko für Phase 7/15/17.
- **(c) Auto-Create in separater Tx VOR Entry-Create:** Wenn Entry-Create fehlschlägt, bleibt die Phase. CONTEXT D-16-04 schliesst diese Variante explizit aus.

**Warning signs:** `git grep "use_transaction(None)" genossi_service_impl/src/` zeigt: alle 8 Service-Methoden öffnen eigene Tx. Keine einzige akzeptiert externen tx-Parameter.

**Empfehlung (Planner-Discretion, NICHT locked):** Variante (b) — kürzester Pfad, kein Schock für Phase 17/v2.

### Pitfall 2: `i32` vs. `i64` für `shares` (Type-Mismatch-Falle)

**What goes wrong:** CONTEXT D-15-13 schreibt `shares: i64` in Trait-Signatur, aber `MemberEntity.current_shares` ([VERIFIED: genossi_dao/src/member.rs:85](file:///home/neosam/programming/rust/projects/genossi3/genossi_dao/src/member.rs)) ist `i32`. `MemberActionEntity.shares_change` ist ebenfalls `i32`. `IncreaseSharesRequestTO.shares` ist ebenfalls `i32` ([VERIFIED: genossi_rest_types/src/lib.rs:532](file:///home/neosam/programming/rust/projects/genossi3/genossi_rest_types/src/lib.rs)). `RepaymentEntryEntity.share_count_to_pay_out` ist `i32` ([VERIFIED: genossi_dao/src/repayment_entry.rs:58](file:///home/neosam/programming/rust/projects/genossi3/genossi_dao/src/repayment_entry.rs)).

**Why it happens:** CONTEXT Z. 12 schreibt `shares: i64` — vermutlich Tippfehler.

**How to avoid:** Planner verwendet `i32` durchgängig (Trait, Request-DTO, validate_partial_repayment_shares, Sum-Check). Sum-Check braucht dann `let sum: i32 = existing.iter()....map(|e| e.share_count_to_pay_out).sum();` — Overflow ist mit `current_shares: i32` per Definition unmöglich, weil die Summe immer ≤ current_shares sein soll.

**Warning signs:** Wenn Planner doch `i64` nimmt, schlägt Trait-Impl-Compile mit Typ-Mismatch auf den Request-DTO oder die Entity fehl.

### Pitfall 3: `RepaymentEntryStatus` hat nur DREI Varianten

**What goes wrong:** D-16-09 filtert `status != PaidOut` "nicht nur status == Open". Klingt nach mehr als nur 2 Varianten. Tatsächlich existieren genau drei: `Open`, `Contacted`, `PaidOut` ([VERIFIED: genossi_dao/src/repayment_entry.rs:15-21](file:///home/neosam/programming/rust/projects/genossi3/genossi_dao/src/repayment_entry.rs)). Filter `!= PaidOut` = `Open || Contacted`.

**Why it happens:** Phase-8-Konvention. v1.1 toggled `Open ↔ Contacted` und einseitig finalisiert `PaidOut`.

**How to avoid:** Filter im Code `existing.iter().filter(|e| e.status != RepaymentEntryStatus::PaidOut)` ist semantisch korrekt: `Contacted` Entries sind angeschriebene-aber-noch-nicht-bezahlte Members, die in der Sum-Counter zählen.

### Pitfall 4: D-16-15 spricht von `i32`, CONTEXT Z. 12 von `i64`

**What goes wrong:** Die CONTEXT-Datei ist intern inkonsistent (Z. 12 sagt `i64`, D-16-15 sagt `i64`, aber Phase 15's `IncreaseSharesRequestTO.shares` ist `i32`).

**How to avoid:** Planner liest **vor Plan-Schreibung** das existing IncreaseSharesRequestTO und entscheidet konsistent. Empfehlung: `i32` für Konsistenz mit dem Rest der Codebase.

### Pitfall 5: `exit_date.is_some()`-Block returnt **Conflict** in Phase 16 (HTTP 409), nicht **ValidationError** (HTTP 400) wie Phase 15

**What goes wrong:** Phase 15 `increase_shares` ([VERIFIED: genossi_service_impl/src/membership_adjust.rs:178-183](file:///home/neosam/programming/rust/projects/genossi3/genossi_service_impl/src/membership_adjust.rs)) returnt `ServiceError::ValidationError` für gekündigte Members (HTTP 400 per Mapping). Phase 16 D-16-10 verlangt `ServiceError::Conflict` (HTTP 409). Wer Phase 15 als Vorlage kopiert, läuft in den Fehler.

**Why it happens:** Phase 15 hat die Entscheidung "ValidationError = 400" für UPGD-04 getroffen; Phase 16 trifft die Entscheidung "Conflict = 409" für PART. Beide sind Locked Decisions.

**How to avoid:** Inline-Comment beim Block: `// D-16-10: Conflict, NICHT ValidationError (Phase 15 UPGD-04 weicht ab)`.

### Pitfall 6: Test-Mock muss `find_by_member_and_phase` explizit setzen

**What goes wrong:** `#[automock]` überschreibt Default-Impls ([VERIFIED: genossi_dao/src/repayment_entry.rs:159-161](file:///home/neosam/programming/rust/projects/genossi3/genossi_dao/src/repayment_entry.rs)). Service-Unit-Tests, die nur `dump_all` mocken, schlagen mit "no expectation" fehl, sobald die Service-Methode `find_by_member_and_phase` aufruft.

**How to avoid:** Per-File-Mock-Pattern (Phase 15 vorgemacht in `service_tests`-Modul Z. 423-1085) muss `find_by_member_and_phase` explizit als `mock!`-Methode listen.

### Pitfall 7: SQLITE_BUSY-Race in E2E-Tests (PITFALLS-Kat-9)

**What goes wrong:** Multi-DAO-Cascade-Tests (Phase + Entry + Audit) im In-Memory-Pool können `SQLITE_BUSY` erzeugen.

**How to avoid:** Phase-15-E2E-Setup wiederverwenden ([VERIFIED: genossi_bin/tests/membership_adjust_e2e.rs:39-53](file:///home/neosam/programming/rust/projects/genossi3/genossi_bin/tests/membership_adjust_e2e.rs)) — `SqlitePool::connect("sqlite::memory:")` + `sqlx::migrate!("../migrations/sqlite").run(&*pool).await`. Phase-9-Pattern `busy_timeout(5000)` ist optional, Phase 15 nutzt es nicht und ist stabil.

### Pitfall 8: PITFALLS-Kat-1 Doppelbuchung — Auto-Fill-Skip muss BEIDE Pfade abdecken

**What goes wrong:** Wenn ein Member sowohl Teil-Rückgabe (v1.2 erzeugt Entry) als auch Kündigung mit Stichtag im selben fiscal_year hat, picked v1.1's Auto-Fill ihn beim Phase-Open zusätzlich auf → Duplikat-Entry. Die D-16-03-Skip-Pattern-Erweiterung in `open_repayment_phase` ist die einzige Mitigation.

**How to avoid:** Skip-Pattern MUSS in `repayment_phase.rs:368` (genau VOR dem `audited_create!` Call) eingebaut werden, sodass die Iteration sich nach `continue;` zum nächsten Member bewegt.

## Code Examples

Verified patterns from existing codebase:

### Skip-Pattern-Insertion-Point (verbatim Z. 366-393, Skip einzufügen ZWISCHEN Z. 368 und Z. 369)

```rust
// Source: genossi_service_impl/src/repayment_phase.rs:366-393
targets.sort_by_key(|m| m.member_number);

for member in targets {
    // ===== NEW (Phase 16 D-16-03 / PART-04 / PITFALLS-Kat-1): Skip-Pattern =====
    // Auto-Fill skippt Members, die durch v1.2-partial_repayment bereits einen
    // Open/Contacted-Entry in dieser Phase haben — verhindert Duplikat.
    let existing = self
        .repayment_entry_dao
        .find_by_member_and_phase(member.id, id, tx.clone())
        .await?;
    if !existing.is_empty() {
        continue;
    }
    // ===== /NEW =====

    let entry_now_offset = time::OffsetDateTime::now_utc();
    let entry_now_pdt =
        time::PrimitiveDateTime::new(entry_now_offset.date(), entry_now_offset.time());
    let new_entry = RepaymentEntryEntity {
        id: self.uuid_service.new_v4().await,
        member_id: member.id,
        phase_id: id,
        share_count_to_pay_out: member.current_shares,
        status: RepaymentEntryStatus::Open,
        created: entry_now_pdt,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    crate::audited_create!(
        self,
        self.repayment_entry_dao,
        &new_entry,
        REPAYMENT_PHASE_PROCESS_OPEN,
        &user_id,
        tx
    );
}
```

### Sum-Check-Snippet (Service-Layer)

```rust
// PART-04 / D-16-08 / D-16-09: Service-Layer-Sum-Check
let existing: Arc<[RepaymentEntryEntity]> = self
    .repayment_entry_dao
    .find_by_member_and_phase(member_id, target_phase.id, tx.clone())
    .await?;

let sum_open: i32 = existing
    .iter()
    .filter(|e| e.status != RepaymentEntryStatus::PaidOut)  // D-16-09
    .map(|e| e.share_count_to_pay_out)
    .sum();

if sum_open + shares > member_entity.current_shares {
    return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
        field: Arc::from("shares"),
        message: Arc::from(format!(
            "sum of open repayments ({}) plus new ({}) exceeds current_shares ({})",
            sum_open, shares, member_entity.current_shares
        )),
    }]));
}
```

### Phase-Lookup-by-fiscal_year (in-memory)

```rust
// D-16-05: latest by fiscal_year via dump_all (SQLite ordnet bereits DESC).
// "All" returns active (deleted IS NULL) — see RepaymentPhaseDao default impl.
let all_phases = self.repayment_phase_dao.all(tx.clone()).await?;

// Target phase for this fiscal_year:
let target_phase_opt = all_phases.iter().find(|p| p.fiscal_year == fiscal_year);

let target_phase = match target_phase_opt {
    Some(p) => p.clone(),
    None => {
        // D-16-05 + D-16-06: share_value aus latest oder Default
        let share_value = all_phases.first()  // ORDER BY fiscal_year DESC → first ist newest
            .map(|p| p.share_value)
            .unwrap_or(DEFAULT_SHARE_VALUE_CENT);
        // Create new phase ...
    }
};
```

### Audit-Process-String-Konstante (Phase 15 Vorbild)

```rust
// Source: genossi_service_impl/src/membership_adjust.rs:24-27 (Phase 15)
const CANCEL_PROCESS: &str = "member-adjust.cancel";
const UPGRADE_PROCESS: &str = "member-adjust.upgrade";

// Phase 16 add (D-16-13):
const PARTIAL_REPAYMENT_PROCESS: &str = "member-adjust.partial-repayment";

// D-16-07:
pub(crate) const DEFAULT_SHARE_VALUE_CENT: i64 = 10000;
```

### REST-Handler-Stub (Phase 15 cancel_membership Vorbild)

```rust
// Source pattern: genossi_rest/src/membership_adjust.rs:50-79
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "/{id}/partial-repayment",
    params(("id" = Uuid, Path, description = "Member ID")),
    request_body = PartialRepaymentRequestTO,
    responses(
        (status = 200, description = "Partial repayment successful", body = PartialRepaymentResponseTO),
        (status = 400, description = "Validation error (shares out of range, date bounds, sum-check violation)"),
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 404, description = "Member not found"),
        (status = 409, description = "Member cancelled (exit_date set)"),
    ),
)]
pub async fn partial_repayment<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(req): Json<PartialRepaymentRequestTO>,
) -> Response {
    error_handler(
        (async {
            let (entry, member, phase) = rest_state
                .membership_adjust_service()
                .partial_repayment(
                    member_id,
                    req.shares,
                    req.willensbekundung_date,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let response = PartialRepaymentResponseTO {
                entry: RepaymentEntryTO::from(&entry),
                member: MemberTO::from(&member),
                phase: phase.as_ref().map(RepaymentPhaseTO::from),
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response)?))
                .unwrap())
        })
        .await,
    )
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Direkter `dao.create(...)`-Aufruf | `audited_create!`-Macro | Phase 5/6/7 | Pflicht für AUDT-01-Grep-Gate |
| `find_by_member_and_phase` als Default-Impl | SQL-WHERE-Override im SQLite-Impl | Phase 14 (PITFALLS Kat 1 Mitigation, Plan 14-02) | Skalierung auf reale Genossi-Größe; bereits implementiert |
| Inline-Datum-Bounds-Check | Pure-Function `validate_willensbekundung_date(date, today)` | Phase 15 (D-15-05..08) | Testbar ohne Clock-Abhängigkeit |
| Phase-Open ohne Skip-Schutz | Skip-Pattern in Auto-Fill-Loop | Phase 16 (D-16-03) | Verhindert Duplikat-Entries; KRITISCH für v1.2-Korrektheit |

**Deprecated/outdated:**
- `.planning/notes/membership-adjust-design.md` Z. 23 sagt "Teil-Rückgabe → MemberAction (−n)" — durch PART-06 / PITFALLS überschrieben. Plan-Implementierer NICHT folgen.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | "Variante (b) Inlining ist der pragmatischste Weg für die Tx-Sharing-Frage" | Open Question #1 | Planner kann Variante (a) Trait-Erweiterung wählen — würde nur Phase-7-Service-Trait einmal anfassen, ist akzeptabel. |
| A2 | "`i32` für shares ist konsistent mit dem Rest" | Pitfall #2, Pitfall #4 | Wenn doch `i64` gewählt wird (CONTEXT Z. 12 wörtlich nehmen), brechen alle TO-Konvertierungen. Compile-time Failure, kein silent bug. |
| A3 | "dump_all+iter().find ist ausreichend für fiscal_year-Lookup" | Code Examples Phase-Lookup | Bei >1000 Phasen wäre eine targeted Query nötig. Realistic <10 Phasen pro Genossi → irrelevant. |

**Empty:** Alle anderen Claims sind via Source-Code-Grep oder explizite CONTEXT-Locked-Decisions verifiziert.

## Open Questions (RESOLVED)

> All four questions were resolved during /gsd-plan-phase 16. Resolutions are reflected in plan files 16-01..16-04. This section retains the analysis for traceability.

1. **Wie wird `RepaymentPhaseService::create_repayment_phase` in derselben Tx wie `audited_create!(RepaymentEntry)` ausgeführt?** (D-16-04 verlangt Single-Tx)
   - What we know: Methode öffnet aktuell eigene Tx via `use_transaction(None)`. Signatur akzeptiert kein tx-Parameter.
   - What's unclear: Trait-Erweiterung (a) vs. Inlining (b) vs. separate Tx (c). CONTEXT bevorzugt (a)+(b), schließt (c) aus.
   - Recommendation: **Planner entscheidet im Plan 01 oder 02.** Inline-Comment auf die gewählte Variante mit Verweis auf D-16-02 + D-16-04.
   - **RESOLVED:** Variante (b) **Inlining** gewählt. Plan 02 reproduziert die ~33 LOC `create_repayment_phase`-Logik inline in `partial_repayment` und nutzt die gemeinsame `tx`. Begründung: minimaler Blast-Radius (kein Touchen von Phase 7/15/17-Code), volle Tx-Atomicity. Inline-Doc-Comment dokumentiert die Entscheidung mit Verweis auf D-16-02 + D-16-04.

2. **Wo lebt `RepaymentEntryTO` und `RepaymentPhaseTO` für die Response?**
   - What we know: `MemberTO`, `MemberActionTO`, `CancelMembershipRequestTO` sind in `genossi_rest_types/src/lib.rs`. `RepaymentEntryTO` und `RepaymentPhaseTO` sind dort — anzunehmen ist, dass sie nicht existieren (oder anders heissen).
   - What's unclear: Existierende TO-Namen für RepaymentEntry/RepaymentPhase im REST-Layer.
   - Recommendation: Planner führt zu Beginn von Plan 04 `grep -n "RepaymentEntryTO\|RepaymentPhaseTO" genossi_rest_types/src/lib.rs` aus und prüft. Falls nicht vorhanden: bestehende ad-hoc Types im REST-Layer (`genossi_rest/src/repayment_entry.rs`, `genossi_rest/src/repayment_phase.rs`) übernehmen oder neue ToSchema-DTOs erstellen.
   - **RESOLVED:** Plan 04 Task 1 startet mit Grep-Discovery für `RepaymentEntryTO`/`RepaymentPhaseTO` in `genossi_rest_types/src/lib.rs` und im REST-Layer; bestehende DTOs werden wiederverwendet bzw. ergänzt. Die Acceptance-Criteria verlangen, dass die DTOs am Ende ToSchema implementieren und im Response-Body korrekt verdrahtet sind.

3. **`shares: i32` oder `i64`?** Siehe Pitfall #2 + #4.
   - Recommendation: `i32` — konsistent mit `MemberEntity.current_shares` (i32), `IncreaseSharesRequestTO.shares` (i32), `RepaymentEntryEntity.share_count_to_pay_out` (i32). CONTEXT Z. 12 ist vermutlich Tippfehler.
   - **RESOLVED:** Plans 01, 02 verwenden durchgängig `i32` für `shares`, Sum, Range-Checks, DTO-Felder. `DEFAULT_SHARE_VALUE_CENT` bleibt `i64` (entspricht dem `share_value`-Feld auf der RepaymentPhase-Entity); shares ≠ share_value-Trennung damit sauber. CONTEXT-Tippfehler bei `i64`-Erwähnungen in D-16-08/D-16-12 ist im Plan-Layer überschrieben.

4. **Soll die auto-erzeugte Phase im Audit-Log denselben Process-String wie der Entry erhalten?**
   - What we know: D-16-13 verlangt für den Entry `"member-adjust.partial-repayment"`. Phase-Auto-Create per D-16-02 nutzt die existing Service-Methode, also `REPAYMENT_PHASE_PROCESS_CREATE = "repayment-phase.create"`. Bei Inlining (Variante b) müsste der Planner entscheiden: gleicher String wie Service-Delegation oder eigener `"member-adjust.partial-repayment.auto-create-phase"`?
   - Recommendation: Bei Variante (b) den existing String `"repayment-phase.create"` beibehalten (semantisch identische Operation; konsistent mit anderen Phase-Create-Audit-Einträgen). Vorstand sieht "Phase wurde von Auto-Create-Mechanismus angelegt" durch die zeitliche Nähe zum Partial-Repayment-Audit-Eintrag.
   - **RESOLVED:** Plan 02 nutzt für die inline-erzeugte Phase den existing String `"repayment-phase.create"` (konsistent mit der Service-Delegations-Semantik); der RepaymentEntry-Create nutzt `"member-adjust.partial-repayment"` per D-16-13. Zwei separate Audit-Transaktionen sind erwartetes Verhalten.

## Environment Availability

(Phase ist reine Code-Erweiterung im bestehenden Cargo-Workspace.)

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build/Test | ✓ (assumed; CLAUDE.md `cargo build` works) | 2021 edition | — |
| sqlx-cli | Migration (no new migration in Phase 16) | n/a | — | — |
| SQLite | Runtime | ✓ | embedded in sqlx | — |

**Missing dependencies with no fallback:** None.

## Sources

### Primary (HIGH confidence — verified by reading source)

- `genossi_service/src/membership_adjust.rs` (Phase 15 trait, lines 1-44)
- `genossi_service_impl/src/membership_adjust.rs` (Phase 15 impl + pure helpers + service_tests, lines 1-1085)
- `genossi_service_impl/src/repayment_phase.rs` (open_repayment_phase Z. 267-423 + create_repayment_phase Z. 94-165)
- `genossi_service/src/repayment_phase.rs` (RepaymentPhaseService trait — `create_repayment_phase` signature Z. 111-115)
- `genossi_dao/src/repayment_entry.rs` (RepaymentEntryStatus enum, RepaymentEntryEntity, Auditable impl, find_by_member_and_phase trait method)
- `genossi_dao/src/repayment_phase.rs` (RepaymentPhaseEntity, share_value: i64, fiscal_year: i32)
- `genossi_dao_impl_sqlite/src/repayment_entry.rs` (SQL-Override für find_by_member_and_phase Z. 185-217)
- `genossi_dao_impl_sqlite/src/repayment_phase.rs` (dump_all ordnet ORDER BY fiscal_year DESC, created DESC Z. 81-88)
- `genossi_dao/src/member.rs` (MemberEntity-Struktur, exit_date: Option<time::Date> Z. 89, current_shares: i32 Z. 85)
- `genossi_service_impl/src/audit_macros.rs` (audited_create! Macro-Definition)
- `genossi_rest/src/member.rs` (generate_route Sub-Route-Pattern Z. 29-77)
- `genossi_rest/src/membership_adjust.rs` (Phase 15 cancel_membership + increase_shares Handler)
- `genossi_rest/src/lib.rs` (RestStateDef trait, membership_adjust_service slot Z. 230-255)
- `genossi_rest/src/test_server.rs` (start_test_server + state-trait-bounds)
- `genossi_rest_types/src/lib.rs` (iso8601_date_required, CancelMembershipRequestTO, IncreaseSharesRequestTO, MembershipAdjustResponseTO Z. 505-544)
- `genossi_bin/src/lib.rs` (RestStateImpl Z. 485-733)
- `genossi_bin/tests/membership_adjust_e2e.rs` (E2E-Test-Pattern aus Phase 15)

### Secondary (HIGH confidence — documented decisions)

- `.planning/phases/16-service-rest-teil-rueckgabe-auto-anlegen-phase/16-CONTEXT.md` (D-16-01..19 Locked Decisions)
- `.planning/research/PITFALLS.md` (Kat 1, 2, 4, 6, 9, 10)
- `.planning/REQUIREMENTS.md` (PART-01..06)
- `.planning/ROADMAP.md` (Phase 16 §)
- `.planning/phases/15-service-rest-kuendigung-aufstockung/15-CONTEXT.md` (D-15-02, D-15-09, D-15-11, D-15-13 carry-forward)

### Tertiary (FYI)

- `.planning/notes/membership-adjust-design.md` — Master-Design-Doc; **WARNUNG**: Z. 23 obsolet (siehe State of the Art Deprecated-Block).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — alle Crates und Patterns verified via source-grep
- Architecture: HIGH — Insertion-Points Z.368-386 in repayment_phase.rs + Z.94-165 für create_repayment_phase + Phase-15-Skelett-Wiederverwendung sind alle direkt aus Source belegt
- Pitfalls: HIGH — Pitfall #1 (Tx-Sharing) ist die einzige echte Open-Question und durch direkten Source-Read verifiziert
- Audit/Auditable: HIGH — `RepaymentEntryEntity` implementiert `Auditable` bereits (genossi_dao/src/repayment_entry.rs:65-87) mit FROZEN-Order-Test (Z. 261-287); `audited_create!`-Macro hat keine Quirks für diesen Entity-Typ
- DAO-find_by_member_and_phase: HIGH — Trait UND SQLite-Impl existieren mit SQL-WHERE-Override; auch Service-Unit-Test- und SQLite-Integration-Tests sind vorhanden

**Research date:** 2026-06-04
**Valid until:** 2026-07-04 (30 Tage — Codebase stable, keine externen Abhängigkeiten)
