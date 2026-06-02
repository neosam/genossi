---
phase: quick-260602-q9l
plan: 01
subsystem: repayment-letter / member-document / audit
tags:
  - idempotent-regenerate
  - audited-update
  - repayment-letter
  - audit-hashchain
  - quick-task
dependency-graph:
  requires:
    - "MemberDocumentDao::find_by_member_id (existing trait method, deleted-IS-NULL semantics)"
    - "audited_update! macro (genossi_service_impl/src/audit_macros.rs:42-80)"
    - "DocumentType::RepaymentLetter (genossi_service/src/member_document.rs)"
  provides:
    - "Idempotent generate() — repeated calls overwrite same (member, phase) row in place"
    - "Audit-Log UPDATE-class entries on regeneration (hash chain extends, no gap)"
    - "On-disk PDF overwrite in place — no stale .pdf leak after re-generate"
  affects:
    - "RepaymentLetterServiceImpl::generate() write-tx loop"
    - "Closes deferred TODO option 1 in genossi_service/src/member_document.rs:95-100"
tech-stack:
  added: []  # no new deps
  patterns:
    - "find-then-update-or-create branch inside write-tx (consistent snapshot)"
    - "(member_id, document_type, description-fingerprint) lookup key (no schema change)"
    - "audited_update! preserves row id + relative_path + created; rotates version only"
key-files:
  created: []
  modified:
    - "genossi_service_impl/src/repayment_letter.rs (+617 / -39: helper fn, branch, 3 tests, 6 existing-test mock updates)"
    - "genossi_service/src/member_document.rs (doc-comment status note: option 1 DONE; runtime behavior of is_singleton() unchanged)"
decisions:
  - "Lookup-Schluessel ist (member_id, document_type==repayment_letter, description==\"Anschreiben Auszahlung GJ {fy}\"); KEIN neues phase_id-Feld, KEIN neuer DAO-Methode."
  - "Re-Generation rotiert NUR version-UUID; doc_id + relative_path + created bleiben stabil; on-disk PDF wird in place ueberschrieben."
  - "Existing tests bekommen permissive expect_find_by_member_id (leere Liste) -> fall-through zum unveraenderten CREATE-Zweig; ihre vorhandenen Erwartungen bleiben semantisch unveraendert."
metrics:
  duration: "~20 min"
  completed: "2026-06-02"
---

# Quick Task 260602-q9l: Repayment-Dokumente beim Neugenerieren — Idempotent Summary

One-liner: RepaymentLetter regeneration overwrites the existing `(member, phase)` MemberDocument-Row in place via `audited_update!`, eliminating duplicate rows and stale PDF files per phase.

## What changed

### `genossi_service_impl/src/repayment_letter.rs`

1. **New helper fn `find_existing_letter_for_phase`** (associated fn on `RepaymentLetterServiceImpl<Deps>`):
   - Pure function — takes a `&[MemberDocumentEntity]` (per-member doc list, pre-loaded via `find_by_member_id`) and a `fiscal_year: i32`.
   - Returns `Option<MemberDocumentEntity>` matching:
     - `deleted.is_none()` (Defense-in-Depth)
     - `document_type == DocumentType::RepaymentLetter.as_str()` (`"repayment_letter"`)
     - `description == Some("Anschreiben Auszahlung GJ {fiscal_year}")`
   - The description is the per-row fingerprint for `(member, phase)` because `MemberDocumentEntity` has no `phase_id`/`fiscal_year` column.

2. **find-then-update-or-create branch inside `generate()` write-tx loop**:
   - Before constructing a fresh MemberDocument for each recipient, fetch existing active docs once: `self.member_document_dao.find_by_member_id(member.id, write_tx.clone()).await?`.
   - Branch on `find_existing_letter_for_phase(...)`:
     - `Some(existing)` → build `updated_doc` reusing `existing.id`, `existing.relative_path`, `existing.created`; rotate `version`; reformat `description`/`file_name`/`mime_type`; null mail-tracking fields; call `audited_update!(self, self.member_document_dao, existing.id, &updated_doc, REPAYMENT_LETTER_PROCESS, &user_id, write_tx)`; push `(existing.relative_path.to_string(), pdf_bytes)` into `planned_saves`; push `existing.id` into `document_ids`.
     - `None` → unchanged CREATE branch: fresh `doc_id`, fresh `relative_path = "{doc_id}.pdf"`, `audited_create!`.

