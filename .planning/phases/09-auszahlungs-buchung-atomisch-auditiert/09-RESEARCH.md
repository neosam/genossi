# Phase 9: Auszahlungs-Buchung (atomisch + auditiert) - Research

**Researched:** 2026-05-31
**Domain:** Cross-Entity-Cascade in Single SQLite-Tx + Audit-Chain + Race-Defense
**Confidence:** HIGH (alle 10 offenen Punkte am Code verifiziert; keine Annahmen über ungesehene Anker)

## Executive Summary

Phase 9 ist eine reine Service-Layer-Erweiterung: kein neues Aggregat, keine Migration, keine neuen DTOs (Response = bestehender `RepaymentEntryTO`). Die Cascade besteht aus genau drei `audited_*!`-Aufrufen in einer SQLite-Tx, eingerahmt von Permission-Check, vier Pre-Condition-Loads (Entry/Phase/Member + Status-Guards) und zwei Re-Read-Stellen nach den `audited_update!`. Phase 8 hat alle Pattern-Anker bereits gesetzt: Re-Read-Pattern (BL-01-Fix), strukturierte 409-Bodies, Audit-Disziplin-Macros, Tx-Scope-Loading. **Race-Defense funktioniert NICHT über `SQLITE_BUSY`** (siehe Frage 1) — die Status-Guard-Logik ist hier die einzige Verteidigungslinie, und sie funktioniert nur, WEIL die einzige Schreib-Tx-Bahn pro Connection serialisiert wird (sqlx Pool gibt jeweils eine Connection, alle Writes innerhalb einer Tx halten die Connection-Lock). Der Verlierer der Race scheitert mit `DaoError::ConflictError("Version mismatch")` auf dem `RepaymentEntry`-Update, weil der Gewinner die `version` zuvor neu gesetzt hat — das ist sauber HTTP 409.

**Primary recommendation:** Plan macht `compute_migration_status` `pub` statt Trait-Methode (Option (a) aus D-10), nutzt das exakte Re-Read+InternalError-Pattern aus `repayment_entry.rs:277-287`, und verlässt sich für SC #5 auf die natürliche `UPDATE ... WHERE id = ? AND version = ?`-Locking-Semantik des RepaymentEntry-DAO. Kein zusätzliches `UPDATE...RETURNING`-Pattern nötig.

## Resolved Open Questions

### Frage 1: D-11 Race-Defense — was passiert wirklich bei `tokio::join!`?

**HIGH confidence** — direkt am Code verifiziert.

**Tatsachen:**
1. `SqlitePool::connect(&database_url)` in `genossi_bin/src/main.rs:25` benutzt sqlx-Defaults — **kein WAL-Mode, kein Custom-PRAGMA, kein BEGIN IMMEDIATE/EXCLUSIVE**. Grep auf `journal_mode|WAL|PRAGMA` über `genossi_bin/` und `genossi_dao_impl_sqlite/` ergibt nur `PRAGMA foreign_keys = ON` in `helper_token.rs:392` (per-statement, nicht pool-weit). Es gibt also weder global aktiviertes WAL noch `journal_mode=WAL` über Connection-Options.
2. `TransactionImpl::new` (`genossi_dao_impl_sqlite/src/transaction.rs:14-24`) ruft `pool.begin()` auf — sqlx übersetzt das in `BEGIN DEFERRED` (sqlx-Default).
3. Pool-Modell: sqlx `SqlitePool` ist ein Connection-Pool; jede `begin()`-Tx hält genau EINE Connection für die ganze Tx-Lebensdauer. Bei `tokio::join!(mark_paid_out, mark_paid_out)` läuft jede Tx auf einer eigenen Connection.

**Was passiert beim Race im Detail (deterministisch):**
- Tx-A und Tx-B starten parallel, beide BEGIN DEFERRED, beide laden Entry → sehen `status=Open`, `version=V1`.
- Beide bauen lokal `entity.status=PaidOut`, beide rufen `audited_update!`.
- `audited_update!` ruft intern erneut `find_by_id` (siehe `audit_macros.rs:47-50`) → beide sehen weiterhin `version=V1` (jeweils im Tx-Scope; sqlite default-isolation ist SERIALIZABLE auf Connection-Ebene, aber wegen rollback-journal nicht zwischen Connections sichtbar bis Commit).
- Tx-A's `dao.update(...)` macht `UPDATE repayment_entry ... WHERE id = ? AND version = V1` (siehe `genossi_dao_impl_sqlite/src/repayment_entry.rs:160-172`) — `rows_affected = 1`, **die Connection eskaliert intern auf einen RESERVED→PENDING→EXCLUSIVE-Lock** (SQLite-Standardverhalten bei erstem Write in einer DEFERRED-Tx).
- Tx-B's `dao.update(...)` wartet auf den EXCLUSIVE-Lock. **Mit Default-Timeout (`busy_timeout = 5s` default in sqlx) wartet B**, bis A entweder committet oder rollbackt.
- Tx-A committet (`tx.commit().await`) — version ist jetzt V2, status=PaidOut.
- Tx-B's `UPDATE` läuft endlich — die `WHERE id = ? AND version = V1`-Klausel matched aber 0 Rows (version ist jetzt V2). `rows_affected == 0` → `DaoError::ConflictError(Arc::from("Version mismatch"))` (Z. 178-180).
- `audited_update!` propagiert das → `update_repayment_entry`-Call returnt `ServiceError::DataAccess`-Variante? Nein: `From<DaoError> for ServiceError` mapped `ConflictError` → `ServiceError::Conflict(msg)`. → HTTP 409. ✓

**SQLITE_BUSY-Pfad (Fallback):** Falls der `busy_timeout` zu kurz wäre und der EXCLUSIVE-Lock nicht beschafft werden kann, würde sqlx einen Database-Fehler zurückgeben — der mapped via `DaoError::DatabaseError` → `ServiceError::DataAccess` → globalem `From<ServiceError> for RestError` → `RestError::InternalError` → HTTP 500. Das ist NICHT der Pfad, den wir wollen. Bei `tokio::join!` zweier Tasks im selben Prozess passiert das praktisch nie, weil die Wait-Time im Mikrosekunden-Bereich ist.

**Verdict:** Die Race-Defense ist robust:
- **Primärer Pfad:** Version-mismatch-409 via `UPDATE ... WHERE version = ?` im DAO (deterministisch).
- **Defense-in-Depth:** Status-Guard `entry.status ∈ {Open, Contacted}` (D-09 Schritt 3) — feuert NICHT in der Race, weil beide Tx den Pre-Status `Open` sehen, sondern bei sequentiellem Doppelaufruf (PAYO-04: zweiter Call sieht `status=PaidOut` → 409).

**Empfehlung an Planner:** **Kein zusätzliches `UPDATE ... WHERE status = ? RETURNING ...`-Pattern nötig.** Die natürliche Version-Bump-Semantik des RepaymentEntry-DAO (`repayment_entry_dao_impl_sqlite.rs:160-172`) plus der Phase-8-Pattern-Status-Guard liefern SC #5 zuverlässig. Der Race-Test in E2E muss den Verlierer-Pfad als HTTP 409 (nicht 500) erwarten — falls in CI gelegentlich 500 statt 409 auftaucht, ist das ein Hinweis auf `busy_timeout`-Konfiguration.

**Citations:**
- `genossi_bin/src/main.rs:24-26` (Pool ohne Custom-Options)
- `genossi_dao_impl_sqlite/src/transaction.rs:14-24` (BEGIN DEFERRED)
- `genossi_dao_impl_sqlite/src/repayment_entry.rs:160-180` (UPDATE+WHERE version=? + rows_affected==0 → ConflictError)
- `genossi_dao/src/lib.rs` `From<DaoError> for ServiceError` mapped `ConflictError(s) → ServiceError::Conflict(s)` (`[VERIFIED]` aus Phase 7 Plan 5 Lifecycle-Tests, die stale-version → 409 verifizieren — siehe Phase-7 STATE-Notiz Plan 07-05).

