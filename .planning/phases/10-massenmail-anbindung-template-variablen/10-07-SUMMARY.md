---
phase: 10-massenmail-anbindung-template-variablen
plan: 07
subsystem: infra
tags: [dependency-injection, mail-worker, audit-log, arc-sharing, transaction-dao, repayment-context, restate-impl]

# Dependency graph
requires:
  - phase: 10-massenmail-anbindung-template-variablen
    provides: 10.06 (start_mail_worker signature extended with 6 new generic deps — MD, AL, MT, RE, RP, TX all sharing MD::Transaction)
provides:
  - genossi_bin/src/lib.rs::RestStateImpl with 5 new persisted DAO fields (member_document_dao, repayment_phase_dao, repayment_entry_dao, mail_template_dao, transaction_dao) alongside the existing audit_log_dao
  - start_mail_worker spawn block wires all 14 args (8 existing + 6 new) via Arc::clone from RestStateImpl fields — Option A enforced (no new Arc::new() in the spawn block)
  - Clean workspace build (cargo build --workspace exit 0)
  - Smoke-test: genossi binary boots without panic with sqlite::memory:, "Mail worker started" log line present
affects: [10-plan-08-e2e-bulk-mail-und-audit-chain]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Persisted-DAO-Field-Pattern for cross-spawn-block Arc sharing: when a tokio::spawn'd worker needs DAOs that other RestStateImpl services already hold, persist those DAOs as named struct fields on RestStateImpl and Arc::clone them into the spawn block — never construct a parallel Arc::new(XDao::new(self.pool.clone())) inside the spawn block (would create a second Arc, fragmenting any shared state — and for audit_log_dao would fragment the per-process hash chain)."
    - "Move-to-Clone transition when persisting a local Arc as a struct field: scan all sites in new() where the local variable was moved (e.g., member_document_dao moved into MemberDocumentServiceImpl, mail_template_dao moved into MailTemplateServiceType::new) and switch them to .clone() — the local binding must remain live until the struct literal at the end of new() picks it up. Arc::clone is O(1) refcount bump."

key-files:
  created:
    - ".planning/phases/10-massenmail-anbindung-template-variablen/10-07-SUMMARY.md (this file)"
  modified:
    - "genossi_bin/src/lib.rs (5 new RestStateImpl fields + 2 .clone() conversion sites in new() + 6-Arg-extension in start_mail_worker; ~48 LOC net added across 4 edit sites)"

key-decisions:
  - "Option A (per Plan + Checker recommendation): The 4 worker-relevant DAOs (member_document_dao, repayment_phase_dao, repayment_entry_dao, mail_template_dao) are promoted from new()-local let-bindings to RestStateImpl fields. The worker spawn block then Arc::clones them out of self. This guarantees the SAME Arc is shared between (a) the audited services that already hold the DAO and (b) the worker — single-Arc invariant per process. Option B (constructing fresh Arc::new(MemberDocumentDao::new(self.pool.clone())) inside the worker block) was explicitly REJECTED by the plan's threat model (T-10-07-02): a second DAO Arc could still talk to the same SQLite pool but would NOT contribute to the same in-memory state if any DAO ever held cached state. More critically, audit_log_dao MUST be the single Arc to keep the hash chain consistent — the same discipline applies prophylactically to the other 4 DAOs."
  - "transaction_dao added to RestStateImpl as a Rule 3 auto-fix. The plan-text said transaction_dao already existed at lib.rs:323 — verification showed that line is DbAssemblyStatusProbe.transaction_dao (a mock_auth-only helper struct), NOT a RestStateImpl field. RestStateImpl had no transaction_dao field at all; all *ServiceImpl deps held their own .clone() of the local Arc constructed at line 547. Without a RestStateImpl.transaction_dao field, start_mail_worker could not write `self.transaction_dao.clone()` — block-compile-fail. The fix: add transaction_dao as a 6th field (alongside the 4 Phase 10 ones + audit_log_dao) and persist the existing local Arc into self via the struct literal. Same Arc as every *ServiceImpl.transaction_dao field uses — single TransactionDaoImpl per process. Documented as Rule 3 deviation below."
  - "Move-to-clone fix applied at 2 sites only: (a) MemberDocumentServiceImpl's member_document_dao field (was `member_document_dao,` shorthand, which moves; switched to `member_document_dao: member_document_dao.clone()`); (b) MailTemplateServiceType::new(mail_template_dao) (positional move; switched to `mail_template_dao.clone()`). The other 2 new DAO fields (repayment_phase_dao, repayment_entry_dao) were already `.clone()`-d at every consumption site (verified at lines 755, 756, 772, 773) — no edit needed there."
  - "Struct-literal init uses Rust shorthand notation (`member_document_dao,` instead of `member_document_dao: member_document_dao.clone()`). At this point in new(), the local Arc bindings are not used anywhere later — moving them into the struct literal is correct and cheaper than .clone() (no refcount bump on the final move). The Plan's acceptance criterion `grep -c 'member_document_dao:\\s*member_document_dao\\.clone\\(\\)' >= 1` is satisfied by the MemberDocumentServiceImpl field on line ~595, not by the struct-literal — both are semantically equivalent (the literal entry plus the explicit clone above keep the same Arc alive)."

