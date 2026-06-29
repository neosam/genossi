---
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
reviewed: 2026-06-05T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - genossi_service_impl/src/membership_adjust.rs
  - genossi_bin/tests/membership_adjust_e2e.rs
findings:
  blocker: 1
  warning: 4
  total: 5
status: issues_found
supersedes: 16-REVIEW.md (commits 62a9dbda, 87f97841)
gap_closure_scope: plan 16-05 — CR-01 Closed-Phase-Status-Guard
carried_forward:
  - CR-02 (permission-check-ordering) — explicitly out-of-scope for this pass per workflow config
---

# Phase 16: Code Review Report (Replacement nach Gap-Closure 16-05)

**Reviewed:** 2026-06-05T00:00:00Z
**Depth:** standard
**Files Reviewed:** 2 (gap-closure-scoped — full prior review siehe Commit 62a9dbda)
**Status:** issues_found
**Supersedes:** prior 16-REVIEW.md (commits 62a9dbda, 87f97841)
**Focus:** Plan 16-05 (CR-01) — Closed-Phase-Status-Guard in `partial_repayment`
**Carried-forward (still open):** CR-02 (permission-check-ordering) — siehe unten

## Summary

Plan 16-05 schliesst CR-01 aus dem urspruenglichen 16-REVIEW (Closed-Phase-Wiederverwendung in `partial_repayment` ohne Status-Guard). Drei Commits liefern die Loesung:
- `87f97841` Guard in `partial_repayment` (Service-Impl Z. 350-361)
- `5b334cc9` Unit-Test `test_partial_repayment_rejects_closed_phase` (Z. 2097-2158)
- `4ec92404` E2E-Test `test_partial_repayment_closed_phase_returns_409` (membership_adjust_e2e.rs Z. 893-961)

**Verifikations-Resultat fuer CR-01:**

| Pruefkriterium | Ergebnis |
|----------------|----------|
| Guard sitzt VOR `audited_create!`-Calls (Audit-Chain-Hygiene) | OK — Z. 354-361 vor Z. 395 (Phase) und Z. 445 (Entry) |
| Guard sitzt VOR der Auto-Create-Branch (existing-only) | OK — `if let Some(ref existing) = target_phase_existing` |
| `Preparation` und `Open` passieren durch | OK — nur `== RepaymentPhaseStatus::Closed` rejected |
| Conflict-Message enthaelt lowercase `'closed'` + `fiscal_year` | OK — `format!("Phase for fiscal_year {} is closed (D-11.1)", ...)` |
| Unit-Test pinnt `expect_create().times(0)` auf beide DAOs | OK — Z. 2119, Z. 2124 |
| Unit-Test pinnt `expect_find_by_member_and_phase().times(0)` (short-circuit BEFORE sum-check) | OK — Z. 2123 |
| Unit-Test assertiert sowohl `'closed'` als auch `fiscal_year` im Body | OK — Z. 2143-2153 |
| E2E-Sequenz create -> open -> close -> POST partial-repayment -> 409 mit `'closed'` Substring | OK — Z. 905-960 |
| E2E assertiert auch `fiscal_year` im Body | OK — Z. 955-960 |
| Auto-Create-Branch (target_phase_existing is None) bleibt unberuehrt | OK — Guard ist innerhalb `if let Some(ref existing) = target_phase_existing` |
| Kein Orphan-Audit-Eintrag durch rejected Request | OK — Reject vor jedem `audited_create!`-Call; `MockTestAuditLogDao` ohne `expect_create_entries`-Mock im Unit-Test wuerde panic, aber `allow_audit_log()` ist generisch erlaubend; entscheidend ist `entity_dao.expect_create().times(0)`, der per Macro-Vertrag den Audit-Schreibvorgang an die Existenz eines DAO-Writes koppelt |
| Tx-Lifecycle: kein `commit()` nach Reject (Rollback implizit via tx-Drop) | OK — `return Err(...)` vor `self.transaction_dao.commit(tx)` Z. 456 |
| Rust-Idiome | `if let Some(ref existing) = target_phase_existing` + `existing.status == RepaymentPhaseStatus::Closed`: idiomatisch; nutzt borrow, kein clone |

CR-01 ist sauber geschlossen. Ein **neuer BLOCKER** ist beim Re-Review aufgetaucht (Audit-Process-String-Bug in der Auto-Create-Branch — siehe CR-01-new unten). Vier WARNINGS aus dem vorherigen Review bleiben gueltig (PII-Leak, unwrap() im REST-Layer, partial_repayment Re-Read-Konsistenz, fehlende Date-Bounds-E2E-Tests). CR-02 (Permission-Check-Ordering) ist per Auftrag explizit out-of-scope fuer diesen Pass und wird unten unveraendert mitgefuehrt.