---

### Frage 2: D-10 `compute_migration_status`-Access — was empfehlen wir?

**HIGH confidence** — `pub(crate)` verifiziert, Größe gemessen, andere Konsumenten gefunden.

**Tatsachen:**
- `compute_migration_status` lebt in `genossi_service_impl/src/member_action.rs:32-69` (38 LOC), aktuell `pub(crate)`.
- `recalc_migrated`-Helper (gleiche Datei Z. 200-224, 25 LOC) ruft es auf — ist als `impl<Deps> MemberActionServiceImpl<Deps>`-Inherent-Methode definiert, nicht im Trait.
- Aktuelle Konsumenten im Crate: nur `MemberActionServiceImpl::recalc_migrated` (`member_action.rs:216`). Grep `compute_migration_status` im Workspace: nur 2 Treffer (Definition + Aufrufer).
- `pub(crate)` war eine **bewusste Sichtbarkeits-Wahl, nicht Trait-Modellierung** — es gibt keinen Sicherheits- oder Korrektheitsgrund, der gegen `pub` spricht. Die Funktion ist pure (kein Tx, keine async, keine I/O); sie liest `MemberEntity` + `&[MemberActionEntity]` und gibt `MigrationStatus` zurück.
- `MemberActionService::recalc_migrated` als Trait-Methode existiert NICHT (gegen-geprüft: `genossi_service/src/member_action.rs` hat nur `get_by_member`, `get`, `create`, `update`, `delete` — Trait-Methoden-Set Phase 4/5).

**Optionen-Analyse:**
- **(a) `compute_migration_status` `pub` machen + RepaymentEntryServiceImpl ruft es direkt + dupliziert `recalc_migrated`-Wrapper (5 LOC: load member + load actions + compute + update_migrated)** — **EMPFOHLEN**. Minimal-invasiv, keine Trait-API-Änderung, hält Phase-2-D-04-Pragma „direkte DAO-Calls aus Cascade-Owner" konsistent.
- **(b) `MemberActionService::recalc_migrated`-Trait-Methode** — bringt Service-zu-Service-Call (RepaymentEntryServiceImpl bekommt `MemberActionService`-Dep statt `MemberActionDao`). Konflikt mit D-08 („Service-zu-Service bewusst vermieden, direkter DAO-Zugriff hält Tx-Atomarität deterministisch"). 12+ LOC Trait + Mock-Aufwand. Schlechter.
- **(c) Vollduplizierung von `compute_migration_status` (38 LOC) + `recalc_migrated` (25 LOC) in RepaymentEntryServiceImpl** — 63 LOC Code-Duplikat einer subtilen Off-by-One-Logik (`expected_action_count = member.action_count + 1`). Hoher Pflege-Aufwand. Schlechter.

**Konkrete Empfehlung — Plan macht (a):**

```rust
// Edit 1: genossi_service_impl/src/member_action.rs:32
- pub(crate) fn compute_migration_status(
+ pub fn compute_migration_status(

// Edit 2: genossi_service_impl/src/repayment_entry.rs (in mark_paid_out, nach Re-Read von Entry, VOR commit):
let actions = self
    .member_action_dao
    .find_by_member_id(entry.member_id, tx.clone())
    .await?;
let member_after = self
    .member_dao
    .find_by_id(entry.member_id, tx.clone())
    .await?
    .ok_or_else(|| ServiceError::InternalError(Arc::from(
        "Member vanished after Cascade — Tx-isolation broken"
    )))?;
let status = crate::member_action::compute_migration_status(&member_after, &actions);
let migrated = status.status == genossi_service::member_action::MigrationState::Migrated;
self.member_dao
    .update_migrated(entry.member_id, migrated, tx.clone())
    .await?;
```

Größe: ca. 18 LOC neuer Inline-Code. Akzeptabel als bewusste Pattern-Wiederholung; die Alternative wäre ein neuer Helper auf `RepaymentEntryServiceImpl<Deps>` (5 LOC) — Planner darf das entscheiden.

**Pitfall:** `find_by_member_id` auf `MemberActionDao` ist eine DAO-Default-Impl (siehe `genossi_dao/src/member_action.rs:142-154`), die intern `dump_all` + In-Memory-Filter macht. Bei wachsender `member_action`-Tabelle ist das O(n). Aktuell akzeptabel; Index-Optimierung wäre tech-debt.

---

### Frage 3: MemberAction.date — exakter Rust-Ausdruck

**HIGH confidence** — Field-Type direkt gelesen.

**Verifikation:**
- `MemberActionEntity.date: time::Date` (`genossi_dao/src/member_action.rs:57`).
- `MemberAction.date: time::Date` (Service-Domain, gleiche Konvertierung wie Entity).
- Pattern-Anker: `MemberActionServiceImpl::create` baut `date: item.date` aus dem Input — der Input ist üblicherweise `time::OffsetDateTime::now_utc().date()` (vgl. `member_action.rs:321-322` für `created`-Initialisierung).

**Exakter Ausdruck:**
```rust
let now = time::OffsetDateTime::now_utc();
let today_date: time::Date = now.date();
// ... später beim Bauen des Entities:
date: today_date,
```

**Pitfall:** `time::Date` hat keinen `From<PrimitiveDateTime>`-Impl — Plan darf NICHT `created.date()` machen auf einer `PrimitiveDateTime`-Variablen ohne `assume_utc()`. Sauberster Pfad: einmal `let now = time::OffsetDateTime::now_utc()`, dann sowohl `today_date = now.date()` als auch `created = time::PrimitiveDateTime::new(now.date(), now.time())` daraus ableiten.

---

### Frage 4: `fiscal_year` für Auto-Comment

**HIGH confidence** — Feld direkt am Entity verifiziert.

**Verifikation:**
- `RepaymentPhaseEntity.fiscal_year: i32` (`genossi_dao/src/repayment_phase.rs:47`).
- Wird in `RepaymentPhaseEntity` audit_fields als `Some(self.fiscal_year.to_string())` serialisiert (`genossi_dao/src/repayment_phase.rs:85`).
- D-09 Schritt 4 lädt `Phase` für den Status-Guard. Das Entity ist also bereits in der Hand des Cascade-Codes — `phase.fiscal_year` ist direkt verfügbar.

**Exakter Ausdruck:**
```rust
let comment = format!("Anteils-Rückzahlung Phase {}", phase.fiscal_year);
// ... beim Bauen des MemberActionEntity:
comment: Some(Arc::from(comment)),
```

**Verifikation Type-Match:** `MemberActionEntity.comment: Option<Arc<str>>` (`genossi_dao/src/member_action.rs:61`) — `Arc::from(String)` ist implementiert. ✓

---

### Frage 5: `shares_change` vs `share_count_to_pay_out` — Typen + Overflow

**HIGH confidence** — beide Felder direkt verifiziert.

**Typen:**
- `MemberActionEntity.shares_change: i32` (`genossi_dao/src/member_action.rs:58`)
- `RepaymentEntryEntity.share_count_to_pay_out: i32` (`genossi_dao/src/repayment_entry.rs` — aus Phase 8 Plan 01)
- `MemberEntity.current_shares: i32` (`genossi_dao/src/member.rs:90`)
- DB-Constraint `CHECK(share_count_to_pay_out > 0)` (Phase 8 Plan 01 Migration, siehe sqlite test setup Z. 207).

**Overflow-Analyse:**
- Negation `-(entry.share_count_to_pay_out)` mit `i32`: Worst-Case ist `share_count_to_pay_out = i32::MAX`. `-(i32::MAX) = i32::MIN + 1` — kein Overflow. Der einzige problematische Wert wäre `i32::MIN`, aber DB-CHECK verhindert ≤0, und ein PAYO-03-Service-Layer-Check verhindert `> current_shares` (ebenfalls i32, also ≤ i32::MAX).
- Subtraktion `member.current_shares - entry.share_count_to_pay_out` mit `i32`: durch PAYO-03-Validation gilt `current_shares ≥ share_count_to_pay_out`, also ist die Differenz ≥ 0 und ≤ current_shares. Kein Overflow.
- Addition `member.action_count + 1` mit `i32`: theoretischer Overflow bei `i32::MAX` actions — praktisch unmöglich (max ~50 actions/member realistisch). Plan darf `wrapping_add(1)` einsetzen oder `checked_add(1).ok_or(InternalError)` als Defense-in-Depth.

**Exakte Cascade-Berechnung:**
```rust
let shares_change: i32 = -entry.share_count_to_pay_out; // <- entry.share_count_to_pay_out > 0 garantiert
let new_current_shares: i32 = member.current_shares - entry.share_count_to_pay_out; // <- garantiert ≥ 0 nach PAYO-03
let new_action_count: i32 = member.action_count + 1; // <- praktisch sicher; defensiv checked_add möglich
```

---

### Frage 6: Auto-Mock-Generierung für `mark_paid_out`

**HIGH confidence** — Trait-Annotation direkt gelesen.

**Verifikation:**
- `RepaymentEntryService` ist mit `#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]` annotiert (`genossi_service/src/repayment_entry.rs:120`).
- mockall generiert automatisch `MockRepaymentEntryService` mit `expect_<method>()` für JEDE Trait-Methode. Erweiterung des Traits um `mark_paid_out` erzeugt automatisch `expect_mark_paid_out()`.
- Der bestehende Compile-Test in `tests::test_mock_repayment_entry_service_compiles` (`genossi_service/src/repayment_entry.rs:272-283`) listet alle 6 derzeitigen Mocks — **Plan muss `let _ = mock.expect_mark_paid_out();` als 7. Zeile hinzufügen**, sonst keine Auto-Verifikation des Mocks.

**Implikation:** Plan braucht KEINEN manuellen Mock-Update für `RepaymentEntryService`; mockall macht das automatisch. ABER: der hand-rolled Test-Mock `TestRepaymentEntryDao` (siehe Frage 9) muss erweitert werden, falls neue DAO-Methoden auf `MemberActionDao` aufgerufen werden, die der bestehende `TestMemberActionDao`-Mock noch nicht hat. Phase 9 ruft auf `MemberActionDao`: `create` (via Macro), `find_by_member_id`. Beide sind bereits Standard-Trait-Methoden (siehe `genossi_dao/src/member_action.rs:101-155`); `automock` würde sie generieren — aber Phase 8 nutzt **hand-rolled mockall::mock!{}**-Mocks für DAO-Mocks (siehe `repayment_entry.rs:556+`), nicht automock — siehe Frage 9.

---

### Frage 7: REST-Router-Registration für `/{id}/mark-paid-out`

**HIGH confidence** — exaktes Pattern aus repayment_phase.rs übernommen.

**Verifikation:**
- `repayment_phase.rs:349-350` zeigt das exakte Pattern für `/{id}/open` und `/{id}/close`:
  ```rust
  .route("/{id}/open", post(open_repayment_phase::<RestState>))
  .route("/{id}/close", post(close_repayment_phase::<RestState>))
  ```
- Axum 0.8 Path-Extractor für single-Uuid: `Path(id): Path<Uuid>` (vgl. `repayment_phase.rs:251` `open_repayment_phase`). **Nicht** `Path((id,))` — das wäre für Tuples.
- `repayment_entry.rs:302-316` zeigt `generate_route()`-Struktur — der neue Sub-Route geht ans Ende, NICHT vor `/{id}`:
  ```rust
  pub fn generate_route<RestState: RestStateDef + RepaymentEntryRestState>() -> Router<RestState> {
      Router::new()
          .route("/batch-status", post(batch_toggle_status::<RestState>))  // VOR /{id} — literal
          .route("/", get(list_repayment_entries::<RestState>).post(create_repayment_entry::<RestState>))
          .route("/{id}", get(...).put(...).delete(...))
          // Phase 9: NEW
          .route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))
  }
  ```

**Pitfall (T-08-05-02-Pattern):** `/{id}/mark-paid-out` ist eindeutig (Sub-Pfad nach Uuid), kollidiert nicht mit `/{id}` oder `/batch-status`. **Reihenfolge ist hier egal**, weil Axum nach Pfad-Spezifität matcht (literal `/mark-paid-out` ist spezifischer als `/{id}` ohne Suffix). Aber Konvention der Codebase setzt Action-Endpoints ans Ende — Plan folgt der Konvention.

**Beispiel-Handler-Signatur:**
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "/{id}/mark-paid-out",
    params(("id" = Uuid, Path, description = "RepaymentEntry ID")),
    responses(
        (status = 200, description = "Marked as PaidOut + MemberAction::Verkauf created + Member.current_shares reduced (atomic, audited)", body = RepaymentEntryTO),
        (status = 400, description = "Validation Error (PAYO-03: current_shares < share_count_to_pay_out)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (entry not Open/Contacted, phase not Open, version race)"),
        (status = 500, description = "Internal consistency error (Re-Read returned None)"),
    ),
)]
pub async fn mark_paid_out<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let entry = rest_state
                .repayment_entry_service()
                .mark_paid_out(id, auth)
                .await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}
