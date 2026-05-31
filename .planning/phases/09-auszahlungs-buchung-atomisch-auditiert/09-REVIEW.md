---
phase: 09-auszahlungs-buchung-atomisch-auditiert
reviewed: 2026-05-31T00:00:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
  - genossi_rest/src/repayment_entry.rs
  - genossi_service/src/repayment_entry.rs
  - genossi_service_impl/src/member_action.rs
  - genossi_service_impl/src/repayment_entry.rs
findings:
  critical: 0
  warning: 4
  info: 6
  total: 10
status: issues_found
---

# Phase 9: Code Review Report

**Reviewed:** 2026-05-31
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Die Phase-9-Implementierung des `mark_paid_out`-Cascade ist insgesamt sauber: sie hält strikt die Phase-8-Pattern-Anker (Re-Read, BL-01-InternalError-Mapping, audited_*!-Disziplin, optimistic locking, all-or-nothing in einer Tx). Alle drei `audited_*!`-Aufrufe nutzen denselben `process`-String, die Cascade-Reihenfolge MemberAction → Member → RepaymentEntry passt zu D-09 und gibt eine lesbare Audit-Sequenz. Die Race-Defense funktioniert deterministisch über den DAO-internen `WHERE version = ?`-Check (RESEARCH Frage 1).

Die kritische sicherheits- und korrektheitsrelevante Fragestellung (Audit-Hash-Chain-Integrität bei Cascade-Fehlern, atomare Rollbacks, Race-Verlierer-Mapping auf 409/500) ist sauber gelöst. Es gibt keine **BLOCKER**-Findings.

Vier **WARNING**-Findings adressieren Lücken in der Test-Abdeckung (Contacted-Status-Pfad fehlt, keine Permission-Test für mark_paid_out, keine Cascade-Rollback-Test wenn audited_create! fehlschlägt) sowie eine offene Authorization-Konsequenz (Phase 9 erlaubt einem reinen `admin` ohne `manage_members`, MemberActions zu erzeugen — was bei direkter `POST /members/{id}/actions`-Nutzung blockiert würde).

Sechs **INFO**-Findings dokumentieren stilistische und mikro-optimierende Beobachtungen ohne Korrektheits-Impact.

## Warnings

### WR-01: Fehlender Test für Cascade-Trigger aus `status=Contacted`

**File:** `genossi_service_impl/src/repayment_entry.rs:549-557`
**Issue:** Der Status-Whitelist-Check erlaubt sowohl `Open` als auch `Contacted` als Pre-Status für `mark_paid_out` (`!matches!(entry.status, Open | Contacted)` → 409). Alle 6 Unit-Tests in `tests::` (Z. 2521, 2675, 2731, 2795, 2875, 2994) sowie alle 4 E2E-Tests (Z. 11996, 12250, 12368, 12451) prüfen nur den `Open`-Pfad. Es gibt KEINEN Test, der den Happy-Path für eine vorher kontaktierte (`Contacted`) Auszahlung exerziert. Bei einer Regression, die den Whitelist-Check versehentlich auf nur `Open` zurückbiegt, würde keine Test-Suite das fangen — und die Verbands-Realität (Kontakt → Auszahlung) wäre gebrochen.

**Fix:** Neuen Unit-Test `test_mark_paid_out_succeeds_for_contacted_entry` ergänzen (kopiere `test_mark_paid_out_happy_path`, ändere `entry_pre.status` von `RepaymentEntryStatus::Open` auf `RepaymentEntryStatus::Contacted`), bzw. mindestens als Parametrisierung des bestehenden Happy-Path-Tests.

```rust
#[tokio::test]
async fn test_mark_paid_out_succeeds_for_contacted_entry() {
    // Wie test_mark_paid_out_happy_path, aber Status=Contacted.
    let entry_pre = RepaymentEntryEntity {
        // ...
        status: RepaymentEntryStatus::Contacted,
        // ...
    };
    // Erwartung: result.status == PaidOut (Cascade akzeptiert Contacted-Pre-Status).
}
```

---

### WR-02: Kein Permission-Test für `mark_paid_out`

**File:** `genossi_service_impl/src/repayment_entry.rs:531-533`
**Issue:** Die Datei enthält im Test-Modul (`tests::` ab Z. 2014, 2042, 2070, 2093) explizite Permission-Tests für `create_*`, `update_*`, `delete_*`, `batch_toggle_status` (`test_*_requires_admin_privilege`), aber **kein** `test_mark_paid_out_requires_admin_privilege`. Eine künftige Refactoring-Änderung könnte den `check_permission`-Call versehentlich entfernen oder vor dem Status-Guard platzieren (so dass ein nicht-admin User eine Entry-Information leaked — z.B. „existiert"/„existiert nicht" via Fehlermeldung), und die Test-Suite würde es nicht fangen.