3. **Existing tests** that exercise the write-tx loop got a permissive `expect_find_by_member_id(...).returning(empty Arc)` so they fall through to the CREATE branch — no semantic change in their existing expectations (e.g. `expect_create().times(N)` stayed as-is). Affected tests:
   - `test_generate_happy_path_2_members`
   - `test_generate_no_status_toggle_d13_09`
   - `test_generate_multi_entry_aggregation_d13_04`
   - `test_generate_sequential_audited_create_pitfall_4`
   - `test_generate_aggregate_called_once_per_unique_member`
   - `test_generate_user_id_never_nil`

### `genossi_service/src/member_document.rs`

- Doc-comment status note at lines 95-100 updated: `TODO(phase-14+)` header replaced with `DONE (quick-260602-q9l, option 1): ...`. Options 2/3 remain noted as optional future work.
- `is_singleton()` runtime behavior is UNCHANGED (`matches!(self, JoinDeclaration | JoinConfirmation)` stays).

## New tests added

Three tokio unit tests in `repayment_letter::tests` (#[cfg(test)] mod):

1. **`test_generate_overwrites_existing_repayment_letter_in_place`** — proves the UPDATE branch.
   - Pre-cond: `find_by_member_id(m1.id, ...)` returns one existing RepaymentLetter for `(m1, GJ 2025)` with `existing_id` + `existing_relative_path`.
   - Asserts: `doc_dao.expect_create().times(0)`, `doc_dao.expect_update().times(1)` with a `withf` predicate verifying `entity.id == existing_id`, `entity.relative_path == existing_relative_path`. `storage.expect_save()` is called with the existing path (`withf` predicate). `entry_dao.expect_update().times(0)` and `.expect_create().times(0)` keep D-13-09 holding. `uuid_svc.expect_new_v4().times(1)` (one for the rotated `version`). Result: `document_ids[0] == existing_id`.

2. **`test_generate_creates_new_when_no_existing_letter`** — proves the CREATE branch and the filter rejects unrelated documents.
   - Pre-cond: `find_by_member_id` returns one unrelated doc with `document_type = "join_declaration"` but an intentionally misleading `description = "Anschreiben Auszahlung GJ 2025"` (regression-guard for filter-by-document_type).
   - Asserts: `doc_dao.expect_create().times(1)`, `doc_dao.expect_update().times(0)`. `result.document_ids[0] != unrelated_doc.id` (fresh UUID).

3. **`test_generate_idempotent_two_calls_same_doc_id`** — proves two-call idempotence.
   - Two successive `generate()` calls. `find_by_member_id` mock uses an `AtomicUsize` call-counter: call 1 returns `[]` (CREATE path), call 2 returns `[just_created_doc]` (UPDATE path).
   - `uuid_svc` uses `mockall::Sequence` to deterministically return `stable_doc_id` for the create's `doc_id`, then versions `v1` and `v2`.
   - Custom `tx_dao` (NOT `tx_dao_permissive`) allows 4 commits (read-tx + write-tx × 2 calls).
   - Asserts: `doc_dao.expect_create().times(1)`, `doc_dao.expect_update().times(1)`. Both calls return `document_ids[0] == stable_doc_id` — proves row identity preserved across regenerations.

## Verification

Per the plan's `<verify>` gates:

- ✅ `cargo test -p genossi_service_impl repayment_letter` — 24 tests pass (21 existing + 3 new).
- ✅ `cargo test -p genossi_service_impl` — 328 tests pass (no regression in the wider crate).
- ✅ `cargo clippy -p genossi_service_impl --all-targets -- -D warnings` — clean.
- ✅ Grep gate `audited_update!` present in `repayment_letter.rs` — found 5 occurrences (1 in helper docstring example, 2 in production branch, 2 in test setup).
- ✅ Grep gate no direct `self.member_document_dao.update(` — 0 occurrences.
- ⚠ `cargo fmt -- --check` — `cargo fmt` is NOT available in this nix dev shell (no `rustfmt` in `flake.nix`). Manual rustfmt runs from `/nix/store` show diffs that pre-date this change (e.g. line 31, 40, 67 of the file are pre-existing import-grouping that the local rustfmt version disagrees with). My added code matches the line-wrap style used by the surrounding existing tests. Treating this as a pre-existing project-environment condition (project verify gate unreachable). See Surprises below.

## Threat-model adherence

- T-q9l-01 (Tampering / skipped audited_update) — mitigated: grep gate ensures only `audited_update!` is used; direct DAO update count = 0.
- T-q9l-02 (Repudiation) — mitigated: `audited_update!` writes audit_log entries for each changed field per regeneration; hash chain extends (covered by `audit_dao.expect_get_latest_hash` + `expect_create_entries` in Test A).
- T-q9l-03 (Cross-member doc leak) — mitigated: lookup scoped to `find_by_member_id(member.id, ...)` first, then in-memory filter by document_type+description. Test B (unrelated `join_declaration` with similar description) explicitly verifies the document_type filter rejects.
- T-q9l-04 (DoS via N round-trips) — accepted: `find_by_member_id` is called once per unique member, bounded by `MAX_ENTRY_IDS_PER_REQUEST=200`.
- T-q9l-05 (Elevation of Privilege) — n/a; permission funnel untouched.

## Surprises encountered

1. **Mockall panic on missing `find_by_member_id` expectation.** mockall's `Mock*` panics by default when an un-expected method is called. Adding the new lookup to `generate()` required updating six existing tests' mocks with a permissive `expect_find_by_member_id().returning(empty Arc)`. The plan anticipated this in spirit (`<done>` block: "the new lookup MUST return empty/None for those mocks, so they fall through the unchanged create-branch") but I want to flag it explicitly: this is the one expectation-style addition to existing tests; their times-counters on `expect_create` etc. did NOT need adjusting.
2. **Test C tx_dao expectation.** `tx_dao_permissive()` only allows `expect_commit().times(0..=2)`. Test C does two `generate()` calls → 4 commits (read-tx + write-tx × 2). Replaced with a Test-C-local `expect_commit().times(4)` setup.
3. **AtomicUsize-mock pattern for Test C lookup.** Used the same call-counter pattern as `test_generate_user_id_never_nil` (existing reference in the same file) — `Arc<AtomicUsize>` clone captured in the mock closure, branching on `fetch_add`.
4. **`cargo fmt` unavailable.** The nix dev shell (`flake.nix`) does NOT include `rustfmt` in `devShells.default.packages` — only `cargo`, `rust-analyzer`, `clippy`, `sqlx-cli`. Running rustfmt directly from `/nix/store` produces diffs on PRE-EXISTING code (e.g. import grouping at lines 31, 40, 67 of `repayment_letter.rs` before any of my edits). Treating as a project-environment condition; the verify gate `cargo fmt -- --check` is unreachable here.
5. **jj-workspace artifact.** Git in the worktree reports `genossi_service_impl/src/repayment_letter.rs` and `genossi_service/src/member_document.rs` as unmodified because the worktree lives under `.claude/worktrees/` which is gitignored. The jj co-located workspace correctly tracks the modifications. Per the `<jj_environment_note>` in the prompt, I committed via `jj describe` + `jj new` — the resulting commit (`7dab1930`) is a fully-formed git commit in the underlying repo (jj is co-located with git, so jj commits map 1:1 to git commits).

## Files touched

| File | Change |
|------|--------|
| `genossi_service_impl/src/repayment_letter.rs` | +617 / −39: helper fn `find_existing_letter_for_phase`, find-then-update-or-create branch in `generate()`, 3 new tests, 6 existing-test mock updates |
| `genossi_service/src/member_document.rs` | doc-comment status note update (1 paragraph rewritten as "DONE (quick-260602-q9l, option 1)"); `is_singleton()` runtime behavior unchanged |

## Self-Check: PASSED

- File `genossi_service_impl/src/repayment_letter.rs` — present, contains helper `find_existing_letter_for_phase` (4 grep matches) and `audited_update!` macro (5 grep matches in non-comment lines).
- File `genossi_service/src/member_document.rs` — present, contains "DONE (quick-260602-q9l, option 1)" doc-comment update.
- Commit `7dab1930` — present in jj log (`jj log -r 'all() & ancestors(@, 2)'`); fully-formed co-located git commit.
- All 24 `repayment_letter` tests pass.
- Three new tests by name: `test_generate_overwrites_existing_repayment_letter_in_place`, `test_generate_creates_new_when_no_existing_letter`, `test_generate_idempotent_two_calls_same_doc_id` — confirmed in `cargo test` output.