patterns-established:
  - "Persisted-DAO-Field-Pattern: when a downstream consumer (worker, background task) lives outside the *ServiceImpl tree but needs DAOs held by audited services, promote those DAOs to RestStateImpl struct fields and Arc::clone into the spawn block. Apply the same discipline to TransactionDao if any spawned task uses transactions. Cross-references the Phase 7 Plan 04 pattern 'audit_log_dao geteilt mit allen audited Services' — Phase 10 extends this from 1 Arc (audit_log_dao) to 6 Arcs."
  - "Shorthand-vs-explicit-clone in struct literals: when a local let-binding will not be used after the struct literal (terminal init), use shorthand `field_name,` (moves the local — zero-cost). When the local Arc is consumed by an earlier *ServiceImpl construction AND must also live in the struct field, use `field: local.clone()` at the consumption site and `field,` (shorthand move) at the literal. Both patterns coexist in genossi_bin/src/lib.rs::RestStateImpl::new()."
  - "Plan-text-claim cross-verification: when a plan asserts that a field exists at line X, verify it before relying on it. Plan 10.07 claimed transaction_dao existed at line 323 — that line is DbAssemblyStatusProbe.transaction_dao, not RestStateImpl. The misread came from a partial-context Codebase Map. Mitigation: planners run `grep -nE '^\\s*field_name:\\s*Arc<...>' file` on the actual struct definition before writing line-citations."

requirements-completed: [MAIL-01, MAIL-02, MAIL-03, MAIL-04]

# Metrics
duration: ~12min
completed: 2026-05-31
---

# Phase 10 Plan 07: Genossi-Bin Worker-Wiring Summary

**RestStateImpl persists 5 new DAO fields (member_document, repayment_phase, repayment_entry, mail_template, transaction) and start_mail_worker Arc::clone-s 6 deps into the spawn block — workspace compiles clean, binary boots without panic, mail worker is now functional with single-hash-chain guarantee preserved.**

## Performance

- **Duration:** ~12 min (Plan start to SUMMARY write)
- **Started:** 2026-05-31 (after Plan 10.06 left genossi_bin intentionally broken with E0061)
- **Completed:** 2026-05-31
- **Tasks:** 2 (both auto, no TDD, no checkpoints)
- **Files modified:** 1 (genossi_bin/src/lib.rs)
- **Commits:** 2 (one per task)
- **LOC delta:** ~48 net added (5 new struct fields + new() init lines + 6 .clone() + 6 new args + 2 doc-comments)
- **Tests passing post-plan:** 740/740 workspace lib tests (no regression)

## Accomplishments

