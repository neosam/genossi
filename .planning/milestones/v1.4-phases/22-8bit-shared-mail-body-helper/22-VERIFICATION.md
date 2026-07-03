---
phase: 22-8bit-shared-mail-body-helper
verified: 2026-07-02T22:15:00Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 22: 8bit + Shared Mail-Body Helper Verification Report

**Phase Goal:** Alle ausgehenden Mails laufen über einen einzigen Body-Bau-Helfer mit konsistentem `charset=utf-8`, und der Text-Teil kann (config-gated) als 8bit gesendet werden, sodass Empfänger keine sichtbaren `=`-Soft-Line-Breaks mehr sehen.

**Verified:** 2026-07-02T22:15:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | MAIL-01: All three send paths converge on single shared `build_message` in `genossi_mail::send` | VERIFIED | `grep -c 'crate::send::build_message'` → worker.rs:1, service.rs:2 = 3 total call sites (worker.rs:662, service.rs:455, service.rs:489). Additionally `grep -rn 'Message::builder()' genossi_mail/src/` returns only send.rs:83 (production) + service.rs:881 (doc `///` comment allowed). digest.rs uses `send_test_mail_with_body` (digest.rs:176), inheriting the fix transitively. |
| 2 | MAIL-02: `MailEncoding::EightBit` yields `Content-Transfer-Encoding: 8bit` + `charset=utf-8` byte-exact; no `=\r\n` in body | VERIFIED | send.rs test `build_message_8bit_has_utf8_charset_and_8bit_cte` (lines 156-194) asserts `charset=utf-8`, `Content-Transfer-Encoding: 8bit`, absence of `quoted-printable` CTE, and absence of runtime-constructed `=\r\n` QP soft-break. Test passes (verified via `cargo test -p genossi_mail --lib`). |
| 3 | MAIL-03: `smtp_encoding` KV config key with tolerant fallback; enum-based (`MailEncoding`), no bool | VERIFIED | `pub enum MailEncoding { QuotedPrintable, EightBit }` at service.rs:126-130. `SmtpConfig.encoding: MailEncoding` field at service.rs:139. `load_smtp_config` parses `smtp_encoding` at service.rs:186-196 with match arms for `Some("8bit")`, `Some("quoted-printable") | Some("") | None`, and wildcard warn+fallback. Three named tests exist and pass: `load_smtp_config_defaults_encoding_to_qp_when_key_missing`, `load_smtp_config_reads_encoding_8bit_when_set`, `load_smtp_config_falls_back_on_unknown_encoding_value`. No `bool` or `smtp_8bit` in service.rs. |
| 4 | MAIL-04: `docs/OPERATIONS.md` § "SMTP-Encoding umschalten (MAIL-04)" documents `openssl s_client` 8BITMIME check | VERIFIED | File exists (53 lines). Contains `# Operations Runbook`, `## SMTP-Encoding umschalten (MAIL-04)`, `### Schritt 1 — 8BITMIME am Relay verifizieren` with `openssl s_client -starttls smtp -connect <relay-host>:<port> -crlf` command, expected `250-8BITMIME` line, `### Schritt 2 — Config-Toggle setzen` gated on Schritt 1, and `### Rollback` documenting either explicit `quoted-printable` or key-delete. Order enforced typographically ("Nur wenn Schritt 1 grün ist"). |
| 5 | MAIL-05: default `SmtpConfig.encoding = QuotedPrintable` preserves existing byte shape | VERIFIED | `load_smtp_config` at service.rs:188 maps `Some("quoted-printable") | Some("") | None` → `MailEncoding::QuotedPrintable`. Test `load_smtp_config_defaults_encoding_to_qp_when_key_missing` (service.rs:1188) confirms behavior. send.rs test `build_message_qp_has_utf8_charset_and_non_7bit_cte` (send.rs:123-154) asserts QP mode yields `charset=utf-8` + non-7bit CTE (`quoted-printable` or `base64`) + NOT `8bit`. Existing tests (224 pass) confirm backward compat. |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `genossi_mail/src/send.rs` | New shared factory | VERIFIED | 338 lines, contains `pub struct LoadedAttachment`, `pub fn build_message` (sync, not async), 7 unit tests. Imports `MailEncoding` from `crate::service`. No `DocumentStorage` reference (D-05 respected). |
| `genossi_mail/src/lib.rs` | `pub mod send;` declaration | VERIFIED | Confirmed via grep (1 occurrence). |
| `genossi_mail/src/service.rs` | `MailEncoding` enum + `SmtpConfig.encoding` field + `smtp_encoding` parse block + 3 fallback tests + rewired test-mail paths | VERIFIED | All present: enum (lines 126-130), field (line 139), parse block (lines 186-196), three tests (lines 1174-1240), `send_test_mail` (lines 447-471) and `send_test_mail_with_body` (lines 473-505) both call `crate::send::build_message`. Body literal + Quick 260603-jtf comment preserved verbatim. |
| `genossi_mail/src/worker.rs` | `send_mail_for_recipient` rewired via `build_message` | VERIFIED | Lines 627-688: loads attachment bytes into `Vec<LoadedAttachment>` (D-03), calls `crate::send::build_message` with `smtp_config.encoding` (line 662), captures Message-ID header, then `transport.send`. No `Message::builder()`, `SinglePart::plain`, `MultiPart::mixed`, or `ContentTransferEncoding` remain. |
| `docs/OPERATIONS.md` | Operator runbook for MAIL-04 | VERIFIED | 53 lines, all grep-based acceptance criteria pass (see Truth 4). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|------|--------|---------|
| `send_mail_for_recipient` (worker.rs:627) | `build_message` (send.rs:46) | `crate::send::build_message` call at worker.rs:662 with `smtp_config.encoding` | WIRED | Direct function call; verified via grep and cargo build. |
| `send_test_mail` (service.rs:447) | `build_message` (send.rs:46) | `crate::send::build_message` call at service.rs:455 | WIRED | `&[], None, smtp_config.encoding` args match D-04 contract. |
| `send_test_mail_with_body` (service.rs:473) | `build_message` (send.rs:46) | `crate::send::build_message` call at service.rs:489 | WIRED | Same signature shape; privacy-defense comment preserved. |
| `digest.rs::worker_tick` | `send_test_mail_with_body` | `mail_service.send_test_mail_with_body(...)` at digest.rs:176 | WIRED | Unchanged; inherits fix via rewired service.rs method. |
| `load_smtp_config` (service.rs:142) | `SmtpConfig.encoding` | KV lookup `smtp_encoding` at service.rs:186-196 | WIRED | Tolerant fallback: 8bit → EightBit; qp/empty/None → QP; unknown → warn+QP. |
| `docs/OPERATIONS.md` | Config-key `smtp_encoding` | Cross-reference string match | WIRED | Doc mentions `smtp_encoding` (3 occurrences), matching the KV key parsed at service.rs:186. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| genossi_mail crate builds | `cargo build -p genossi_mail` | Finished dev profile in 0.71s | PASS |
| genossi_mail lib tests pass | `cargo test -p genossi_mail --lib` | 224 passed, 0 failed | PASS |
| send.rs tests exist and pass | `cargo test -p genossi_mail --lib -- --list \| grep send::tests` | 7 tests enumerated (build_message_* all present) | PASS |
| load_smtp_config tests exist | `cargo test -p genossi_mail --lib -- --list \| grep load_smtp_config` | 3 fallback tests enumerated | PASS |
| Full workspace builds | `cargo build` | Finished dev profile in 1m 09s | PASS |
| `Message::builder()` outside send.rs | `grep -rn 'Message::builder()' genossi_mail/src/` filtered | Only 1 occurrence at service.rs:881 (inside `///` doc comment; allowed) | PASS |
| digest.rs unchanged | `grep 'send_test_mail_with_body' genossi_mail/src/digest.rs` | Line 176: `.send_test_mail_with_body(recipient, &subject, &body)` — routes through rewired service.rs method | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| MAIL-01 | 22-02 | Single shared body-build helper for all outgoing mail | SATISFIED | Truth 1 above |
| MAIL-02 | 22-02 | 8bit encoding option eliminates visible `=` soft-line-breaks | SATISFIED | Truth 2 above |
| MAIL-03 | 22-01 | Encoding switchable via config; QP default | SATISFIED | Truth 3 above |
| MAIL-04 | 22-03 | Documented 8BITMIME verify step for prod relay | SATISFIED (doc deliverable) | Truth 4 above. REQUIREMENTS.md marks MAIL-04 `pending` because the operator step must be executed against prod; the code+doc deliverable is complete. |
| MAIL-05 | 22-02 | Existing plain-text mail unchanged (backward compat) | SATISFIED | Truth 5 above |

