---
phase: 09-auszahlungs-buchung-atomisch-auditiert
plan: 03
subsystem: di-wiring
tags: [di-wiring, configuration, arc-sharing, w-02]

# Dependency graph
requires:
  - phase: 09-auszahlungs-buchung-atomisch-auditiert
    plan: 01
    provides: "MemberActionDao als 8. assoziierter Typ im RepaymentEntryServiceDeps-Trait; RepaymentEntryServiceImpl-Struct mit member_action_dao-Feld; Trait-Methode mark_paid_out"
  - phase: 09-auszahlungs-buchung-atomisch-auditiert
    plan: 02
    provides: "REST-Handler POST /api/repayment-entry/{id}/mark-paid-out (registriert; ohne lauffaehiges Binary nicht testbar)"
  - phase: 08-repaymententry-auto-bef-llung
    plan: 05
    provides: "RepaymentEntryServiceDependencies-Marker-Struct + bestehende 7-Dep-Konstruktor-Stelle in RestStateImpl::new()"
provides:
  - "Lauffaehiges genossi_bin-Binary mit Phase-9-mark_paid_out-Endpoint exposed"
  - "DI-Wiring fuer MemberActionDao an RepaymentEntryServiceImpl (Konsument #6)"
  - "Pattern-Anker: 6-fache Arc-Sharing-Demonstration von MemberActionDao (W-02-konform)"
affects: [09-04-e2e, 09-05-requirements-signoff]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single DAO instance per process (W-02) ueber 6 Konsumenten — Pattern-Vorlage fuer Phase 9+ neue Service-Konsumenten existierender DAOs"
    - "Type-Alias-Wiederverwendung — `type MemberActionDao = MemberActionDao;` zeigt RHS auf die globale Type-Alias-Definition Z. 364"
    - "Reihenfolge Felder im Struct-Literal MUSS gen_service_impl!-Reihenfolge folgen — natuerliche Compile-Time-Mitigation gegen T-09-03-03 Configuration-Drift"

key-files:
  created: []
  modified:
    - "genossi_bin/src/lib.rs (+14 LOC -8 LOC: Type-Alias `type MemberActionDao = MemberActionDao;` ergaenzt in RepaymentEntryServiceDeps-Block; `member_action_dao: member_action_dao.clone(),` als 8. Feld im Konstruktor-Aufruf; Kommentare aktualisiert auf '8 deps + Phase 9 Begruendung')"

key-decisions:
  - "Edit Position 1: MemberActionDao-Type-Alias kommt ZWISCHEN MemberDao und AuditLogDao (Z. 231) — exakt die Reihenfolge wie Plan 09-01 sie im gen_service_impl!-Block gewaehlt hat. Reihenfolge ist Compile-Time-erzwungen via Trait-Definition."
  - "Edit Position 2: member_action_dao.clone() kommt ZWISCHEN member_dao und audit_log_dao im Struct-Literal — exakt die Plan-09-01-Reihenfolge im RepaymentEntryServiceImpl-Struct."
  - "W-02 explizit ueberprueft: nur 1 `let member_action_dao = Arc::new(MemberActionDao::new(pool.clone()))` (Z. 563); 6 `member_action_dao: member_action_dao.clone(),` (Konsumenten: MemberService Z. 568, MemberActionService Z. 577, ValidationService Z. 604, MemberImportService Z. 625, ApplicationService Z. 709, RepaymentEntryService NEW Z. 772)."
  - "Inline-Kommentar-Block ueber dem Konstruktor dokumentiert explizit alle 5 bestehenden Konsumenten mit Zeilennummern — hilft kuenftige Phasen, Konsumenten zu finden ohne Re-Discovery."

patterns-established:
  - "DI-Wiring-Plan = 1-Task atomar: bei Plan-Sequenz die in Plan N einen DAO-Dep ergaenzt, kann Plan N+2 (Wiring in genossi_bin) als minimal-invasiver Single-Task-Plan ausgefuehrt werden — exakt 2 Edits + 1 Commit. Vorlage fuer kuenftige Service-Dep-Erweiterungen."
  - "Kommentar-Annotation-Pattern: Inline-Comment ueber Konstruktor listet alle existierenden Arc-Konsumenten + Zeilennummern auf — Defense gegen Refactor-Drift (z.B. wenn jemand spaeter `let member_action_dao = Arc::new(...)` versehentlich nochmal anlegt)."

requirements-completed: []  # PAYO-01..04 werden erst in Plan 09-05 als [x] markiert (per ROADMAP-Konvention nach E2E-Verifikation)

