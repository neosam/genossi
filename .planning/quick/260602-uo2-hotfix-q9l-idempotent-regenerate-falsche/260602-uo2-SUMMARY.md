---
phase: quick-260602-uo2
plan: 01
type: tdd
status: complete
completed: 2026-06-02
commits:
  - 845105a: "test(quick-260602-uo2): RED — assert 1 doc + version stability + audit-verify after idempotent regenerate (Hotfix q9l)"
  - 1f37542: "fix(quick-260602-uo2): preserve existing version in idempotent repayment-letter UPDATE-Branch (q9l hotfix)"
files_changed:
  - genossi_service_impl/src/repayment_letter.rs
  - genossi_bin/tests/repayment_letter_e2e.rs
  - .planning/quick/260602-sgp-bulk-download-aller-repaymentletter-doku/deferred-items.md
requirements_satisfied:
  - "uo2-FIX-01: Idempotenter regenerate-Branch passt korrekte (alte) Version an audited_update! / DAO::update an"
  - "uo2-FIX-02: q9l-Semantik (in-place replace = 1 Doc nach 2 Calls) ist im E2E-Test verankert"
  - "uo2-FIX-03: Service-Layer-Regression-Guard: doc_dao.update wird mit existing_doc.version (NICHT mit neuer UUID) aufgerufen"
tags:
  - hotfix
  - optimistic-locking
  - repayment-letter
  - audit-hashchain
  - tdd
---

# Phase quick-260602-uo2: Hotfix q9l Idempotent Regenerate — Summary

**One-Liner:** 1-Zeilen-Fix in `RepaymentLetterServiceImpl::generate` UPDATE-Branch — `version: existing_doc.version` statt `self.uuid_service.new_v4()` — repariert die in q9l etablierte In-Place-Replace-Semantik, sodass die zweite sequenzielle `POST /letters/generate` HTTP 200 statt 409 Version-mismatch liefert.

## Task

Repariere den seit q9l broken E2E-Test `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09`. Die zweite sequenzielle `POST /api/repayment-phase/{id}/letters/generate` antwortet mit HTTP 409 statt 200, weil der idempotente UPDATE-Branch dem DAO eine neue UUID als `version` schickt, der DAO aber `entity.version` als ALTE Version fuer den Optimistic-Lock-Match liest.

Zusaetzlich Test an die q9l-In-Place-Replace-Semantik anpassen (1 Doc statt 2 nach 2 Calls + Stabilitaets-Checks + Audit-Hashchain-Verify) und einen Service-Layer-Regression-Guard hinzufuegen.

## Root Cause Analysis

**Bug-Site:** `genossi_service_impl/src/repayment_letter.rs:423` (vor Fix):
```rust
version: self.uuid_service.new_v4().await, // rotate per optimistic-locking.
```

**DAO-Vertrag (`genossi_dao_impl_sqlite/src/member_document.rs:170-244`):**
```rust
let old_version = entity.version.as_bytes().to_vec();   // entity.version IST die ALTE Version
let new_version = Uuid::new_v4().as_bytes().to_vec();   // DAO rotiert intern
// ...
// UPDATE ... SET version = new_version WHERE id = ? AND version = old_version
```

**Konsequenz:** Wenn Service `entity.version = neue UUID` setzt, matcht der `WHERE version = ?` nicht die in der DB persistierte Version → 0 affected rows → `DaoError::ConflictError("Version mismatch")` → HTTP 409.

Der `audited_update!`-Makro (genossi_service_impl/src/audit_macros.rs) reicht `entity` unveraendert an den DAO durch — der Service ist die einzige Stelle, an der `entity.version` auf die richtige (alte) Version gesetzt werden kann.

## Approach

TDD-disziplin gemaess plan.md:
1. **RED:** Bestehenden E2E-Test an q9l-Semantik anpassen + neuen Service-Layer-Regression-Guard hinzufuegen, beide auf base commit `fbb945e` RED beweisen.
2. **GREEN:** 1-Zeilen-Fix anwenden, Mock-Counts in zwei pre-existing q9l-Tests anpassen, deferred-items.md im sgp-Quick-Task als RESOLVED markieren.

