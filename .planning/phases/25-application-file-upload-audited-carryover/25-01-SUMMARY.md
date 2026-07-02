---
phase: 25-application-file-upload-audited-carryover
plan: 01
subsystem: docs
tags: [requirements, roadmap, apdoc, move-semantics, ownership-transfer]

# Dependency graph
requires:
  - phase: 25-discuss/plan
    provides: "CONTEXT.md decision #3 (Move semantics — Ownership-Übergabe) and 25-RESEARCH.md 'Runtime Requirements Wording Update (APDOC-03)' verbatim replacement text"
provides:
  - "APDOC-03 wording aligned in REQUIREMENTS.md and ROADMAP.md on Move / Ownership-Übergabe semantics"
  - "Contradicting 'Move-on-Activation' bullet removed from REQUIREMENTS.md Out-of-Scope"
  - "MemberDocument description format spelled out verbatim in both docs: 'Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)'"
affects: [25-02-PLAN, 25-03-PLAN, 25-04-PLAN, 25-05-PLAN, verify-work, audit-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Documentation-first: doku-fix runs as Wave 1a before any code so subsequent plans check against consistent APDOC-03"

key-files:
  created:
    - .planning/phases/25-application-file-upload-audited-carryover/25-01-SUMMARY.md
  modified:
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "APDOC-03 wording follows CONTEXT.md decision #3: Move semantics (Ownership-Übergabe), not Copy"
  - "Both source-of-truth docs use verbatim identical description format string for the transferred MemberDocument"
  - "Removed the Out-of-Scope 'Move-on-Activation' bullet entirely (would have contradicted the new APDOC-03 wording)"

patterns-established:
  - "Wording-drift fix: when a discussion decision inverts an earlier printed requirement, ship the doku sync as its own Wave 1a plan before implementation, so reviewers/auditors always see a consistent goal"

requirements-completed: []  # APDOC-03 stays 'pending' in the Traceability table — this plan only fixes the wording, not the implementation. Implementation lives in 25-04-PLAN.

coverage:
  - id: D1
    description: "REQUIREMENTS.md APDOC-03 rewritten from 'kopiert (nicht verschoben)' to 'übernommen (Ownership-Übergabe — Move-Semantik)'; description format 'Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)' added"
    requirement: "APDOC-03"
    verification:
      - kind: automated_ui
        ref: "grep -c 'kopiert (nicht verschoben)' .planning/REQUIREMENTS.md == 0"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'Ownership-Übergabe' .planning/REQUIREMENTS.md >= 1"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)' .planning/REQUIREMENTS.md >= 1"
        status: pass
    human_judgment: false
  - id: D2
    description: "ROADMAP.md Phase 25 Success Criteria #3 rewritten to match REQUIREMENTS.md APDOC-03 verbatim on the move-semantics claim and the description format string"
    requirement: "APDOC-03"
    verification:
      - kind: automated_ui
        ref: "grep -c 'kopiert (nicht verschoben)' .planning/ROADMAP.md == 0"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'Ownership-Übergabe' .planning/ROADMAP.md >= 1"
        status: pass
      - kind: automated_ui
        ref: "grep -c 'Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)' .planning/ROADMAP.md >= 1"
        status: pass
    human_judgment: false
  - id: D3
    description: "Contradicting 'Move-on-Activation: Antrags-Dokument wird kopiert, nicht verschoben' bullet removed from REQUIREMENTS.md Out-of-Scope section"
    requirement: "APDOC-03"
    verification:
      - kind: automated_ui
        ref: "grep -c 'Move-on-Activation' .planning/REQUIREMENTS.md == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Traceability table in REQUIREMENTS.md preserves 'APDOC-03 | Phase 25 | pending' — this plan does NOT flip status (implementation ships in 25-04)"
    requirement: "APDOC-03"
    verification:
      - kind: automated_ui
        ref: "grep -q 'APDOC-03 | Phase 25 | pending' .planning/REQUIREMENTS.md"
        status: pass
    human_judgment: false

# Metrics
duration: 2min
completed: 2026-07-02
status: complete
---

# Phase 25 Plan 01: APDOC-03 wording sync (Move semantics) Summary

**REQUIREMENTS.md and ROADMAP.md aligned on Move / Ownership-Übergabe semantics for APDOC-03 with the verbatim MemberDocument description format „Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)".**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-07-02T23:55:42Z
- **Completed:** 2026-07-02T23:57:xxZ
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Rewrote APDOC-03 in `REQUIREMENTS.md` from "kopiert (nicht verschoben)" to "übernommen (Ownership-Übergabe — Move-Semantik: die `application_documents`-Zeile wird soft-deleted und die Datei physisch an den Member-Pfad verschoben)"
- Rewrote ROADMAP.md Phase 25 Success Criteria #3 to match verbatim on the move-semantics claim and description format
- Spelled out the auditable MemberDocument description format in both docs: „Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)"
- Removed the contradicting "Move-on-Activation: Antrags-Dokument wird kopiert, nicht verschoben" bullet from the Out-of-Scope section
- Preserved the Traceability table entry `APDOC-03 | Phase 25 | pending` (no accidental status flip — implementation ships in 25-04)

## Task Commits

Each task was committed atomically via jj:

1. **Task 1: APDOC-03 wording sync in REQUIREMENTS.md and ROADMAP.md + remove contradicting Out-of-Scope bullet** — `bc825850` (docs)

## Files Created/Modified

- `.planning/REQUIREMENTS.md` — APDOC-03 line rewritten (Move / Ownership-Übergabe wording + description format); Out-of-Scope "Move-on-Activation" bullet removed
- `.planning/ROADMAP.md` — Phase 25 Success Criteria #3 rewritten to match REQUIREMENTS.md verbatim

## Decisions Made

None new — this plan implements CONTEXT.md decision #3 (Move semantics — Ownership-Übergabe) verbatim per 25-RESEARCH.md `## Runtime Requirements Wording Update (APDOC-03)`.

## Deviations from Plan

None — plan executed exactly as written. Three scoped Edit operations, all acceptance-criteria greps returned expected counts (0/0/0 for the old wording, ≥1/≥1 for the new wording in both files, traceability row untouched).

## Issues Encountered

None.

## User Setup Required

None — documentation-only change.

## Next Phase Readiness

- APDOC-03 is now consistent across both source-of-truth documents; plans 25-02..25-05 can implement Move semantics without appearing as a scope reduction against the printed requirement.
- Plan 25-02 (SQLx migration + `ApplicationDocumentDao`) is unblocked and can run in Wave 1 in parallel with any follow-up doku plans.

## Self-Check: PASSED

- SUMMARY.md exists at `.planning/phases/25-application-file-upload-audited-carryover/25-01-SUMMARY.md`
- Commit `bc825850` found in jj history with message `docs(25-01): sync APDOC-03 wording to Move semantics (per CONTEXT.md decision #3)`
- All acceptance-criteria greps pass (0 old-wording hits in both files, ≥1 new-wording hits in both files, traceability row preserved)

---
*Phase: 25-application-file-upload-audited-carryover*
*Completed: 2026-07-02*
