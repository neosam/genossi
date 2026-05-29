# Phase 1: Assembly-Aggregat + Audit-Hardening - Research

**Researched:** 2026-05-02
**Domain:** Backend-Aggregat (DAO + Service + REST) mit Audit-Hashchain-Integration
**Confidence:** HIGH (alle Patterns vom Bestand verifiziert per Read; keine externen Frameworks neu)

## Summary

Phase 1 etabliert das `Assembly`-Aggregat als drittes auditiertes Backend-Aggregat (nach `Member` und `Application`). Die Implementation ist primär Code-Replikation: bestehende Patterns (`MemberStatus`-Enum, `ApplicationDaoImpl`-DAO-Layout, `audited_create!/audited_update!`-Macros, Axum-Handler-Pattern mit `error_handler()`) werden eins zu eins für Assembly übernommen. Drei genuin neue Aspekte: (1) Member-Universe-Snapshot in eigener Tabelle, atomar zusammen mit dem Status-Übergang `Preparation → Open` befüllt; (2) Status-Transition-Guard im Service-Layer, der `ServiceError::Conflict` bei illegalen Übergängen wirft; (3) explizite Process-Identifier-Strategie `"assembly.create"` / `"assembly.open"` / `"assembly.close"` (Punkt-Notation), damit der Audit-Log-Endpoint später nach Lifecycle-Aktion filtern kann.

Der Hauptrisikobereich ist die atomare Snapshot-Befüllung: Service-Methode `open_assembly` muss in einer einzigen Transaktion (a) `assembly`-Update mit `status=Open` + `opened_at=now` via `audited_update!`, (b) Snapshot-Query auf aktive Member, (c) Batch-Insert in `assembly_member_snapshot`, (d) gemeinsamer Commit. Fehler in einem der Schritte muss alles rollbacken — Standard-Genossi-Pattern, kein neues Konzept, aber kritisch korrekt zu replizieren.

**Primary recommendation:** Den `Member`/`Application`-Stack 1:1 als Vorlage kopieren; nur bei Status-Lifecycle-Logik und Snapshot-Befüllung neuen Code schreiben. Den `count_active`-Filter-Predikat aus `genossi_dao/src/member.rs:172-184` direkt in der Snapshot-Befüll-Methode verwenden (per Aufruf von `MemberDao::all()` + Inline-Filter), damit die Logik genau einmal existiert.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Assembly-Persistenz (Tabelle, CRUD) | DAO (`genossi_dao_impl_sqlite`) | DAO-Trait (`genossi_dao`) | Genossi-Konvention: jede neue Entität erhält eigenen DAO-Trait + Sqlite-Impl |
| Snapshot-Persistenz | DAO (`genossi_dao_impl_sqlite`) | DAO-Trait (`genossi_dao`) | Eigene Tabelle, eigener DAO; zwei Tabellen → zwei DAOs |
| Lifecycle-Logik (Preparation → Open → Closed) | Service (`genossi_service_impl`) | — | Service ist die Domain-Schicht; DAO bleibt agnostisch |
| Snapshot-Befüllung beim Öffnen | Service (`genossi_service_impl`) | DAO (`MemberDao::all` + Filter) | Service orchestriert; nutzt MemberDao über DI |
| Audit-Logging der Lifecycle-Calls | Service (`audited_*!`-Macros) | DAO (`AuditLogDao`) | Macros leben im Service-Layer; AuditLogDao schreibt physisch |
| HTTP-Endpoints (POST/PUT/GET) | REST (`genossi_rest`) | Service (`AssemblyService`) | Axum-Handler delegieren an Service; REST kennt keine Domain-Logik |
| Permission-Check (admin) | REST (Wrapper) → Service (`PermissionService`) | — | Pattern in jedem Service-Aufruf: `permission_service.check_permission("admin", auth)` |
| OpenAPI-Schema-Generierung | REST (`utoipa::OpenApi`) | Types (`genossi_rest_types::AssemblyTO`) | Utoipa-Annotationen am Handler + ToSchema am TO |
| DI-Wiring | Binary (`genossi_bin/src/lib.rs::RestStateImpl::new`) | — | Genossi-Konvention: alle Services werden hier instanziiert und im RestStateImpl gehalten |

## Standard Stack

### Core (bestehend, nichts Neues)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `axum` | 0.8.3 | HTTP-Server, Handler-Routing | Bereits projektweit für REST [VERIFIED: genossi_rest/src/lib.rs] |
| `sqlx` | 0.8 | DB-Queries, Transaction-Management | Bereits für alle Tabellen in Verwendung [VERIFIED: genossi_dao_impl_sqlite/src/application.rs:5] |
| `tokio` | 1.35+ | Async runtime | Standard für gesamten Backend-Stack |
| `utoipa` | 5.0 | OpenAPI-Schema aus Rust-Types | Bereits projektweit; `#[derive(ToSchema)]` und `#[utoipa::path(...)]` [VERIFIED: genossi_rest/src/application.rs:118] |
| `time` | 0.3 | `PrimitiveDateTime`, `Date` für DB | Standard im DAO-Layer [VERIFIED: genossi_dao/src/application.rs:58] |
| `uuid` | 1.6 | Entity-IDs (BLOB in SQLite) | Standard [VERIFIED: genossi_dao/src/application.rs:46] |
| `serde` / `serde_json` | 1.0 | TO-Serialization | Standard im REST-Types-Layer |
| `tracing` | 0.1 | `#[instrument]` auf Handlern | Standard [VERIFIED: genossi_rest/src/application.rs:117] |
| `async-trait` | — | Trait-Definitions mit async Methoden | Standard im DAO/Service [VERIFIED: genossi_dao/src/application.rs:1] |
| `mockall` | 0.13 | `#[automock]` für Trait-Mocks | Standard im DAO/Service-Layer [VERIFIED: genossi_dao/src/application.rs:2] |

### Supporting (intern, bestehende Macros/Helfer)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `gen_service_impl!` | intern | Generiert Service-Struct + Deps-Trait | Für `AssemblyServiceImpl` zwingend [VERIFIED: genossi_service_impl/src/macros.rs] |
| `audited_create!` | intern | DAO-Insert + Audit-Entries atomar | `create_assembly` [VERIFIED: genossi_service_impl/src/audit_macros.rs:6] |
| `audited_update!` | intern | DAO-Update + Diff-Audit-Entries atomar | `open_assembly`, `close_assembly`, `update_assembly` [VERIFIED: audit_macros.rs:43] |
| `Auditable`-Trait | intern | `entity_type/entity_id/audit_fields` für Diff | `AssemblyEntity` impl [VERIFIED: genossi_dao/src/auditable.rs] |
| `error_handler()` | intern | Wrappt async Block, mappt RestError → HTTP | Jeder Axum-Handler [VERIFIED: genossi_rest/src/lib.rs:130] |
| `extract_auth_context()` | intern | Extension<Context> → Authentication | Jeder Handler [VERIFIED: genossi_rest/src/lib.rs:50] |
| `iso8601_datetime` serde-Modul | intern | ISO8601-Roundtrip für `Option<PrimitiveDateTime>` | TO-Felder `opened_at`, `closed_at`, `date` [VERIFIED: genossi_rest_types/src/lib.rs:9] |

### Alternatives Considered (alle abgelehnt)
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Eigene Snapshot-Tabelle | Snapshot-JSON-Spalte in `assembly` | CONTEXT D-01 lockt eigene Tabelle (Indexierbarkeit, JOIN-Fähigkeit gegen `member`) — kein Re-Design |
| Cache `member_universe_count` in `assembly` | Ad-hoc COUNT-Query | CONTEXT D-04 lockt ad-hoc COUNT (kein Cache-Drift-Risiko) |
| `manage_assemblies`-Permission | Bestehende `admin`-Permission | CONTEXT D-14 lockt `admin` |
| Hand-gerollte SHA256-Audit-Logik | Bestehende Macros | Bestehende Macros sind explizit für genau diesen Use-Case da |

**Installation:** Keine neuen Dependencies. Alle benötigten Crates sind bereits Workspace-Mitglieder.

**Version verification:** Skipped — keine neuen externen Pakete.

## Architecture Patterns

### System Architecture Diagram

```
┌─────────────┐
│ HTTP-Client │  POST /api/assembly
└──────┬──────┘  POST /api/assembly/{id}/open
       │        POST /api/assembly/{id}/close
       │        PUT  /api/assembly/{id}
       │        GET  /api/assembly[, /{id}]
       v
┌─────────────────────────────────────────────────┐
│ Axum-Router (genossi_rest)                       │
│  - Auth-Middleware (extract Context → Extension) │
│  - CORS / Rate-Limit / Security-Headers          │
└──────┬──────────────────────────────────────────┘
       v
┌─────────────────────────────────────────────────┐
│ AssemblyHandler (genossi_rest/src/assembly.rs)   │
│  - error_handler() wrap                          │
│  - extract_auth_context(Some(context))           │
│  - rest_state.assembly_service().<method>(...)   │
└──────┬──────────────────────────────────────────┘
       v
┌─────────────────────────────────────────────────┐
│ AssemblyServiceImpl (genossi_service_impl)       │
│  - PermissionService::check_permission("admin")  │
│  - TransactionDao::use_transaction(None)         │
│  - State-Transition-Guard (Preparation→Open→…)   │
│  - audited_create! / audited_update! Macros      │
│  - bei open: MemberDao::all + Filter + Snapshot  │
│  - TransactionDao::commit                        │
└──────┬───────────────────┬───────────────────────┘
       v                   v
┌──────────────┐   ┌────────────────────────┐
│ AssemblyDao  │   │ AssemblyMemberSnapshot │
│ Impl (sqlx)  │   │ DaoImpl (sqlx)          │
└──────┬───────┘   └────────┬───────────────┘
       v                    v
┌─────────────────────────────────────────────────┐
│ SQLite (assembly, assembly_member_snapshot,      │
│         audit_log, member)                       │
└─────────────────────────────────────────────────┘
```