```

**ApiDoc-Erweiterung:** `mark_paid_out` zur `paths(...)`-Liste in `ApiDoc` (`repayment_entry.rs:319-327`) hinzufügen.

---

### Frage 8: Phase-2 Race-Test-Pattern

**HIGH confidence** — exakte Stelle gefunden.

**Citation:** `genossi_bin/tests/e2e_tests.rs:8783-8821` — `test_helper_token_redeem_race_one_succeeds_one_fails`.

**Pattern (zusammengefasst):**
```rust
#[tokio::test]
async fn test_mark_paid_out_race_one_succeeds_one_conflicts() {
    let server = setup().await;
    let client = reqwest::Client::new();
    // ... setup phase + member + entry as in Phase 8 helpers ...
    let entry_id = create_entry_in_open_phase(&client, &server, ...).await;

    let url = server.url(&format!("/api/repayment-entry/{}/mark-paid-out", entry_id));

    // Two parallel requests via tokio::join!
    let (resp_a, resp_b) = tokio::join!(
        client.post(&url).send(),
        client.post(&url).send(),
    );
    let status_a = resp_a.unwrap().status();
    let status_b = resp_b.unwrap().status();

    // Exactly one 200 and one 409 (NOT 500, NOT 2x200, NOT 2x409).
    let mut statuses = [status_a, status_b];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(statuses[0], StatusCode::OK, "one must succeed; got {:?}", statuses);
    assert_eq!(statuses[1], StatusCode::CONFLICT, "other must be 409; got {:?}", statuses);
}
```

**Zweiter Anker:** `e2e_tests.rs:9371-9384` — `test_attendance_upsert_race_one_row_two_200ok` zeigt das idempotente UPSERT-Pattern (beide 200, ein Row in stats). Phase 9 ist NICHT idempotent — Plan nutzt das HLPR-04-Pattern (8783), nicht ATTN-03 (9371).

**Pitfall:** Race-Tests sind theoretisch nondeterministisch, aber in der Praxis (sqlx + tokio + SQLite-Default-busy_timeout=5s) sehr zuverlässig auf 1-Core- und Multi-Core-CI. Falls in CI Flakiness auftritt: das ist ein Hinweis auf zu kurzen `busy_timeout` und gehört als Tech-Debt nach Phase 9.

---

### Frage 9: TestMemberActionDao-Mock-Pattern + bestehende Mocks

**HIGH confidence** — alle bestehenden Mocks gelesen.

**Bestehende Mocks in `genossi_service_impl/src/repayment_entry.rs:556-813`:**
- `MockTestTxDao` (Z. 542-554) — `TransactionDao` mit `TestTransaction`-AT
- `MockTestRepaymentEntryDao` (Z. 556-592) — alle 6 `RepaymentEntryDao`-Methoden
- `MockTestRepaymentPhaseDao` (Z. 594-625) — alle 5 `RepaymentPhaseDao`-Methoden
- `MockTestMemberDao` (Z. 627-682) — alle 9 `MemberDao`-Methoden (inkl. `update_dates`, `update_migrated`, `next_member_number`)
- `MockTestAuditLogDao` (Z. 684-718) — alle 6 `AuditLogDao`-Methoden
- `MockTestPermissionService` (Z. 720-813) — alle 19+ `PermissionService`-Methoden

**Fehlend für Phase 9: `MockTestMemberActionDao`.**

**Konkretes Pattern für Plan (anhand `genossi_dao/src/member_action.rs:101-155`):**
```rust
mock! {
    pub TestMemberActionDao {}
    #[async_trait]
    impl MemberActionDao for TestMemberActionDao {
        type Transaction = TestTransaction;
        async fn dump_all(
            &self,
            tx: TestTransaction,
        ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        async fn create(
            &self,
            entity: &MemberActionEntity,
            process: &str,
            tx: TestTransaction,
        ) -> Result<(), DaoError>;
        async fn update(
            &self,
            entity: &MemberActionEntity,
            process: &str,
            tx: TestTransaction,
        ) -> Result<(), DaoError>;
        async fn all(
            &self,
            tx: TestTransaction,
        ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        async fn find_by_id(
            &self,
            id: Uuid,
            tx: TestTransaction,
        ) -> Result<Option<MemberActionEntity>, DaoError>;
        async fn find_by_member_id(
            &self,
            member_id: Uuid,
            tx: TestTransaction,
        ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
    }
}
```

**`TestDeps`-Erweiterung (Z. 824-835):**
```rust
struct TestDeps;
impl RepaymentEntryServiceDeps for TestDeps {
    type Context = MockContext;
    type Transaction = TestTransaction;
    type RepaymentEntryDao = MockTestRepaymentEntryDao;
    type RepaymentPhaseDao = MockTestRepaymentPhaseDao;
    type MemberDao = MockTestMemberDao;
    type MemberActionDao = MockTestMemberActionDao;  // <-- NEU
    type AuditLogDao = MockTestAuditLogDao;
    type PermissionService = MockTestPermissionService;
    type UuidService = StaticUuidService;
    type TransactionDao = MockTestTxDao;
}
```

**`build_service*`-Helper anpassen (Z. 939-968):** Neuer Parameter `action_dao: MockTestMemberActionDao`, neue Konstruktor-Zeile in `RepaymentEntryServiceImpl { ..., member_action_dao: Arc::new(action_dao), ... }`.

**Import-Ergänzung (Z. 517-524):**
```rust
use genossi_dao::member_action::{MemberActionDao, MemberActionEntity};
```

---

### Frage 10: Audit-Chain-Test-Strategie für SC #3

**HIGH confidence** — REST-Endpoints und Filter direkt verifiziert.

**Verifikation:**
- `AuditQueryFilter` (`genossi_dao/src/audit_log.rs:25-33`) hat 6 Felder: `entity_type`, `entity_id`, `user_id`, `action`, `from`, `to`. **KEIN `process`-Filter, kein `transaction_id`-Filter.**
- `GET /api/audit` (`genossi_rest/src/audit_log.rs:92-155`) akzeptiert `AuditQueryParams` mit denselben 6 Feldern.
- `GET /api/audit/verify` (`genossi_rest/src/audit_log.rs:218-256`) lädt ALLE Entries (`get_all_ordered`), läuft `verify_chain`, gibt `VerifyResponseTO { valid: bool, total_entries: usize, broken_links: [...] }` zurück.
- `GET /api/audit/{entity_type}/{entity_id}` (`audit_log.rs:171-206`) filtert auf eine spezifische Entity.
- `build_audit_entries` in `genossi_service_impl/src/audit_log.rs:52-113` generiert pro Aufruf eine **neue `transaction_id`** (Z. 65 `let transaction_id = uuid_fn();`). D.h. 3 `audited_*!`-Aufrufe in Phase 9 erzeugen 3 verschiedene `transaction_id`s — CONTEXT.md D-01 hat das bereits akzeptiert.

**Empfohlene E2E-Assertion-Strategie für SC #3:**

```rust
// 1. /api/audit/verify — Hash-Chain ist valide nach der Cascade.
let verify_resp = client.get(server.url("/api/audit/verify")).send().await.unwrap();
let verify: VerifyResponseTO = verify_resp.json().await.unwrap();
assert!(verify.valid, "audit chain broken: {:?}", verify.broken_links);

// 2. Audit-Entries pro Entity holen + assert process-string.
// member_action — 1 neuer Entry per audit_field (action_type, date, shares_change, member_id, comment)
let ma_resp = client.get(server.url(&format!("/api/audit/member_action/{}", action_id))).send().await.unwrap();
let ma_entries: Vec<AuditLogEntryTO> = ma_resp.json().await.unwrap();
assert!(ma_entries.iter().all(|e| e.process == "repayment-entry.mark-paid-out"));
assert!(ma_entries.iter().any(|e| e.field_name == "shares_change" && e.new_value.as_deref() == Some("-3")));

// 3. RepaymentEntry-Audit: 1 Entry für status-change (status=Open→PaidOut).
let re_resp = client.get(server.url(&format!("/api/audit/repayment_entry/{}", entry_id))).send().await.unwrap();
let re_entries: Vec<AuditLogEntryTO> = re_resp.json().await.unwrap();
let last_status_change = re_entries.iter().rev().find(|e| e.field_name == "status").unwrap();
assert_eq!(last_status_change.process, "repayment-entry.mark-paid-out");
assert_eq!(last_status_change.new_value.as_deref(), Some("PaidOut"));

// 4. Member-Audit: 2 neue Entries (current_shares + action_count) mit process="repayment-entry.mark-paid-out".
let m_resp = client.get(server.url(&format!("/api/audit/member/{}", member_id))).send().await.unwrap();
let m_entries: Vec<AuditLogEntryTO> = m_resp.json().await.unwrap();
let cascade_entries: Vec<_> = m_entries.iter().filter(|e| e.process == "repayment-entry.mark-paid-out").collect();
let field_names: HashSet<&str> = cascade_entries.iter().map(|e| e.field_name.as_str()).collect();
assert!(field_names.contains("current_shares"));
assert!(field_names.contains("action_count"));
```

**Optional (stärkere Assertion):** Chronologische Ordnung der 3 `transaction_id`-Gruppen prüfen (`MemberAction.transaction_id ≠ Member.transaction_id ≠ RepaymentEntry.transaction_id`, alle Timestamps streng monoton steigend). Das beweist „atomare Sequenz im selben Geschäftsvorfall" trotz fehlender UUID-Gleichheit.

**Pitfall:** `entity_type`-String für `RepaymentEntry` ist `"repayment_entry"` (Underscore, nicht Bindestrich) — analog Phase 7 D-05 `"repayment_phase"`. Verifiziert in `genossi_dao/src/repayment_entry.rs` (Phase-8-Plan-01-Lektion) und konsistent mit Phase 7 Plan 5 Audit-Endpoint-Pfad-Konvention.

---

## Cascade Implementation Walkthrough

```rust
// genossi_service_impl/src/repayment_entry.rs (NEU am Ende des `impl<Deps> RepaymentEntryService for RepaymentEntryServiceImpl<Deps>`-Blocks)

const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT: &str = "repayment-entry.mark-paid-out";

async fn mark_paid_out(
    &self,
    id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<RepaymentEntry, ServiceError> {
    // ===== Schritt 1: Tx beginnen =====
    let tx = self.transaction_dao.use_transaction(None).await?;

    // ===== Schritt 2: Permission + user_id =====
    let user_id = self
        .permission_service
        .current_user_id(context.clone())
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service
        .check_permission(ADMIN_PRIVILEGE, context)
        .await?;

    // ===== Schritt 3: Entry laden + Status-Guards =====
    let entry = self
        .repayment_entry_dao
        .find_by_id(id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    if entry.status == RepaymentEntryStatus::PaidOut {
        return Err(ServiceError::Conflict(Arc::from(
            "Entry already paid out (final per PAYO-04)",
        )));
    }
    if !matches!(
        entry.status,
        RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted
    ) {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Entry status is '{}', expected Open or Contacted",
            entry.status.as_str()
        ))));
    }

    // ===== Schritt 4: Phase laden + Status-Guard (Defense-in-Depth) =====
    let phase = self
        .repayment_phase_dao
        .find_by_id(entry.phase_id, tx.clone())
        .await?
        .ok_or_else(|| {
            ServiceError::InternalError(Arc::from(format!(
                "Entry {} references missing Phase {}",
                id, entry.phase_id
            )))
        })?;
    if phase.status != RepaymentPhaseStatus::Open {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Phase status is '{}', expected 'Open'",
            phase.status.as_str()
        ))));
    }

    // ===== Schritt 5: Member laden + PAYO-03-Validation =====
    let member = self
        .member_dao
        .find_by_id(entry.member_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(entry.member_id))?;
    if member.current_shares < entry.share_count_to_pay_out {
        return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
            field: Arc::from("share_count_to_pay_out"),
            message: Arc::from(format!(
                "Member.current_shares ({}) is less than entry.share_count_to_pay_out ({})",
                member.current_shares, entry.share_count_to_pay_out
            )),
        }]));
    }

    // ===== Schritt 6: audited_create! MemberAction::Verkauf =====
    let now = time::OffsetDateTime::now_utc();
    let today: time::Date = now.date();
    let created = time::PrimitiveDateTime::new(now.date(), now.time());
    let comment_str = format!("Anteils-Rückzahlung Phase {}", phase.fiscal_year);
    let action_entity = genossi_dao::member_action::MemberActionEntity {
        id: self.uuid_service.new_v4().await,
        member_id: entry.member_id,
        action_type: genossi_dao::member_action::ActionType::Verkauf,
        date: today,
        shares_change: -entry.share_count_to_pay_out, // negative, validate_action would pass
        transfer_member_id: None,
        effective_date: None,
        comment: Some(Arc::from(comment_str)),
        created,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    crate::audited_create!(
        self,
        self.member_action_dao,
        &action_entity,
        REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT,
        &user_id,
        tx
    );

    // ===== Schritt 7: audited_update! Member =====
    let mut member_new = member.clone();
    member_new.current_shares = member.current_shares - entry.share_count_to_pay_out;
    member_new.action_count = member.action_count + 1;
    crate::audited_update!(
        self,
        self.member_dao,
        entry.member_id,
        &member_new,
        REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT,
        &user_id,
        tx
    );

    // ===== Schritt 8: Re-Read Member (BL-01-Pattern, 500 on None) =====
    let _member_refreshed = self
        .member_dao
        .find_by_id(entry.member_id, tx.clone())
        .await?
        .ok_or_else(|| {
            ServiceError::InternalError(Arc::from(format!(
                "Re-Read after audited_update! returned None for Member {} — same-tx invariant violated",
                entry.member_id
            )))
        })?;

    // ===== Schritt 9: audited_update! RepaymentEntry (status=PaidOut) =====
    let mut entry_new = entry.clone();
    entry_new.status = RepaymentEntryStatus::PaidOut;
    crate::audited_update!(
        self,
        self.repayment_entry_dao,
        id,
        &entry_new,
        REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT,
        &user_id,
        tx
    );

    // ===== Schritt 10: Re-Read RepaymentEntry (für Response.version) =====
    let entry_refreshed = self
        .repayment_entry_dao
        .find_by_id(id, tx.clone())
        .await?
        .ok_or_else(|| {
            ServiceError::InternalError(Arc::from(format!(
                "Re-Read after audited_update! returned None for RepaymentEntry {} — same-tx invariant violated",
                id
            )))
        })?;

    // ===== Schritt 11: recalc_migrated (D-10 Option (a)) =====
    let actions_for_member = self
        .member_action_dao
        .find_by_member_id(entry.member_id, tx.clone())
        .await?;
    let member_after_update = self
        .member_dao
        .find_by_id(entry.member_id, tx.clone())
        .await?
        .ok_or_else(|| {
            ServiceError::InternalError(Arc::from(
                "Member vanished before recalc_migrated — Tx-isolation broken",
            ))
        })?;
    let mig_status =
        crate::member_action::compute_migration_status(&member_after_update, &actions_for_member);
    let migrated = mig_status.status == genossi_service::member_action::MigrationState::Migrated;
    self.member_dao
        .update_migrated(entry.member_id, migrated, tx.clone())
        .await?;

    // ===== Schritt 12: commit =====
    self.transaction_dao.commit(tx).await?;
    Ok(RepaymentEntry::from(&entry_refreshed))
}
```

**Reihenfolge-Begründung (D-09):** MemberAction → Member → RepaymentEntry ist chronologisch lesbar im Audit-Log (Aktion → Effekt-auf-Mitglied → Effekt-auf-Eintrag). Planner darf abweichen, aber Status-Quo ist die empfohlene Reihenfolge.

**Cascade ist atomisch:** ein einziger `commit` am Ende. Falls IRGENDEIN Schritt mit `?` fehlschlägt, wird `tx` gedropped → `TransactionImpl::rollback` läuft implizit via Drop-Order von sqlx (siehe `genossi_dao_impl_sqlite/src/transaction.rs:47-57`). Alle drei `audited_*!`-Schreibvorgänge plus Audit-Entries werden zurückgerollt.

---

## REST Handler Sketch

```rust
// genossi_rest/src/repayment_entry.rs (Erweiterung)