### Anti-Patterns Found

No anti-patterns detected. Notable observations:
- No debt markers (`TBD`, `FIXME`, `XXX`) introduced by this phase's modified files.
- Pre-existing `clippy::unnecessary_sort_by` warning at worker.rs:105 remains (documented in both 22-01-SUMMARY.md and 22-02-SUMMARY.md as pre-existing, out-of-scope; not introduced by Phase 22).
- Pre-existing e2e test failure `test_mail_preview_repayment_no_entries_does_not_default_to_one` documented as pre-existing on parent commit `4a1d2747` (unrelated to mail-send path, exercises `/api/mail/preview` render path).

### D-01..D-12 Decision Verification

| Decision | Status | Evidence |
|----------|--------|----------|
| D-01: `build_message` extracted to shared sync module | VERIFIED | `pub fn build_message` at send.rs:46 (sync, not async). |
| D-02: Seam at loaded-attachment bytes; no DocumentStorage | VERIFIED | `LoadedAttachment { file_name, mime_type, bytes }` at send.rs:26-31; `grep DocumentStorage send.rs` returns 0. |
| D-03: Attachment loading stays in worker (async I/O) | VERIFIED | worker.rs:645-660 loops `document_storage.load()` then constructs `LoadedAttachment`. |
| D-04: Test-mail + digest converge on build_message | VERIFIED | service.rs:455, service.rs:489 call build_message with `&[], None`; digest.rs:176 delegates to send_test_mail_with_body. |
| D-05: Test-mail sync, no MailJob persistence, no DI generic | VERIFIED | Neither send_test_mail method persists MailJob; no DocumentStorage generic on MailServiceImpl. |
| D-06: Three `.parse()` blocks consolidated into build_message | VERIFIED | `grep 'Invalid from address' worker.rs service.rs` returns 0 (only present in send.rs). |
| D-07: `MailEncoding` enum, no bool | VERIFIED | service.rs:126-130 with 2 variants; `grep 'smtp_8bit\|encoding: bool' service.rs` returns 0. |
| D-08: `smtp_encoding` KV key with tolerant fallback | VERIFIED | service.rs:186-196 with warn+fallback for unknown values; 3 tests confirm behavior. |
| D-09: `SinglePart::builder()` with explicit CTE header in both branches | VERIFIED | send.rs:72-79 uses match on encoding + `SinglePart::builder().header(ContentType::TEXT_PLAIN).header(cte).body(...)`; no `SinglePart::plain` in send.rs. |
| D-10: build_message is the tested single source (both modes) | VERIFIED | 7 tests in send.rs including both encoding modes (QP + 8bit) with byte-level assertions. |
| D-11: Worker tests call build_message instead of re-inlining | VERIFIED | 5 legacy worker MIME-byte tests deleted (`grep 'plain_mail_body_has_utf8_charset\|multipart_mail_body_has_utf8_charset\|reply_mail_includes_in_reply_to_header\|non_reply_mail_has_no_in_reply_to_header\|built_message_exposes_message_id_header' worker.rs` returns 0), covered by send.rs tests. |
| D-12: Runbook documents openssl s_client + 8BITMIME check | VERIFIED | docs/OPERATIONS.md § "SMTP-Encoding umschalten (MAIL-04)". |

### Gaps Summary

No gaps. All phase Success Criteria are met, all 5 MAIL-* requirements are satisfied at the code/doc level, and all 12 locked decisions from CONTEXT.md are implemented as specified.

**Note on MAIL-04 status in REQUIREMENTS.md:** MAIL-04 is marked `pending` in REQUIREMENTS.md because the operator-side execution of the openssl s_client check against the production relay is inherently out-of-band (requires prod-network access). The phase deliverable — the documented runbook — is complete. This is the intended split per D-12 ("verify-in-prod, aus Dev nicht automatisierbar").

---

_Verified: 2026-07-02T22:15:00Z_
_Verifier: Claude (gsd-verifier)_