Dataflow primary use-case (open_assembly):
1. Client POST `/api/assembly/{id}/open`
2. Auth-Middleware injects `Extension<Context>`
3. Handler ruft `service.open_assembly(id, auth)`
4. Service: permission check → tx begin → load assembly → guard (status==Preparation) → set status=Open + opened_at → `audited_update!` → load active members via `MemberDao::all`+Filter → batch insert snapshot rows → `transaction_dao.commit(tx)` → return `Assembly`
5. Handler serialisiert zu `AssemblyTO` → 200 OK

### Recommended Project Structure

Genossi-Konvention: jede neue Entität erhält Files in **8 Locations**.

```
genossi_dao/src/
└── assembly.rs                           # NEW: AssemblyEntity, AssemblyStatus, AssemblyDao trait, Auditable impl
└── assembly_member_snapshot.rs           # NEW: AssemblyMemberSnapshotEntity, AssemblyMemberSnapshotDao trait

genossi_dao_impl_sqlite/src/
└── assembly.rs                           # NEW: AssemblyDaoImpl (SQLx)
└── assembly_member_snapshot.rs           # NEW: AssemblyMemberSnapshotDaoImpl (SQLx)

genossi_service/src/
└── assembly.rs                           # NEW: Assembly DTO, AssemblySubmission, AssemblyUpdate, AssemblyService trait

genossi_service_impl/src/
└── assembly.rs                           # NEW: AssemblyServiceImpl, AssemblyServiceDeps via gen_service_impl!

genossi_rest_types/src/
└── lib.rs                                # MODIFY: append AssemblyStatusTO, AssemblyTO, CreateAssemblyRequest, UpdateAssemblyRequest, AssemblyDetailTO

genossi_rest/src/
└── assembly.rs                           # NEW: Axum handler functions, generate_route(), ApiDoc

genossi_bin/src/
└── lib.rs                                # MODIFY: AssemblyDaoImpl, AssemblyMemberSnapshotDaoImpl, AssemblyServiceImpl wiring; impl AssemblyRestState

genossi_bin/tests/
└── e2e_tests.rs                          # MODIFY: append create→open→close→verify test (D-12)

migrations/sqlite/
└── YYYYMMDDHHMMSS_create_assembly_table.sql               # NEW
└── YYYYMMDDHHMMSS_create_assembly_member_snapshot_table.sql # NEW

genossi_rest/src/lib.rs                   # MODIFY: register module, nest("/api/assembly", ...), nest in OpenAPI ApiDoc
```

### Pattern 1: DAO-Trait-Definition mit `#[automock]`
**What:** Trait mit minimalen Methoden (`dump_all`, `create`, `update`); Default-Impls für `all`, `find_by_id`. Mockable per `#[automock]`.
**When to use:** Für `AssemblyDao` (mit Status-Lifecycle ähnlich `ApplicationDao`) und für `AssemblyMemberSnapshotDao` (Insert + Lookup).
**Example (Vorlage `ApplicationDao`):**
```rust
// Source: genossi_dao/src/application.rs:98-140 [VERIFIED]
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait ApplicationDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ApplicationEntity]>, DaoError>;
    async fn create(&self, entity: &ApplicationEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn update(&self, entity: &ApplicationEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[ApplicationEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ApplicationEntity> = all_entities
            .iter().filter(|e| e.deleted.is_none()).cloned().collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<ApplicationEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities.iter().find(|e| e.id == id && e.deleted.is_none()).cloned())
    }
}
```

### Pattern 2: SQLite DAO-Impl mit Optimistic Locking + Soft Delete
**What:** SQLx-basierte Impl. `update`-Query enthält `WHERE id = ? AND version = ? AND deleted IS NULL`; bei `rows_affected == 0` → `ConflictError`.
**When to use:** Für `AssemblyDaoImpl::update`. Snapshot-Tabelle hat KEIN Version/Deleted (immutable nach Open) → simplerer DAO.
**Example:**
```rust
// Source: genossi_dao_impl_sqlite/src/application.rs:158-233 [VERIFIED]
async fn update(&self, entity: &ApplicationEntity, _process: &str, tx: Self::Transaction) -> Result<(), DaoError> {
    let id = entity.id.as_bytes().to_vec();
    let old_version = entity.version.as_bytes().to_vec();
    let new_version = Uuid::new_v4().as_bytes().to_vec();
    // ... bind all fields ...
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM application WHERE id = ? AND deleted IS NULL"
    ).bind(id.clone()).fetch_one(tx.tx.lock().await.as_mut()).await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    if exists == 0 { return Err(DaoError::NotFound); }

    let rows_affected = sqlx::query(
        "UPDATE application SET ..., version = ? WHERE id = ? AND version = ? AND deleted IS NULL"
    ).bind(/* fields */).bind(new_version).bind(id).bind(old_version)
        .execute(tx.tx.lock().await.as_mut()).await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?
        .rows_affected();
    if rows_affected == 0 { return Err(DaoError::ConflictError(Arc::from("Version mismatch"))); }
    Ok(())
}
```

### Pattern 3: Service via `gen_service_impl!` mit Audit-Macros
**What:** Macro generiert `Deps`-Trait + `Impl`-Struct mit Arc-DAOs. Methoden öffnen Tx, prüfen Permission, rufen `audited_*!`-Macros, committen.
**When to use:** `AssemblyServiceImpl` — Pflicht-Pattern; alle bestehenden Services (Member, Application, MemberAction) folgen ihm.
**Example:**
```rust
// Source: genossi_service_impl/src/application.rs:23-35, 268-402 [VERIFIED]
const APPLICATION_SERVICE_PROCESS: &str = "application-service";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";

gen_service_impl! {
    struct ApplicationServiceImpl: ApplicationService = ApplicationServiceDeps {
        ApplicationDao: ApplicationDao<Transaction = Self::Transaction> = application_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        // ... weitere Deps ...
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}

// Status-Lifecycle-Methode mit Audit + Permission + Tx + Guard
async fn confirm(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<Application, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(MANAGE_MEMBERS_PRIVILEGE, context).await?;

    let mut entity = self.application_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;
    if entity.status != ApplicationStatus::Offen {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Application status is '{}', expected 'Offen'", entity.status.as_str()
        ))));
    }
    // ... mutations ...
    entity.status = ApplicationStatus::Bestaetigt;
    crate::audited_update!(self, self.application_dao, id, &entity, APPLICATION_SERVICE_PROCESS, &user_id, tx);
    self.transaction_dao.commit(tx).await?;
    Ok(Application::from(&entity))
}
```

### Pattern 4: Axum-Handler mit error_handler + Utoipa
**What:** Handler-Signatur: `State<RestState>`, `Extension<Context>`, `Path/Query/Json`. Body in `error_handler((async { ... }).await)`-Wrapper.
**When to use:** Jeder der 6 Endpoints in D-13.
**Example:**
```rust
// Source: genossi_rest/src/application.rs:208-256 [VERIFIED]
#[instrument(skip(rest_state))]
#[utoipa::path(
    get, tag = "Applications", path = "",
    params(("status" = Option<String>, Query, description = "Filter")),
    responses(
        (status = 200, description = "List", body = [ApplicationTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_applications<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<ApplicationListQuery>,
) -> Response {
    error_handler((async {
        let apps: Arc<[ApplicationTO]> = rest_state.application_service()
            .list(status_filter, crate::extract_auth_context(Some(context))?)
            .await?
            .iter().map(ApplicationTO::from).collect();
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&apps)?))
            .unwrap())
    }).await)
}
```

### Anti-Patterns to Avoid
- **Hard Delete:** Niemals `DELETE FROM assembly`. Soft-Delete nur falls überhaupt erforderlich (Phase 1 hat KEIN Delete-Endpoint laut D-13 → kein Delete-Code nötig).
- **Service erzeugt eigene Transaktion mit `transaction()`:** Stattdessen `transaction_dao.use_transaction(None)` — erlaubt späteren Caller-bestimmten Tx-Scope.
- **Audit-Logging mit Direkt-Aufruf von `AuditLogDao::create_entries`:** IMMER über `audited_create!`/`audited_update!`-Macros — sonst Hash-Chain-Bruch-Risiko.
- **Status-Übergang im DAO prüfen:** DAO bleibt agnostisch; Status-Guard gehört in Service.
- **Snapshot ohne Transaktion befüllen:** `open_assembly` MUSS atomar sein (Update + Snapshot in einer Tx).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Hash-Chain-Berechnung | Eigene SHA256-Logik | `audited_create!`/`audited_update!`-Macros | Macros laden prev_hash, bauen Entries, schreiben atomar [VERIFIED: audit_macros.rs] |
| DAO-Impl-Boilerplate | Manuelle DI-Setup-Funktion | `gen_service_impl!`-Macro | Generiert Trait + Struct + new() — eliminiert ~30 LOC pro Service |
| ISO8601-Datetime-Parsing | Eigene Format-String-Logik | `iso8601_datetime`-Modul aus genossi_rest_types | Bereits implementiert mit Multi-Format-Fallback |
| Permission-Check-Logik | Eigene Privilege-Resolution | `PermissionService::check_permission("admin", auth)` | Existiert; Privilege-Names sind String-Konstanten |
| `count_active`-Filter | Neues SQL `WHERE deleted IS NULL AND ...` | `MemberDao::all()` + dieselbe Filter-Closure aus `count_active` | CONTEXT D-02: identische Logik einmal |
| Optimistic-Locking | Eigene Version-Check-Query | Bestehende Pattern: `WHERE id = ? AND version = ?` + `rows_affected == 0` → ConflictError | Pattern in jedem `*DaoImpl::update` |
| Test-Server-Aufbau | Eigene Axum-App-Spawn | `genossi_rest::test_server::test_support::start_test_server` | Existiert; In-Memory-SQLite mit Random-Port [VERIFIED: e2e_tests.rs:11] |

