---
phase: 07-repaymentphase-backend-foundation
reviewed: 2026-05-29T20:48:27Z
depth: deep
files_reviewed: 10
files_reviewed_list:
  - migrations/sqlite/20260529190437_create_repayment_phase_table.sql
  - genossi_dao/src/repayment_phase.rs
  - genossi_dao_impl_sqlite/src/repayment_phase.rs
  - genossi_service/src/repayment_phase.rs
  - genossi_service_impl/src/repayment_phase.rs
  - genossi_rest/src/repayment_phase.rs
  - genossi_rest_types/src/lib.rs (RepaymentPhase-Abschnitt, Z. 1143–1260)
  - genossi_bin/src/lib.rs (DI-Wiring-Block)
  - genossi_bin/tests/e2e_tests.rs (Phase-7-Abschnitt, Z. 10554–11006)
  - genossi_service_impl/src/audit_macros.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 7: Code Review Report

**Reviewed:** 2026-05-29T20:48:27Z
**Depth:** deep (Cross-File-Analyse, Call-Chain-Trace, Audit-Disziplin-Prüfung)
**Files Reviewed:** 10
**Status:** issues_found

## Zusammenfassung

Phase 7 liefert das `RepaymentPhase`-Aggregat vollständig im erwarteten 1:1-Assembly-Pattern. Die Implementierung ist insgesamt solide: SQL-Injection-Vektoren existieren nicht (ausschließlich parametergebundene Queries), Audit-Disziplin ist eingehalten (Grep-Gate 0 direkte DAO-Calls außerhalb Audit-Macros), und die State-Machine (Decisions D-04..D-12) ist korrekt implementiert und durch 17 Unit-Tests + 7 E2E-Tests abgesichert.

Drei Befunde mit **WARNING**-Level wurden identifiziert:

1. **Stale Version im Update-Response** (pre-existing, aber in Phase 7 dokumentiert und relevant): PUT, POST /open und POST /close geben die Pre-Update-Version zurück. Die API-Konsumenten können die erhaltene Version nicht direkt für Folgeoperationen nutzen — sie müssen stattdessen ein GET machen. Dieses Verhalten verletzt die übliche Optimistic-Locking-Erwartung und ist in Plan 05 als architektonischer Tech-Debt dokumentiert.

2. **E2E-Test 2 verwendet eine stale Version** (`test_update_repayment_phase_fiscal_year_in_open_returns_conflict`): Der Test nutzt die Version aus der POST-/open-Response (nicht aus einem darauffolgenden GET). Damit ist er de facto kein isolierter D-04/D-07-Test — er testet zufällig, welche der zwei 409-Quellen (Edit-Matrix vs. Version-Mismatch) zuerst feuert. Durch die Body-Assertion `body.contains("fiscal_year")` ist der Test für das aktuelle Verhalten korrekt, aber er ist fragil gegen Reordering der Checks.

3. **Fehlende Testabdeckung für DELETE auf Closed Phase**: D-09 schreibt 409 für jeden Status außer Preparation vor. Es gibt einen Unit-Test und E2E-Test für DELETE auf Open, aber keinen für DELETE auf Closed. Da die Service-Implementierung korrekt `!= Preparation` prüft, ist das Verhalten richtig — aber nicht explizit verifiziert.

---

## Warnings

### WR-01: Stale Version im PUT/open/close-Response verletzt Optimistic-Locking-Vertrag

**File:** `genossi_service_impl/src/repayment_phase.rs:208–219` (update), `264–281` (open), `325–336` (close)

**Issue:** Das Macro `audited_update!` ruft intern `dao.update(&entity)` auf, wobei `dao.update` die DB-Version atomar auf einen neuen UUID bumpt (`let new_version = Uuid::new_v4()`). Der gebumpte Wert wird jedoch nicht zurückpropagiert: weder das Macro noch die Service-Methoden refreshen `entity.version` nach dem Update. Die Service-Methoden geben anschließend `RepaymentPhase::from(&entity)` zurück — mit dem alten `entity.version`.

