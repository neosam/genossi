---
phase: 03-attendance-aggregat-cascade-invalidation
plan: 02
subsystem: database
tags: [sqlite, dao, helper-token, cascade-discovery, automock, async-trait]

# Dependency graph
requires:
  - phase: 02-helfer-token-session-authcontext-helper
    provides: helper_token-Tabelle inkl. session_id-Spalte (D-01) — der FK-Anker, den diese Method liest
provides:
  - "HelperTokenDao::list_session_ids_for_assembly Trait-Method (genossi_dao/src/helper_token.rs)"
  - "SQLx-Impl der Method (genossi_dao_impl_sqlite/src/helper_token.rs)"
  - "MockHelperTokenDao (#[automock]) bekommt automatisch expect_list_session_ids_for_assembly()-Builder"
  - "TestHelperTokenDao (handgeschrieben in genossi_service_impl/src/helper_token.rs::tests) erweitert um die neue Method-Signatur"
  - "3 grüne Modul-Tests gegen sqlite::memory: (filter-by-state, empty-for-unknown, cross-assembly-isolation)"
affects:
  - "Plan 03-03 (Cascade-Invalidation in close_assembly) — konsumiert list_session_ids_for_assembly als Discovery-Schritt"
  - "Plan 03-05 (AssemblyServiceImpl::close_assembly Cascade-Loop) — die Method ist hier der Iterations-Input für PermissionDao::delete_session"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "query_scalar für single-column SELECT statt query_as<_, FromRow>-Boilerplate (wenn nur 1 Spalte gebraucht wird)"
    - "Vec<Arc<str>> als Returntype für nullbar-gefilterte String-Listen — kein Option-Wrapping pro Eintrag, da Filter session_id IS NOT NULL den NULL-Fall ausschließt"
    - "Trait-Erweiterung ohne Schema-Migration (Phase 2 hat session_id schon angelegt)"
    - "Pitfall-4-Forward-Mitigation: Handgeschriebene mock! { TestHelperTokenDao { ... } } muss bei Trait-Erweiterungen synchron mitziehen, sonst E0046"

key-files:
  created: []
  modified:
    - "genossi_dao/src/helper_token.rs (Trait-Method nach all_for_assembly hinzugefügt)"
    - "genossi_dao_impl_sqlite/src/helper_token.rs (SQLx-Impl + 3 Modul-Tests)"
    - "genossi_service_impl/src/helper_token.rs (Pitfall 4: hand-rolled TestHelperTokenDao mock_set ergänzt)"

key-decisions:
  - "Plan-Dokument lokalisierte den hand-rolled Mock fälschlicherweise in genossi_service_impl/src/assembly.rs::tests — der reale Mock liegt in genossi_service_impl/src/helper_token.rs::tests (Plan 05's vorgesehene Erweiterung in assembly.rs::tests existiert noch nicht). Da das Done-Kriterium `cargo test -p genossi_service_impl assembly` einen kompilierenden Service-Crate verlangt, wurde der existierende Mock im helper_token.rs erweitert (reine Symbol-Synchronisation, keine expect_*-Erwartungen geändert)."

patterns-established:
  - "DAO-Trait-Erweiterungs-Workflow im Genossi-Workspace: 1) Trait-Method in genossi_dao, 2) SQLx-Impl in genossi_dao_impl_sqlite, 3) hand-rolled mock_set in genossi_service_impl/src/<entity>.rs::tests synchron pflegen — sonst kompiliert der Service-Crate-Test nicht."

requirements-completed: [ATTN-06]

# Metrics
duration: ~10 min
completed: 2026-05-04
---

# Phase 3 Plan 02: HelperTokenDao Cascade-Discovery Summary

**Eine neue Trait-Method `list_session_ids_for_assembly` auf HelperTokenDao + SQLite-Impl + 3 grüne Tests; Cascade-Anker (D-12) für Plan 05's `AssemblyServiceImpl::close_assembly`-Erweiterung.**

## Performance

- **Duration:** ~10 min
- **Tasks:** 1 TDD-Task (RED + GREEN, kein REFACTOR nötig)
- **Files created:** 0
- **Files modified:** 3 (`genossi_dao/src/helper_token.rs`, `genossi_dao_impl_sqlite/src/helper_token.rs`, `genossi_service_impl/src/helper_token.rs`)
- **Tests added:** 3 (alle gegen sqlite::memory:, alle grün)
- **Commits:** 1 RED + 1 GREEN + 1 finaler Doc-Commit (folgt nach diesem SUMMARY)