**Key insight:** Genossi hat eine sehr gut etablierte „neue Entität in 8 Schritten"-Konvention. Phase 1 ist 90 % Replikation; Originalcode entsteht nur an drei Stellen: Status-Transition-Guard, Snapshot-Befüllung, atomare Update+Snapshot-Tx in `open_assembly`.

## Runtime State Inventory

> Phase 1 ist Greenfield (neue Tabellen, neuer Code). Kein Rename, kein Refactor von bestehender Runtime-State. Eintrag dennoch vollständig:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — neue Tabellen `assembly`, `assembly_member_snapshot` werden frisch erzeugt; kein bestehender Datensatz wird umbenannt oder umgezogen | None |
| Live service config | None — keine n8n/Datadog/Cloudflare-Integrationen werden berührt | None |
| OS-registered state | None — kein systemd/launchd/Task-Scheduler-Bezug | None |
| Secrets/env vars | None — keine neuen Secrets; OIDC-Config bleibt unverändert | None |
| Build artifacts | None — keine umbenannten Crates oder Binaries; `cargo build` baut die neuen Module ohne Altlast-Konflikte | None |

**Nothing found in any category** — verifiziert durch CONTEXT-Lektüre und Codebase-Read der relevanten Wiring-Files.

## Common Pitfalls

### Pitfall 1: Hash-Chain-Bruch durch direktes `AuditLogDao::create_entries`
**What goes wrong:** Wer Audit-Entries unter Umgehung der Macros schreibt, vergisst typischerweise den `prev_hash`-Lookup oder die transaktionale Atomarität → Verify-Endpoint findet Mismatch.
**Why it happens:** Verlockend, "nur eben schnell" eine Audit-Zeile zu schreiben (z. B. für nicht-Auditable-Entitäten).
**How to avoid:** Phase 1 nutzt AUSSCHLIESSLICH `audited_create!`/`audited_update!`. Snapshot-Befüllung in `assembly_member_snapshot` darf KEINE Audit-Einträge erzeugen — Snapshot ist Daten, nicht Lifecycle-Event. Der Lifecycle-Event ist `assembly.open`, der via `audited_update!` auf der `assembly`-Zeile geloggt wird.
**Warning signs:** Test `test_audit_verify_after_assembly_lifecycle` schlägt mit `valid: false` und Broken-Links fehl.

### Pitfall 2: Tx-Scope-Bruch beim Open-Atomarität
**What goes wrong:** `open_assembly` macht `audited_update!` mit Commit dazwischen, dann separat Snapshot-Insert → bei Insert-Fehler ist Status schon `Open`, aber Snapshot leer.
**Why it happens:** `audited_update!`-Macro committet NICHT selbst — aber ein versehentliches `transaction_dao.commit(tx).await?` zwischen Update und Snapshot zerlegt die Atomarität.
**How to avoid:** Eine einzige `let tx = transaction_dao.use_transaction(None).await?;` ganz am Anfang, ein einziges `transaction_dao.commit(tx).await?;` ganz am Ende. Dazwischen `tx.clone()` für jeden Sub-Call. Der `audited_update!`-Macro nutzt seinerseits `tx.clone()` — er committet nicht.
**Warning signs:** E2E-Test mit fehlerhaft mockedem MemberDao zeigt halb-offene Assembly mit leerem Snapshot. Oder: integration test, das Snapshot-Insert künstlich fehlschlägt → Assembly muss in `Preparation` bleiben, nicht in `Open`.

### Pitfall 3: Status-Übergang ohne Guard im Service
**What goes wrong:** Vorstand ruft direkt POST `/api/assembly/{id}/close` auf eine Assembly im Status `Preparation` → State-Maschine springt `Preparation → Closed`, ASSY-03 verletzt.
**Why it happens:** Vergessener Guard. DAO blockt nicht, weil DAO agnostisch.
**How to avoid:** In `close_assembly` ein `if entity.status != AssemblyStatus::Open` early-return mit `ServiceError::Conflict`. Ebenso `open_assembly`: nur aus `Preparation` heraus. Vorlage: `ApplicationServiceImpl::confirm` Zeile 291-296.
**Warning signs:** Unit-Test `test_close_from_preparation_returns_conflict` fehlt im Plan.

### Pitfall 4: Englische Status-Werte mit deutschem `MemberStatus`-Code-Pfad verwechseln
**What goes wrong:** Entwickler kopiert `MemberStatus::Normal`-Code („Normal" englisch zufällig identisch mit dt.) und tippt für Assembly versehentlich `"Vorbereitung"` als DB-Wert.
**Why it happens:** `MemberStatus`/`ApplicationStatus` sind deutsch (`Offen`/`Bestaetigt`/`Abgelehnt`), Assembly ist explizit englisch (D-06, D-17). Inkonsistenz innerhalb des Projekts.
**How to avoid:** `AssemblyStatus::as_str()` MUSS `"Preparation"` / `"Open"` / `"Closed"` zurückgeben. Migration `DEFAULT 'Preparation'`. Roundtrip-Test im DAO-Test (`test_assembly_status_roundtrip`).
**Warning signs:** SQLite zeigt Mixed-Language-Status-Werte; OpenAPI-Schema zeigt deutsche Strings.

### Pitfall 5: Snapshot-Insert ohne UNIQUE-Constraint → Doppelreihen bei Idempotenz-Mishap
**What goes wrong:** Wenn `open_assembly` retry-bar wäre, könnten zwei Inserts dieselbe (assembly_id, member_id)-Kombination einfügen → `count_snapshot` = 2*Y.
**Why it happens:** Phase 1 ist nicht idempotent (open ist Einmal-Übergang), aber UNIQUE-Constraint ist trotzdem Sicherheitsnetz.
**How to avoid:** Migration: `UNIQUE (assembly_id, member_id)` auf der Snapshot-Tabelle. Wenn ein Re-Insert fälschlich versucht würde, gibt SQLite einen Fehler — der dann den Service abbricht und die Tx rollback'd.
**Warning signs:** Manueller SQL-Test: `INSERT INTO assembly_member_snapshot VALUES (?, ?, ?)` zweimal mit gleichen IDs muss fehlschlagen.

### Pitfall 6: `count_active`-Filter schief kopiert (z. B. status-Check vergessen)
**What goes wrong:** Snapshot enthält Member mit `status = FehlerhaftErfasst` → Y wird zu groß.
**Why it happens:** `count_active`-Filter hat 3 Komponenten (`deleted IS NULL`, `exit_date > today`, `status.is_normal()`); leicht eine zu vergessen.
**How to avoid:** Im Service: `let active = member_dao.all(tx.clone()).await?;` (filtert bereits `deleted IS NULL`), dann `.iter().filter(|m| m.status.is_normal() && m.exit_date.map_or(true, |d| d > opened_date)).collect()`. Vergleich gegen `count_active`-Test-Case-Suite (`test_count_active_excludes_fehlerhaft_erfasst`) im Unit-Test.
**Warning signs:** Test mit FehlerhaftErfasst-Member im Setup → Snapshot-Count > Aktiv-Count.

## Code Examples

Verifizierte Patterns aus dem Bestand, exakt vom Read-Ergebnis übernommen.

### Beispiel 1: `MemberStatus`-Enum (Vorlage für `AssemblyStatus`, aber **englische Werte**)

```rust
// Source: genossi_dao/src/member.rs:37-71 [VERIFIED]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberStatus {
    Normal,
    FehlerhaftErfasst,
}

impl MemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberStatus::Normal => "Normal",
            MemberStatus::FehlerhaftErfasst => "FehlerhaftErfasst",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Normal" => Ok(MemberStatus::Normal),
            "FehlerhaftErfasst" => Ok(MemberStatus::FehlerhaftErfasst),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown member status: {}", s
            )))),
        }
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, MemberStatus::Normal)
    }
}

impl Default for MemberStatus {
    fn default() -> Self { MemberStatus::Normal }
}
```

**Adaption für AssemblyStatus (englische Werte per D-06):**
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyStatus {
    Preparation,
    Open,
    Closed,
}

impl AssemblyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssemblyStatus::Preparation => "Preparation",
            AssemblyStatus::Open => "Open",
            AssemblyStatus::Closed => "Closed",
        }
    }
    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Preparation" => Ok(AssemblyStatus::Preparation),
            "Open" => Ok(AssemblyStatus::Open),
            "Closed" => Ok(AssemblyStatus::Closed),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown assembly status: {}", s
            )))),
        }
    }
}

impl Default for AssemblyStatus {
    fn default() -> Self { AssemblyStatus::Preparation }
}
```

`AssemblyStatusTO` analog zu `ApplicationStatusTO` in `genossi_rest_types/src/lib.rs:807-831`. ToSchema-Derive nicht vergessen.

### Beispiel 2: `count_active`-Filter (CONTEXT D-02)

```rust
// Source: genossi_dao/src/member.rs:172-185 [VERIFIED]
async fn count_active(&self, today: time::Date, tx: Self::Transaction) -> Result<u64, DaoError> {
    let all_entities = self.dump_all(tx).await?;
    let count = all_entities
        .iter()
        .filter(|e| e.deleted.is_none())
        .filter(|e| e.status.is_normal())
        .filter(|e| e.exit_date.map_or(true, |d| d > today))
        .count();
    Ok(count as u64)
}
```

**Empfehlung Snapshot-Befüllung im AssemblyServiceImpl:** den Filter inline replizieren (nicht `count_active` aufrufen — das gibt nur `u64`, wir brauchen die Member-IDs):

```rust
// Im open_assembly:
let opened_date = time::OffsetDateTime::now_utc().date();
let all_members = self.member_dao.all(tx.clone()).await?;  // bereits deleted-gefiltert
let active_member_ids: Vec<Uuid> = all_members.iter()
    .filter(|m| m.status.is_normal())
    .filter(|m| m.exit_date.map_or(true, |d| d > opened_date))
    .map(|m| m.id)
    .collect();