Konsequenz für API-Konsumenten: Nach einem erfolgreichen `PUT /api/repayment-phase/{id}` enthält die 200-Response die Version **vor** dem Update. Jeder sofortige Folge-PUT mit dieser Version schlägt mit 409 "Version mismatch" fehl. Der Konsument muss stattdessen ein GET machen, um die aktuelle Version zu holen. Dieses Verhalten ist undokumentiert in der OpenAPI-Spec und verletzt die übliche Erwartung an Optimistic-Locking-APIs.

Das Verhalten ist codebase-weit konsistent (Assembly, Member, etc.) und wird in Plan 05 als architektonischer Tech-Debt dokumentiert. Phase 7 führt das Muster nicht neu ein, aber perpetuiert es.

**Fix:**

Option A (minimaler Fix, nur für neue Entität): In jeder Service-Methode nach `audited_update!` ein `find_by_id` aufrufen und die frische Entity für die Response verwenden:

```rust
// Nach audited_update! in update_repayment_phase:
crate::audited_update!(self, self.repayment_phase_dao, id, &entity, ...);

// Refreshe Entity für korrekte Version im Response:
let fresh = self
    .repayment_phase_dao
    .find_by_id(id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(id))?;
self.transaction_dao.commit(tx).await?;
Ok(RepaymentPhase::from(&fresh))
```

Option B (korrekter Fix für die gesamte Codebase): `audited_update!`-Macro gibt die neue Version zurück (`Result<Uuid, _>`) und der Aufrufer setzt `entity.version = new_version` vor dem Return.

---

### WR-02: E2E-Test 2 (`test_update_repayment_phase_fiscal_year_in_open_returns_conflict`) nutzt stale Version

**File:** `genossi_bin/tests/e2e_tests.rs:10828–10829`

**Issue:** Der Test extrahiert die Version aus der `POST /{id}/open`-Response:

```rust
let opened: RepaymentPhaseTO = response.json().await.unwrap();
let version = opened.version.expect("version must be present");
```

Da `/open` die Pre-Open-Version zurückgibt (WR-01), ist `version` hier die Version aus der Create-Response — nicht die aktuelle DB-Version nach dem Open. Der anschließende `PUT` mit `fiscal_year=2027` und dieser stale Version würde in der Service-Implementierung:
1. Edit-Matrix prüfen: `entity.fiscal_year(2026) != update.fiscal_year(2027)` → 409 "Cannot change fiscal_year" — **D-04 feuert zuerst**
2. Version-Check würde **auch** 409 ergeben (V_open != V_create) — aber dieser Pfad wird nie erreicht

Der Test besteht aus dem richtigen Grund (D-04 + `body.contains("fiscal_year")`), aber er testet nicht die Situation "korrekte frische Version + versuchte fiscal_year-Änderung in Open". Ein Refactoring der Service-Check-Reihenfolge könnte diesen Test zum "grün aus falschem Grund" machen.

**Fix:**

Vor dem `PUT` ein GET machen um die frische Version zu holen (analog zu Test 1 Schritt 3):

```rust
// Nach assert_eq!(response.status(), StatusCode::OK):
let get_response = client
    .get(server.url(&format!("/api/repayment-phase/{}", phase_id)))
    .send()
    .await
    .unwrap();
let fresh: RepaymentPhaseTO = get_response.json().await.unwrap();
let version = fresh.version.expect("version must be present");
// Danach PUT mit version verwenden
```

---

### WR-03: Fehlende Testabdeckung für DELETE im Status `Closed` (D-09 Lücke)

**File:** `genossi_service_impl/src/repayment_phase.rs:339–381` / `genossi_bin/tests/e2e_tests.rs`