## Accomplishments

- **D-12 (Cascade-Discovery via session_id-FK) auf DAO-Ebene umgesetzt:** Method liefert exakt die in der angefragten Assembly aktiven, nicht-soft-gelöschten helper-session-IDs zurück.
- **DSGVO/Threat-T-03-02-01 (Information Disclosure) mitigiert:** Method exponiert nur session_id-Strings (keine Member-PII); `query_scalar` projiziert auf eine einzige Spalte.
- **SQL-Injection (T-03-02-02) mitigiert:** assembly_id wird als `Vec<u8>` parameterisiert via `.bind(aid)` übergeben — keine String-Konkatenation.
- **Filter-Korrektheit verifiziert:** Test 1 prüft die 3-fache Bedingung (revoked excluded via `session_id IS NULL` ODER `deleted IS NOT NULL`), Test 3 die `assembly_id`-Isolation zwischen GVs.
- **Pitfall 4 vermieden:** Handgeschriebene `mock! { TestHelperTokenDao { ... } }` in `genossi_service_impl/src/helper_token.rs::tests` wurde synchron erweitert — alle 11 Phase-2-Assembly-Tests bleiben grün (keine Regression).
- **Trait-Mockability erhalten:** `#[automock]` regeneriert `MockHelperTokenDao` automatisch; Plan 05/06 können `expect_list_session_ids_for_assembly()` verwenden ohne weitere Modifikation.

## Task Commits

| # | Task | Commit | Type | Files |
|---|------|--------|------|-------|
| 1a | RED: 3 failing tests in helper_token-Impl-Modul | `8b37b32` | test | `genossi_dao_impl_sqlite/src/helper_token.rs` |
| 1b | GREEN: Trait-Method + SQLx-Impl + Pitfall-4-Sync | `c25d48f` | feat | `genossi_dao/src/helper_token.rs`, `genossi_dao_impl_sqlite/src/helper_token.rs`, `genossi_service_impl/src/helper_token.rs` |

**Plan metadata commit:** wird im Final-Commit nach diesem SUMMARY angefügt (siehe state_updates).

## Files Modified

### `genossi_dao/src/helper_token.rs` (Trait-Erweiterung)

Neue Method-Signatur, eingefügt nach `all_for_assembly` (vor dem schließenden `}` des Trait-Blocks):

```rust
/// D-12: Cascade-Discovery for AssemblyServiceImpl::close_assembly (Phase 3).
/// Returns all currently-bound helper-session ids for the given assembly.
/// Filters out null session_ids (revoked or never-redeemed tokens) and
/// soft-deleted token rows. Order is implementation-defined but stable
/// within a single SQLite snapshot.
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Vec<Arc<str>>, DaoError>;
```

### `genossi_dao_impl_sqlite/src/helper_token.rs` (SQLx-Impl + Tests)

Verbatim-Impl-Body:

```rust
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Vec<Arc<str>>, DaoError> {
    // D-12 (Phase 3): Cascade-Discovery via session_id-FK.
    // Caller (AssemblyServiceImpl::close_assembly, Plan 05) iterates the
    // result and calls PermissionDao::delete_session for each id.
    // Filters: assembly_id parameterized via bind (T-03-02-02 mitigation),
    // session_id IS NOT NULL excludes revoked/never-redeemed tokens,
    // deleted IS NULL excludes soft-deleted token rows.
    let aid = assembly_id.as_bytes().to_vec();
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT session_id FROM helper_token \
         WHERE assembly_id = ? AND session_id IS NOT NULL AND deleted IS NULL",
    )
    .bind(aid)
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    Ok(rows.into_iter().map(|s| Arc::from(s.as_str())).collect())
}
```

Verbatim SQL-Statement:

```sql
SELECT session_id FROM helper_token
WHERE assembly_id = ? AND session_id IS NOT NULL AND deleted IS NULL
```

### `genossi_service_impl/src/helper_token.rs` (Pitfall-4-Sync)

In `mod tests::mock! { pub TestHelperTokenDao { ... } }` direkt nach `all_for_assembly`:

```rust
// Phase 3 Plan 02 (D-12): Cascade-Discovery for close_assembly.
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: TestTransaction,
) -> Result<Vec<Arc<str>>, DaoError>;
```