// Batch-Insert in assembly_member_snapshot
for member_id in &active_member_ids {
    self.assembly_member_snapshot_dao.create(
        AssemblyMemberSnapshotEntity {
            assembly_id,
            member_id: *member_id,
            captured_at: now_pdt,
        },
        "assembly.open",
        tx.clone(),
    ).await?;
}
```

Alternativ: `AssemblyMemberSnapshotDao::create_batch(&[...])` für eine einzelne `INSERT ... VALUES (?,?,?), (?,?,?), ...`-Query — 1 Roundtrip statt N. Empfehlung: Batch-Methode nehmen, die sich am `MemberActionDao`-Pattern orientiert (siehe genossi_service_impl/src/application.rs:343-386 für ein Multi-Insert-Pattern).

### Beispiel 3: Auditable-Trait-Impl (Vorlage Application)

```rust
// Source: genossi_dao/src/application.rs:63-96 [VERIFIED]
impl crate::auditable::Auditable for ApplicationEntity {
    fn entity_type() -> &'static str { "application" }
    fn entity_id(&self) -> Uuid { self.id }
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("first_name", Some(self.first_name.to_string())),
            ("last_name", Some(self.last_name.to_string())),
            ("salutation", self.salutation.as_ref().map(|s| s.as_str().to_string())),
            ("title", self.title.as_ref().map(|s| s.to_string())),
            // ...
            ("status", Some(self.status.as_str().to_string())),
        ]
    }
}
```

**Adaption für AssemblyEntity (D-10):**
```rust
impl crate::auditable::Auditable for AssemblyEntity {
    fn entity_type() -> &'static str { "assembly" }
    fn entity_id(&self) -> Uuid { self.id }
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        let format_dt = |dt: &time::PrimitiveDateTime| {
            dt.assume_utc().format(&time::format_description::well_known::Iso8601::DEFAULT)
                .unwrap_or_default()
        };
        vec![
            ("name", Some(self.name.to_string())),
            ("date", Some(format_dt(&self.date))),
            ("location", self.location.as_ref().map(|s| s.to_string())),
            ("status", Some(self.status.as_str().to_string())),
            ("opened_at", self.opened_at.as_ref().map(format_dt)),
            ("closed_at", self.closed_at.as_ref().map(format_dt)),
        ]
    }
}
```

**Field count: 6.** Excluded per D-10: `id`, `version`, `created`, `deleted`. Ein passender Test (analog `genossi_dao/src/application.rs:178-185`) muss `assert_eq!(fields.len(), 6)` und das Fehlen der ausgeschlossenen Felder prüfen.

### Beispiel 4: Audit-Macro-Aufrufe für Lifecycle (D-11 Process-Identifier)

```rust
// CREATE: in create_assembly()
crate::audited_create!(
    self,
    self.assembly_dao,
    &entity,
    "assembly.create",     // process per D-11
    &user_id,
    tx
);

// OPEN: in open_assembly() — entity wird mutiert (status, opened_at), DANN audited_update!
let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
    .ok_or(ServiceError::EntityNotFound(id))?;
if entity.status != AssemblyStatus::Preparation {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Assembly status is '{}', expected 'Preparation'", entity.status.as_str()
    ))));
}
let now_pdt = {
    let now = time::OffsetDateTime::now_utc();
    time::PrimitiveDateTime::new(now.date(), now.time())
};
entity.status = AssemblyStatus::Open;
entity.opened_at = Some(now_pdt);
crate::audited_update!(
    self,
    self.assembly_dao,
    id,
    &entity,
    "assembly.open",       // process per D-11
    &user_id,
    tx
);
// Audit-Diff erkennt: status=Preparation→Open + opened_at=null→<ts> → genau 2 Audit-Rows

