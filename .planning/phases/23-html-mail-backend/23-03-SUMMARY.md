---
phase: 23-html-mail-backend
plan: 03
subsystem: mail
tags: [rust, lettre, mime, multipart, alternative, html]

requires:
  - phase: 23-html-mail-backend
    plan: 02
    provides: RenderedContent { subject, body, body_html } + sanitize_html + html_env
provides:
  - "genossi_mail::send::build_message(from, to, subject, body, html_body, attachments, in_reply_to, encoding) — extended signature with 4-branch alternative/mixed decision tree"
  - "multipart/alternative{text, html} MIME shape (no attachments) verified byte-level"
  - "multipart/mixed{multipart/alternative{text, html}, attachments...} MIME shape verified byte-level"
  - "Text-first ordering in alternative wrapper pinned by byte-offset assertion (RESEARCH Pitfall 5)"
affects: [23-04-worker-persist]

tech-stack:
  added: []
  patterns:
    - "4-branch (html_body, attachments) match tree — RESEARCH Pattern 3"
    - "MultiPart::alternative().singlepart(text_part).singlepart(html_part) — text FIRST (RFC 2046 §5.1.4)"
    - "MultiPart::mixed().multipart(alternative) — nested alternative inside mixed for the html+attachments case"

key-files:
  created:
    - .planning/phases/23-html-mail-backend/23-03-SUMMARY.md
  modified:
    - genossi_mail/src/send.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/service.rs

key-decisions:
  - "html_body parameter positioned between body and attachments (D-10) — no tuple/struct wrapper, natural read order text-then-html-then-attach"
  - "Same CTE (Content-Transfer-Encoding) applied to both text and HTML SinglePart (D-01 encoding config uniformity)"
  - "Text part is raw body, never derived from html_body (HTML-02) — pinned by build_message_alternative_text_part_is_verbatim_body test"
  - "Text-first ordering pinned by explicit byte-offset assertion (text.find('text/plain') < text.find('text/html')) — RFC 2046 §5.1.4 preference model / RESEARCH Pitfall 5"
  - "3 non-test call sites received mechanical `None,` insertion — worker.rs one, service.rs two — Plan 04 will replace with real values"
  - "Nested wrapping for html+attachments: mixed{alternative{text,html}, attachments...} — the alternative is the FIRST body inside the outer mixed; RFC 2046 conformant"

patterns-established:
  - "4-branch match on (html_part_opt, attachments.is_empty()) — single decision tree, all four MIME shapes visible on one screen"
  - "Byte-offset ordering assertions (text.find(A) < text.find(B)) for MIME structure verification — stronger than 'contains' checks"

requirements-completed: [HTML-01, HTML-02]

coverage:
  tests_added: 5
  by_module:
    - "genossi_mail::send (5): build_message_alternative_text_then_html_no_attachments, build_message_alternative_text_part_is_verbatim_body, build_message_mixed_wraps_alternative_when_attach, build_message_legacy_singlepart_text_unchanged, build_message_html_part_declares_text_html_charset_utf8"
  updated:
    - "7 pre-existing send::tests updated with mechanical None insertion (no test intent change) — the Phase-22 MAIL-01/02/05 regression bar remains green"

metrics:
  duration_minutes: ~8
  completed: "2026-07-02"

status: complete
---

# Phase 23 Plan 03: MIME assembly — multipart/alternative Summary

**One-liner:** Extend `build_message` in `genossi_mail::send` with an optional `html_body: Option<&str>` and a 4-branch decision tree producing `multipart/alternative{text, html}` (nested inside `mixed{…, attachments}` when attachments are present), with text-first ordering pinned by byte-offset assertion.

## Objective

Wire the shared `build_message` factory from Phase 22 with the HTML-01/D-09 MIME shape:

- `html_body=None` — Phase-22 legacy paths preserved byte-identically (regression contract).
- `html_body=Some` — new `multipart/alternative{text-first, html-second}` shape (with optional nested wrapping in `multipart/mixed` when attachments accompany it).

This is a **single-file behavioral change** in `send.rs` with a mechanical `None,` insertion at the three non-test call sites to keep the crate build-green. Plan 04 will replace those `None`s with real rendered HTML values from `RenderedContent`.

## What Was Built

### Task 1: `build_message` signature evolution + 4-branch decision tree (commit `90484d48`)

**Updated `build_message` signature:**