## Files Changed

| File | Change |
|------|--------|
| `genossi_service_impl/src/repayment_letter.rs` | (1) Bug-Fix: `version: existing_doc.version` (Zeile 423). (2) Neuer Test `test_generate_update_branch_passes_existing_version_to_dao` (Regression-Guard via mockall `.withf(entity.version == existing_version)`). (3) Mock-Anpassungen: `test_generate_overwrites_existing_repayment_letter_in_place` (`expect_new_v4().times(1)` → `times(0)`), `test_generate_idempotent_two_calls_same_doc_id` (dritter sequenced UUID-Call entfernt). |
| `genossi_bin/tests/repayment_letter_e2e.rs` | E2E `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` an q9l-Semantik angepasst: 1 Doc-Erwartung statt 2, Stabilitaets-Asserts (id / file_name / created stable, version rotated), audit-verify-Block. |
| `.planning/quick/260602-sgp-bulk-download-aller-repaymentletter-doku/deferred-items.md` | RESOLVED-Note mit Root-Cause + Fix-Link. |

## Test Results

### Vor Fix (base commit `fbb945e`)

| Test | Status | Failure |
|------|--------|---------|
| `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` | FAILED | `resp2.status() == 409 Conflict` at line 855 (vor Plan-Anpassung) |
| `test_generate_update_branch_passes_existing_version_to_dao` | RED | mockall: "No matching expectation" on `doc_dao.update` (entity.version != existing_version) |

### Nach Fix (HEAD `1f37542`)

| Test-Suite | Result |
|------------|--------|
| `cargo test --test repayment_letter_e2e --features mock_auth` | 14 passed; 0 failed; 0 ignored |
| `cargo test -p genossi_service_impl repayment_letter` | 37 passed; 0 failed |
| `cargo test --workspace --features mock_auth` | 1150 passed; 0 failed |
| `cargo clippy --workspace --all-targets` | clean (0 errors, 0 new warnings) |
| `rustfmt --check` (via `/nix/store/.../rustfmt-preview-1.93.0/bin/rustfmt --edition 2021`) | clean |

### Grep-Gates (Plan-Requirements)

```bash
$ grep -nE "version:\s*existing_doc\.version" genossi_service_impl/src/repayment_letter.rs
423:                    version: existing_doc.version, // OLD version — DAO::update rotates internally (genossi_dao_impl_sqlite/src/member_document.rs:178).
2456:    /// Nach dem 1-Zeilen-Fix (`version: existing_doc.version`) matchet das
```
- Produktions-Treffer: **1** (Zeile 423), Plan-konform.
- Doc-Comment-Treffer in neuem Test ist erwuenscht (dokumentiert die Pattern-Aussage).

```bash
$ grep -nE "version:\s*self\.uuid_service\.new_v4\(\)\.await" genossi_service_impl/src/repayment_letter.rs
455:                    version: self.uuid_service.new_v4().await,
```
- Verbleibender Treffer: **1** (Zeile 455 = CREATE-Branch), Plan-konform (CREATE-Branch MUSS frische UUID generieren, das ist der Initial-Write).

### Audit-Hashchain Verify

Die neue Assertion im E2E-Test ruft `GET /api/audit/verify` nach beiden Bulk-Calls auf und prueft `body.valid == true`. Der Test laeuft gruen — Hash-Chain bleibt nach UPDATE-Branch valide.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Stabilitaets-Assert auf `file_name` statt `relative_path` | `MemberDocumentTO` exponiert `relative_path` nicht; `file_name` ist das TO-Level-Aequivalent fuer stabile PDF-Identitaet (Service uebernimmt es unveraendert aus `existing_doc`). Inline-Doc im Test dokumentiert die Rule-1-Deviation vom PLAN. |
| `uuid_svc.expect_new_v4().times(0..=1)` im Regression-Guard-Test | Tolerantes Predicate: aktuell (vor Fix) ruft Service `new_v4` einmal; nach Fix 0x — `0..=1` deckt beides ab, sodass der Test ausschliesslich am `doc_dao.update`-Predicate (der eigentlichen Regression-Aussage) failt, nicht am UUID-Count. |
| Mock-Count-Anpassungen in pre-existing q9l-Tests als Rule-3-Auto-Fix | Plan vorgesehen: UPDATE-Branch zieht nach Fix keine UUID mehr — die in q9l etablierten `uuid_svc.expect_new_v4().times(1)`-Erwartungen muessen mitziehen. `test_generate_overwrites_existing_repayment_letter_in_place` runter auf `times(0)`, `test_generate_idempotent_two_calls_same_doc_id` dritter sequenced Call entfernt. |
| `rustfmt` via Nix-Store-Lookup statt "not installed" | User-Memory-Regel "Nix-Toolchain nicht sofort aufgeben — `/nix/store` durchsuchen". `rustfmt-preview-1.93.0` ist in `/nix/store` verfuegbar; direkter Aufruf produziert `--check`-clean Output. |