// CLOSE: analog
entity.status = AssemblyStatus::Closed;
entity.closed_at = Some(now_pdt);
crate::audited_update!(
    self,
    self.assembly_dao,
    id,
    &entity,
    "assembly.close",      // process per D-11
    &user_id,
    tx
);
```

**Wichtig:** Macro-Internal: `action`-Spalte in `audit_log` wird von `build_create_entries` / `build_update_entries` hardcodiert auf `"create"` / `"update"` gesetzt (siehe genossi_service_impl/src/audit_log.rs:133, 171). Der `process`-String („assembly.create" etc.) ist ein separates Feld. Filterung im Audit-Endpoint geht über beide.

### Beispiel 5: Service-Methode `open_assembly` — komplettes Skelett

```rust
async fn open_assembly(
    &self,
    id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self.permission_service
        .current_user_id(context.clone())
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());

    self.permission_service
        .check_permission("admin", context)
        .await?;

    // Load + Guard
    let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;
    if entity.status != AssemblyStatus::Preparation {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot open assembly: status is '{}', expected 'Preparation'",
            entity.status.as_str()
        ))));
    }

    // Mutate + Audit
    let now_offset = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
    let opened_date = now_offset.date();
    entity.status = AssemblyStatus::Open;
    entity.opened_at = Some(now_pdt);

    crate::audited_update!(
        self,
        self.assembly_dao,
        id,
        &entity,
        "assembly.open",
        &user_id,
        tx
    );

    // Snapshot-Befüllung in derselben Tx
    let all_members = self.member_dao.all(tx.clone()).await?;
    let active_member_ids: Vec<Uuid> = all_members.iter()
        .filter(|m| m.status.is_normal())
        .filter(|m| m.exit_date.map_or(true, |d| d > opened_date))
        .map(|m| m.id)
        .collect();

    for member_id in &active_member_ids {
        let snapshot_entity = AssemblyMemberSnapshotEntity {
            assembly_id: id,
            member_id: *member_id,
            captured_at: now_pdt,
        };
        self.assembly_member_snapshot_dao
            .create(&snapshot_entity, "assembly.open", tx.clone())
            .await?;
        // KEIN audited_create! — Snapshot-Rows sind keine Lifecycle-Events
    }

    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```

### Beispiel 6: E2E-Test-Vorlage (D-12)

```rust
// Source: genossi_bin/tests/e2e_tests.rs:7499-7523 [VERIFIED] — als Vorlage übernehmen, anpassen
#[tokio::test]
async fn test_assembly_lifecycle_audit_chain_intact() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // 1) Create
    let create_body = serde_json::json!({
        "name": "GV 2026",
        "date": "2026-06-15T18:00:00.000000000Z",
        "location": "Vereinsheim",
    });
    let response = client.post(server.url("/api/assembly"))
        .json(&create_body).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: AssemblyTO = response.json().await.unwrap();
    let assembly_id = created.id;
    assert_eq!(created.status, AssemblyStatusTO::Preparation);

    // 2) Open
    let response = client.post(server.url(&format!("/api/assembly/{}/open", assembly_id)))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let opened: AssemblyTO = response.json().await.unwrap();
    assert_eq!(opened.status, AssemblyStatusTO::Open);
    assert!(opened.opened_at.is_some());

    // 3) Close
    let response = client.post(server.url(&format!("/api/assembly/{}/close", assembly_id)))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let closed: AssemblyTO = response.json().await.unwrap();
    assert_eq!(closed.status, AssemblyStatusTO::Closed);
    assert!(closed.closed_at.is_some());

    // 4) Verify Hash-Chain intakt
    let response = client.get(server.url("/api/audit/verify")).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result: genossi_rest_types::VerifyResponseTO = response.json().await.unwrap();
    assert!(result.valid, "Hash chain must be valid after lifecycle");
    assert!(result.broken_links.is_empty());
    assert!(result.total_entries >= 3, "expected ≥3 audit entries (create+open+close)");

    // 5) Filter nach assembly-Process: drei Lifecycle-Einträge
    let response = client.get(server.url(&format!("/api/audit/assembly/{}", assembly_id)))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let entries: Vec<genossi_rest_types::AuditLogEntryTO> = response.json().await.unwrap();
    let processes: Vec<&str> = entries.iter().map(|e| e.process.as_str()).collect();
    assert!(processes.iter().any(|p| *p == "assembly.create"));
    assert!(processes.iter().any(|p| *p == "assembly.open"));
    assert!(processes.iter().any(|p| *p == "assembly.close"));
}
```

Datei: in `genossi_bin/tests/e2e_tests.rs` anhängen (D-12: kein neues File). `setup()` aus Zeile 23 wiederverwenden — In-Memory-SQLite + Migrations + `RestStateImpl::new()`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Excel-Listen für GV-Anwesenheit | Backend-Aggregat mit Audit-Hashchain | Phase 1 (Beginn) | Verbandskonforme Nachvollziehbarkeit; Phase 1 liefert die Backend-Grundlage |
| (Innerhalb Genossi) hand-gerollte Audit-Logging | `Auditable`-Trait + Macros | bestehend | Phase 1 nutzt das bestehende Pattern unverändert |

**Keine deprecated Konzepte in Phase 1** — alle Patterns sind aktiv und in Member/Application referenziert.

## Project Constraints (from CLAUDE.md)

Direkte CLAUDE.md-Direktiven, die der Plan respektieren muss:

- **Tests:** "Always make sure you have tests for the changes" (User-globale Direktive). Phase 1 verlangt Unit-Tests für Status-Enum-Roundtrip, Status-Transition-Guard, Auditable-Field-Count + E2E-Test (D-12). Pflicht.
- **Layered Architecture:** "Layered DAO/Service/REST muss eingehalten werden". Kein Sprung von REST direkt auf SQLx. ✅ in allen Pattern-Beispielen oben.
- **Audit-Pflicht:** "Bestehende auditierte Entitäten müssen weiterhin Audit-Macros verwenden; neue GV-Entitäten benötigen das nicht" — Achtung: CONTEXT D-10/D-11 OVERRIDE diese CLAUDE.md-Aussage für `Assembly`. Phase 1 audited Assembly. Snapshot-Tabelle wird NICHT auditiert.
- **ISO8601-Datetime:** TOs nutzen `iso8601_datetime`-Modul. ✅ siehe Pattern 4.
- **Soft-Delete via `deleted`:** Assembly hat `deleted: Option<PrimitiveDateTime>` (CONTEXT D-05). Snapshot-Tabelle hat keins (D-01).
- **Component-First-Frontend:** Nicht relevant für Phase 1 (Backend-only, Phase 4 ist UI).
- **GSD-Workflow:** "Before using Edit/Write … start work through a GSD command". Phase 1 läuft via `/gsd-plan-phase` → `/gsd-execute-phase`.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `assembly_member_snapshot` braucht eine `id`-Spalte als PRIMARY KEY (BLOB UUID) zusätzlich zu `(assembly_id, member_id)` UNIQUE — sonst hat SQLite keine eindeutige Row-Referenz für ON CONFLICT-Handling | §1 Migration SQL | Wenn falsch, würde der Plan eine zusätzliche `id`-Spalte vorschlagen, die später entfernt werden müsste; SQLite akzeptiert auch Composite-PK ohne Surrogate-ID. **Empfehlung:** Composite-PK `PRIMARY KEY (assembly_id, member_id)` — keine surrogate ID nötig, weil Snapshot-Rows immutable sind und nicht via FK aus Phase 3 referenziert werden. |
| A2 | Phase 3 `attendance` referenziert `member_id` direkt, nicht `assembly_member_snapshot.id` — daher braucht Snapshot keine surrogate ID | §1 Migration SQL | Wenn Phase 3 doch eine `snapshot_id`-FK fordert (z. B. um Y rückwirkend zu rekonstruieren), müsste Snapshot doch eine surrogate ID erhalten. **Mitigation:** Phase 3 ist explizit out-of-scope; sollte sich beim Phase-3-Discuss zeigen, wäre eine Migration mit `ALTER TABLE` möglich. |
| A3 | RFC 3339 / ISO 8601 mit `T` und `Z` ist das akzeptierte Datetime-Format für `date` (Datum mit Uhrzeit per D-05) im Request-Body | §9 REST Handler | Wenn Tests Date-Only-Input erwarten (`"2026-06-15"`), schlagen Validation-Tests fehl. Mitigation: das Genossi-Pattern `iso8601_datetime` akzeptiert volle ISO8601-Strings; das Discuss-Doc legt explizit fest, dass `date` DateTime ist (nicht Date) — siehe D-05. |
| A4 | Der `process`-String im audit_log-Endpoint kann nicht direkt nach `process LIKE 'assembly.%'` gefiltert werden, weil der Endpoint `AuditQueryFilter::action` (nicht `process`) als Filter exponiert | §11 E2E Test | E2E-Test im Pattern oben filtert nach `entity_type=assembly` (das Endpoint `/api/audit/assembly/{assembly_id}` liefert nach Entity-ID), nicht nach Process-Prefix. Der "Process-Filter über Punkt-Notation" aus CONTEXT specifics ist ein POTENTIAL für Phase 3+. **Test-Adaption:** Stattdessen Entity-ID-Filter nutzen, dann auf `entries[*].process` per Rust-Iter filtern. Bereits so implementiert im Pattern oben. |
| A5 | `AssemblyMemberSnapshotDao` verwendet weder Audit noch Soft-Delete — ein simpler `create`/`find_by_assembly_id`/`count_by_assembly_id`-Trait reicht | §5 DAO Shape | Wenn der Reviewer doch Audit auf Snapshot-Inserts fordert, müssten 100x Audit-Rows pro `open_assembly` geschrieben werden. **Mitigation:** Konsistent mit CONTEXT-Note "neue GV-Entitäten benötigen das nicht" (CLAUDE.md) und CONTEXT D-10 (das nur `Assembly`, nicht Snapshot, als auditiert markiert). |

**Hinweis für Discuss/Plan-Check:** Die Discretion-Punkte aus CONTEXT (Index-Strategie, ON-DELETE-Verhalten) werden in §1 mit klarer Empfehlung beantwortet — nicht als Annahme markiert, weil CONTEXT explizit sagt „Claude wählt während des Plans". Falls der User abweichen will, ist das jederzeit per Override im Plan möglich.

## Open Questions

1. **Soll der `GET /api/assembly/{id}`-Endpoint die volle Snapshot-Member-Liste zurückgeben oder nur den Snapshot-Count?**
   - What we know: D-13 sagt „mit Snapshot-Liste oder zumindest Snapshot-Count". Erlaubt beide Optionen.
   - What's unclear: Bei großen Genossenschaften (>200 Mitglieder) ist die volle Liste im Detail-Endpoint potenziell zu viel.
   - Recommendation: Phase 1 liefert NUR den Count im `AssemblyDetailTO` (Feld `snapshot_member_count: u64`); die volle Liste bleibt Phase 3 vorbehalten (gemeinsam mit Helfer-Member-View). Y wird ad-hoc gerechnet via `assembly_member_snapshot_dao.count_by_assembly_id(id, tx)`. Phase 4-Frontend braucht in Phase 1 noch nichts, also reicht der Count.

2. **Hat `update_assembly` (PUT in Status `Preparation`) eine Version-Check-Pflicht analog zu `update_application`?**
   - What we know: D-07 erlaubt Stamm-Daten-Update nur in `Preparation`. Optimistic-Locking ist projektweit Standard.
   - What's unclear: Ob das `UpdateAssemblyRequest`-TO zwingend ein `version: Uuid`-Feld haben muss.
   - Recommendation: JA, `UpdateAssemblyRequest` enthält `version: Uuid`. Service prüft `if entity.version != update.version → ConflictError`. 1:1 wie `update_application` (genossi_service_impl/src/application.rs:498-502).

3. **Wie soll das `date`-Feld der Assembly in der Migration heißen — `date` als SQL-Spalte ist kein reserviertes Keyword, aber unschön?**
   - What we know: D-05 nennt das Feld „date" auf Domain-Ebene.
   - What's unclear: Ob die Plan-Phase „date" als SQL-Spalten-Name übernehmen soll oder z. B. `assembly_date` / `scheduled_at`.
   - Recommendation: `date` ist in SQLite kein reserviertes Wort und kollidiert nicht — direkte Verwendung ist OK. Falls der Plan ein klareres Name will: `scheduled_at` (deutet auf `PrimitiveDateTime` hin). Empfehlung: bei `date` bleiben, da CONTEXT explizit so benannt.

## Environment Availability

> Phase 1 ist code/migration-only. Keine externen Tools über bestehende Toolchain hinaus.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` (Rust toolchain) | Build | ✓ (annahme — Genossi entwickelt aktiv damit) | 1.70+ | — |
| `sqlx-cli` | Migration-Run beim Start | ✓ (in flake.nix laut STACK.md) | — | Migrations laufen automatisch beim Server-Start |
| SQLite | Datenbank | ✓ | embedded | — |
| `tokio` test runtime | E2E-Tests | ✓ (workspace-dep) | 1.35+ | — |

**Missing dependencies with no fallback:** Keine.
**Missing dependencies with fallback:** Keine.

Phase 1 ist ein reines Backend-Code-Phase ohne neue Tools.

## Security Domain

> security_enforcement ist nicht explizit in der `.planning/config.json` gesetzt. Behandelt als enabled. Phase 1 berührt allerdings nur internen REST-Layer hinter bestehender Auth-Middleware — kein neues Auth, keine Krypto-Primitiven.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (delegiert) | Bestehende OIDC/Mock-Auth-Middleware in `genossi_rest/src/auth_middleware.rs` — Phase 1 nutzt sie unverändert |
| V3 Session Management | yes (delegiert) | `tower-sessions` + axum-oidc — bestehend |
| V4 Access Control | yes | `PermissionService::check_permission("admin", auth)` in jedem Handler-Pfad — D-14 |
| V5 Input Validation | yes | TO-Validation analog `validate_join_request` (genossi_rest/src/application.rs:61) — `name` nicht leer, `date` parsable, `location` Längen-Limit |
| V6 Cryptography | yes (delegiert) | Audit-Hashchain SHA256 via `compute_entry_hash` — bereits implementiert, nicht neu |
| V7 Error Handling and Logging | yes | `error_handler()` mappt `ServiceError` → `RestError` → HTTP, `tracing::error!` für InternalError; PII darf nicht in Logs (Audit-Macros loggen nur strukturiert) |
| V8 Data Protection | yes | Soft-Delete + Audit-Trail — bestehend |