## Blocker Issues

### CR-01-new: BLOCKER — Audit-Process-String fuer Auto-Created Phase wird als "repayment-phase.create" geschrieben, NICHT als `PARTIAL_REPAYMENT_PROCESS` — Audit-Spur fuer Plan-16-05-Gap unklar (re-review-Befund)

**Stance:** Tatsachenbefund — vorhandener Code-Pfad, kein hypothetischer.

**File:** `genossi_service_impl/src/membership_adjust.rs:48`, `395-402`
**Issue:**
```rust
const REPAYMENT_PHASE_CREATE_PROCESS: &str = "repayment-phase.create";
...
crate::audited_create!(
    self,
    self.repayment_phase_dao,
    &auto_phase,
    REPAYMENT_PHASE_CREATE_PROCESS,  // <- gleicher Prozess wie regulaerer Create
    &user_id,
    tx
);
```

Re-Review-Befund: Dies ist KEIN durch Plan 16-05 eingefuehrter Bug — die String-Duplizierung war bereits im urspruenglichen 16-REVIEW als **WR-05** klassifiziert. Aber die Re-Read-Pruefung der Guard-Interaktion zeigt, dass das Risiko durch Plan 16-05 NICHT entschaerft wurde: Wenn ein Caller versehentlich eine Closed-Phase fuer `effective.fiscal_year` UND ein passendes vorhandenes Preparation-Phase-Setup hat (was per `find()` durch `fiscal_year`-Match exklusiv ist — also nicht parallel moeglich), ist die Auto-Create-Branch unerreichbar. Korrekt. Damit ist die Auto-Create-Branch nur in der "noch keine Phase fuer fiscal_year"-Situation aktiv — dort ist sie sauber.

**Re-Klassifizierung:** Dies bleibt **WARNING** (nicht BLOCKER) und wird unten als **WR-05** mitgefuehrt. Der Guard selbst ist korrekt; die String-Drift-Sorge ist orthogonal.

→ Es gibt **keinen** neuen Blocker durch Plan 16-05. CR-01 ist geschlossen. Status-Klassifikation wird unten zu `issues_found` mit 1 BLOCKER (CR-02 carried-forward) heruntergesetzt.

## Critical Issues (carried forward)

### CR-02: BLOCKER (carried forward — out of scope per workflow config) — Permission-Check laeuft NACH `current_user_id` und kann Side-Channel zur User-Existence-Pruefung bilden

**Status:** EXPLICITLY OUT OF SCOPE fuer diesen Review-Pass (per `<config>` "CR-02 remains open and is OUT-OF-SCOPE"). Carry-forward zur Sicherstellung, dass der nachfolgende Workflow den Befund nicht verliert.

**File:** `genossi_service_impl/src/membership_adjust.rs:295-304` (auch Z. 87-96 + Z. 177-186)
**Issue (verbatim aus prior REVIEW):**
```rust
let user_id = self
    .permission_service
    .current_user_id(context.clone())  // <- API-Call vor Permission-Check
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());

self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)  // <- erst danach
    .await?;
```
`current_user_id()` wird VOR `check_permission()` aufgerufen. Wenn `current_user_id()` z.B. fuer ungueltige/abgelaufene Sessions einen Fehler ueber `?` durchpropagiert (z.B. `SessionExpired`, `AuthenticationFailed`), wird die Permission-Pruefung nie erreicht — Side-Channel zur Session-Status-Pruefung. Schwerer: bei `Ok(None)` wird `"SYSTEM"` als Actor in den Audit-Log geschrieben, BEVOR `check_permission()` bestaetigt, dass ein Login vorliegt. Wenn das Auth-System spaeter einen Default-Admin-Mock einbaut, entstuende ein Audit-Eintrag unter `"SYSTEM"` ohne nachvollziehbaren Akteur.

**Re-Verifikation gegen aktuellen Code:** Status unveraendert. Plan 16-05 hat den Code-Pfad nicht angefasst. Befund bleibt.

**Fix (siehe prior REVIEW):**
```rust
// 1) Permission-Funnel ZUERST.
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context.clone())
    .await?;

// 2) Erst danach user_id aufloesen.
let user_id = self
    .permission_service
    .current_user_id(context)
    .await?
    .ok_or(ServiceError::Unauthorized)?;  // KEIN "SYSTEM"-Fallback
```
Identisches Refactoring auch in `cancel_membership` und `increase_shares`. Dasselbe Anti-Pattern existiert auch in `repayment_phase.rs` (alle 5 Methoden) — extrahier-faehig in `gen_auth_admin!`-Helper.

## Warnings

### WR-01: Inkonsistente Check-Reihenfolge zwischen partial_repayment und cancel_membership (carried forward, unveraendert nach 16-05)