#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "/{id}/mark-paid-out",
    params(("id" = Uuid, Path, description = "RepaymentEntry ID")),
    responses(
        (status = 200,
         description = "Entry marked as PaidOut. Cascade: MemberAction::Verkauf created, \
                       Member.current_shares reduced by share_count_to_pay_out, \
                       Member.action_count incremented. All three writes in a single \
                       SQLite transaction with shared audit-process \
                       'repayment-entry.mark-paid-out'. Final per PAYO-04.",
         body = RepaymentEntryTO),
        (status = 400,
         description = "Validation Error (PAYO-03: Member.current_shares < entry.share_count_to_pay_out)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Entry not found or soft-deleted (before re-read)"),
        (status = 409,
         description = "Conflict: entry status is not Open/Contacted (PAYO-04 final on PaidOut), \
                       OR phase status is not Open (defense-in-depth), \
                       OR concurrent race produced version mismatch on the entry update."),
        (status = 500,
         description = "Internal consistency error: Re-Read after audited_update! returned None \
                       (Phase-8 BL-01 pattern — same-Tx invariant broken)."),
    ),
)]
pub async fn mark_paid_out<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let entry = rest_state
                .repayment_entry_service()
                .mark_paid_out(id, auth)
                .await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}

// In generate_route(), ans Ende anhängen:
.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))

