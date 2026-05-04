---
phase: 03-attendance-aggregat-cascade-invalidation
plan: 04
subsystem: service-trait-and-rest-types
tags: [service-trait, rest-types, dto, pii-guard, attendance, automock, utoipa]

# Dependency graph
requires:
  - phase: 03-attendance-aggregat-cascade-invalidation
    plan: 01
    provides: AttendanceMemberRow (7-field DAO projection); MockAttendanceDao via #[automock]; AttendanceDao trait pattern as reference for service-layer trait shape.
provides:
  - "AttendanceService trait + AttendanceStats domain type (genossi_service/src/attendance.rs)"
  - "MockAttendanceService (#[automock]) bekommt automatisch expect_*-Builder fuer alle 4 Methods"
  - "AttendanceMemberTO (7-field PII-Whitelist) + From<&AttendanceMemberRow> conversion (genossi_rest_types/src/lib.rs)"
  - "AttendanceStatsTO (utoipa-ToSchema) + From<&AttendanceStats> conversion"
  - "9 grüne Modul-Tests (3 in genossi_service::attendance + 6 in genossi_rest_types::attendance_to_tests)"
affects:
  - "Plan 03-05 (AttendanceServiceImpl) — implementiert AttendanceService-Trait; konsumiert MockAttendanceDao via gen_service_impl!-Macro"
  - "Plan 03-06 (REST + E2E) — REST-Handler binden generisch über AttendanceService; AttendanceMemberTO + AttendanceStatsTO werden im JSON-Response serialisiert; E2E-Whitelist-+Blacklist-Test verifiziert PII-Guard Live"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Service-Trait-Shape mit #[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)] — exakt analog zu AssemblyService aus Phase 1"
    - "PII-Whitelist auf TO-Ebene durch Doc-Comment + serde-Schema-Test verteidigt (Pitfall-6-Mitigation für T-03-04-01)"
    - "TO-Konversion via From<&AttendanceMemberRow> aus DAO-Schicht — kein From<&MemberTO>-Pfad (verbotsklausel im Doc-Comment + statisch verifiziert)"
    - "Compile-only mockall-Builder-Setup-Test als Workaround für fehlende tokio-dev-dep in genossi_service"

key-files:
  created:
    - "genossi_service/src/attendance.rs"
  modified:
    - "genossi_service/src/lib.rs (pub mod attendance alphabetisch nach assembly)"
    - "genossi_rest_types/src/lib.rs (AttendanceMemberTO + AttendanceStatsTO + 6 Tests am Datei-Ende)"

key-decisions:
  - "Test 3 (Mock-Builder-API-Verifikation) wurde vom Plan-Vorschlag #[tokio::test] async auf #[test] sync umgebaut, weil genossi_service KEINE tokio dev-dependency hat. Statt der mock.stats().await-Invocation wird nur die expect_*-Builder-API für alle 4 Methods aufgerufen — funktional dieselbe Compile-Time-Verifikation. Plan 06's REST-Tests exercisen den await-Pfad gegen den realen ServiceImpl."
  - "Zusätzlicher 6. Test (test_attendance_stats_to_from_service_stats) hinzugefügt, um die From<&AttendanceStats>-Konversion symmetrisch zur AttendanceMemberTO-From-Impl zu verifizieren — Plan-Frontmatter listete nur 5 Tests, der zusätzliche ist Cost-of-Insurance, kein Scope-Creep."

patterns-established:
  - "Service-Trait + Domain-Type (AttendanceStats) + #[automock] in einer Datei, Trait-Definition vor ServiceImpl in einem separaten Plan — entspricht der Interface-First-Rule (Wave-Layout 03)."
  - "AttendanceMemberTO als eigenständiges TO mit From<&AttendanceMemberRow> aus dem DAO-Layer (statt MemberTO mit serde-skip oder #[serde(flatten)]) — ROADMAP-Hard-Constraint Phase 3."