- **RestStateImpl struct extended with 5 new persisted DAO fields:** `member_document_dao: Arc<MemberDocumentDao>`, `repayment_phase_dao: Arc<RepaymentPhaseDao>`, `repayment_entry_dao: Arc<RepaymentEntryDao>`, `mail_template_dao: Arc<MailTemplateDaoType>`, `transaction_dao: Arc<TransactionDao>` — placed adjacent to the existing `audit_log_dao: Arc<AuditLogDao>` field with Phase 10 D-11 doc-comment block. (Plan called for 4 fields; transaction_dao was an additional Rule 3 auto-fix — see Deviations.)
- **new() struct-literal initializer** reuses the already-existing local Arc bindings (`let member_document_dao = Arc::new(MemberDocumentDao::new(pool.clone()))` at line 589, `let mail_template_dao = ...` at line 833, `let repayment_phase_dao = ...` at line 751, `let repayment_entry_dao = ...` at line 752, `let transaction_dao = ...` at line 547). No new Arc::new() calls were introduced.
- **2 Move-to-Clone conversions in new() applied** to keep the local bindings alive until the struct-literal initializer: `MemberDocumentServiceImpl.member_document_dao` and `MailTemplateServiceType::new(mail_template_dao)`. The Arc<...> bumps a refcount instead of moving ownership — both consumer and RestStateImpl now hold the same Arc.
- **start_mail_worker spawn block extended** from 8 args to 14: 6 new bindings (`member_document_dao`, `audit_log_dao`, `mail_template_dao`, `repayment_entry_dao`, `repayment_phase_dao`, `transaction_dao`) each via `self.X.clone()`. Worker DAO call-graph: Worker.try_create_member_document_audited → MemberDocumentDao.create + AuditLogDao.{get_latest_hash,create_entries} + TransactionDao.{transaction,commit,rollback}, all over the same TransactionImpl associated type from MemberDocumentDaoImpl.
- **Single-hash-chain invariant preserved:** the `self.audit_log_dao.clone()` passed to the worker is the SAME Arc that every audited *ServiceImpl uses (verified: same Arc<AuditLogDao> constructed at lib.rs:567 is .clone()-d into 6 different consumer sites including the worker). T-10-07-02 mitigation enforced.
- **Workspace compiles clean:** `cargo build --workspace` exits 0; only pre-existing warnings (unused `Auditable` import on lib.rs:966 from Plan 10.02 era, unused axum routing imports in genossi_rest from earlier phases). No new warnings from my code.
- **Tests pass:** 740/740 workspace lib tests green (276 genossi_service_impl + 128 genossi_mail + 70 genossi_rest + others — full list captured in the build output).
- **Smoke-test:** `DATABASE_URL=sqlite::memory: timeout 5 cargo run --bin genossi` boots successfully and emits:
  - `Mail worker started`
  - `Inbox worker started`
  - `Backup worker started`
  - `Timestamp worker started`
  - `Running server at 0.0.0.0:3000`

  No panic, no worker-thread crash, no Audit-Chain initialization error.

## Task Commits

1. **Task 1: Persist 4 worker-relevant DAOs as RestStateImpl fields** — `8f5f690` (feat)
2. **Task 2: Wire 6 new DAOs into start_mail_worker spawn (+ Rule 3 fix: add transaction_dao field)** — `5ba4e7a` (feat)

_Plan metadata + STATE/ROADMAP update will be added as a separate docs-commit after this SUMMARY._

## Files Created/Modified

### Created
- `.planning/phases/10-massenmail-anbindung-template-variablen/10-07-SUMMARY.md` (this file)

### Modified
- `genossi_bin/src/lib.rs` (+48 LOC, 4 edit sites)
  - Site 1 (~Z. 524-532): 5 new struct fields with Phase 10 D-11 doc-comment block
  - Site 2 (~Z. 591-600): MemberDocumentServiceImpl init switched to `member_document_dao: member_document_dao.clone()` with doc-comment
  - Site 3 (~Z. 836-839): MailTemplateServiceType::new() switched to `mail_template_dao.clone()` with doc-comment
  - Site 4 (~Z. 905-912): Struct-literal initializer reordered to include 5 new fields between `audit_log_dao` and `timestamp_service`
  - Site 5 (~Z. 1146-1188): start_mail_worker extended with 6 `self.X.clone()` let-bindings and the spawn call now passes 14 positional args (matches genossi_mail/src/worker.rs:152 signature)

## Decisions Made

1. **Option A (persisted DAO fields) over Option B (Arc::new in spawn block):** Plan explicitly required Option A. Verified by 3-line check after both tasks: `grep -c "Arc::new(MemberDocumentDao::new(self\.pool" genossi_bin/src/lib.rs` returns 0 (KEIN parallel-Arc-construction im Worker-Block). All 6 worker deps come from RestStateImpl fields via Arc::clone — single Arc per DAO per process.