### Known Threat Patterns for Rust + Axum + SQLx

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SQL Injection | Tampering | SQLx Parameterized Queries — bereits projektweit konsequent (siehe `application.rs:132-156`); KEINE String-Concatenation in Queries |
| Race Condition bei `open_assembly` | TOCTOU / Tampering | Optimistic Locking via `version` UUID; bei Mismatch → `ConflictError` (Pattern aus `application.rs:228-230`) |
| Audit-Hash-Chain-Tampering | Tampering | Bestehende Macros + Verify-Endpoint; Phase 1 verlangt E2E-Test gegen `/api/audit/verify` (D-12) |
| Privilege Escalation | Elevation | Permission-Check ist Pflicht in JEDEM Service-Aufruf (`check_permission("admin", auth).await?`) — kein Endpoint-Skip |
| Information Disclosure | Disclosure | Phase 1 exposed nur Lifecycle-Daten (name, date, status, opened_at, closed_at, snapshot_count). Keine Member-PII; kein DSGVO-Issue |
| Denial of Service via Massiver Snapshot | Availability | Bei einer Genossenschaft mit Tausenden Mitgliedern: Batch-Insert (1 Query mit N Rows) statt N Queries; Tx in vernünftiger Zeit. Mitigation: SQLite kann ~10k Rows pro Tx problemlos verkraften |
| Cross-Site Scripting (XSS) | Tampering | Phase 1 hat keine HTML-Rendering — nur JSON-API. Frontend (Phase 4) verantwortlich für Rendering-Sanitization |

**Zusätzliche Genossi-spezifische Punkte:**
- Permission `"admin"` ist OIDC-Group-basiert (in Produktion); im `mock_auth`-Modus akzeptiert `MockContext` jeden Request — Phase 1 E2E-Tests laufen unter `mock_auth`, OK.
- Audit-Logging der drei Lifecycle-Events ist Pflicht (D-11) — wenn das fehlt, wäre das Compliance-Issue für Verband.

## Sources

### Primary (HIGH confidence)
- `genossi_dao/src/member.rs` — MemberStatus-Pattern, count_active-Filter, Auditable-Impl [Read 1-580]
- `genossi_dao/src/application.rs` — ApplicationStatus, ApplicationDao-Trait, Auditable-Impl [Read 1-208]
- `genossi_dao/src/auditable.rs` — Auditable-Trait, AuditFieldChange [Read 1-140]
- `genossi_dao/src/lib.rs` — TransactionDao trait, DaoError [Read 1-70]
- `genossi_dao_impl_sqlite/src/application.rs` — SQLx DAO-Impl, Optimistic-Locking-Update [Read 1-235]
- `genossi_service_impl/src/audit_macros.rs` — audited_create!/audited_update!/audited_delete! [Read 1-128]
- `genossi_service_impl/src/audit_log.rs` — build_create_entries hardcodiert action="create" etc. [Read 1-194]
- `genossi_service_impl/src/macros.rs` — gen_service_impl! Macro [Read 1-42]
- `genossi_service_impl/src/application.rs` — ApplicationServiceImpl mit Audit + Permission + Tx [Read 1-528]
- `genossi_service_impl/src/member.rs` — Service-Pattern mit Optional-Tx und gen_service_impl! [Read 1-110]
- `genossi_service/src/application.rs` — ApplicationService trait, DTOs, From-Impls [Read 1-155]
- `genossi_rest/src/application.rs` — Axum-Handler mit error_handler, Utoipa, generate_route, ApiDoc [Read 1-617]
- `genossi_rest/src/audit_log.rs` — Verify-Endpoint, AuditRestState trait [Read 1-266]
- `genossi_rest/src/lib.rs` — RestStateDef, error_handler, route-Registration [Read 1-708]
- `genossi_rest_types/src/lib.rs` — TO-Patterns, ISO8601-Module, ApplicationStatusTO [Read 1-200, 800-900]
- `genossi_bin/src/lib.rs` — RestStateImpl::new DI-Wiring, RestState-Impls [Read 1-1100]
- `genossi_bin/tests/e2e_tests.rs` — setup(), test_audit_verify_after_operations als Vorlage [Read 1-200, 7480-7560]
- `migrations/sqlite/20260413000000_create_application_table.sql` — Migration-Vorlage [Read]
- `migrations/sqlite/20260331000000_create_member_table.sql` — Migration-Vorlage Member [Read]
- `.planning/phases/01-assembly-aggregat-audit-hardening/01-CONTEXT.md` — Locked decisions D-01..D-17 [Read]
- `.planning/REQUIREMENTS.md` — ASSY-01..ASSY-07 [Read]
- `.planning/ROADMAP.md` — Phase 1 Goal & Success Criteria [Read]
- `.planning/config.json` — `nyquist_validation: false` confirmed [Read]
- `/CLAUDE.md` — Projekt-Konventionen, Audit-System-Schritte [Read]

### Secondary (MEDIUM confidence)
- Keine — alle Findings sind in der Genossi-Codebase verifiziert.

### Tertiary (LOW confidence)
- Keine — keine externen Web-Sources benötigt.

## Detailed Findings (numerierte Antworten zum Research-Focus)

### 1. Migration SQL — full DDL

**Datei 1:** `migrations/sqlite/YYYYMMDDHHMMSS_create_assembly_table.sql` (D-15: filename englisch)

```sql
CREATE TABLE IF NOT EXISTS assembly (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    date TEXT NOT NULL,                                  -- PrimitiveDateTime ISO8601
    location TEXT,                                       -- Optional
    status TEXT NOT NULL DEFAULT 'Preparation',          -- D-06: englisch
    opened_at TEXT,                                      -- Option<PrimitiveDateTime>
    closed_at TEXT,                                      -- Option<PrimitiveDateTime>
    created TEXT NOT NULL,
    deleted TEXT,                                        -- Soft-Delete
    version BLOB NOT NULL                                -- Optimistic Locking
);

CREATE INDEX IF NOT EXISTS idx_assembly_status ON assembly(status);
CREATE INDEX IF NOT EXISTS idx_assembly_deleted ON assembly(deleted);
CREATE INDEX IF NOT EXISTS idx_assembly_date ON assembly(date);
```

**Datei 2:** `migrations/sqlite/YYYYMMDDHHMMSS_create_assembly_member_snapshot_table.sql`

```sql
CREATE TABLE IF NOT EXISTS assembly_member_snapshot (
    assembly_id BLOB NOT NULL,
    member_id BLOB NOT NULL,
    captured_at TEXT NOT NULL,                          -- PrimitiveDateTime ISO8601
    PRIMARY KEY (assembly_id, member_id),               -- Composite PK
    FOREIGN KEY (assembly_id) REFERENCES assembly(id),  -- Default ON DELETE NO ACTION
    FOREIGN KEY (member_id) REFERENCES member(id)
);

CREATE INDEX IF NOT EXISTS idx_assembly_member_snapshot_assembly_id ON assembly_member_snapshot(assembly_id);
```