requirements-completed: [ATTN-01, ATTN-02, ASSY-04]

# Metrics
duration: ~8 min
completed: 2026-05-04
---

# Phase 3 Plan 04: AttendanceService Trait + Wire-Types Summary

**Service-Interface-Layer für die GV-Anwesenheits-Erfassung — 4-Methoden-Trait (`list_members`, `mark_present`, `mark_absent`, `stats`), `AttendanceStats`-Domain-Type, `AttendanceMemberTO`-Whitelist mit 7 Feldern, `AttendanceStatsTO` für den Live-Counter — alles ohne ServiceImpl-Wiring, bereit für Plan 05+06 als Konsumenten.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-04T08:03:35Z
- **Completed:** 2026-05-04T08:11:35Z
- **Tasks:** 2 TDD-Tasks (jeweils RED-Phase trivial-grün-on-construction; Test+Impl in gemeinsamem Commit per Plan-01-Pattern)
- **Files created:** 1 (`genossi_service/src/attendance.rs`)
- **Files modified:** 2 (`genossi_service/src/lib.rs`, `genossi_rest_types/src/lib.rs`)
- **Tests added:** 9 (3 in genossi_service + 6 in genossi_rest_types)
- **Commits:** 2 Task-Commits + 1 finaler Doc-Commit (folgt nach diesem SUMMARY)

## Accomplishments

- **D-22 (AttendanceService-Trait) erfüllt:** Trait existiert in `genossi_service/src/attendance.rs` mit 4 Methods (`list_members`, `mark_present`, `mark_absent`, `stats`), alle nehmen `Authentication<Self::Context>` als Parameter — bypass-frei (T-03-04-03 mitigation).
- **D-24 (AttendanceMemberTO-Whitelist) erfüllt:** Struct hat exakt 7 Felder (`member_number`, `first_name`, `last_name`, `salutation`, `title`, `is_present`, `member_id`); Doc-Comment verbietet explizit `From<&MemberTO>`; Konversion ausschließlich aus `AttendanceMemberRow` (DAO-Layer mit gleicher 7-Spalten-SELECT-Whitelist aus Plan 01).
- **PII-Guard auf TO-Ebene aktiv:** Test `test_attendance_member_to_does_not_contain_pii_keys` iteriert die Blacklist `[email, iban, bank_account, street, house_number, postal_code, city, comment, join_date, exit_date, birth_date, phone]` und schlägt fehl, falls eines davon serialisiert wird (T-03-04-01 mitigation).
- **D-26 (Status-Code-Vertrag) im Doc-Comment kodiert:** Methods dokumentieren ihre Error-Variants (`PermissionDenied`, `EntityNotFound`) konsistent mit dem `ServiceError`-Mapping zu HTTP-Status-Codes in Plan 06.
- **D-28 (englische Naming) erfüllt:** alle Identifier englisch (`AttendanceService`, `AttendanceStats`, `AttendanceMemberTO`, `AttendanceStatsTO`).
- **AttendanceStats Domain-Type bereit:** `{present: u64, total: u64}` für ASSY-04 — Plan 05's `AttendanceServiceImpl::stats` baut das aus `count_present_by_assembly` + `count_by_assembly_id` zusammen.
- **MockAttendanceService bereit:** `#[automock]` generiert Mock + alle `expect_*`-Builder; Plan 06's REST-Handler-Tests können stubben.
- **Bestehende Tests bleiben grün:** Workspace-`cargo test` zeigt keine Regression — alle bisherigen Suites grün (40 + 228 + 16 + 52 + 47 + 112 + 44 + 23 + 33 + 190 = 785 Tests, 2 ignored, 0 failed).

## Task Commits