**Fix:** Test analog zu `test_create_entry_requires_admin_privilege` (Z. 2014-2040) ergänzen:

```rust
#[tokio::test]
async fn test_mark_paid_out_requires_admin_privilege() {
    let entry_dao = MockTestRepaymentEntryDao::new();
    let phase_dao = MockTestRepaymentPhaseDao::new();
    let member_dao = MockTestMemberDao::new();
    let action_dao = MockTestMemberActionDao::new();
    let service = RepaymentEntryServiceImpl {
        repayment_entry_dao: Arc::new(entry_dao),
        repayment_phase_dao: Arc::new(phase_dao),
        member_dao: Arc::new(member_dao),
        member_action_dao: Arc::new(action_dao),
        audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
        permission_service: Arc::new(make_permission_service_admin_denied()),
        uuid_service: Arc::new(StaticUuidService),
        transaction_dao: Arc::new(setup_mock_tx_dao()),
    };
    let result = service
        .mark_paid_out(Uuid::new_v4(), Authentication::Full)
        .await;
    match result {
        Err(ServiceError::PermissionDenied) => {}
        other => panic!("expected PermissionDenied, got {:?}", other),
    }
}
```

---

### WR-03: Authorization-Diskrepanz — `admin`-only umgeht `manage_members`-Permission für MemberAction-Schreiben