**Begründung der Entscheidungen (Claude's Discretion aus CONTEXT):**

- **Composite Primary Key `(assembly_id, member_id)` statt surrogate `id`-Spalte:** D-01 sagt explizit „Keine eigene id" — direkt umsetzen. Kein Re-Insert möglich (per Definition unique-by-pair). Kein Snapshot-Join-Problem, weil Phase 3 `member_id` direkt referenziert.
- **`FOREIGN KEY ... REFERENCES ...` ohne explizites `ON DELETE`:** SQLite-Default ist `NO ACTION` (≈ RESTRICT). Soft-Delete ist projektweit Norm; Hard-Delete würde Snapshot-Daten beschädigen. NO ACTION ist hier korrekt: Hard-Delete eines Members oder einer Assembly würde via SQLite-FK fehlschlagen — Soft-Delete via `deleted`-Timestamp bleibt für beide funktional unverändert.
- **Kein `UNIQUE`-Constraint extra nötig:** Composite PK ist bereits `UNIQUE` per Definition. Doppelter Insert → SQLite-Constraint-Error → Tx rollback (Pitfall 5 abgedeckt).
- **Index auf `(assembly_id)`:** für `count_snapshot(assembly_id)`-Query (D-04 ad-hoc COUNT) — der Composite-PK liefert zwar einen Index, aber moderne SQLite optimiert COUNT(*) WHERE assembly_id = ? am besten mit dediziertem `idx_assembly_member_snapshot_assembly_id`. Im Composite-PK ist `assembly_id` an erster Stelle → eigentlich redundant, aber harmlos und explizit.
- **Index auf `assembly(status)` und `assembly(deleted)`:** Standard-Pattern, entspricht `application`-Migration.
- **Index auf `assembly(date)`:** Optional, hilft bei späteren GET-Queries mit Datums-Filter (Phase 5 evtl.). Empfehlung: aufnehmen — günstig, vorausschauend.

**Hinweis zum Filename-Timestamp:** Der nächste freie Slot nach `20260417000000_add_session_last_used_at.sql` ist die laufende Datums-Sequenz. Bei der tatsächlichen Erstellung Datum von heute / Plan-Tag verwenden, z. B. `20260502000000_create_assembly_table.sql` und `20260502000001_create_assembly_member_snapshot_table.sql`. Sortier-Reihenfolge: assembly zuerst (FK-Target).

### 2. AssemblyStatus Enum

Siehe Code-Beispiel 1 oben. Plus TO-Pendant in `genossi_rest_types/src/lib.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AssemblyStatusTO {
    Preparation,
    Open,
    Closed,
}

impl From<&AssemblyStatus> for AssemblyStatusTO {
    fn from(s: &AssemblyStatus) -> Self {
        match s {
            AssemblyStatus::Preparation => AssemblyStatusTO::Preparation,
            AssemblyStatus::Open => AssemblyStatusTO::Open,
            AssemblyStatus::Closed => AssemblyStatusTO::Closed,
        }
    }
}

impl From<&AssemblyStatusTO> for AssemblyStatus {
    fn from(s: &AssemblyStatusTO) -> Self {
        match s {
            AssemblyStatusTO::Preparation => AssemblyStatus::Preparation,
            AssemblyStatusTO::Open => AssemblyStatus::Open,
            AssemblyStatusTO::Closed => AssemblyStatus::Closed,
        }
    }
}
```

**Utoipa:** `ToSchema`-Derive reicht; kein extra Annotation nötig. Schema-Discriminator ist auto-generiert.

### 3. Auditable-Trait-Impl für Assembly

Siehe Code-Beispiel 3 oben. **Field count: 6** (`name`, `date`, `location`, `status`, `opened_at`, `closed_at`). Excluded per D-10: `id`, `version`, `created`, `deleted`.

### 4. count_active-Filter — Empfehlung

Siehe Code-Beispiel 2 oben. **Empfehlung:** Im AssemblyServiceImpl::open_assembly inline filtern:

```rust
let all_members = self.member_dao.all(tx.clone()).await?;  // bereits deleted-gefiltert
let active_member_ids: Vec<Uuid> = all_members.iter()
    .filter(|m| m.status.is_normal())
    .filter(|m| m.exit_date.map_or(true, |d| d > opened_date))
    .map(|m| m.id)
    .collect();
```

**Begründung:**
- `count_active` liefert nur `u64`, wir brauchen die Member-IDs.
- Eine separate Methode `MemberDao::find_all_active(today, tx)` zu introduceren (DAO-Trait-Erweiterung) wäre über-engineered für ein einziges Use-Case.
- Inline-Filter ist 4 Zeilen, identisch zum count_active-Predikat aus Member-DAO Zeilen 180-182, und in einem Test verifizierbar.
- Falls Phase 3 dieselbe Logik nochmal braucht (für Helfer-Member-View): dann erst eine gemeinsame Funktion extrahieren. YAGNI für Phase 1.

### 5. AssemblyDao-Trait-Shape

```rust
// genossi_dao/src/assembly.rs
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
    async fn create(&self, entity: &AssemblyEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn update(&self, entity: &AssemblyEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    // Defaults:
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError> { /* ... */ }
    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<AssemblyEntity>, DaoError> { /* ... */ }
}
```

```rust
// genossi_dao/src/assembly_member_snapshot.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyMemberSnapshotEntity {
    pub assembly_id: Uuid,
    pub member_id: Uuid,
    pub captured_at: time::PrimitiveDateTime,
}

#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyMemberSnapshotDao {
    type Transaction: crate::Transaction;

    /// Insert a single snapshot row. process is the Audit-process-string from the caller (informational, NOT logged).
    async fn create(&self, entity: &AssemblyMemberSnapshotEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    /// Optional batch helper for performance; can be omitted if iteration is fine.
    async fn create_batch(&self, entities: &[AssemblyMemberSnapshotEntity], process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    /// Find all snapshot member IDs for an assembly.
    async fn find_by_assembly_id(&self, assembly_id: Uuid, tx: Self::Transaction) -> Result<Arc<[AssemblyMemberSnapshotEntity]>, DaoError>;

    /// Ad-hoc COUNT for Y per D-04.
    async fn count_by_assembly_id(&self, assembly_id: Uuid, tx: Self::Transaction) -> Result<u64, DaoError>;
}
```

**Wichtig:** AssemblyMemberSnapshot implementiert KEINEN Auditable-Trait — Snapshot-Rows sind Daten, kein Lifecycle-Event (siehe Pitfall 1). `process`-Parameter im Trait ist nur Konvention für API-Konsistenz mit anderen DAOs; wird in der SQL-Impl nicht verwendet.

### 6. Service-Shape

```rust
// genossi_service/src/assembly.rs
#[derive(Clone, Debug)]
pub struct Assembly { /* alle Entity-Felder */ }
impl From<&AssemblyEntity> for Assembly { /* ... */ }

#[derive(Clone, Debug)]
pub struct AssemblySubmission {
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
}

#[derive(Clone, Debug)]
pub struct AssemblyUpdate {
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
    pub version: Uuid,  // Optimistic Locking, siehe Open Q2
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AssemblyService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn create_assembly(&self, submission: &AssemblySubmission, context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn update_assembly(&self, id: Uuid, update: &AssemblyUpdate, context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn open_assembly(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn close_assembly(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn get_assembly(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<AssemblyDetail, ServiceError>;
    async fn get_all_assemblies(&self, context: Authentication<Self::Context>) -> Result<Arc<[Assembly]>, ServiceError>;
}

#[derive(Clone, Debug)]
pub struct AssemblyDetail {
    pub assembly: Assembly,
    pub snapshot_member_count: u64,  // siehe Open Q1
}
```

```rust
// genossi_service_impl/src/assembly.rs
const ASSEMBLY_PROCESS_CREATE: &str = "assembly.create";
const ASSEMBLY_PROCESS_OPEN: &str = "assembly.open";
const ASSEMBLY_PROCESS_CLOSE: &str = "assembly.close";
const ASSEMBLY_PROCESS_UPDATE: &str = "assembly.update";  // für update_assembly
const ADMIN_PRIVILEGE: &str = "admin";

gen_service_impl! {
    struct AssemblyServiceImpl: AssemblyService = AssemblyServiceDeps {
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        AssemblyMemberSnapshotDao: AssemblyMemberSnapshotDao<Transaction = Self::Transaction> = assembly_member_snapshot_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

### 7. Audit-Macro-Invocations

Siehe Code-Beispiel 4 oben.

| Lifecycle | Macro | Process-String | Was wird geloggt |
|-----------|-------|----------------|------------------|
| `create_assembly` | `audited_create!` | `"assembly.create"` | Alle 6 audit_fields mit `Some`-Wert (name, date, status=Preparation; location/opened_at/closed_at als None werden NICHT geloggt — `build_create_entries` filtert `is_some`) → 3 Audit-Rows |
| `open_assembly` | `audited_update!` | `"assembly.open"` | Diff: `status: Preparation → Open` + `opened_at: None → <ts>` → 2 Audit-Rows |
| `close_assembly` | `audited_update!` | `"assembly.close"` | Diff: `status: Open → Closed` + `closed_at: None → <ts>` → 2 Audit-Rows |
| `update_assembly` | `audited_update!` | `"assembly.update"` | Diff der geänderten Felder (name/date/location) — variabel |

Die `action`-Spalte des audit_log-Records ist immer `"create"` oder `"update"` (hardcodiert in build_*_entries; siehe genossi_service_impl/src/audit_log.rs:133, 171). Die Lifecycle-Aktion liegt in der `process`-Spalte.

### 8. State-Transition Guard

Code direkt im Service:

```rust
// open_assembly:
if entity.status != AssemblyStatus::Preparation {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Cannot open assembly: status is '{}', expected 'Preparation'",
        entity.status.as_str()
    ))));
}

// close_assembly:
if entity.status != AssemblyStatus::Open {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Cannot close assembly: status is '{}', expected 'Open'",
        entity.status.as_str()
    ))));
}

// update_assembly:
if entity.status != AssemblyStatus::Preparation {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Cannot update assembly: status is '{}', expected 'Preparation' (D-07)",
        entity.status.as_str()
    ))));
}
```

`ServiceError::Conflict` mappt auf HTTP 409 via genossi_rest/src/lib.rs:101. Pattern entspricht `ApplicationServiceImpl::confirm` (Zeile 291-296).

### 9. REST-Handler — Endpoints

Sechs Endpoints (D-13). Alle mit `permission_service.check_permission("admin", auth)` (D-14).

```rust
// genossi_rest/src/assembly.rs
pub fn generate_route<RestState: RestStateDef + AssemblyRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_assemblies::<RestState>).post(create_assembly::<RestState>))
        .route("/{id}", get(get_assembly::<RestState>).put(update_assembly::<RestState>))
        .route("/{id}/open", post(open_assembly::<RestState>))
        .route("/{id}/close", post(close_assembly::<RestState>))
}
```

| Method | Path | Handler | Service-Call | Status-Codes |
|--------|------|---------|--------------|--------------|
| POST | `/api/assembly` | `create_assembly` | `service.create_assembly(submission, auth)` | 201, 401, 422 |
| GET | `/api/assembly` | `list_assemblies` | `service.get_all_assemblies(auth)` | 200, 401 |
| GET | `/api/assembly/{id}` | `get_assembly` | `service.get_assembly(id, auth)` (Returns `AssemblyDetail` with snapshot_member_count) | 200, 401, 404 |
| PUT | `/api/assembly/{id}` | `update_assembly` | `service.update_assembly(id, update, auth)` | 200, 401, 404, 409, 422 |
| POST | `/api/assembly/{id}/open` | `open_assembly` | `service.open_assembly(id, auth)` | 200, 401, 404, 409 |
| POST | `/api/assembly/{id}/close` | `close_assembly` | `service.close_assembly(id, auth)` | 200, 401, 404, 409 |

`AssemblyRestState`-Trait analog `ApplicationRestState` (genossi_rest/src/application.rs:22-33). Utoipa-`ApiDoc` analog Zeile 493-511. Permission-Check passiert im Service-Layer (Service ruft `check_permission("admin", ...)`); Handler delegiert nur. Validation für TO-Felder (name nicht leer, location max-len) im Handler analog `validate_join_request`.

### 10. Open-Assembly Atomicity

Siehe Code-Beispiel 5 (`open_assembly`-Skelett) oben. Die kritische Sequenz:

```
1. let tx = transaction_dao.use_transaction(None).await?;          // Tx eröffnen
2. permission + load + guard
3. mutate entity (status=Open, opened_at=now)
4. audited_update!(...)                                            // Update assembly + Audit-Rows in tx
5. let active_members = member_dao.all(tx.clone()).await?;
6. for each active member: assembly_member_snapshot_dao.create(..., tx.clone())
7. transaction_dao.commit(tx).await?;                              // EINMAL committen
```

**Wichtig:** Ein einziger Commit ganz am Ende. `tx.clone()` für jeden DAO-Call (clone gibt einen neuen Handle auf dieselbe Tx). `audited_update!` committet NICHT (siehe audit_macros.rs Zeilen 43-80; Macro ruft nur DAO-update + audit_log_dao.create_entries). Bei jedem Fehler: Rust `?`-Operator returns früh; tx wird gedroppt → kein Commit → SQLite rollbacked automatisch beim Drop.

### 11. E2E-Test-Sketch (D-12)

Siehe Code-Beispiel 6 oben. Einbettung: am Ende von `genossi_bin/tests/e2e_tests.rs` anhängen (D-12: kein neues File). `setup()` aus Zeile 23-37 wiederverwenden (`SqlitePool::connect("sqlite::memory:") + sqlx::migrate!`).

**Was der Test verifiziert:**
1. HTTP 201/200/200 für create/open/close
2. Status-Werte korrekt (Preparation → Open → Closed)
3. `opened_at`, `closed_at` gesetzt
4. `GET /api/audit/verify` liefert HTTP 200 mit `valid: true` und `broken_links: []`
5. `total_entries >= 3` (Lifecycle-Events)
6. `GET /api/audit/assembly/{id}` enthält Audit-Einträge mit Process `"assembly.create"`, `"assembly.open"`, `"assembly.close"`

**Phase-1-spezifisch:** `feature = "mock_auth"` (e2e_tests.rs:1) ist gesetzt; `MockContext` lässt jeden Request durch, also Permission-Check wird erfüllt. In Produktion (OIDC) bräuchte der Test einen Login-Pfad — out-of-scope Phase 1.

### 12. DI-Wiring Delta in `genossi_bin/src/lib.rs`

**Änderungen in `RestStateImpl::new()`:**

```rust
// 1. Neue Type-Aliase nahe Zeile 122 (ApplicationDao):
type AssemblyDao = genossi_dao_impl_sqlite::assembly::AssemblyDaoImpl;
type AssemblyMemberSnapshotDao = genossi_dao_impl_sqlite::assembly_member_snapshot::AssemblyMemberSnapshotDaoImpl;