# Metrics
duration: 3min
completed: 2026-05-31
---

# Phase 9 Plan 03: DI-Wiring fuer MemberActionDao Summary

**Workspace-blockierender 2-Zeilen-Fix in `genossi_bin/src/lib.rs`: `type MemberActionDao = MemberActionDao;` im RepaymentEntryServiceDeps-Block + `member_action_dao: member_action_dao.clone(),` im Konstruktor-Aufruf — heilt E0046+E0063 aus Plan 09-01, macht Workspace-Build clean.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-05-31T10:32:26Z
- **Completed:** 2026-05-31T10:35:38Z
- **Tasks:** 1 (T1: DI-Wiring mit 2 Edits in 1 Datei)
- **Files modified:** 1

## Accomplishments

- **`RepaymentEntryServiceDependencies`-Trait-Impl** (Z. 221-234) bekam `type MemberActionDao = MemberActionDao;` als 8. assoziierten Typ — exakt zwischen `type MemberDao` und `type AuditLogDao`. Reihenfolge folgt der gen_service_impl!-Reihenfolge aus Plan 09-01.
- **RepaymentEntryServiceImpl-Konstruktor** (Z. 765-776) bekam `member_action_dao: member_action_dao.clone(),` als 8. Feld — exakt zwischen `member_dao` und `audit_log_dao`. Der bereits in Z. 563 als `Arc::new(MemberActionDao::new(pool.clone()))` instanziierte DAO wird via `Arc::clone()` weitergereicht — Phase 9 ist Konsument #6.
- **Inline-Block-Kommentare** ueber beiden Edit-Punkten aktualisiert: Type-Alias-Block dokumentiert nun "8 deps" + Phase-9-PAYO-01-Begruendung; Konstruktor-Block dokumentiert die 5 anderen Arc-Konsumenten mit Zeilennummern (568/577/604/625/709) als Defense gegen Refactor-Drift.
- **Workspace-Build heilt** (`cargo build --workspace` exits 0; `cargo build --bin genossi` exits 0). Vor diesem Plan war `cargo build -p genossi_bin` mit E0046 (missing type MemberActionDao) + E0063 (missing field member_action_dao) gebrochen.
- **Audit-Disziplin-Grep + W-02-Single-Instance-Gate** weiterhin compliant: exakt 1× `let member_action_dao = Arc::new(MemberActionDao::new(pool.clone()))` (Z. 563), 6× `member_action_dao: member_action_dao.clone(),` (Z. 568, 577, 607, 628, 712, 775 — Phase 9 ist die 6.).
- **Keine Test-Regressionen:** `cargo test -p genossi_service_impl --lib repayment_entry` 29/29 grün, workspace-lib-Tests 0 failed.

## Task Commits

1. **Task 1: Wire MemberActionDao in RepaymentEntryServiceDependencies + Konstruktor** — `7c1e72d` (feat)
   - Edit 1: Type-Alias `type MemberActionDao = MemberActionDao;` (Z. 231) + Kommentar-Update (Z. 211-217)
   - Edit 2: `member_action_dao: member_action_dao.clone(),` (Z. 772) + Kommentar-Update (Z. 764-769)
   - Netto: +14 LOC -8 LOC (1 Datei)

## Files Created/Modified

- `genossi_bin/src/lib.rs` — +14 LOC -8 LOC: 2 Edit-Stellen (Type-Alias-Block + Konstruktor-Aufruf), beide mit erweiterten Inline-Kommentar-Bloecken die Phase-9-Begruendung und Arc-Sharing-Pattern dokumentieren.

## Verification

**Build (workspace + Bin):**

```text
$ cargo build -p genossi_bin 2>&1 | tail -3
warning: `genossi_rest` (lib) generated 2 warnings (run `cargo fix --lib -p genossi_rest` to apply 2 suggestions)
warning: `genossi_bin` (lib) generated 1 warning (run `cargo fix --lib -p genossi_bin` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 59.00s
```

```text
$ cargo build --workspace 2>&1 | tail -2
warning: `genossi_bin` (lib) generated 1 warning (run `cargo fix --lib -p genossi_bin` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s
```

```text
$ cargo build --bin genossi 2>&1 | tail -2
warning: `genossi_bin` (lib) generated 1 warning (run `cargo fix --lib -p genossi_bin` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.46s
```