**Issue:** D-09 schreibt vor: Soft-Delete ist **nur im Status Preparation** erlaubt. Die Service-Implementierung prüft korrekt `entity.status != RepaymentPhaseStatus::Preparation` (deckt Open AND Closed ab). Es existieren:
- Unit-Test `test_delete_repayment_phase_in_open_returns_conflict` (Open → 409, gut)
- Unit-Test `test_delete_repayment_phase_in_preparation_succeeds` (Preparation → OK, gut)
- E2E-Test `test_delete_repayment_phase_in_open_returns_conflict` (E2E für Open, gut)

**Fehlend:** Kein Unit-Test und kein E2E-Test für DELETE im Status `Closed`. Wenn jemand die Bedingung von `!= Preparation` auf `== Open` änderte, würden alle bestehenden Tests weiterhin grün sein, obwohl Closed-Delete dann fälschlicherweise erlaubt wäre.

**Fix:**

Neuer Unit-Test in `genossi_service_impl/src/repayment_phase.rs`:

```rust
#[tokio::test]
async fn test_delete_repayment_phase_in_closed_returns_conflict() {
    let entity = phase_in_status(RepaymentPhaseStatus::Closed);
    let entity_id = entity.id;
    let mut dao = MockTestRepaymentPhaseDao::new();
    dao.expect_find_by_id()
        .returning(move |_, _| Ok(Some(entity.clone())));
    dao.expect_update().times(0).returning(|_, _, _| Ok(()));
    let service = build_service(dao);
    let result = service
        .delete_repayment_phase(entity_id, Authentication::Full)
        .await;
    assert!(matches!(result, Err(ServiceError::Conflict(_))));
}
```

---

## Info

### IN-01: Doppelter `find_by_id`-Aufruf pro Update (dokumentierter WR-04, Erklärung fehlt im Kontext)

**File:** `genossi_service_impl/src/repayment_phase.rs:165–168` und `genossi_service_impl/src/audit_macros.rs:47–50`

**Issue:** Bei jedem `update_repayment_phase`-Aufruf werden zwei `find_by_id`-Calls gemacht: einer in der Service-Methode für den Edit-Matrix/Version-Check, und ein weiterer intern in `audited_update!` für den Audit-Diff. Beide rufen `dump_all` auf (default DAO-Impl lädt alle Rows). Der Kommentar "WR-04" benennt das, erklärt aber nicht explizit, warum der erste Load notwendig ist (Edit-Matrix-Check vor der Mutation).

**Bemerkung:** Die Duplizierung ist korrekt und beabsichtigt — beide Reads sehen denselben DB-Snapshot innerhalb derselben Transaktion (SQLite WAL, serializable). Kein funktionales Problem. Nur Verständlichkeits-Gap für zukünftige Maintainer, da das "Warum" im Kommentar etwas implizit bleibt.

**Fix:** Den WR-04-Kommentar ergänzen: „Beide Reads laufen in derselben Transaktion und sehen identischen Snapshot; kein TOCTOU-Risiko."

---

### IN-02: `RepaymentPhaseDaoImpl.pool` ist öffentlich deklariert

**File:** `genossi_dao_impl_sqlite/src/repayment_phase.rs:62`

**Issue:** Das Feld `pub pool: Arc<SqlitePool>` ist als `pub` deklariert. Damit ist der SQLite-Pool von außen zugänglich und könnten externe Aufrufer Queries direkt am Pool absetzen, ohne das DAO-Interface zu nutzen. Das Pattern ist identisch mit `AssemblyDaoImpl` (pre-existing) — keine Phase-7-Neuerung. Wird im DI-Wiring nie über das Feld, sondern nur über den Konstruktor `::new(pool)` zugegriffen.

**Fix:** `pool`-Feld auf `pub(crate)` oder privat beschränken:

```rust
pub struct RepaymentPhaseDaoImpl {
    pool: Arc<SqlitePool>,  // war: pub pool
}
```

Da AssemblyDaoImpl dasselbe Muster hat, empfiehlt sich eine koordinierte Bereinigung.

---

### IN-03: Fehlende E2E-Tests für `share_value=0` auf `PUT` und für GET auf soft-gelöschte Phase

**File:** `genossi_bin/tests/e2e_tests.rs`