**File:** `genossi_service_impl/src/membership_adjust.rs:283-335` vs. `99-115`
**Issue:** `cancel_membership` validiert `willensbekundung_date` VOR dem Member-Load (Z. 99-103); `partial_repayment` macht den Member-Load + exit_date-Check ZUERST und validiert das Datum erst nach `validate_partial_repayment_shares` (Z. 330-335). Konsequenz: Bad-Date-Spam triggert pro Anfrage eine SELECT. Plan 16-05 hat dieses Pattern nicht angefasst — Befund bleibt gueltig.

**Fix:** Cheap Pure-Function (`validate_willensbekundung_date`) vor `member_dao.find_by_id` ziehen. Sum-Check + shares-Range-Check brauchen weiterhin `current_shares` (also nach Member-Load) — nur die Date-Validierung ist vorziehbar.

### WR-02: PII-Leak in `Conflict`-Error-Message via `{:?}` auf `Option<Date>` (carried forward, unveraendert nach 16-05)

**File:** `genossi_service_impl/src/membership_adjust.rs:319-322`
**Issue:**
```rust
return Err(ServiceError::Conflict(Arc::from(format!(
    "Cannot start partial repayment for cancelled member (exit_date={:?})",
    member_entity.exit_date
))));
```
`exit_date` ist Mitgliederdaten und wird via `From<ServiceError> for RestError` in den HTTP-409-Body geschrieben. Vergleich `cancel_membership` (Z. 114): `"member already cancelled"` — keine PII.

**Hinweis fuer Konsistenz:** Die neue Closed-Phase-Conflict-Message (Z. 357-359, `"Phase for fiscal_year {} is closed (D-11.1)"`) ist **PII-frei** — `fiscal_year` ist eine Jahreszahl (kein Personendatum). Pattern hier sauber.

**Fix:**
```rust
return Err(ServiceError::Conflict(Arc::from(
    "Cannot start partial repayment for cancelled member"
)));
```

### WR-03: `unwrap()` auf `Response::builder()` koennte panicken — kein Crash-Schutz (carried forward, REST-Layer NICHT in dieser Review-Scope)

**File:** `genossi_rest/src/membership_adjust.rs:73-77, 119-123, 176-180` (nicht im Review-Scope, aber carry-forward zur Vollstaendigkeit)
**Issue:** `.unwrap()` auf `Response::builder()`. Hier mit statischen Headern infallible, aber Pattern-Drift-Risiko fuer Zukunft. Niedrige Prio.

**Fix:** Helper-Function `fn json_response<T: Serialize>(status: u16, body: &T) -> Result<Response, RestError>`.

### WR-04: `partial_repayment` retourniert pre-Read Member ohne re-read nach Tx-Commit (carried forward, unveraendert nach 16-05)

**File:** `genossi_service_impl/src/membership_adjust.rs:456-466`
**Issue:** PART-06/D-16-19 sagt explizit, dass Member nicht mutiert wird — daher korrekt. Aber Pattern-Inkonsistenz zu `repayment_phase.rs` (re-read nach `audited_*!`-Macros).

**Fix:** Niedrige Prio. Wenn die Konvention "Re-Read nach Write" projektweit etabliert ist, sollte sie hier konsistent angewendet werden, auch wenn die Member-Row nicht direkt geschrieben wurde.

### WR-05: Audit-Process-String-Duplizierung statt Cross-Modul-Konstante (carried forward, Plan 16-05 hat das nicht beruehrt)

**File:** `genossi_service_impl/src/membership_adjust.rs:40-48` und `repayment_phase.rs:45`
**Issue:** `const REPAYMENT_PHASE_CREATE_PROCESS: &str = "repayment-phase.create"` ist als Literal in beiden Modulen dupliziert. String-Drift-Risiko: bei Aenderung in einem Modul nicht synchron. Test-Coverage `test_partial_repayment_auto_create_*_share_value` (Z. 1893, Z. 1955) und `test_partial_repayment_auto_create_fallback_default_share_value` (Z. 1955) pinnen den String, aber pro Modul separat — kein Cross-Pinning.

**Fix:** Sync-Test ergaenzen:
```rust
#[test]
fn test_audit_process_string_sync() {
    assert_eq!(
        REPAYMENT_PHASE_CREATE_PROCESS,
        crate::repayment_phase::REPAYMENT_PHASE_PROCESS_CREATE
    );
}
```

## Gap-Closure-Verifikation (Plan 16-05)

### Code (Service-Layer, Z. 350-361)

```rust
// Phase 16.05 / CR-01 — D-11.1-Status-Guard: Eine geschlossene Phase darf
// keinen neuen Entry aufnehmen. Preparation und Open passieren (Preparation =
// Phase-14-Pre-Workflow-Reuse, Open = Standardfall, Auto-Create unten erzeugt
// ohnehin Open). Closed -> HTTP 409 Conflict.
if let Some(ref existing) = target_phase_existing {
    if existing.status == RepaymentPhaseStatus::Closed {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Phase for fiscal_year {} is closed (D-11.1)",
            effective.fiscal_year
        ))));
    }
}
```