```rust
pub fn build_message(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    html_body: Option<&str>,               // NEW — HTML-01/D-09
    attachments: &[LoadedAttachment],
    in_reply_to: Option<&str>,
    encoding: MailEncoding,
) -> Result<Message, MailServiceError>
```

**Byte-shape decisions for the four `(html_body, attachments)` combinations:**

| `html_body` | `attachments` | Output MIME shape | Notes |
|-------------|---------------|-------------------|-------|
| `None`      | `[]`          | `SinglePart` `text/plain` | Phase-22 legacy — byte-identical. |
| `None`      | `[a…]`        | `multipart/mixed{ text/plain, a… }` | Phase-22 legacy — byte-identical. |
| `Some(html)`| `[]`          | `multipart/alternative{ text/plain (first), text/html (second) }` | Text-first per RFC 2046 §5.1.4 (Pitfall 5). |
| `Some(html)`| `[a…]`        | `multipart/mixed{ multipart/alternative{ text/plain, text/html }, a… }` | Outer mixed wraps alt + attachments (HTML-01 with attach). |

**Implementation:** A single `match (html_part_opt, attachments.is_empty()) { … }` statement covering all four arms — the entire decision tree is visible on one screen.

### Five new MIME-byte tests (send.rs::tests)

| Test | Assertion summary |
|------|-------------------|
| `build_message_alternative_text_then_html_no_attachments` | Output contains `multipart/alternative`, does NOT contain `multipart/mixed`, AND `text.find("text/plain") < text.find("text/html")` (byte-offset ordering — Pitfall 5). |
| `build_message_alternative_text_part_is_verbatim_body` | Text part contains verbatim `"plain-verbatim-marker"`, HTML part contains `"totally-different-html-marker"`, and the plain marker is NOT embedded inside `<p>…</p>` — pins HTML-02 no-derivation contract. |
| `build_message_mixed_wraps_alternative_when_attach` | Output contains BOTH `multipart/mixed` AND `multipart/alternative`; `find("multipart/mixed") < find("multipart/alternative")` (outer mixed, inner alt); attachment payload (`application/pdf` or `test.pdf`) present. |
| `build_message_legacy_singlepart_text_unchanged` | Output does NOT contain `multipart/` at all AND declares `Content-Type: text/plain` — HTML-01 "legacy stays legacy" regression bar. |
| `build_message_html_part_declares_text_html_charset_utf8` | Output contains `text/html` AND `charset=utf-8` appears in the substring AFTER the `text/html` header — pins MAIL-01-style charset contract on the HTML part. |

### Mechanical `None,` insertion at 3 non-test call sites

| File | Line (approx) | Function | Purpose of `None` |
|------|---------------|----------|-------------------|
| `genossi_mail/src/worker.rs` | 668 | `send_mail_for_recipient` | Bulk worker — Plan 04 replaces with `rendered.body_html.as_deref()`. |
| `genossi_mail/src/service.rs` | 458 | `send_test_mail` | Smoke-test only — stays `None` permanently (no HTML for the SMTP-config test-mail). |
| `genossi_mail/src/service.rs` | 492 | `send_test_mail_with_body` | Template-test path — Plan 04 replaces with the rendered HTML body. |

### Existing Phase-22 tests preserved

All 7 pre-existing `send::tests` received the mechanical `None,` positional-argument insertion — no test intent changes. This preserves the Phase-22 MAIL-01/02/05 byte-shape regression bar (charset=utf-8, QP vs 8bit CTE, multipart/mixed attachment shape, In-Reply-To/References headers, Message-ID auto-generation, malformed address error paths).

## Verification

- `cargo build -p genossi_mail` → 0 errors.
- `cargo test -p genossi_mail --lib send::` → **12 passed / 0 failed** (7 Phase-22 + 5 new = 12; matches acceptance criterion `>= 12`).
- `cargo test -p genossi_mail --lib` → **242 passed / 0 failed** (was 237 before this plan; +5 net new tests match `tests_added: 5`).
- `cargo clippy -p genossi_mail --lib` → clean for touched code (1 pre-existing warning `unnecessary_sort_by` at `worker.rs:105` — documented as out-of-scope by Plan 22-02 SUMMARY; scope-boundary rule applies).
- Grep-based invariants verified per plan's acceptance criteria:
  - `grep -c 'html_body: Option<&str>' genossi_mail/src/send.rs` → **1** ✓
  - `grep -c 'MultiPart::alternative()' genossi_mail/src/send.rs` → **2** (>= 1) ✓
  - `grep -c 'ContentType::TEXT_HTML' genossi_mail/src/send.rs` → **1** (>= 1) ✓
  - `grep -c 'match (html_part_opt' genossi_mail/src/send.rs` → **1** ✓
  - Test-fn count grep → **5** ✓
  - `grep -c 'crate::send::build_message' genossi_mail/src/worker.rs` → **1** ✓
  - `grep -c 'crate::send::build_message' genossi_mail/src/service.rs` → **2** ✓
  - `grep -c 'find("text/plain")' genossi_mail/src/send.rs` → **1** (byte-offset ordering assertion present — Pitfall 5 pinned) ✓