Keine `expect_*`-Aufrufe in bestehenden Tests gesetzt — die Erweiterung ist eine reine Symbol-Synchronisation, damit das `impl HelperTokenDao for TestHelperTokenDao`-Macro alle Trait-Methods abdeckt.

## Test Suite

| # | Datei | Test | Status |
|---|-------|------|--------|
| 1 | `genossi_dao_impl_sqlite/src/helper_token.rs` | `test_list_session_ids_for_assembly_returns_redeemed_only` | green |
| 2 | `genossi_dao_impl_sqlite/src/helper_token.rs` | `test_list_session_ids_for_assembly_empty_for_unknown_assembly` | green |
| 3 | `genossi_dao_impl_sqlite/src/helper_token.rs` | `test_list_session_ids_for_assembly_excludes_other_assemblies` | green |

**Gesamt-Regression-Check:**

- `cargo test -p genossi_dao_impl_sqlite helper_token`: 11/11 grün (8 Phase-2 + 3 neu).
- `cargo test -p genossi_service_impl assembly`: 11/11 grün (Phase-2-Assembly-Lifecycle nicht regressed → Pitfall 4 ok).
- `cargo test --workspace`: alle Suites grün, keine Failures.

## Decisions Made

- **Plan-Dokument falsch lokalisierte den hand-rolled Mock:** Das PLAN.md-Frontmatter und das `<action>` sagten "KEIN Touch der handgeschriebenen mock! { TestHelperTokenDao { ... } } in `genossi_service_impl/src/assembly.rs::tests`". In der Realität liegt dieser Mock aber **nicht** in `assembly.rs`, sondern in `genossi_service_impl/src/helper_token.rs::tests::mock!` (Zeile 619). Der `assembly.rs::tests`-Mock, den Plan 05 erweitern soll, **existiert noch gar nicht** — Plan 05 wird ihn erst dort anlegen. Da das Done-Kriterium `cargo test -p genossi_service_impl assembly` einen kompilierenden Service-Crate verlangt (E0046 wäre andernfalls geschossen worden), wurde der existierende Mock in `helper_token.rs::tests` synchronisiert. Diese Korrektur ist semantisch leer (keine `expect_*`-Aufrufe geändert) und stellt sicher, dass Phase 2's HelperTokenService-Tests nicht regressed sind.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Pitfall-4-Pfad falsch im Plan dokumentiert; hand-rolled Mock-Sync nötig**

- **Found during:** Task 1 GREEN-Phase, beim ersten `cargo build --workspace` nach Trait-Erweiterung.
- **Issue:** Plan-Dokument behauptete, der hand-rolled `TestHelperTokenDao`-Mock liege in `genossi_service_impl/src/assembly.rs::tests` und solle in Plan 05 mitgezogen werden. Tatsächlich liegt der einzige existierende hand-rolled Mock in `genossi_service_impl/src/helper_token.rs::tests::mock!` (Zeile 619) und implementiert das gesamte `HelperTokenDao`-Trait. Beim Erweitern des Trait um `list_session_ids_for_assembly` brach der Service-Crate-Build mit `E0046: not all trait items implemented` — was direkt das Done-Kriterium `cargo test -p genossi_service_impl assembly` blockiert hätte.
- **Fix:** Mock-Block in `genossi_service_impl/src/helper_token.rs::tests::mock! { pub TestHelperTokenDao { ... } impl HelperTokenDao for TestHelperTokenDao { ... } }` um eine `list_session_ids_for_assembly`-Signatur erweitert. Keine `expect_*`-Aufrufe in bestehenden Tests verändert — die Erweiterung ist reine Symbol-Synchronisation.
- **Files modified:** `genossi_service_impl/src/helper_token.rs` (4 Zeilen, mock-Block).
- **Verification:** `cargo build -p genossi_service_impl --tests` exit 0; `cargo test -p genossi_service_impl assembly` 11/11 grün; `cargo test -p genossi_service_impl helper_token` ebenfalls grün.
- **Committed in:** `c25d48f` (zusammen mit Trait + Impl, da semantisch eine Einheit).
- **Forward impact:** Plan 05 wird laut PLAN.md einen **neuen** hand-rolled Mock in `genossi_service_impl/src/assembly.rs::tests` anlegen müssen (für die `close_assembly`-Cascade-Tests). Dieser neue Mock muss `list_session_ids_for_assembly` ebenfalls listen — und kann es jetzt, weil das Trait-Symbol existiert.