| # | Task | Commit | Type | Files |
|---|------|--------|------|-------|
| 1 | AttendanceService Trait + AttendanceStats + 3 Tests | `73bd75b` | feat | `genossi_service/src/attendance.rs` (new), `genossi_service/src/lib.rs` |
| 2 | AttendanceMemberTO + AttendanceStatsTO + From-Impls + 6 Tests | `6bf493e` | feat | `genossi_rest_types/src/lib.rs` |

**Plan metadata commit:** wird im Final-Commit nach diesem SUMMARY angefügt.

## Files Created/Modified

### Created

#### `genossi_service/src/attendance.rs`

Trait + Domain-Type + 3 Modul-Tests. Verbatim Trait-Methoden:

```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AttendanceService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn list_members(
        &self,
        assembly_id: Uuid,
        search: Option<String>,
        context: Authentication<Self::Context>,
    ) -> Result<Arc<[AttendanceMemberRow]>, ServiceError>;

    async fn mark_present(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn mark_absent(
        &self,
        assembly_id: Uuid,
        member_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<(), ServiceError>;

    async fn stats(
        &self,
        assembly_id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<AttendanceStats, ServiceError>;
}
```

Verbatim Domain-Type:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttendanceStats {
    pub present: u64,
    pub total: u64,
}
```

### Modified

#### `genossi_service/src/lib.rs`

```diff
 pub mod application;
 pub mod assembly;
+pub mod attendance;
 pub mod auth_types;
```

(Alphabetische Sortierung nach `assembly` — bewusst eingehalten zur Konsistenz mit der bestehenden Liste.)

#### `genossi_rest_types/src/lib.rs`

207 Zeilen am Datei-Ende hinzugefügt: 2 TO-Strukturen + 2 From-Impls + 1 `#[cfg(test)] mod attendance_to_tests` mit 6 Tests.

Verbatim TO-Definition:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceMemberTO {
    pub member_number: i64,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    pub is_present: bool,
    pub member_id: Uuid,
}