Build clean. Die 3 Warnings (unused imports in genossi_rest::lib.rs Z. 32, genossi_rest::permission.rs Z. 780, genossi_bin::lib.rs Z. 940 — `genossi_dao::auditable::Auditable`) sind **pre-existing** aus früheren Phasen und NICHT durch Plan 09-03 eingeführt.

**Tests (kein Regress):**

```text
$ cargo test -p genossi_service_impl --lib repayment_entry 2>&1 | tail -3
test repayment_entry::tests::test_mark_paid_out_rereads_member_none_yields_internal_error ... ok
test repayment_entry::tests::test_update_repayment_entry_rereads_none_yields_internal_error ... ok
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 249 filtered out
```

```text
$ cargo test --workspace --lib 2>&1 | grep -c "0 failed"
10  # 10 Test-Suites, alle "0 failed"
```

**Acceptance-Criteria-Greps:**

| Grep | Result | Expected | Status |
|------|--------|----------|--------|
| `grep -c "let member_action_dao = Arc::new(MemberActionDao::new" genossi_bin/src/lib.rs` | 1 | =1 (W-02 single instance) | PASSED |
| `grep -c "member_action_dao: member_action_dao.clone()," genossi_bin/src/lib.rs` | 6 | =6 (Phase 9 ist Konsument #6) | PASSED |
| `grep -nE "type MemberActionDao = MemberActionDao;" genossi_bin/src/lib.rs` | 6 (Z. 117, 142, 231, 357, 388, 430) | ≥6 (5 bestehende + 1 neu in RepaymentEntryServiceDeps) | PASSED |
| `cargo build -p genossi_bin` exits 0 | yes | exits 0 | PASSED |
| `cargo build --workspace` exits 0 | yes | exits 0 | PASSED |
| `cargo build --bin genossi` exits 0 | yes | exits 0 | PASSED |
| `cargo test -p genossi_service_impl --lib repayment_entry` | 29/29 passed | "0 failed" | PASSED |

**Arc-Konsumenten-Topologie (W-02 Single-Instance-Demonstration):**

```text
$ grep -nE "(let member_action_dao = Arc::new|member_action_dao: member_action_dao.clone\(\))" genossi_bin/src/lib.rs
566:        let member_action_dao = Arc::new(MemberActionDao::new(pool.clone()));   # Konstruktor (1×)
571:            member_action_dao: member_action_dao.clone(),                       # Konsument 1: MemberService
580:                member_action_dao: member_action_dao.clone(),                   # Konsument 2: MemberActionService
607:                member_action_dao: member_action_dao.clone(),                   # Konsument 3: ValidationService
628:                member_action_dao: member_action_dao.clone(),                   # Konsument 4: MemberImportService
712:                member_action_dao: member_action_dao.clone(),                   # Konsument 5: ApplicationService
775:                member_action_dao: member_action_dao.clone(),                   # Konsument 6: RepaymentEntryService (NEW)
```

Alle 6 Konsumenten teilen sich exakt EINEN `Arc<MemberActionDaoImpl>` — single source of truth pro Prozess. Hash-Chain-Audit-Konsistenz (T-09-03-01/02 Mitigation) gewaehrleistet, weil alle Schreibvorgaenge ueber denselben DB-Pool laufen.

## Decisions Made

- **2 Edits in 1 Datei in 1 Commit** statt Aufteilung in Edit-1-Commit + Edit-2-Commit. Begruendung: Beide Edits MUSSTEN zusammen kommen, weil entweder allein dem Compiler einen E0046/E0063-Fehler liefert. Plan-Action-Section spezifizierte auch atomare Single-Task-Ausfuehrung.
- **Type-Alias-RHS `MemberActionDao`** (statt voll qualifiziertem Pfad `genossi_dao_impl_sqlite::member_action::MemberActionDaoImpl`). Begruendung: globale Type-Alias-Definition existiert bereits in Z. 364; alle anderen RepaymentEntryServiceDeps-Type-Aliases nutzen identisch die globalen Aliases — Konsistenz mit bestehender Pattern.
- **Inline-Block-Kommentar im Konstruktor listet alle Konsumenten mit Zeilennummern** — Defense gegen Refactor-Drift. Wenn jemand in einer kuenftigen Phase versehentlich eine zweite `Arc::new(MemberActionDao::new(...))`-Instanz anlegt, koennte das W-02-Invariante brechen — Inline-Kommentar mit Zeilennummern macht das beim Code-Review sofort sichtbar.
- **Kommentar-Header `// Phase 8 Plan 05 (D-DI) + Phase 9 (PAYO-01)`** statt nur Phase 9. Begruendung: bewahrt die Phase-8-Historie + dokumentiert die inkrementelle Evolution. Pattern-Anker fuer kuenftige Erweiterungen ("Phase 8 + Phase 9 + Phase X").

## Deviations from Plan

None — Plan wurde 1:1 ausgefuehrt. Beide Edits exakt an den im Plan spezifizierten Stellen (Z. 224-232 + Z. 765-775 in der Pre-Edit-Numerierung; Post-Edit: Z. 221-234 + Z. 765-777).

Die Plan-Action-Section dokumentierte "Reihenfolge der Felder im Struct-Literal MUSS mit der gen_service_impl!-Block-Reihenfolge uebereinstimmen — `member_action_dao` kommt zwischen `member_dao` und `audit_log_dao`." Verifiziert durch Read von Plan 09-01 Edit 3 (gen_service_impl!-Block): exakt diese Reihenfolge wurde gewaehlt. Compile-Verifikation (`cargo build`) wuerde abweichende Reihenfolge mit "no field with this name" abfangen — natuerliche Mitigation gegen T-09-03-03 Configuration-Drift.

## Issues Encountered

Keine. Erster `cargo build -p genossi_bin` direkt grün, alle Grep-Gates beim ersten Versuch passend.

## User Setup Required

None — keine externe Service-Konfiguration noetig.

## Next Phase Readiness

- **Plan 09-04 (E2E-Tests):** Das Binary kompiliert und kann jetzt mit `cargo run --bin genossi` gestartet werden. Der `mark_paid_out`-REST-Endpoint `POST /api/repayment-entry/{id}/mark-paid-out` (Plan 09-02) ist exposed und delegiert an den voll-gewireten `RepaymentEntryServiceImpl::mark_paid_out` (Plan 09-01) der jetzt Zugriff auf `MemberActionDao` hat. Die geplanten E2E-Tests (Happy-Cascade, PAYO-03-ValidationError, PAYO-04-Double-mark-paid-out, Race-via-`tokio::join!`) koennen direkt gegen einen lauffaehigen Test-Server arbeiten.
- **Plan 09-05 (Requirements-Sign-off):** Voraussetzung — E2E in Plan 09-04 muss erfolgreich abgeschlossen sein. Erst danach werden PAYO-01..04 in REQUIREMENTS.md als [x] markiert.
- **Swagger-UI:** sobald `cargo run --bin genossi` läuft, ist `POST /api/repayment-entry/{id}/mark-paid-out` unter `/swagger-ui/` mit allen 5 Status-Codes (200/400/401/404/409/500) sichtbar.

## TDD Gate Compliance

Plan-Frontmatter sagt `type: execute` (nicht `type: tdd`); Task 1 hat `tdd="false"`. Keine TDD-Gates erwartet.

Die Compile-Verifikation `cargo build` ist die effektive RED→GREEN-Sequenz fuer diesen DI-Wiring-Plan: vor Plan 09-03 war `cargo build -p genossi_bin` rot (E0046+E0063); nach Plan 09-03 ist `cargo build --workspace` gruen. Plan-09-01-Mocks `MockRepaymentEntryService::expect_mark_paid_out` decken den Service-Layer-Vertrag ab; reine DI-Wiring-Aenderungen brauchen keine eigenen Tests (sie sind kompilatortisch verifiziert).

## Self-Check: PASSED

- File `genossi_bin/src/lib.rs` exists: FOUND
- Commit `7c1e72d` (Task 1) exists: FOUND (`git log --oneline -5`)
- `type MemberActionDao = MemberActionDao;` in Z. 231 (RepaymentEntryServiceDeps-Block): FOUND
- `member_action_dao: member_action_dao.clone(),` in Z. 775 (RepaymentEntryServiceImpl-Konstruktor): FOUND
- W-02 Single-Instance-Gate: 1 `let member_action_dao = Arc::new(...)` (Z. 566): PASSED
- Phase 9 Konsument #6: 6 `member_action_dao: member_action_dao.clone(),` total: PASSED
- `cargo build --workspace` exits 0: PASSED
- `cargo build --bin genossi` exits 0: PASSED
- `cargo test -p genossi_service_impl --lib repayment_entry`: 29/29 grün, 0 failed: PASSED
- No accidental file deletions in commit: PASSED (diff-filter=D returns empty)
- No untracked files after commit: PASSED (git status --short returns empty)

---

*Phase: 09-auszahlungs-buchung-atomisch-auditiert*
*Plan: 03*
*Completed: 2026-05-31*