## Deviations from Plan

### Rule 1 (Bug) — PLAN-zitiertes `relative_path` ist im REST-TO nicht exponiert

- **Found during:** Task 1 (RED) — E2E-Compile-Fehler `no field 'relative_path' on type 'MemberDocumentTO'`.
- **Issue:** PLAN's `<behavior>`-Spec verlangte `assert_eq!(letter_docs[0].relative_path, doc_after_call_1.relative_path, ...)`, aber `MemberDocumentTO` (genossi_rest_types/src/lib.rs:478-507) hat keine `relative_path`-Spalte. Die DAO-Entity hat `relative_path`, aber der REST-TO exponiert nur `id`, `member_id`, `document_type`, `description`, `file_name`, `mime_type`, `created`, `deleted`, `version`.
- **Fix:** Stabilitaets-Assert auf `file_name` umgeschwenkt — semantisch aequivalent fuer den Plan-Intent (stabiler PDF-Identifikator), weil der Service `file_name` aus `existing_doc` unveraendert uebernimmt. Inline-Kommentar im Test dokumentiert den Schritt.
- **Files modified:** `genossi_bin/tests/repayment_letter_e2e.rs` (innerhalb der gleichen RED-Edit)
- **Commit:** `845105a` (RED-Phase)

### Worktree-Layout

Das `.claude/worktrees/agent-...`-Verzeichnis ist kein echter git-Worktree, sondern ein File-Mirror (wie in der sgp-Quick-Task). Branch-Check ergab `git merge-base HEAD fbb945e == fbb945e` und `git rev-parse HEAD == fbb945e` (der Worktree-Branch entspricht der echten `main`-Branch im Repo). Beide Commits landeten direkt auf `main` — dokumentiert hier transparent als bekanntes Worktree-Layout, nicht als Bug.

## Self-Check: PASSED

- `genossi_service_impl/src/repayment_letter.rs:423` enthaelt `version: existing_doc.version` — verifiziert via grep.
- `genossi_bin/tests/repayment_letter_e2e.rs` enthaelt die 5 neuen Asserts (1 Doc + 4 Stabilitaets-Checks) + audit-verify-Block.
- Commit `845105a` (RED) und `1f37542` (GREEN) in `git log --oneline -5` sichtbar.
- `cargo test --test repayment_letter_e2e test_letter_idempotency_d13_08_and_no_status_toggle_d13_09 --features mock_auth` → 1 passed.
- `cargo test -p genossi_service_impl test_generate_update_branch_passes_existing_version_to_dao` → 1 passed.
- `cargo test --workspace --features mock_auth` → 1150 passed; 0 failed.
- `cargo clippy --workspace --all-targets` → clean.
- `rustfmt --check` (Nix-Store) → clean.
- `deferred-items.md` enthaelt RESOLVED-Note.

## TDD Gate Compliance

- **RED-Gate:** `test(quick-260602-uo2): ...` — commit `845105a`, beide Tests FAILED auf base commit `fbb945e`.
- **GREEN-Gate:** `fix(quick-260602-uo2): ...` — commit `1f37542`, beide Tests PASSED nach 1-Zeilen-Fix.
- REFACTOR-Gate nicht erforderlich (1-Zeile-Production-Change, keine Code-Smell-Inflation).