**Bewertung:** Minimal, idiomatisch, korrekt positioniert. Drei wichtige Eigenschaften:

1. **Borrow-Pattern (`if let Some(ref existing)`):** Verhindert Move; `target_phase_existing` wird unten in der `match`-Arms weiterverwendet (Z. 366-406). Ohne `ref` waere ein zweiter Clone noetig.

2. **Position zwischen DAO-Read und beiden `audited_create!`-Calls:** Reject vor `repayment_phase_dao.create` (Z. 395) UND vor `repayment_entry_dao.create` (Z. 445). Damit kein Orphan-Audit-Row und kein Partial-Write.

3. **Status-Match nur fuer `Closed`:** `Preparation` und `Open` werden bewusst durchgelassen (siehe Kommentar Z. 351-353). Das ist konsistent mit dem Auto-Create-Default in Z. 383 (`status: RepaymentPhaseStatus::Open`) und dem Pre-Workflow-Reuse von Preparation.

### Unit-Test (Z. 2097-2158)

```rust
#[tokio::test]
async fn test_partial_repayment_rejects_closed_phase() {
    let mut closed_phase = sample_repayment_phase(target_fy, 10000);
    closed_phase.status = RepaymentPhaseStatus::Closed;
    ...
    repayment_phase_dao.expect_create().times(0);            // Z. 2119
    repayment_entry_dao.expect_find_by_member_and_phase().times(0); // Z. 2123
    repayment_entry_dao.expect_create().times(0);            // Z. 2124
    ...
    match result {
        Err(ServiceError::Conflict(msg)) => {
            assert!(text.contains("closed"), ...);
            assert!(text.contains(&target_fy.to_string()), ...);
        }
        ...
    }
}
```

**Bewertung:** Korrekt. Der Test verifiziert:
- `expect_create().times(0)` auf beide DAOs (entry + phase) — keine Audit-Spur
- `expect_find_by_member_and_phase().times(0)` — Guard short-circuited VOR Sum-Check
- Conflict-Body enthaelt `'closed'` UND fiscal_year

### E2E-Test (membership_adjust_e2e.rs Z. 893-961)

Die Sequenz **create -> open -> close -> POST /partial-repayment** ist sauber implementiert:
- Z. 905: `create_repayment_phase(&client, &server, target_fy, 10000)` (Preparation)
- Z. 909-919: POST /api/repayment-phase/{id}/open
- Z. 922-932: POST /api/repayment-phase/{id}/close
- Z. 935-940: POST /api/members/{id}/partial-repayment mit H1-Datum
- Z. 942-948: `assert_eq!(status, StatusCode::CONFLICT)`
- Z. 950-954: `assert!(body_text.contains("closed"))`
- Z. 955-960: `assert!(body_text.contains(&target_fy.to_string()))`

Damit ist der HTTP-409-Mapping-Pfad `ServiceError::Conflict -> RestError::Conflict -> 409` (per `genossi_rest/src/lib.rs:115`) end-to-end nachgewiesen.

### Zusammenfassung Gap-Closure

CR-01 ist **vollstaendig geschlossen**. Code, Unit-Test und E2E-Test decken den Pfad konsistent ab. Keine neuen Bugs durch die Aenderung eingefuehrt.

---

## Carry-Forward-Klassifizierung — Status

| Befund (prior REVIEW) | Status nach 16-05 |
|------|---|
| CR-01 (closed-phase-status-guard) | **CLOSED** durch Plan 16-05 |
| CR-02 (permission-check-ordering) | **STILL OPEN** (out-of-scope dieser Pass) |
| WR-01 (check-reihenfolge) | unveraendert, carry-forward |
| WR-02 (PII-Leak `exit_date={:?}`) | unveraendert, carry-forward |
| WR-03 (`unwrap()` REST-Layer) | unveraendert, carry-forward (REST nicht im Scope) |
| WR-04 (member-re-read-konsistenz) | unveraendert, carry-forward |
| WR-05 (audit-process-string-duplizierung) | unveraendert, carry-forward |
| WR-06 (E2E-Coverage-Luecke) | **Teil-geschlossen:** Closed-Phase-E2E-Test ist jetzt vorhanden (Z. 893-961). Date-Bounds-E2E-Test fuer `partial_repayment` und Out-of-Bound-shares-E2E-Test bleiben ausstehend. |

---

_Reviewed: 2026-06-05T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Scope: Plan 16-05 gap-closure (CR-01) + carry-forward CR-02_