## Phase-22 Regression Contract

The **`(None, …)` paths of the 4-branch tree produce byte-identical output to Phase-22.** Verified by:

- The `(None, true)` arm calls `builder.singlepart(text_part)` — literally the same expression as the Phase-22 `attachments.is_empty()` branch.
- The `(None, false)` arm reconstructs the same `MultiPart::mixed().singlepart(text_part).<attachment loop>` — line-for-line equivalent to the Phase-22 else-branch.
- All 7 Phase-22 tests still pass (which they wouldn't if the byte-shape had drifted; they assert exact CTE headers, charsets, `multipart/mixed`, In-Reply-To/References text, etc.).

MAIL-01/02/05 regression bar remains green.

## Deviations from Plan

**None significant.**

- Plan spec said "5 new tests" and specified exactly 5 test names in its acceptance criterion — matched exactly.
- Plan spec said "grep for `find(\"text/plain\")` … at least 1 occurrence" — implementation has exactly 1 (in `build_message_alternative_text_then_html_no_attachments`; the second byte-offset test uses `find("multipart/mixed")` / `find("multipart/alternative")` since it verifies outer/inner wrapping rather than alternative ordering).
- Plan spec said `MultiPart::alternative()` `>= 1`; implementation has 2 — once in the `(Some, true)` arm, once in the `(Some, false)` arm. Both are production code, no test-side occurrence. Justified: DRY-refactoring to a shared local `let alt = …` binding would have reduced this to 1 but obscured the arm-by-arm shape parity; kept per-arm construction for readability. Well within `>= 1` acceptance.
- Doc-comment update: I extended the module-level doc-comment on `build_message` with a one-line description of the new `html_body` parameter, per the plan's `<action>` instructions.

## Threat Model Compliance

- **T-23-06 (Spoofing, mail-header injection via body / html_body newlines)** — mitigated by construction: both bodies enter via `SinglePart::body(String)`; lettre composes MIME headers from typed inputs, cannot be injected by payload bytes. No test needed per plan's disposition rationale.
- **T-23-07 (Tampering, alternative ordering downgrade)** — pinned by test `build_message_alternative_text_then_html_no_attachments`, which asserts `text.find("text/plain") < text.find("text/html")`. RFC 2046 §5.1.4 preference model enforced by construction (text `.singlepart()` first, then HTML `.singlepart()`) AND by regression test.

## Files Touched

| File | Kind | Purpose |
|------|------|---------|
| `genossi_mail/src/send.rs` | modified | `build_message` signature + 4-branch match tree + 5 new tests + mechanical None in 7 existing tests |
| `genossi_mail/src/worker.rs` | modified | Mechanical `None,` insertion at `send_mail_for_recipient` call site |
| `genossi_mail/src/service.rs` | modified | Mechanical `None,` insertion at `send_test_mail` and `send_test_mail_with_body` call sites |

## Commits (jj)

| Task | Commit | Description |
|------|--------|-------------|
| 1 | `90484d48` | `feat(23-03): extend build_message with html_body for multipart/alternative` |

## Self-Check: PASSED

- `genossi_mail/src/send.rs` — MODIFIED, all 5 new test fns present, `match (html_part_opt` present, `html_body: Option<&str>` present exactly once
- `genossi_mail/src/worker.rs` — MODIFIED, call site count = 1
- `genossi_mail/src/service.rs` — MODIFIED, call site count = 2
- Commit `90484d48` — FOUND in `jj log`
- `cargo build -p genossi_mail` — OK
- `cargo test -p genossi_mail --lib send::` — 12 passed / 0 failed
- `cargo test -p genossi_mail --lib` — 242 passed / 0 failed
- Text-first ordering byte-offset assertion — PRESENT (`find("text/plain")` at 1 occurrence, `< find("text/html")` compared)