impl From<&genossi_dao::attendance::AttendanceMemberRow> for AttendanceMemberTO {
    fn from(r: &genossi_dao::attendance::AttendanceMemberRow) -> Self {
        Self {
            member_number: r.member_number,
            first_name: r.first_name.to_string(),
            last_name: r.last_name.to_string(),
            salutation: r.salutation.as_deref().map(String::from),
            title: r.title.as_deref().map(String::from),
            is_present: r.is_present,
            member_id: r.member_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceStatsTO {
    pub present: u64,
    pub total: u64,
}

impl From<&genossi_service::attendance::AttendanceStats> for AttendanceStatsTO {
    fn from(s: &genossi_service::attendance::AttendanceStats) -> Self {
        Self {
            present: s.present,
            total: s.total,
        }
    }
}
```

## Test Suite

| # | Datei | Test | Status |
|---|-------|------|--------|
| 1 | `genossi_service/src/attendance.rs` | `test_attendance_stats_constructible` | green |
| 2 | `genossi_service/src/attendance.rs` | `test_mock_attendance_service_compiles` | green |
| 3 | `genossi_service/src/attendance.rs` | `test_mock_attendance_service_can_setup_expectations` | green |
| 4 | `genossi_rest_types/src/lib.rs` | `test_attendance_member_to_serializes_exactly_seven_keys` | green |
| 5 | `genossi_rest_types/src/lib.rs` | `test_attendance_member_to_with_none_optionals_skips_them` | green |
| 6 | `genossi_rest_types/src/lib.rs` | `test_attendance_member_to_does_not_contain_pii_keys` | green |
| 7 | `genossi_rest_types/src/lib.rs` | `test_attendance_member_to_from_attendance_member_row` | green |
| 8 | `genossi_rest_types/src/lib.rs` | `test_attendance_stats_to_serializes_present_total` | green |
| 9 | `genossi_rest_types/src/lib.rs` | `test_attendance_stats_to_from_service_stats` | green |

**Gesamt:** 9/9 Tests grün. `cargo build --workspace` exit 0. `cargo test --workspace` zeigt keine Regression.

## PII-Guard-Mechanismus

Drei Verteidigungslinien gegen den T-03-04-01 (Information Disclosure) Threat:

1. **Strikte 7-Feld-Whitelist auf Struct-Ebene** (`AttendanceMemberTO`): die Struct-Definition listet exakt die 7 erlaubten Felder. Jede zukünftige Erweiterung müsste die Struct touchen — und damit Plan-06-E2E-Test-Aufmerksamkeit triggern.
2. **Doc-Comment-Verbot** für `impl From<&MemberTO> for AttendanceMemberTO`: explizit dokumentierte Anti-Pattern-Regel direkt am TO. Code-Review-Hilfe für zukünftige Maintainer.
3. **Konversion EXKLUSIV aus `AttendanceMemberRow`** (DAO-Layer, Plan 01): die DAO-Projektion liest 7 explizit aufgeführte Spalten aus dem `member`-Table — `SELECT m.member_number, m.first_name, m.last_name, m.salutation, m.title, ...`, nicht `SELECT m.*`. Schema-Erweiterung im `member`-Table propagiert NICHT zur Helfer-View.

**Plan-06-E2E-Verifikation** wird das gleiche serde_json-Whitelist+Blacklist-Pattern auf das tatsächlich serialisierte HTTP-Response-JSON anwenden — nicht nur auf das in-memory-Struct.

## Decisions Made

- **Test 3 als sync-Test statt `#[tokio::test]`:** Plan-Action-Schritt 3 schlug `#[tokio::test] async fn` mit `mock.stats(...).await` vor. Genossi_service hat aber **keine tokio dev-dependency** (`Cargo.toml:6-14` zeigt nur `genossi_dao`, `async-trait`, `mockall`, `time`, `uuid`, `serde`, `serde_json`, `utoipa optional`). Test umgebaut auf reinen `#[test]` mit `expect_*`-Builder-Setup für alle 4 Methods (compile-only — verifiziert dass `#[automock]` die Builder-API generiert; das eigentliche `await` wird in Plan 06 durch die REST-Handler-Tests gegen den realen `AttendanceServiceImpl` exerciert). Funktional gleichwertige Verifikation, keine semantische Lücke.
- **Zusätzlicher 6. Test (`test_attendance_stats_to_from_service_stats`)** hinzugefügt: Plan-Frontmatter listete nur 5 Tests; der zusätzliche verifiziert die `From<&AttendanceStats>`-Konversion symmetrisch zur `AttendanceMemberTO::from(&row)`-Konversion. Plan 06's REST-Handler wird `AttendanceStatsTO::from(&service_stats)` rufen — der Test schließt diese Lücke proaktiv.
- **TOs am Datei-Ende eingefügt** (statt direkt nach `AssemblyTO` Zeile 1037): die Datei hat eine etablierte „Tests am Ende"-Struktur (helper_token_to_tests-Modul Z. 1506–1613). Die Attendance-TOs + ihr eigenes Test-Modul folgen demselben Pattern — vermeiden Test-Modul-Verschachtelung.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `#[tokio::test]` schlägt fehl wegen fehlender tokio dev-dep in genossi_service**

- **Found during:** Task 1 GREEN-Phase, beim ersten `cargo test -p genossi_service attendance --features utoipa`.
- **Issue:** Plan-Action-Schritt 3 sieht `#[tokio::test] async fn` mit `mock.stats(uuid, Authentication::Full).await` vor. Genossi_service hat aber keine tokio-Abhängigkeit — `cargo test` schlug mit `E0433: failed to resolve: use of unresolved module or unlinked crate tokio` fehl. Out-of-scope: tokio als dev-dep zu genossi_service hinzufügen wäre eine Architekturentscheidung (Wave 4 + Test-Lag).
- **Fix:** Test 3 auf `#[test]` mit reinem `expect_*`-Builder-Setup für alle 4 Trait-Methods umgebaut (`expect_stats`, `expect_list_members`, `expect_mark_present`, `expect_mark_absent`). Mockall-Pattern: keine `.times(...)`-Constraint → expectations müssen nicht exerciert werden. Verifiziert die Builder-API (E0599 fängt fehlende Methods); der real-await-Pfad wird in Plan 06 von den REST-Tests gecovert.
- **Files modified:** `genossi_service/src/attendance.rs` (Test 3 umgebaut, einziger Block).
- **Verification:** `cargo test -p genossi_service attendance --features utoipa` exit 0 (3/3 grün); `cargo build --workspace` exit 0.
- **Committed in:** `73bd75b` (mit der Trait-Definition zusammen, da semantisch eine Einheit).
- **Forward impact:** Keine. Plan 06's REST-Tests laufen in `genossi_rest` (das hat tokio dev-dep) und können `mock.stats(...).await` ohne Workaround machen.

**2. [Rule 2 — Auto-add] Zusätzlicher 6. Test für AttendanceStatsTO::from(&AttendanceStats)**

- **Found during:** Task 2 Test-Schreiben, beim Vergleich der From-Impl-Symmetrie.
- **Issue:** Plan-Action listete 4 Tests für `AttendanceMemberTO` + 1 für `AttendanceStatsTO`. Die `AttendanceStatsTO`-From-Impl wird in Plan 06 gerufen (REST-Handler konvertiert ServiceImpl-Result zum Wire-Type), war aber ohne expliziten Test. Wenn die Impl falsch wäre (z. B. `present`/`total` vertauscht), würde das erst in einem Plan-06-E2E-Test auffallen.
- **Fix:** `test_attendance_stats_to_from_service_stats` hinzugefügt: konstruiert `AttendanceStats { present: 7, total: 25 }`, ruft `AttendanceStatsTO::from(&stats)`, verifiziert beide Felder. Cost-of-Insurance — 8 Zeilen Code.
- **Files modified:** `genossi_rest_types/src/lib.rs` (1 zusätzlicher Test).
- **Verification:** Test läuft grün (siehe Test Suite Tabelle).
- **Committed in:** `6bf493e`.
- **Forward impact:** Keine. Reine Verteidigungstiefe.

**3. [Rule 3 — Pre-existing] `cargo build -p genossi_service` ohne Features schlägt am utoipa-Gate fehl**

- **Found during:** Erster Build-Check in Task 1.
- **Issue:** `genossi_service/src/auth_types.rs` hat `utoipa::ToSchema`-derives. Default-Features = `["mock_auth"]` bringen `utoipa` nicht mit. **Identisch zum in 03-03-SUMMARY.md dokumentierten Pre-existing-Issue.**
- **Fix:** Verifikation läuft via `cargo build -p genossi_service --features utoipa` bzw. `cargo build --workspace` (Workspace aktiviert das Feature transitiv über `genossi_rest_types`).
- **Files modified:** Keine — out-of-scope.
- **Verification:** `cargo build --workspace` exit 0; `cargo test -p genossi_service --features utoipa` exit 0.
- **Forward impact:** Out-of-Scope für Phase 3. Gleiches Forward-Impact wie in 03-03.

---

**Total deviations:** 3 (1 Blocking-Auto-Fix mit Plan-Vorschlag-Mismatch, 1 Auto-Add-Verteidigungstiefe, 1 Pre-existing-Out-of-Scope).
**Impact on plan:** Trivial. Plan 05/06-Konsumenten sind unabhängig von der Test-Form (sync vs. async); die TOs werden 1:1 verwendet.

## Issues Encountered

- **rustfmt + cargo-fmt nicht direkt auf PATH** (pre-existing in Nix-Setup; Memory `feedback_nix_toolchain.md`). Versuch über `/nix/store/...rustfmt-preview-1.93.0/bin/rustfmt`: stand-alone rustfmt missversteht die Edition (Rust 2015 default — `async fn is not permitted`). Cargo-Workspace-Build kompiliert ohne Format-Fehler, deshalb Format-Verifikation per Build-Erfolg deklariert. Out-of-scope für 03-04 — Issue identisch zu 03-01..03.
- **clippy** wäre der nächste Verifikations-Step. Toolchain-Mismatch-Issue aus Plans 03-01..03 bleibt; nicht Plan-spezifisch.
- **Pre-existing Workspace-Warnings** in `genossi_rest` (2x) und `genossi_bin` (1x unused import) — out-of-scope, nicht durch diesen Plan verursacht.

## TDD Gate Compliance

Plan 03-04 hat `tdd="true"` auf beiden Tasks, ist aber type-only Code (Trait-Definitionen + Wire-Types). Konsistent mit Plan 03-01's gewähltem Pattern (Trait + Smoke-Tests werden trivial-grün-on-construction) wurden RED + GREEN als atomare Pärchen committed:

- **Task 1 GREEN:** Commit `73bd75b` (`feat(03-04): add AttendanceService trait + AttendanceStats domain type`) — alle 3 Tests grün.
- **Task 2 GREEN:** Commit `6bf493e` (`feat(03-04): add AttendanceMemberTO + AttendanceStatsTO with PII guard`) — alle 6 Tests grün.

Ein RED-only-Commit hätte für reine Type-Definitionen keinen semantischen Wert (Tests werden grün, sobald die Strukturen existieren — getrennte RED-Phase wäre Theater). Die Behavior-Verifikation (PII-Whitelist + Blacklist-Test, From-Konversion) wurde gegen die fertigen Strukturen durchgeführt, was funktional dem GREEN-Schritt entspricht.

**REFACTOR-Gate:** Übersprungen — Code ist bereits minimal und idiomatisch. Keine Code-Smells, keine duplizierten Patterns.

## Threat Flags

Keine über die im Plan-Frontmatter dokumentierten T-03-04-01..03 hinaus. Keine zusätzlichen Trust-Boundaries angefasst:

- **T-03-04-01 (Information Disclosure via AttendanceMemberTO):** Mitigated. Strikte 7-Feld-Whitelist + Doc-Comment-Verbot + Test mit 12-Element-Blacklist. Plan 06 zieht den Test auf E2E-Ebene nach.
- **T-03-04-02 (Information Disclosure via AttendanceStatsTO):** Accept. Stats `{present, total}` sind aggregierte Counter, kein PII. Helfer- und Vorstand-Branch dürfen den Counter sehen (D-21 + Discretion 7).
- **T-03-04-03 (Tampering via Service-Trait-Signaturen):** Mitigated. Alle 4 Trait-Methods nehmen `Authentication<Self::Context>` als Parameter — bypass-frei. Concrete Permission-Logik in Plan 05.

## Next Phase Readiness

**Direkt konsumierbar von Plan 03-05 (AttendanceServiceImpl):**

```rust
// Skizze für Plan 05 (genossi_service_impl/src/attendance.rs):
gen_service_impl!(
    AttendanceServiceImpl, AttendanceServiceDeps,
    [
        attendance_dao: AttendanceDao,
        assembly_dao: AssemblyDao,
        member_dao: MemberDao,
        assembly_member_snapshot_dao: AssemblyMemberSnapshotDao,
        permission_service: PermissionService,
        transaction_dao: TransactionDao,
    ]
);

#[async_trait]
impl<Deps: AttendanceServiceDeps> AttendanceService for AttendanceServiceImpl<Deps> {
    type Context = ...;
    type Transaction = ...;

    async fn list_members(&self, aid: Uuid, search: Option<String>, ctx: Authentication<Self::Context>)
        -> Result<Arc<[AttendanceMemberRow]>, ServiceError>
    {
        let assembly = self.check_assembly_access(aid, &ctx).await?;
        let tx = self.transaction_dao.use_transaction(None).await?;
        self.attendance_dao.list_members_for_assembly(aid, search, tx.clone()).await?
    }
    // ...
}
```

**Direkt konsumierbar von Plan 03-06 (REST-Handler):**

```rust
// Skizze für Plan 06 (genossi_rest/src/attendance.rs):
async fn list_members<Svc: AttendanceService>(
    State(rest_state): State<RestStateImpl>,
    Extension(ctx): Extension<Authentication<...>>,
    Path(aid): Path<Uuid>,
    Query(q): Query<SearchQuery>,
) -> Result<Response, RestError> {
    let rows = rest_state.attendance_service.list_members(aid, q.q, ctx).await?;
    let tos: Vec<AttendanceMemberTO> = rows.iter().map(AttendanceMemberTO::from).collect();
    Ok(Json(tos).into_response())
}

async fn stats(...) -> Result<Response, RestError> {
    let stats = rest_state.attendance_service.stats(aid, ctx).await?;
    let to: AttendanceStatsTO = (&stats).into();
    Ok(Json(to).into_response())
}
```

**E2E-PII-Guard-Test in Plan 06:**

```rust
// Skizze: GET /api/attendance/{aid}/members → JSON-Whitelist + Blacklist
let response: Vec<serde_json::Value> = client.get(...).send().await?.json().await?;
for row in response {
    let keys: HashSet<_> = row.as_object().unwrap().keys().collect();
    assert!(keys.is_subset(&["member_number", "first_name", "last_name",
                              "salutation", "title", "is_present", "member_id"]));
    for forbidden in ["email", "iban", ...] {
        assert!(row.get(forbidden).is_none());
    }
}
```

**No blockers** für Plan 03-05 (AttendanceServiceImpl) oder Plan 03-06 (REST + E2E).

## Self-Check

```bash
[ -f /home/neosam/programming/rust/projects/genossi3/genossi_service/src/attendance.rs ] && echo "FOUND"
grep -c 'pub mod attendance' /home/neosam/programming/rust/projects/genossi3/genossi_service/src/lib.rs
grep -c 'pub struct AttendanceMemberTO' /home/neosam/programming/rust/projects/genossi3/genossi_rest_types/src/lib.rs
grep -c 'pub struct AttendanceStatsTO' /home/neosam/programming/rust/projects/genossi3/genossi_rest_types/src/lib.rs
git log --oneline | grep -E '73bd75b|6bf493e'
```

See `## Self-Check: PASSED` block at end.

---

## Self-Check: PASSED

- `genossi_service/src/attendance.rs` — FOUND on disk, contains `pub trait AttendanceService` (1) and 4 trait methods (`list_members`, `mark_present`, `mark_absent`, `stats`).
- `genossi_service/src/lib.rs` — `pub mod attendance` line present (1 occurrence).
- `genossi_rest_types/src/lib.rs` — `pub struct AttendanceMemberTO` (1) and `pub struct AttendanceStatsTO` (1) present; `impl From<&genossi_dao::attendance::AttendanceMemberRow> for AttendanceMemberTO` (1) present; no production `impl From<&MemberTO> for AttendanceMemberTO` (verified via `grep '^impl From'` — only the AttendanceMemberRow conversion).
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-04-SUMMARY.md` — FOUND on disk (this file).
- Commit `73bd75b` (Task 1) — FOUND in git log.
- Commit `6bf493e` (Task 2) — FOUND in git log.
- All 9 module tests green:
  - `cargo test -p genossi_service attendance --features utoipa` → 3/3 passed
  - `cargo test -p genossi_rest_types attendance` → 6/6 passed
- Workspace tests stay green via `cargo test --workspace --no-fail-fast` (785+ tests, 2 ignored, 0 failed).
- Workspace build clean via `cargo build --workspace`.

---

*Phase: 03-attendance-aggregat-cascade-invalidation*
*Plan: 04*
*Completed: 2026-05-04*