**Issue:** Die Validation-Tests (Test 6 und 7) testen `fiscal_year=1999` und `share_value=0` nur bei `POST`. Die Validation läuft in `validate_phase_fields`, die auch bei `update_repayment_phase` aufgerufen wird — aber dieser Pfad ist nur durch Unit-Tests (Service-Layer) abgedeckt. Kein E2E-Test für:
- `PUT /api/repayment-phase/{id}` mit `share_value=0` → 400
- `GET /api/repayment-phase/{id}` nach Soft-Delete → 404

Beide Verhaltensweisen sind durch Unit-Tests abgedeckt; E2E-Lücken erhöhen das Risiko, dass HTTP-Mapping-Bugs unentdeckt bleiben.

**Fix:** Optionale Ergänzung in `genossi_bin/tests/e2e_tests.rs`:

```rust
// share_value=0 auf PUT
async fn test_validation_share_value_zero_on_put_returns_400() { ... }

// GET auf soft-deleted Phase
async fn test_get_deleted_repayment_phase_returns_404() {
    // create -> delete -> GET -> 404
}
```

---

## Nicht gefundene Defekte (explizit verifiziert)

Die folgenden kritischen Bereiche wurden geprüft und sind **korrekt** implementiert:

- **SQL-Injection**: Alle SQL-Queries in `genossi_dao_impl_sqlite/src/repayment_phase.rs` verwenden ausschließlich `sqlx::query`/`query_as` mit `.bind(...)`. Kein `format!`-konstruiertes SQL im Produktionscode (nur in ParseError-Messages).
- **Audit-Disziplin (T-07-03-01)**: Grep-Gate bestätigt 0 direkte `self.repayment_phase_dao.create()/update()` Aufrufe außerhalb der `audited_*!`-Macro-Expansionen. Alle 5 Schreibwege (create/update/open/close/delete) laufen durch die Macros.
- **Optimistic Locking (D-03/D-07)**: `dao.update` prüft `WHERE version = ?` und gibt `ConflictError` bei `rows_affected == 0`. Service prüft Version zusätzlich vor Mutation. Pre-Exists-Check trennt NotFound (404) von ConflictError (409).
- **State-Machine-Korrektheit (D-04..D-09)**: Edit-Matrix ist korrekt implementiert. Open-→Preparation und Closed-→Open sind verboten. DELETE auf Open gibt 409. Alle Lifecycle-Transitions werden in Unit- und E2E-Tests verifiziert.
- **Validierung (D-11/D-12)**: `validate_phase_fields` prüft `fiscal_year ∈ 2000..=2100` und `share_value > 0`. Wird sowohl bei Create als auch bei Update aufgerufen. ValidationError → 400 BadRequest via globales `From<ServiceError> for RestError`.
- **Permission-Check**: Alle 7 Service-Methoden beginnen mit `check_permission("admin", ...)`. `extract_auth_context` in allen 7 REST-Handlern.
- **Soft-Delete-Filterung (D-10)**: `all()` und `find_by_id()` Default-Impl filtern `deleted.is_none()`. Soft-gelöschte Entities sind nicht sichtbar.
- **ISO8601-Datetime-Handling**: `format_dt` und `parse_datetime` (re-used from `crate::assembly`) sind korrekt eingesetzt. `format_dt`-Closure in `audit_fields()` verwendet tracing::error! + Sentinel statt unwrap (WR-08-Lesson).
- **Guarded i32-Cast**: `i32::try_from(db.fiscal_year)` statt `as i32` verhindert Panic bei korrupten DB-Werten (T-07-02-05).
- **ServiceError-Mapping**: `PermissionDenied → 401`, `EntityNotFound → 404`, `Conflict → 409`, `ValidationError → 400` via globales `From<ServiceError> for RestError` in `genossi_rest/src/lib.rs`.

---

*Reviewed: 2026-05-29T20:48:27Z*
*Reviewer: Claude (gsd-code-reviewer)*
*Depth: deep*