// 2. Deps-Struct + Service-Type nahe Zeile 124-144 (ApplicationServiceDependencies):
pub struct AssemblyServiceDependencies;
unsafe impl Send for AssemblyServiceDependencies {}
unsafe impl Sync for AssemblyServiceDependencies {}

impl genossi_service_impl::assembly::AssemblyServiceDeps for AssemblyServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AssemblyDao = AssemblyDao;
    type AssemblyMemberSnapshotDao = AssemblyMemberSnapshotDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}
type AssemblyService = genossi_service_impl::assembly::AssemblyServiceImpl<AssemblyServiceDependencies>;

// 3. RestStateImpl-Struct nahe Zeile 290: neues Feld
pub struct RestStateImpl {
    // ... existing ...
    assembly_service: Arc<AssemblyService>,
    // ...
}

// 4. RestStateImpl::new() nahe Zeile 409 (nach application_dao instantiation):
let assembly_dao = Arc::new(AssemblyDao::new(pool.clone()));
let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
let assembly_service = Arc::new(genossi_service_impl::assembly::AssemblyServiceImpl {
    assembly_dao,
    assembly_member_snapshot_dao,
    member_dao: member_dao.clone(),
    audit_log_dao: audit_log_dao.clone(),
    permission_service: permission_service.clone(),
    uuid_service: uuid_service.clone(),
    transaction_dao: transaction_dao.clone(),
});

// 5. Self { ... } nahe Zeile 525: assembly_service hinzufügen

// 6. Neuer Trait-Impl am Ende der Datei (analog ApplicationRestState Zeile 976-997):
impl genossi_rest::assembly::AssemblyRestState for RestStateImpl {
    type AssemblyService = AssemblyService;
    fn assembly_service(&self) -> Arc<Self::AssemblyService> {
        self.assembly_service.clone()
    }
}

// 7. initialize_audit_snapshot() nahe Zeile 566-722 erweitern:
//    Block analog zu Member/Application: Snapshot aller bestehenden Assemblies in den Audit-Log.
//    Phase 1 hat aber noch keine Assemblies → Block ist optional, kann beim ersten Production-Deploy nach Phase 1 leer durchlaufen.
//    Empfehlung: Block hinzufügen, weil Audit-Hashchain-Konsistenz das vorsieht.
```

### 13. Router-Registration in `genossi_rest/src/lib.rs`

**Änderungen:**

```rust
// 1. Top-of-File (Zeile 1-22): module declaration
pub mod assembly;

// 2. ApiDoc nest (Zeile 232-256): neue Zeile
#[derive(OpenApi)]
#[openapi(
    nest(
        // ... existing ...
        (path = "/api/assembly", api = assembly::ApiDoc),
        // ...
    )
)]
pub struct ApiDoc;

// 3. create_app() Type-Bound (Zeile 410-417): + assembly::AssemblyRestState
pub async fn create_app<
    RestState: RestStateDef
        + public_stats::PublicStatsState
        + application::ApplicationRestState
        + audit_log::AuditRestState
        + audit_timestamp::TimestampRestState
        + assembly::AssemblyRestState,    // NEW
>(
    rest_state: RestState,
) -> Router { /* ... */ }

// 4. Router-Nest (Zeile 559-571): neuen .nest hinzufügen
let app = app
    // ... existing ...
    .nest("/api/assembly", assembly::generate_route::<RestState>())
    // ...

// 5. start_server() Type-Bound (Zeile 674-680) gleich wie create_app (#3)
```

`AssemblyRestState`-Trait-Definition analog `ApplicationRestState` in `genossi_rest/src/assembly.rs`.

### 14. Files to create/modify

**NEW (10 Dateien):**
- `genossi_dao/src/assembly.rs` — AssemblyEntity, AssemblyStatus, AssemblyDao trait, Auditable-Impl
- `genossi_dao/src/assembly_member_snapshot.rs` — AssemblyMemberSnapshotEntity, AssemblyMemberSnapshotDao trait
- `genossi_dao_impl_sqlite/src/assembly.rs` — AssemblyDaoImpl
- `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs` — AssemblyMemberSnapshotDaoImpl
- `genossi_service/src/assembly.rs` — Assembly DTO, AssemblySubmission, AssemblyUpdate, AssemblyDetail, AssemblyService trait
- `genossi_service_impl/src/assembly.rs` — AssemblyServiceImpl, AssemblyServiceDeps, Lifecycle-Methods
- `genossi_rest/src/assembly.rs` — Axum-Handler, AssemblyRestState trait, generate_route, ApiDoc
- `migrations/sqlite/YYYYMMDDHHMMSS_create_assembly_table.sql`
- `migrations/sqlite/YYYYMMDDHHMMSS_create_assembly_member_snapshot_table.sql`

**MODIFY (5 Dateien):**
- `genossi_dao/src/lib.rs` — `pub mod assembly; pub mod assembly_member_snapshot;`
- `genossi_dao_impl_sqlite/src/lib.rs` — `pub mod assembly; pub mod assembly_member_snapshot;`
- `genossi_service/src/lib.rs` — `pub mod assembly;`
- `genossi_service_impl/src/lib.rs` — `pub mod assembly;` (+ ggf. re-exports)
- `genossi_rest_types/src/lib.rs` — append AssemblyStatusTO, AssemblyTO, CreateAssemblyRequest, UpdateAssemblyRequest, AssemblyDetailTO + ToSchema-Derives + From-Impls
- `genossi_rest/src/lib.rs` — `pub mod assembly;`, ApiDoc nest, `+ assembly::AssemblyRestState` Type-Bound, `.nest("/api/assembly", ...)`
- `genossi_bin/src/lib.rs` — Type-Aliase, AssemblyServiceDependencies, RestStateImpl-Field, ::new()-Wiring, AssemblyRestState-Impl, optional initialize_audit_snapshot-Block
- `genossi_bin/tests/e2e_tests.rs` — append `test_assembly_lifecycle_audit_chain_intact` + ggf. weitere Lifecycle-Negativ-Tests (Conflict bei falschem Status etc.)

**Test-Files (NEW innerhalb der `mod tests` der jeweiligen `.rs`-Dateien):**
- DAO-Tests in `genossi_dao/src/assembly.rs::tests` — Status-Roundtrip, Auditable-Field-Count
- DAO-Tests in `genossi_dao_impl_sqlite/src/assembly.rs::tests` — In-Memory-SQLite create/update/find
- Service-Tests in `genossi_service_impl/src/assembly.rs::tests` — mit `MockAssemblyDao`/`MockMemberDao`/`MockAuditLogDao`/`MockPermissionService`: Lifecycle-Guards (open from Closed → Conflict), Snapshot-Filter-Logik
- REST-Tests in `genossi_rest/src/assembly.rs::tests` — falls Validation-Helper (analog `validate_join_request`-Tests in application.rs:545-616)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — alle Patterns aus Bestand verifiziert per Read; keine externen Pakete
- Architecture: HIGH — 1:1 Replication of Application-Aggregat; Snapshot ist neu, aber simple Insert-Logik
- Pitfalls: HIGH — Pitfalls 1-6 sind aus Code-Analyse + CONTEXT-Decisions abgeleitet, alle mit konkretem Vermeidungs-Ansatz
- Audit-Macros: HIGH — Verhalten der Macros 1:1 aus Source verifiziert
- Migration: HIGH — Vorlagen Application + Member-Migrations geprüft; Snapshot-DDL ist konservativ (Composite-PK, NO ACTION FK)
- E2E-Test: HIGH — bestehender `test_audit_verify_after_operations`-Test als 1:1-Vorlage; Erweiterung trivial
- Open Q1 (snapshot list vs count): MEDIUM — D-13 lässt beide Optionen offen; Empfehlung "nur Count" basiert auf YAGNI + Phase-3-Scope

**Research date:** 2026-05-02
**Valid until:** 2026-06-01 (30 Tage; Codebase ist stabil und Phase-1-relevante Patterns ändern sich nicht)