---

**Total deviations:** 1 auto-fixed (Rule 3 — Blocking, Pfad-Korrektur im Plan, Compile-Block ohne Fix).
**Impact on plan:** Trivial; reine Symbol-Erweiterung in einem hand-rolled Mock. Keine semantische Änderung an bestehenden Tests, keine Forward-Impact-Risiken auf Plan 05.

## Issues Encountered

- **`cargo fmt -- --check` und `cargo clippy` nicht direkt auf PATH** (pre-existing in Nix-Setup; siehe Memory `feedback_nix_toolchain.md`). rustfmt aus `/nix/store` läuft, zeigt aber nur **bestehende** Format-Diffs (wrap-style in Trait-Signaturen, vec!-Layout in Phase-2-Tests) — keine im neu-geschriebenen Code dieses Plans. Out-of-scope für 03-02. cargo-clippy aus `/nix/store` schlägt mit Toolchain-Mismatch fehl (gleicher Befund wie in Plan 03-01).
- **Pre-existing Workspace-Warnings** in `genossi_rest`, `genossi_bin`, `genossi_mail` — out-of-scope, nicht durch diesen Plan verursacht.

## Self-Check

Verification commands run after SUMMARY drafting (siehe Self-Check-Block am Ende).

## Threat Flags

Nichts Neues über die im Plan-Frontmatter dokumentierten T-03-02-01..03 hinaus. Keine zusätzlichen Trust-Boundaries angefasst.

## TDD Gate Compliance

- **RED-Gate:** Commit `8b37b32` (`test(03-02): add failing tests for list_session_ids_for_assembly`) — verifiziert per `cargo test -p genossi_dao_impl_sqlite --no-run` mit E0599-Fehlern (Method existierte nicht).
- **GREEN-Gate:** Commit `c25d48f` (`feat(03-02): implement HelperTokenDao::list_session_ids_for_assembly`) — alle 3 Tests grün, alle Phase-2-Tests bleiben grün.
- **REFACTOR-Gate:** Übersprungen — Impl ist bereits minimal und idiomatisch; `Vec<String> → Vec<Arc<str>>`-Conversion via `into_iter().map(...)` ist kanonisch im Genossi-Codebase.

## Next Phase Readiness

**Direkt konsumierbar von Plan 03-05 (AssemblyServiceImpl::close_assembly Cascade-Loop):**

```rust
// Skizze für Plan 05:
let session_ids = self.helper_token_dao
    .list_session_ids_for_assembly(assembly_id, tx.clone()).await?;
for sid in session_ids.iter() {
    self.permission_dao.delete_session(sid.as_ref(), tx.clone()).await?;
}
```

**Direkt mockable in Plan 03-05 Tests:**

```rust
// Skizze für Plan 05's neuer assembly.rs::tests::mock!-Block:
mock_helper_token.expect_list_session_ids_for_assembly()
    .with(eq(assembly_id), always())
    .returning(|_, _| Ok(vec![Arc::from("sess-1"), Arc::from("sess-2")]));
```

**Pitfall-4-Note für Plan 05:** Wenn Plan 05 in `genossi_service_impl/src/assembly.rs::tests` einen **neuen** hand-rolled `TestHelperTokenDao`-Mock anlegt (statt den existierenden in `helper_token.rs::tests` zu re-exportieren), MUSS dieser neue Mock alle 9 `HelperTokenDao`-Methoden inkl. `list_session_ids_for_assembly` listen. Das Trait-Symbol existiert ab diesem Plan und führt sonst zu E0046.

**No blockers** für Plan 03 Wave 1 (Plans 03/04 parallelisierbar) oder Wave 2 (Plans 05/06).

## Self-Check: PASSED

- `genossi_dao/src/helper_token.rs` — FOUND on disk, contains `fn list_session_ids_for_assembly`
- `genossi_dao_impl_sqlite/src/helper_token.rs` — FOUND on disk, contains both impl and 3 new tests
- `genossi_service_impl/src/helper_token.rs` — FOUND on disk, mock_set updated
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-02-SUMMARY.md` — FOUND on disk (this file)
- Commit `8b37b32` (RED) — FOUND in git log
- Commit `c25d48f` (GREEN) — FOUND in git log

---
*Phase: 03-attendance-aggregat-cascade-invalidation*
*Plan: 02*
*Completed: 2026-05-04*
