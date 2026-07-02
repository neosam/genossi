---
phase: 22
plan: 03
subsystem: docs/operations
tags: [runbook, smtp, mail, 8bitmime, mail-04]
status: complete
requires: []
provides:
  - docs/OPERATIONS.md
  - MAIL-04 verify-in-prod runbook
affects: []
tech_stack_added: []
patterns: [operator-runbook, verify-in-prod, tolerant-config-fallback]
key_files_created:
  - docs/OPERATIONS.md
key_files_modified: []
decisions:
  - "Chose docs/OPERATIONS.md as the new home for operator runbooks (RESEARCH § Runbook Location Research). docs/ previously only contained audit/, so no clash; the file is set up to host further ##-sections for Phases 23+."
  - "Documented the exact order verbindlich (Schritt 1 before Schritt 2), with Step 2 explicitly gated by \"Nur wenn Schritt 1 grün ist\" — mirrors D-12 and T-22-09 typographic mitigation."
  - "Rollback documented as equivalent options (explicit quoted-printable OR delete key) — matches the tolerant fallback landed by Plan 01 in load_smtp_config."
metrics:
  duration: ~5min
  tasks_completed: 1
  files_created: 1
  files_modified: 0
  completed_date: 2026-07-02
requirements:
  - MAIL-04
---

# Phase 22 Plan 03: 8bit + Shared Mail-Body Helper Summary

Operator-facing runbook `docs/OPERATIONS.md` created with the MAIL-04 § "SMTP-Encoding umschalten (MAIL-04)" section — documents the verify-in-prod ONE-SHOT `openssl s_client` / EHLO / `250-8BITMIME` check that must precede any flip of the `smtp_encoding` config key from the safe `quoted-printable` default to `8bit`.

## What Was Built

A single new file — `docs/OPERATIONS.md` — with:

- Top-level `# Operations Runbook` header + short scope-clarifying intro.
- `## SMTP-Encoding umschalten (MAIL-04)` section containing:
  - Rationale in German: default `quoted-printable` is safe, `8bit` is opt-in and requires prod-relay `8BITMIME` advertisement, dev cannot reach prod relay.
  - `### Schritt 1 — 8BITMIME am Relay verifizieren`: exact `openssl s_client -starttls smtp -connect <relay-host>:<port> -crlf` command in a fenced code block, `EHLO genossi.local` follow-up, expected `250-8BITMIME` line in its own code block, and explicit negative-case wording ("Wenn diese Zeile FEHLT, ist Schritt 2 nicht erlaubt").
  - `### Schritt 2 — Config-Toggle setzen`: gated with "Nur wenn Schritt 1 grün ist"; documents key `smtp_encoding` = `8bit`, no service restart (SmtpConfig loaded fresh per send), key lives in existing Config-UI / `config_entries` KV table.
  - `### Rollback`: setting `smtp_encoding` back to `quoted-printable` OR deleting the key (tolerant fallback re-engages `MailEncoding::QuotedPrintable`).
  - Closing note: future ops runbooks (Phase 23+) live in this file under new `##` sections.

## Verification

All 14 grep-based acceptance criteria from `22-03-PLAN.md` Task 1 pass:

| Check | Expected | Actual |
|-------|----------|--------|
| `test -f docs/OPERATIONS.md` | file exists | OK |
| `grep -c '^# Operations Runbook'` | 1 | 1 |
| `grep -c '^## SMTP-Encoding umschalten (MAIL-04)'` | 1 | 1 |
| `grep -c 'openssl s_client -starttls smtp'` | ≥ 1 | 1 |
| `grep -c '250-8BITMIME'` | ≥ 1 | 2 |
| `grep -c '8BITMIME'` | ≥ 2 | 5 |
| `grep -c 'smtp_encoding'` | ≥ 2 | 3 |
| `grep -c 'quoted-printable'` | ≥ 1 | 3 |
| `grep -c '### Schritt 1'` | 1 | 1 |
| `grep -c '### Schritt 2'` | 1 | 1 |
| `grep -c '### Rollback'` | 1 | 1 |
| `grep -c 'EHLO'` | ≥ 1 | 5 |
| `grep -Ec 'cargo\|xtask\|#\[test\]\|cargo test'` | 0 | 0 |
| `grep -Ec 'APDOC\|EDIT-\|HTML-'` | 0 | 0 |

## Cross-reference alignment with Plan 01

Confirmed at commit time: the config-key name `smtp_encoding` used in the runbook matches the KV key parsed by `load_smtp_config` at `genossi_mail/src/service.rs:186`:

```
genossi_mail/src/service.rs:186:    let encoding = match find("smtp_encoding").map(|e| e.value.as_ref()) {
```

The runbook's Step 2 and Rollback wording ("Key löschen — Default greift automatisch") match Plan 01's tolerant fallback behavior (`Some("") | None => MailEncoding::QuotedPrintable`).

## Deviations from Plan

None. Plan executed exactly as written; the drafted structure from RESEARCH § "Content shape for D-12" was adopted with small German-prose polish.

## Threat Flags

None — this plan is a documentation deliverable with no new code surface, no network endpoints, no auth changes, no data-flow changes. The two threats in the plan's `<threat_model>` (T-22-09 operator-error via wrong order, T-22-10 repudiation of documentation) are both mitigated by the shipped artifact: order is typographically enforced (Schritt 1 precedes Schritt 2, Schritt 2 opens with "Nur wenn Schritt 1 grün ist"), and the doc lives in the repo under jj version control.

## Commits

- `6f97ce46` — docs(22-03): add MAIL-04 SMTP-Encoding runbook in docs/OPERATIONS.md

## Self-Check: PASSED

- `docs/OPERATIONS.md` exists (verified via `test -f`).
- All 14 grep-based acceptance criteria pass (see table above).
- Cross-reference key name `smtp_encoding` matches `genossi_mail/src/service.rs:186` (Plan 01).