**File:** `genossi_service_impl/src/repayment_entry.rs:531-533`
**Issue:** `mark_paid_out` prüft `ADMIN_PRIVILEGE = "admin"` (Z. 48) und erzeugt damit transitiv eine `MemberAction::Verkauf` über `audited_create!`. Wenn ein User dieselbe Action aber direkt über `POST /api/members/{id}/actions` (→ `MemberActionServiceImpl::create`) erzeugen wollte, würde `MANAGE_MEMBERS_PRIVILEGE = "manage_members"` (genossi_service_impl/src/member_action.rs:19, geprüft Z. 303-305) gefordert. Phase 9 öffnet damit einen Side-Channel: ein User mit `admin`, aber ohne `manage_members`, kann via `mark_paid_out` MemberAction-Writes erzwingen. Je nach RBAC-Konvention der Genossenschaft kann das gewollt sein (Auszahlungs-Verantwortung = Vorstand = admin), aber CONTEXT.md und RESEARCH.md adressieren das Thema nicht. Bei einem strikteren RBAC-Setup (z.B. separater „Kassier"-Rolle, die auszahlen darf aber keine sonstigen Mitglieds-Operationen) ist das ein Privileg-Eskalations-Pfad.

**Fix:** Entscheidung in CONTEXT.md ausdrücklich festhalten („Phase-9 `mark_paid_out` requires only `admin`, NOT `manage_members` — bewusste Verkleinerung der Berechtigungsanforderung weil Auszahlung ein Admin-Vorgang ist") und einen Audit-Log-Marker-Kommentar im Code:

```rust
// SECURITY-NOTE: mark_paid_out requires `admin` (not `manage_members` like
// MemberActionServiceImpl::create). This is intentional per D-XX — the
// payout cascade is a board-level operation, not a general member-management
// operation. Reviewers: any future RBAC tightening should add a separate
// privilege check or escalate this to `admin && manage_members`.
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)
    .await?;
```

Alternativ: zusätzliche `check_permission(MANAGE_MEMBERS_PRIVILEGE, ...)` einbauen, damit die Side-Channel-Lücke geschlossen ist.

---

### WR-04: Kein Test für Cascade-Rollback wenn `audited_create!` fehlschlägt

**File:** `genossi_service_impl/src/repayment_entry.rs:620-627`
**Issue:** Der Cascade besteht aus 3 `audited_*!`-Calls. SC #1 verlangt Atomarität: wenn IRGENDEINER fehlschlägt, MUSS der gesamte Cascade rollen. Es gibt zwar `test_mark_paid_out_rereads_member_none_yields_internal_error` (Z. 2875), der einen Fehler MITTEN im Cascade (Step 8 Re-Read Member) simuliert — aber keinen Test, der den klassischeren Fehler-Pfad prüft: was passiert, wenn `member_action_dao.create()` selbst fehlschlägt (z.B. DAO-Constraint-Violation, DB-Connection-Drop)? Atomarität ist hier verlassen sich auf `?`-Propagation + Tx-Drop, und es gibt keinen Beweis-Test, dass das funktioniert. Bei einer künftigen Refactoring-Änderung, die versehentlich `audited_create!` durch einen Form ersetzt, der den Fehler swallowt (z.B. `let _ = ...await;`), würde nichts brechen.

**Fix:** Negativtest ergänzen, der `expect_create()` auf dem `member_action_dao`-Mock einen DaoError zurückgeben lässt, und assertiert dass:
1. Kein `entry_dao.update()` aufgerufen wird (times(0))
2. Kein `member_dao.update()` aufgerufen wird (times(0))
3. Die Funktion `ServiceError::DataAccess(...)` (oder spezifischeren Mapping-Wert) zurückgibt.

```rust
#[tokio::test]
async fn test_mark_paid_out_rolls_back_when_member_action_create_fails() {
    // Setup wie happy_path, aber action_dao.create() returnt DaoError.
    let mut action_dao = MockTestMemberActionDao::new();
    action_dao
        .expect_create()
        .times(1)
        .returning(|_, _, _| Err(DaoError::DatabaseError(Arc::from("simulated"))));
    // entry_dao.update + member_dao.update muss times(0) sein.
    // Assertion: result.is_err() UND kein Partial-Commit.
}
```

---

## Info

### IN-01: Redundanter Member-Re-Read zwischen Step 8 und Step 11

**File:** `genossi_service_impl/src/repayment_entry.rs:652-662, 701-709`
**Issue:** Step 8 liest den Member via `find_by_id` ein und verwirft das Ergebnis (`_member_refreshed`). Step 11 liest denselben Member im selben Tx zum zweiten Mal für `compute_migration_status`. Das ist eine redundante Round-Trip an die DB innerhalb der gleichen Tx. Step 8 dient dem BL-01-Invariant-Check, aber Step 11 würde den gleichen Fehler entdecken (auch mit `InternalError`-Mapping). Korrektheits-relevant ist es nicht — eine Tx-Isolation kann sich zwischen den beiden Reads nicht ändern.

**Fix:** Step 8 entweder löschen (BL-01-Check ist Defense-in-Depth, Step 11 deckt es ab) ODER Step 11 nutzt das Ergebnis aus Step 8:

```rust
let member_refreshed = self.member_dao.find_by_id(...).await?
    .ok_or_else(|| ServiceError::InternalError(...))?;
// ...
let actions_for_member = self.member_action_dao.find_by_member_id(...).await?;
let mig_status = crate::member_action::compute_migration_status(
    &member_refreshed,
    &actions_for_member,
);
```

Hinweis: Diese Refactoring würde den Unit-Test `test_mark_paid_out_happy_path` beeinflussen (Mock-Sequence: 4 find_by_id auf MemberDao würde auf 3 reduziert). Bewusst nicht „fixen", weil der explizite BL-01-Check zwischen Step 7 und Step 9 dokumentarischen Wert hat.

---

### IN-02: Off-by-One-Invariant um `action_count + 1` unintuitiv, aber Phase-9-kompatibel

**File:** `genossi_service_impl/src/repayment_entry.rs:631-632`, `genossi_service_impl/src/member_action.rs:57`
**Issue:** Die `compute_migration_status`-Konvention rechnet `expected_action_count = member.action_count + 1`. Phase 9 bumpt `member.action_count += 1` VOR `recalc_migrated`, was den Off-by-One auf `expected = (action_count+1) + 1 = OLD + 2` schiebt. Math: solange pre-cascade `actual == expected = OLD+1`, ist post-cascade `actual = OLD+2 == OLD+2 = expected`. Korrektheit bleibt gewahrt.

ABER: `MemberActionServiceImpl::create` (Z. 326-355) bumpt `action_count` NICHT vor `recalc_migrated`. Dort führt jede neue Action zu `actual = OLD+1, expected = OLD+1` initial, dann nach jeder weiteren Action zu `actual = OLD+N, expected = OLD+1` — d.h. ein Member, der einmal Migrated ist, fällt nach jeder weiteren Action auf Pending zurück. Phase 9 hingegen bewahrt Migrated, weil es `action_count` explizit synchronisiert.

Das ist KEIN Phase-9-Bug, sondern eine Inkonsistenz in `MemberActionServiceImpl::create` vs. Phase 9's `mark_paid_out`. Phase 9 verhält sich korrekt im Sinne des Migrated-Invariant.

**Fix:** Doc-Comment in Phase 9's Step 7 ergänzen, dass die `action_count += 1`-Bump bewusst die `MemberActionServiceImpl::create`-Konvention OVERRIDED, weil Phase 9 den Migrated-Status korrekt erhalten will. Optional: Konsistenz-Refactor von `MemberActionServiceImpl::create` zu „`action_count` bumpen vor `recalc_migrated`", aber das wäre eine Cross-Phase-Änderung.

---

### IN-03: Inkonsistenz im error-message-Format zwischen `share_count_to_pay_out`-Validation

**File:** `genossi_service_impl/src/repayment_entry.rs:590-598`
**Issue:** `mark_paid_out`'s ValidationError-Message benutzt das Format `"Member.current_shares (X) is less than entry.share_count_to_pay_out (Y)"` — gleicher Konvention wie `validate_entry_create` (Z. 84-87) `"must be <= member current_shares (X), got Y"`. Die zwei Messages sind aber unterschiedlich strukturiert — ersteres ist Verb-zentrisch („is less than"), zweiteres ist Imperativ („must be <="). Ein Frontend, das diese Messages programmatisch parsed (z.B. für Localization), müsste zwei Pattern matchen.

**Fix:** Beide Messages auf das gleiche Format umstellen, oder beide auf ein strukturiertes Error-Detail-Objekt (z.B. `{"actual": X, "expected_max": Y}`) umstellen. Konsistenz mit BatchFailureResponse (D-08, Plan 05) wäre ein Vorbild.

---

### IN-04: `member_action_dao` ist unused warning-Quelle bei nicht-default Test-Builds

**File:** `genossi_service_impl/src/repayment_entry.rs:1203`
**Issue:** Der `build_service`-Helper (Z. 1193-1209) erzeugt `member_action_dao: Arc::new(MockTestMemberActionDao::new())` ohne Expectations für die meisten bestehenden Tests. Mockall-Default ist „panic bei jedem Aufruf einer nicht-erwarteten Methode", was hier sicher ist (kein Phase-1-7 Test-Pfad ruft `member_action_dao`). Aber: wenn eine künftige Phase eine `member_action_dao`-Call in eine bestehende Methode einfügt, ohne `build_service` anzupassen, würden alle nicht-Phase-9 Tests in der suite mit panic abbrechen — der Compiler hilft hier nicht. Das ist eine subtile coupling-Falle.

**Fix:** Doc-Kommentar in `build_service` ergänzen: „Wenn eine RepaymentEntryService-Methode eine `member_action_dao`-Call hinzufügt, MUSS `build_service` durch `build_service_admin_with_action_dao` ersetzt werden — die Default-`MockTestMemberActionDao` ohne Expectations panickt bei jedem Aufruf."

---

### IN-05: Test `test_mark_paid_out_rereads_member_none_yields_internal_error` lässt `action_count`-Bump-Side-Effect zu

**File:** `genossi_service_impl/src/repayment_entry.rs:2944-2948`
**Issue:** Im BL-01-Test wird `member_dao.expect_update()` mit `times(1)` gesetzt (das ist der `audited_update!` Step 7 für Member), aber `member_dao.expect_update_migrated()` mit `times(0)` — was korrekt ist, weil der Cascade abbrechen sollte BEVOR Step 11 (update_migrated) erreicht wird. Aber: der Test hat KEINEN Negativ-Constraint, dass der `entry_dao.update()` (Step 9) auch nicht aufgerufen wird. Der Code-Flow stellt das durch das `?` an Z. 656 sicher, aber der Test belegt es nicht explizit.

**Fix:** Im Test explizit `entry_dao.expect_update().times(0)` ergänzen (in der Sequence VOR dem Re-Read), um zu beweisen, dass Step 9 nicht erreicht wird wenn Step 8 fehlschlägt.

```rust
entry_dao
    .expect_update()
    .times(0)  // <-- explizit: Step 9 darf nicht laufen
    .returning(|_, _, _| Ok(()));
```

---

### IN-06: `unsafe impl Send for RepaymentEntryServiceDependencies` ohne sichtbare Begründung

**File:** `genossi_bin/src/lib.rs:220-221`
**Issue:** Die ZSTs `RepaymentEntryServiceDependencies` (und alle anderen `*Dependencies` in der Datei) tragen `unsafe impl Send` / `unsafe impl Sync`. Diese Trait-Impls sind für ZSTs (unit-structs) **trivial** — ein leerer Type kann nicht unsafe sein in Bezug auf Send/Sync. Trotzdem sind sie als `unsafe impl` deklariert (was Rust nicht zwingend erfordert). Ein Reviewer, der das zum ersten Mal sieht, fragt sich legitim warum.

Das ist eine Phase-7/8-Konvention, keine Phase-9-Änderung. Nur als Beobachtung erwähnt.

**Fix:** Doc-Kommentar in der Code-Stelle: „SAFETY: ZST has no fields, so Send + Sync are trivially safe. `unsafe impl` is used because the type appears in generic bounds requiring explicit Send + Sync; the auto-impl is suppressed by the `for<'a>`-style trait projection in `RepaymentEntryServiceDeps`."

---

_Reviewed: 2026-05-31T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer, Opus 4.7 1M)_
_Depth: standard_