2. **Mixed clone-styles in new() are correct:** the MemberDocumentServiceImpl init uses explicit `.clone()` (because the local must stay alive for the struct-literal), but the struct-literal itself uses shorthand `member_document_dao,` (terminal move). This is idiomatic Rust and not a code smell. Doc-comments at each site explain the move-vs-clone choice.

3. **transaction_dao auto-fix scope:** Rather than restructure the plan to handle the discovered missing field as a separate task, I treated it as a Rule 3 blocking-fix and added the field in the SAME commit as the worker wiring (Task 2). Rationale: the field is logically part of the worker-wiring change set (worker calls `self.transaction_dao.clone()`); separating the commits would leave the tree in a state where Task 1's commit promises 5 fields but only delivers 4. Single-commit-per-logical-change preserved.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `transaction_dao` as RestStateImpl field**
- **Found during:** Task 2 (first workspace build attempt after writing the 6-arg extension)
- **Issue:** Workspace build fail: `error[E0609]: no field 'transaction_dao' on type '&RestStateImpl'`. The plan-text (line 21, line 47 of 10.07-genossi-bin-worker-wiring-PLAN.md) claimed transaction_dao already existed as RestStateImpl field "at Z. 323". Verification via `grep -nE '^\s*transaction_dao:\s*Arc<TransactionDao>,' genossi_bin/src/lib.rs` showed line 323 is `DbAssemblyStatusProbe.transaction_dao`, a mock_auth-only helper struct — NOT a RestStateImpl field. RestStateImpl had transaction_dao as a local Arc binding in new() at line 547, .clone()-d into every *ServiceImpl, but never persisted in self.
- **Fix:** Added `transaction_dao: Arc<TransactionDao>` as a 6th persisted field (5 Phase 10 + 1 already-existing audit_log_dao field — well, now 5 new fields + audit_log_dao = 6 worker-relevant fields total). Initialized in the struct-literal via Rust shorthand `transaction_dao,` (the local binding's last use). Added Phase 10 D-11 doc-comment block noting the auto-fix lineage.
- **Files modified:** genossi_bin/src/lib.rs (struct-def block + struct-literal initializer in new())
- **Verification:** `grep -A30 "pub fn start_mail_worker" genossi_bin/src/lib.rs | grep -c "self\.transaction_dao\.clone"` returns 1 (the worker call site); `cargo build --workspace` exits 0. Plan's must_haves.truths #1 says "RestStateImpl-Struct enthaelt nach Task 1 fuenf neue Felder" — my final state has 5 new fields **plus** the auto-fix transaction_dao field, fully meeting the plan intent (the worker needs all 6 to compile).
- **Committed in:** `5ba4e7a` (Task 2 GREEN)

---

**Total deviations:** 1 auto-fixed (Rule 3 - blocking).
**Impact on plan:** Plan intent fully preserved. The "5 new fields" must_have-truth is satisfied (member_document_dao, repayment_phase_dao, repayment_entry_dao, mail_template_dao + the Rule-3 auto-fix transaction_dao). Plan's must_haves.truths #4 ("Geteilte DAOs (audit_log_dao, transaction_dao, plus die 4 neuen) via Arc::clone aus RestStateImpl-Feldern") explicitly listed transaction_dao among the 6 expected shared DAOs — so the plan's success criteria assumed the field would be available. The plan-text claim "Z. 323 als RestStateImpl-Feld" was simply incorrect; the auto-fix repairs the discrepancy.

## Issues Encountered

- **rustfmt/clippy not on default PATH:** Known Nix-Toolchain issue (memory: `feedback_nix_toolchain`). Resolution: `find /nix/store -name rustfmt -type f` and `find /nix/store -name cargo-clippy -type f` located the binaries. Invoked via PATH-prefix. No blocker. rustfmt --check passes; clippy emits zero NEW warnings on my changes (only pre-existing warnings throughout the workspace).

- **Misread plan-text line citations:** The plan claimed two RestStateImpl fields existed: `audit_log_dao (Z. 523)` (correct — verified) and `transaction_dao (Z. 323)` (incorrect — Z. 323 is DbAssemblyStatusProbe). I caught this during the first compile attempt after Task 2, applied Rule 3 auto-fix, and proceeded. Net cost: 1 extra grep + edit cycle (~2 min). Pattern (see patterns-established): always verify field-line citations against the actual struct definition before relying on them in a multi-step plan.

## Threat Surface Scan

Plan's `<threat_model>` lists 3 STRIDE threats:

| Threat ID | Mitigation status | Verified by |
|-----------|-------------------|-------------|
| T-10-07-01 (T: DI-substitute hostile DAO via reflection) | accepted | Rust monomorphization — no runtime DI substitution; Arcs come from RestStateImpl::new fixed-construction. No code-path change in Plan 10.07. |
| T-10-07-02 (R: Worker shares audit_log_dao with services) | mitigated | `self.audit_log_dao.clone()` at start_mail_worker line is the SAME Arc constructed at lib.rs:567 and passed into the 6 audited *ServiceImpl fields. Single hash chain across the workspace preserved. Option A enforced — `grep -c "Arc::new(MemberDocumentDao::new(self\.pool" genossi_bin/src/lib.rs` returns 0 (no parallel-Arc-construction). Plan 10.08 E2E /api/audit/verify will lock the chain end-to-end. |
| T-10-07-03 (I: Wiring inadvertently exposes additional state) | accepted | Plan only adds existing Arcs to spawn-args; no new fields on the public Self surface beyond the additive DAO fields. None of the 5 new fields are pub. |

No NEW threat surface introduced. **No threat flags emitted.**

## Known Stubs

None. The Plan 10.06 worker accepts `_mail_template_dao: Arc<MT>` with a `_`-prefix indicating Plan 10.06 reserved the slot but does not yet use the DAO (the REST handler in Plan 10.04 resolves the template). Plan 10.07's job is wiring, not template-resolution — the `_`-prefix in worker.rs is a Plan-10.06-internal convention and not a stub introduced by Plan 10.07.

## User Setup Required

None. No new env vars, no schema migrations (Plan 10.01 + 10.02 already deployed the schema changes), no external services. The change is purely Rust DI wiring.

## Next Phase Readiness

- **Plan 10.08 (E2E bulk-mail + audit-chain):** All prerequisites met:
  - Worker is now spawnable without compile errors (`cargo build --workspace` exit 0).
  - Worker has access to MemberDocumentDao + AuditLogDao + TransactionDao through the same per-process Arcs that audited services use — Plan 10.08's `/api/audit/verify` will verify chain consistency end-to-end.
  - Repayment-context aggregation (D-04) is wired (Plan 10.06 worker.rs:181-249) and ready for an E2E bulk-send test with a repayment_phase_id.
  - All 14 worker args populated from real (not mock) Arcs — production-realistic test path.
- **No blockers** for the final Phase 10 plan or for v1.1 milestone closure.

## Self-Check: PASSED

**Files verified to exist:**
- `genossi_bin/src/lib.rs` ✓ (modified)
- `.planning/phases/10-massenmail-anbindung-template-variablen/10-07-SUMMARY.md` ✓ (this file)

**Commits verified to exist:**
- `8f5f690` (Task 1) — verified via `git log --oneline | grep 8f5f690`
- `5ba4e7a` (Task 2) — verified via `git log --oneline | grep 5ba4e7a`

**Acceptance criteria grep-checks (all green):**
- Task 1: 4 struct fields present (1 each), 5 struct-literal init lines (1 each in new()), MemberDocumentServiceImpl clone()-init present
- Task 2: 6 `self.X.clone()` lines in start_mail_worker (1 each), 0 parallel-Arc-construction (`Arc::new(MemberDocumentDao::new(self\.pool`)

**Verification commands all green:**
- `cargo build --workspace` → exit 0 (only pre-existing warnings)
- `cargo build -p genossi_bin` → exit 0
- `cargo test --workspace --lib` → 740 passed / 0 failed (full breakdown: 40+16+70+61+128+62+35+52+276+0 from 10 crates)
- `rustfmt --edition 2021 --check genossi_bin/src/lib.rs` → FMT OK
- `cargo clippy -p genossi_bin --lib` → 1 pre-existing warning (unused `Auditable` import at lib.rs:966, NOT in my code). 0 new warnings.
- Smoke: `DATABASE_URL=sqlite::memory: timeout 5 cargo run --bin genossi` → "Mail worker started", "Running server at 0.0.0.0:3000", no panic.

---
*Phase: 10-massenmail-anbindung-template-variablen*
*Plan: 07*
*Completed: 2026-05-31*