// In ApiDoc::paths(...), `mark_paid_out` zur Liste hinzufügen.
```

**Status-Code-Mapping** wird durch das **globale** `From<ServiceError> for RestError` (`genossi_rest/src/lib.rs:97-113`) abgedeckt:
- `ServiceError::ValidationError(_)` → `RestError::BadRequest(...)` → HTTP 400 ✓
- `ServiceError::EntityNotFound(_)` → `RestError::NotFound` → HTTP 404 ✓
- `ServiceError::Conflict(s)` → `RestError::Conflict(s)` → HTTP 409 ✓
- `ServiceError::InternalError(_)` → Catch-all → `RestError::InternalError(...)` → HTTP 500 ✓
- `ServiceError::PermissionDenied` → `RestError::Unauthorized` → HTTP 401 ✓

**Kein lokaler `map_*_error`-Override nötig.** Konsistent mit Phase 7/8 (D-04 RepaymentEntry-REST-Layer).

---

## Testing Strategy

### Unit-Test-Scaffolding

**Neue Mock-Erweiterung** in `genossi_service_impl/src/repayment_entry.rs` (Tests-Modul, nach Z. 720):
- `MockTestMemberActionDao` (siehe Frage 9 für volles Code-Beispiel).
- `TestDeps` erweitern um `type MemberActionDao = MockTestMemberActionDao`.
- `build_service*`-Helper um neuen Parameter erweitern.

**Neue Unit-Tests (mindestens 6):**
1. `test_mark_paid_out_happy_path` — Entry Open, Phase Open, Member ausreichend Shares → Cascade läuft, Status=PaidOut, audited_create und 2× audited_update aufgerufen.
2. `test_mark_paid_out_rejects_paid_out_entry` — Entry bereits PaidOut → 409 mit Message-Substring „already paid out".
3. `test_mark_paid_out_rejects_when_phase_not_open` — Phase Preparation oder Closed → 409 mit „Phase status".
4. `test_mark_paid_out_rejects_when_current_shares_insufficient` — PAYO-03: member.current_shares=2, entry.share_count_to_pay_out=5 → ValidationError mit Field „share_count_to_pay_out", Message enthält beide Werte.
5. `test_mark_paid_out_rereads_none_yields_internal_error` — Mock so setzen, dass `find_by_id` nach `audited_update!` `Ok(None)` returnt → `ServiceError::InternalError` (NICHT EntityNotFound) (BL-01-Pattern, Test-Vorlage in REVIEW.md Z. 78).
6. `test_mark_paid_out_member_action_has_correct_fields` — Capture-Closure auf `audited_create.expect_create()` mit `withf(|action_entity, ...| { action_entity.action_type == ActionType::Verkauf && action_entity.shares_change == -3 && action_entity.comment.as_deref() == Some("Anteils-Rückzahlung Phase 2026") })`.

### E2E-Tests (mindestens 5)

Alle in `genossi_bin/tests/e2e_tests.rs`, nutzen die bestehende `create_member_with_exit_date` + `create_open_repayment_phase`-Helper aus Phase 8 (E2E-Test-Set 08-06).

1. **`test_mark_paid_out_happy_path_cascade`** — Setup: Member mit `current_shares=10`, Open-Phase mit Auto-Entry `share_count_to_pay_out=3`. POST `/api/repayment-entry/{id}/mark-paid-out` → 200 + RepaymentEntryTO mit `status=PaidOut`. Verify:
   - GET Entry → status=PaidOut.
   - GET Member → current_shares=7, action_count=member.action_count_before+1.
   - GET `/api/member-action?member_id=...` (oder via member-detail-Endpoint) → neue Action mit type=Verkauf, shares_change=-3, comment startsWith "Anteils-Rückzahlung Phase".
   - GET `/api/audit/verify` → `valid: true`.
2. **`test_mark_paid_out_validates_insufficient_shares`** — PAYO-03: Member current_shares=2, Entry share_count_to_pay_out=5 (manuelle Anlage, weil Auto-Fill nicht mehr als current_shares setzt). POST → 400 mit Body enthält „share_count_to_pay_out".
3. **`test_mark_paid_out_blocks_double_payout`** — PAYO-04: erster POST → 200. Zweiter POST auf gleichem Entry → 409 mit „already paid out" oder „PaidOut".
4. **`test_mark_paid_out_blocks_when_phase_closed`** — Setup-Trick: Phase im Status=Preparation lassen, Entry manuell mit dem direkten DAO einfügen wäre best (geht aber nur über Workaround); ALTERNATIV: Open-Phase + Entry, dann via PUT Phase nach Closed (PHAS-03: blockiert wenn pending Entries → wir müssen den Entry vorher PaidOut machen — aber dann ist er final, blockt eh nicht). **Empfehlung Planner:** Test über "Phase im Vorbereitung" und manual entry insertion ist zu komplex; stattdessen ein einfacherer Unit-Test (Test 3 oben) deckt den Phase-Status-Guard ab. E2E-Test 4 kann gestrichen werden, ODER der Plan baut einen Test, der Phase im Status `Preparation` einrichtet UND Entry-Existenz durch direkten DB-Insert in der Test-Setup-Phase erzwingt.
5. **`test_mark_paid_out_race_one_succeeds_one_conflicts`** — exakt das Pattern aus `e2e_tests.rs:8783-8821`, angepasst für mark-paid-out + URL. Erwartung: `sorted_statuses == [200, 409]`.

### Audit-Chain-Assertion

Wie in Frage 10 beschrieben. Plan empfiehlt:
- `/api/audit/verify` → `valid: true` als baseline.
- `/api/audit/member/{member_id}` mit Filter `process == "repayment-entry.mark-paid-out"` → enthält genau `current_shares` und `action_count` Field-Names.
- `/api/audit/member_action/{action_id}` → alle Entries haben `process == "repayment-entry.mark-paid-out"`, `field_name == "shares_change"`-Entry hat `new_value == "-3"`.

**Pitfall:** `AuditQueryFilter` hat keinen `process`-Filter — Plan filtert im Client clientseitig nach `process`-Field aus dem `AuditLogEntryTO`.

---

## File-by-File Change Manifest

Geordnet nach Abhängigkeit (Trait-Erweiterung → Impl → REST → Wiring → Tests):

| # | Datei | Art | Beschreibung |
|---|-------|-----|-------------|
| 1 | `genossi_service_impl/src/member_action.rs:32` | MOD (1 LOC) | `pub(crate) fn compute_migration_status` → `pub fn compute_migration_status` |
| 2 | `genossi_service/src/repayment_entry.rs` | MOD (~12 LOC) | `RepaymentEntryService`-Trait um `async fn mark_paid_out(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<RepaymentEntry, ServiceError>` erweitern. Doc-Comment mit Verweis auf PAYO-01..04. `tests::test_mock_repayment_entry_service_compiles` um `let _ = mock.expect_mark_paid_out();` ergänzen. |
| 3 | `genossi_service_impl/src/repayment_entry.rs:48-58` | MOD (1 Zeile) | `gen_service_impl!`-Block: neue Zeile `MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,` einfügen. |
| 4 | `genossi_service_impl/src/repayment_entry.rs` (imports) | MOD | `use genossi_dao::member_action::{ActionType, MemberActionDao, MemberActionEntity};` ergänzen. |
| 5 | `genossi_service_impl/src/repayment_entry.rs` (consts) | MOD | `const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT: &str = "repayment-entry.mark-paid-out";` ergänzen. |
| 6 | `genossi_service_impl/src/repayment_entry.rs` (impl-Block) | MOD (~110 LOC) | `mark_paid_out`-Methode wie im Cascade-Walkthrough. |
| 7 | `genossi_service_impl/src/repayment_entry.rs` (Tests-Modul) | MOD (~60 LOC) | `MockTestMemberActionDao`-Definition; `TestDeps::MemberActionDao`; `build_service*`-Helper-Param. |
| 8 | `genossi_service_impl/src/repayment_entry.rs` (Tests) | MOD (~250 LOC) | 6 neue Unit-Tests (siehe Testing Strategy). |
| 9 | `genossi_rest/src/repayment_entry.rs` (Handler) | MOD (~40 LOC) | `mark_paid_out` Axum-Handler + `#[utoipa::path]`. |
| 10 | `genossi_rest/src/repayment_entry.rs` (`generate_route`) | MOD (1 Zeile) | `.route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))` |
| 11 | `genossi_rest/src/repayment_entry.rs` (ApiDoc) | MOD (1 Zeile) | `mark_paid_out` zur `paths(...)`-Liste. |
| 12 | `genossi_bin/src/lib.rs:216-237` | MOD (1 Zeile) | `RepaymentEntryServiceDependencies::MemberActionDao = MemberActionDao;` (`type MemberActionDao = MemberActionDao;`-Zeile einfügen). |
| 13 | `genossi_bin/src/lib.rs:765-775` | MOD (1 Zeile) | `RepaymentEntryServiceImpl { ..., member_action_dao: member_action_dao.clone(), ... }` — `member_action_dao` ist bereits oben (Z. 563) als `Arc::new(MemberActionDao::new(...))` definiert und an viele Services geteilt; einfacher `.clone()`. |
| 14 | `genossi_bin/tests/e2e_tests.rs` | MOD (~300 LOC) | 5 neue E2E-Tests (siehe Testing Strategy). Reuse `create_member_with_exit_date` + `create_open_repayment_phase` aus 08-06. |
| 15 | `.planning/REQUIREMENTS.md` | MOD | PAYO-01..04 auf `[x]` setzen nach Phase-9-Verification (NICHT in der Implementation; erst nach `/gsd-verify-phase 9`). |

**Keine neuen Dateien.** Keine Migrations. Keine neuen TOs (Response = `RepaymentEntryTO`).

---

## Pitfalls / Landmines

1. **`recalc_migrated` braucht `member` POST-Update + `actions` POST-MemberAction-Create.** Die Reihenfolge muss sein: (a) audited_create MemberAction, (b) audited_update Member, (c) [optional Re-Read Member], (d) load actions + load member, (e) compute_migration_status, (f) update_migrated. Wenn `recalc_migrated` VOR Schritt (b) läuft, ist member.current_shares noch alt → Status wäre fälschlich „Pending" statt „Migrated". (Anker: `MemberServiceImpl::update` Z. 341 ruft `recalc_migrated` NACH `audited_update!`.)
2. **`compute_migration_status` hat die Off-by-One-Konvention `expected_action_count = member.action_count + 1`** (`member_action.rs:52`). Das ist eine **bewusste** Tracking-Convention (vermutlich für den initialen Eintritt-Action-Stub). Plan darf das NICHT „korrigieren" — der `recalc_migrated`-Wrapper relies auf das exakte Verhalten.
3. **`fiscal_year`-String-Substitution:** `phase.fiscal_year` ist `i32`. `format!("Anteils-Rückzahlung Phase {}", phase.fiscal_year)` produziert z.B. `"Anteils-Rückzahlung Phase 2026"`. Wenn die Verbands-Realität später ein anderes Format will (z.B. Geschäftsjahres-Range `"2025/2026"`), ist das ein deferred-Backlog-Item.
4. **Re-Read-`None` MUSS `InternalError` → 500 sein**, NICHT `EntityNotFound` → 404 (siehe REVIEW.md BL-01 für Phase 8). Das ist im Cascade-Walkthrough oben berücksichtigt.
5. **Phase-`find_by_id`-Failure ist InternalError, nicht NotFound.** Wenn Entry existiert aber Phase nicht (referentielle Inkonsistenz), ist das ein DB-Bug, NICHT „Phase wurde gelöscht". Plan sollte das als `InternalError` mappen (siehe Cascade Walkthrough Schritt 4).
6. **mockall-`Sequence` ist nötig für Tests mit Re-Read,** weil pro Iteration mehrere `find_by_id`-Aufrufe in unterschiedlicher Reihenfolge anders antworten müssen (pre-Update, audit-macro-internal, post-Update Re-Read). Pattern: siehe `test_update_entry_status_open_to_contacted_succeeds` (`repayment_entry.rs:1294-1360`) — exakt 1:1 für Phase-9-Unit-Tests übernehmen.
7. **`MemberActionDao` ist Auto-Mock-bar** mit `#[automock]` (Annotation in `genossi_dao/src/member_action.rs:99`), aber der Test-Modul-Stil in Phase 8 nutzt hand-rolled `mockall::mock!{}`-Mocks. Konsistenz beachten — neuer Mock ist auch hand-rolled (siehe Frage 9).
8. **OpenAPI-Body-Schema:** `mark_paid_out` hat KEIN Request-Body. In `#[utoipa::path]` darf `request_body = ...` NICHT spezifiziert sein (sonst meckert utoipa). Beispiel-Anker: `open_repayment_phase` in `repayment_phase.rs:235-268` hat ebenfalls keinen `request_body`.
9. **Audit-Chain-Verify ist O(n) über ALLE Entries** — `verify_chain` lädt `get_all_ordered` und iteriert linear. Bei wachsendem Audit-Log werden E2E-Tests langsamer. Aktuell akzeptabel.
10. **Phase-Status-Guard-Test (E2E #4) ist schwer zu setzen** ohne direkte DB-Manipulation. Planner sollte das als Unit-Test (mit Mock-Phase im Preparation-Status) abdecken und auf den E2E-Test verzichten, ODER eine `repayment_phase`-PUT-Sequenz nutzen, die Phase zurückbiegt. Empfehlung: **Unit-Test reicht**; E2E-Liste auf 4 Tests reduzieren (1, 2, 3, 5).
11. **Race-Test (E2E #5) ist eventuell flaky in CI** wenn busy_timeout zu kurz oder Tasks zu schnell starten. Mitigation: vor dem `tokio::join!` ein `tokio::time::sleep(Duration::from_millis(1)).await` einbauen — oder den Test als `#[ignore]` markieren falls CI-Stabilität ein Problem wird. **Nicht** als `#[ignore]` empfehlen, solange er erstmal funktioniert.
12. **`#[automock]`-Regeneration auf `RepaymentEntryService`** — die Compile-Test-Suite (`test_mock_repayment_entry_service_compiles`, Z. 272-283) MUSS um `let _ = mock.expect_mark_paid_out();` erweitert werden, sonst keine Regressions-Sicherung der Auto-Mock-Generation für die neue Methode.
13. **`current_user_id().unwrap_or_else(|| "SYSTEM".to_string())`-Pattern** ist konsistent in Phase 8 — Planner muss das identisch übernehmen. „SYSTEM" als String taucht im Audit-Log auf, ist semantisch korrekt für „Tx wurde von einem System-Trigger ausgelöst" (was bei mark_paid_out aber NIE der Fall ist, weil Permission-Check vorher läuft).
14. **`assume_utc().format(Iso8601)` für Datetime-Felder** — `RepaymentPhaseEntity.audit_fields` hat eine `tracing::error!` + Sentinel-Fallback für Format-Fehler (`repayment_phase.rs:72-83`). Phase 9 berührt das nicht direkt (MemberAction.date ist `time::Date`, nicht `PrimitiveDateTime`), aber der Pattern sollte respektiert werden falls Plan sentinel-Fallbacks ergänzt.

---

## References

### CONTEXT.md & Roadmap
- `.planning/phases/09-auszahlungs-buchung-atomisch-auditiert/09-CONTEXT.md` (gesamtes Dokument; D-01..D-14)
- `.planning/REQUIREMENTS.md:29-32` (PAYO-01..04)
- `.planning/ROADMAP.md:97-108` (Phase 9 Goal + 5 SC)

### Cascade-Owner-Anker (Phase 7/8)
- `genossi_service/src/repayment_entry.rs:120-178` (Trait-Definition + `#[automock]`)
- `genossi_service_impl/src/repayment_entry.rs:48-58` (gen_service_impl! mit 7 Deps)
- `genossi_service_impl/src/repayment_entry.rs:169-291` (`update_repayment_entry` — Re-Read-Anker Z. 265-291)
- `genossi_service_impl/src/repayment_entry.rs:379-512` (`batch_toggle_status` — Multi-Step-Cascade in Tx, Re-Read pro Iteration)
- `genossi_service_impl/src/repayment_entry.rs:556-720` (hand-rolled Test-Mocks für Entry/Phase/Member/AuditLog/Permission)

### MemberAction-Anker
- `genossi_dao/src/member_action.rs:8-50` (`ActionType::Verkauf`)
- `genossi_dao/src/member_action.rs:53-65` (`MemberActionEntity` — `date: time::Date`, `shares_change: i32`)
- `genossi_dao/src/member_action.rs:67-97` (`Auditable`-Impl mit 7 audit_fields)
- `genossi_dao/src/member_action.rs:99-155` (`MemberActionDao`-Trait mit `find_by_member_id`-Default-Impl)
- `genossi_service_impl/src/member_action.rs:32-69` (`compute_migration_status`, **`pub(crate)` → `pub` in Phase 9**)
- `genossi_service_impl/src/member_action.rs:91-97` (`validate_action` für Verkauf — `shares_change < 0`)
- `genossi_service_impl/src/member_action.rs:174-225` (`recalc_dates` und `recalc_migrated`-Helper als Vorlage)
- `genossi_service_impl/src/member_action.rs:284-352` (`MemberActionServiceImpl::create` — Pattern für audited_create + recalc; Phase 9 bypasst Service, ruft Macros direkt)
- `genossi_dao_impl_sqlite/src/member_action.rs:188-225` (DAO UPDATE mit Pre-Exists-Check + version-Bump + Version-Mismatch-ConflictError)

### Member-Anker
- `genossi_dao/src/member.rs:73-100` (`MemberEntity.current_shares: i32`, `action_count: i32`)
- `genossi_dao/src/member.rs:116-122` (`MemberDao::update`-Signatur)
- `genossi_dao/src/member.rs:145-150` (`MemberDao::update_migrated`)
- `genossi_service_impl/src/member.rs:295-352` (`MemberServiceImpl::update` — Re-Read Z. 343-348; `recalc_migrated` AFTER update Z. 341)

### RepaymentPhase-Anker
- `genossi_dao/src/repayment_phase.rs:44-55` (`RepaymentPhaseEntity.fiscal_year: i32`, `status: RepaymentPhaseStatus`)
- `genossi_dao/src/repayment_phase.rs:9-36` (`RepaymentPhaseStatus { Preparation, Open, Closed }`)
- `genossi_rest/src/repayment_phase.rs:247-268` (`open_repayment_phase` — Action-Endpoint-Pattern ohne Body)
- `genossi_rest/src/repayment_phase.rs:337-351` (`generate_route()` mit `.route("/{id}/open", post(...))`)

### Audit-Macros + Hash-Chain
- `genossi_service_impl/src/audit_macros.rs:5-36` (`audited_create!` 6-Arg-Signatur)
- `genossi_service_impl/src/audit_macros.rs:42-80` (`audited_update!` 7-Arg-Signatur; lädt `old` intern)
- `genossi_service_impl/src/audit_log.rs:52-113` (`build_audit_entries` — neue `transaction_id` pro Aufruf, Z. 65)
- `genossi_dao/src/audit_log.rs:8-23` (`AuditLogEntry`-Felder inkl. `process`, `transaction_id`, `prev_hash`, `entry_hash`)
- `genossi_dao/src/audit_log.rs:25-33` (`AuditQueryFilter` — **kein process-, kein transaction_id-Feld**)

### REST + Wiring
- `genossi_rest/src/repayment_entry.rs:302-316` (`generate_route()` — Phase 9 hängt `.route("/{id}/mark-paid-out", ...)` an)
- `genossi_rest/src/repayment_entry.rs:319-337` (ApiDoc — `mark_paid_out` zur paths-Liste)
- `genossi_rest/src/lib.rs:97-113` (globales `From<ServiceError> for RestError` — keine lokalen Mappings nötig)
- `genossi_rest/src/audit_log.rs:82-256` (Audit-REST-Endpoints — `GET /api/audit`, `GET /api/audit/{entity_type}/{id}`, `GET /api/audit/verify`)
- `genossi_bin/src/lib.rs:216-237` (`RepaymentEntryServiceDependencies` — Phase 9 ergänzt `type MemberActionDao = MemberActionDao;`)
- `genossi_bin/src/lib.rs:563` (`let member_action_dao = Arc::new(MemberActionDao::new(pool.clone()))` — bereits gebaut, wird an viele Services geteilt; Phase 9 hängt sich an)
- `genossi_bin/src/lib.rs:765-775` (`RepaymentEntryServiceImpl{}`-Konstruktion — Phase 9 fügt `member_action_dao: member_action_dao.clone(),` als 8. Feld ein)

### SQLite-Tx + Race-Verhalten
- `genossi_bin/src/main.rs:24-26` (`SqlitePool::connect` — kein WAL, keine Custom-Options)
- `genossi_dao_impl_sqlite/src/transaction.rs:8-91` (TransactionImpl + TransactionDaoImpl — BEGIN DEFERRED)
- `genossi_dao_impl_sqlite/src/repayment_entry.rs:128-183` (DAO update mit Pre-Exists-Check + `WHERE version = ?` + ConflictError bei rows_affected==0)
- `genossi_dao_impl_sqlite/src/member_action.rs:188-225` (gleiches Pattern für member_action)

### Test-Pattern
- `genossi_bin/tests/e2e_tests.rs:8783-8821` (`test_helper_token_redeem_race_one_succeeds_one_fails` — exakte tokio::join!-Vorlage)
- `genossi_bin/tests/e2e_tests.rs:9362-9399` (`test_attendance_upsert_race_one_row_two_200ok` — alternative idempotente Variante, NICHT relevant für Phase 9)
- `genossi_service_impl/src/repayment_entry.rs:1294-1360` (`test_update_entry_status_open_to_contacted_succeeds` — Sequence-basiertes Mock-Pattern für Tests mit Re-Read)
- `.planning/phases/08-repaymententry-auto-bef-llung/08-REVIEW.md` BL-01 (`Z. 37-78` — Re-Read-None → InternalError-Pattern + Test-Empfehlung)

### Project Constraints (from CLAUDE.md)
- §"Audit Log System" — Member, MemberAction, MemberDocument, Application sind audited; Phase 9 nutzt 3 bestehende `Auditable`-Impls (MemberAction, Member, RepaymentEntry); KEINE neue Auditable-Impl nötig.
- §"Architecture Overview" — Layered DAO/Service/REST muss eingehalten werden; soft-delete; optimistic locking; Phase 9 folgt allen Patterns.
- §"Important Files" — `genossi_bin/tests/e2e_tests.rs` + `genossi_rest/src/test_server.rs` als E2E-Anker bestätigt.

---

## Confidence Breakdown

| Bereich | Level | Begründung |
|---------|-------|-----------|
| Cascade-Implementation | HIGH | Alle Code-Anker direkt gelesen; Pattern 1:1 aus Phase 8 übernommen |
| Race-Defense | HIGH | DAO-UPDATE-Code + sqlx-Pool-Setup direkt verifiziert; Race-Pfad analytisch durchgespielt |
| `compute_migration_status`-Refactor | HIGH | `pub(crate)`-Visibility, Konsumenten, alle 3 Optionen Größenanalyse |
| Audit-Chain-Test | HIGH | Alle 3 Audit-REST-Endpoints + Filter-Felder + verify-Response direkt gelesen |
| Race-Test-Pattern | HIGH | Exakter Test im Workspace gefunden (`e2e_tests.rs:8783-8821`) |
| TestMemberActionDao-Mock | HIGH | Volles Pattern aus 5 bestehenden Mocks abgeleitet |
| Field-Typen | HIGH | `MemberActionEntity.date: time::Date`, `shares_change: i32`, `phase.fiscal_year: i32` direkt verifiziert |
| Phase-Status-Guard-E2E | MEDIUM | Setup ist umständlich (PHAS-Open vs. manual entry direct DB), Plan darf auf Unit-Test reduzieren |

---

*Research date: 2026-05-31*
*Valid until: 2026-06-30 (Codebase ist stabil; Phase 8 ist abgeschlossen; keine erwarteten Refactorings im Cascade-Owner-Pfad)*

## RESEARCH COMPLETE

- Open questions resolved: 10/10
- File-by-file manifest: 15 files (14 Code/Tests + 1 REQUIREMENTS.md post-verify)
- Pitfalls flagged: 14
