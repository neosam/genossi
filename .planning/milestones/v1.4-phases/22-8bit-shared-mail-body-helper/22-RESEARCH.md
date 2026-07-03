# Phase 22: 8bit + Shared Mail-Body Helper — Research

**Researched:** 2026-07-02
**Domain:** `genossi_mail` — lettre 0.11 MIME construction, MailEncoding config toggle, converging three divergent send paths
**Confidence:** HIGH — every load-bearing claim was verified against the vendored lettre 0.11.20 source in `~/.cargo/registry` and against the actual codebase.

## Summary

Phase 22 is a **narrow service-layer refactor** in `genossi_mail`. All twelve decisions D-01..D-12 are locked in `22-CONTEXT.md`. The scope is:

1. Extract `worker.rs::send_mail_for_recipient`'s **pure MIME construction** (lines 636–702) into one sync function `build_message(...)` in a new module.
2. Redirect `service.rs::send_test_mail` (lines 415–445), `service.rs::send_test_mail_with_body` (447–488), and — transitively — `digest.rs::worker_tick` (170–187) through `build_message`, which fixes the missing `charset=utf-8` bug at all three sites at once.
3. Add a `MailEncoding { QuotedPrintable, EightBit }` enum on `SmtpConfig`, driven by a new optional KV config key `smtp_encoding` (default `"quoted-printable"`), parsed with the same tolerant-fallback pattern as the existing `smtp_tls` key at `service.rs:163–165`.
4. Emit MIME-byte tests for BOTH encoding modes, calling `build_message` (not re-inlining lettre calls as today's tests at `worker.rs:977,1061` do).
5. Document the `8BITMIME` EHLO verification step for the operator to run against the prod relay before flipping the toggle.

**Primary recommendation:** Put `build_message` in a new file `genossi_mail/src/send.rs` with a struct `LoadedAttachment { file_name: Arc<str>, mime_type: Arc<str>, bytes: Vec<u8> }` (mirrors the existing `MailRecipientAttachment` shape at `dao.rs:103` — see D-02, load-first-then-build). Runbook goes in a new file `docs/OPERATIONS.md` § "SMTP-Encoding umschalten (MAIL-04)" — no `docs/` runbook file exists today (only `docs/audit/`), so create one. The existing `worker.rs` tests should be moved into `send.rs`'s `#[cfg(test)]` block as they naturally follow the code under test.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** Pure **synchronous** `build_message(...)` extracted from `worker.rs::send_mail_for_recipient` (worker.rs:627-720) into a shared location (planner picks the module name, e.g. `genossi_mail/src/send.rs`). It owns BOTH MIME pieces: (1) the text `SinglePart` with `charset=utf-8` + configurable CTE, and (2) the subject/`message_id(None)`/`in_reply_to`/`references`/`MultiPart::mixed()` wrapping for attachments.

**D-02:** The seam is at the **already-loaded attachment bytes**, NOT at `DocumentStorage`. `build_message` takes a list of loaded attachments (`filename`, `mime`, `bytes`) — so it needs no `DocumentStorage`, stays sync, and is purely MIME-testable.

**D-03:** Attachment **loading** (`document_storage.load().await`, worker.rs:678) stays path-specific in the worker (real async I/O). The worker loads bytes → calls `build_message(...)` → calls `transport.send()`.

**D-04:** Test-mail (`send_test_mail`, service.rs:415), test-mail-with-body (`send_test_mail_with_body`, service.rs:447), and digest (`digest.rs:174`, runs via `send_test_mail_with_body`) call `build_message(..., &[], None, encoding)` (empty attachments, no in_reply_to) + `transport.send()`. Test-mail runs through **exactly the same Message-construction code** as real send → the charset bug structurally cannot hide anymore.

**D-05:** Test-mail stays **synchronous** (immediate button feedback) and is NOT persisted. NO `DocumentStorage` generic on `MailServiceImpl` (no DI wiring in `genossi_bin`). Deliberate diff to rejected Option E.

**D-06:** Bonus consolidation: the three copied `.parse()` address blocks (worker.rs:642-650, service.rs:422-434, service.rs:464-477) move INTO `build_message` (from/to as `&str`, parsed there).

**D-07:** New internal enum `MailEncoding { QuotedPrintable, EightBit }` (never `bool`). Flows as parameter into `build_message` — the ONE place where CTE is decided.

**D-08:** New optional KV config key `smtp_encoding` with string values `"quoted-printable"` (default) / `"8bit"`, read in `load_smtp_config` (service.rs:127) analogous to `smtp_tls` (service.rs:163-165). Unknown/empty values fall cleanly back to the default. New field on `SmtpConfig` (service.rs:118-125). Default stays quoted-printable until operator opts in (MAIL-03).

**D-09:** For 8bit the body-part construction must switch from `SinglePart::plain()` to `SinglePart::builder().header(ContentType::TEXT_PLAIN).header(ContentTransferEncoding::EightBit).body(...)`. The QP branch can stay `SinglePart::plain` OR also set CTE explicitly — planner detail.

**D-10:** `build_message` is the tested **single source**. Unit tests assert on MIME byte level (`email.formatted()` + `String::from_utf8_lossy`, following existing worker tests) for BOTH `charset=utf-8` AND the `Content-Transfer-Encoding` in BOTH modes: `quoted-printable`/`base64` (default) and `8bit` (opt-in). 8bit CTE is byte-exact covered despite no prod-relay test.

**D-11:** Existing worker tests (`plain_mail_body_has_utf8_charset` worker.rs:977, `multipart_mail_body_has_utf8_charset` worker.rs:1061) should CALL `build_message` instead of RE-INLINING the build logic (today they duplicate it). Charset coverage for the test-mail/digest path is added (currently untested).

**D-12:** Slim **runbook/deployment-doc section** (planner picks location — operator doc) with the concrete `openssl s_client -starttls smtp -connect <relay>:<port>` → EHLO → check for `250-8BITMIME`, plus explicit order "**first** verify at prod relay, **then** set `smtp_encoding=8bit`". Verify-in-prod, cannot be automated from dev (relay reachable only through prod network).

### Claude's Discretion

- Exact module/file name for the shared function (`send.rs` or similar).
- Exact signature details (struct for attachment triple vs tuple slice).
- Whether the quoted-printable branch keeps `SinglePart::plain` or also explicitly sets CTE.
- Location of operator doc for D-12.

### Deferred Ideas (OUT OF SCOPE)

- **Option E** — persisted Test-Mails as hidden `MailJob` rows with a `MailJobKind::{Normal, Test}` enum: rejected for Phase 22 (would need schema migration + async test UX).
- **FMT-01** (German date format `DD.MM.YYYY` in template variables): belongs to **Phase 23**, not 22.
- HTML mail / `multipart/alternative` → Phase 23.
- WYSIWYG editor → Phase 24.
- Application file upload → Phase 25.
- **`html2text` derivation of text part**: not applicable to Phase 22, but explicitly rejected for the milestone.
- **`DocumentStorage` DI refactor on `MailServiceImpl`**: rejected in D-05.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MAIL-01 | Single shared body-build helper for worker/test-mail/digest, consistent `charset=utf-8` | D-01, D-04, D-06 realised via `build_message` in `send.rs`; three callers converge (§ "Cascade Check for Callers" below) |
| MAIL-02 | Text part can be encoded as `8bit` instead of `quoted-printable` | `lettre::message::header::ContentTransferEncoding::EightBit` exists (verified in `content.rs:26`, str = `"8bit"`); `SinglePart::builder().header(ContentType::TEXT_PLAIN).header(ContentTransferEncoding::EightBit).body(...)` is the sanctioned constructor per lettre docs (`mimebody.rs:66-77`) |
| MAIL-03 | Encoding is config-switchable, default remains QP | New `smtp_encoding` KV key with tolerant fallback like `smtp_tls` (§ "Configuration Plumbing" below); `SmtpConfig` gets `encoding: MailEncoding` field |
| MAIL-04 | Documented `8BITMIME` EHLO check before enabling 8bit in prod | Runbook section in `docs/OPERATIONS.md` (new file; `docs/` currently only holds `audit/` — no clash) |
| MAIL-05 | Existing plain-text mails unchanged with default config | `MailEncoding::QuotedPrintable` default path keeps semantic parity: `SinglePart::plain` already emits `text/plain; charset=utf-8` + `quoted-printable` (verified from `mimebody.rs:114-118`) |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `build_message` pure MIME assembly | genossi_mail (service layer, sync helper) | — | Pure function; no DB/network; MIME is a service-layer concern |
| Attachment byte loading | genossi_mail worker (async I/O) | DocumentStorage | Real async I/O stays where it is; only the seam moves per D-03 |
| SMTP transport (`transport.send()`) | genossi_mail service layer | — | Unchanged from today |
| `smtp_encoding` config parsing | genossi_mail::service::load_smtp_config | genossi_config KV store | Fits the existing config-parsing pattern (KV lookup + fallback) |
| Runbook doc | Operator documentation | — | Verify-in-prod step, not code |
| Frontend | — | — | Phase 22 has NO frontend surface (no UI toggle in scope) |

## Standard Stack

### Core (already installed — no new deps)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `lettre` | 0.11 (workspace-pinned in `Cargo.toml:60`), features = `["tokio1-rustls-tls", "smtp-transport", "builder", "hostname"]` — resolved to 0.11.20 in registry | MIME construction + async SMTP transport | Already the mail crate for the whole project |

### Supporting (already present)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio` | 1.35+ | Async runtime for `transport.send()` (unchanged) | Around `build_message`, not inside it |
| `tracing` | 0.1 | Log the encoding mode at load time (nice-to-have) | Optional: `tracing::debug!("Using SMTP encoding: {encoding:?}")` in `load_smtp_config` |
| `mockall` | 0.13 | Existing `MockConfigService` used for `load_smtp_config` unit tests | Reuse pattern from `service.rs:517-550` when adding `smtp_encoding` fallback tests |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New `send.rs` module | Inline extraction inside `worker.rs` | Rejected: worker already 1300+ lines; extraction demands its own module for testability |
| `LoadedAttachment` struct | 3-tuple `(&str, &str, &[u8])` | Struct is grep-able and mirrors `MailRecipientAttachment` (`dao.rs:103`) — safer refactor per project conventions |
| `SinglePart::plain(body)` in QP branch | Explicit `SinglePart::builder().header(TEXT_PLAIN).header(QuotedPrintable).body(body)` | Both are semantically identical (`mimebody.rs:114-118` shows `plain` = `builder + TEXT_PLAIN + body`; body-encoder auto-picks QP for non-ASCII). Recommendation: **use explicit builder in BOTH branches** so the encoding branch is symmetric and one-line diff-visible in code review. |

**No new package installation is required for this phase.** No package-legitimacy audit needed.

### Verified lettre 0.11.20 API (constructive proof)

From vendored source `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lettre-0.11.20/`:

`ContentTransferEncoding` — enum with `SevenBit`, `QuotedPrintable`, `Base64`, `EightBit`, `Binary` (verified in `src/message/header/content.rs:17-27`). Serialized strings: `"7bit"`, `"quoted-printable"`, `"base64"`, `"8bit"`, `"binary"` (lines 49-53). This maps 1:1 to the assertion strings we already have in existing tests.

`SinglePartBuilder::header<H: Header>(header: H)` at `mimebody.rs:54` — sets any typed header including `ContentTransferEncoding`. `SinglePartBuilder::body<T: IntoBody>(body: T)` at `mimebody.rs:66-76` reads the header we just set: if a CTE was pre-set (as `EightBit`), it is used verbatim; otherwise auto-selection kicks in.

`SinglePart::plain(body)` at `mimebody.rs:114-118` is literally `builder().header(TEXT_PLAIN).body(body)` — no hidden magic. Confirms that both branches can share the same explicit form.

Import paths (verified from existing tests in `worker.rs:1063-1065`):
```rust
use lettre::message::header::{ContentTransferEncoding, ContentType};
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::Message;
```

## Package Legitimacy Audit

**Not applicable.** Phase 22 installs no external packages. All required functionality (`lettre::message::header::ContentTransferEncoding::EightBit`, `SinglePart::builder`, `MultiPart::mixed`) is already present in the workspace-pinned `lettre = "0.11"` (registered in `Cargo.toml:60`).

## Architecture Patterns

### System Architecture Diagram

```
                     ┌────────────────────────┐
                     │  ConfigService (KV)    │
                     │  keys: smtp_host, …    │
                     │  NEW: smtp_encoding    │
                     └───────────┬────────────┘
                                 │ get_all()
                                 ▼
                     ┌────────────────────────┐
                     │ load_smtp_config()     │
                     │  → SmtpConfig {…,      │
                     │      encoding:         │
                     │      MailEncoding }    │
                     └───────────┬────────────┘
                                 │
        ┌────────────────────────┼─────────────────────────────┐
        │                        │                             │
        ▼                        ▼                             ▼
 ┌──────────────┐        ┌───────────────┐            ┌──────────────────┐
 │ worker.rs::  │        │ service.rs::  │            │ service.rs::     │
 │ send_mail_   │        │ send_test_    │            │ send_test_mail_  │
 │ for_recipient│        │ mail          │            │ with_body        │
 │ (async I/O)  │        │ (sync UX)     │            │ (sync UX +       │
 │              │        │               │            │  digest.rs)      │
 └──────┬───────┘        └──────┬────────┘            └──────┬───────────┘
        │                       │                            │
        │ 1. load attachment    │                            │
        │    bytes via          │                            │
        │    DocumentStorage    │                            │
        │    (async)            │                            │
        │                       │                            │
        │ 2. call ──────────────┴────────────────────────────┘
        ▼
 ┌──────────────────────────────────────────────────┐
 │  NEW: build_message(from, to, subject, body,     │  ◄── PURE SYNC
 │                    attachments: &[LoadedAtt.],   │      no DB, no I/O
 │                    in_reply_to: Option<&str>,    │      only lettre calls
 │                    encoding: MailEncoding)       │
 │                    -> Result<Message, …>         │
 │                                                  │
 │  • parses from/to (consolidates D-06 blocks)     │
 │  • builds text_part with explicit               │
 │      ContentType::TEXT_PLAIN + CTE per encoding │
 │  • wraps MultiPart::mixed{ text, attachments }  │
 │  • sets message_id(None) + In-Reply-To/         │
 │      References if in_reply_to.is_some()        │
 └──────────────────────┬───────────────────────────┘
                        │ Message
                        ▼
                 ┌──────────────┐
                 │ transport    │
                 │ .send()      │
                 └──────────────┘
```

### Recommended Project Structure (delta only)

```
genossi_mail/src/
├── send.rs           # NEW — hosts build_message + LoadedAttachment
│                     #   + #[cfg(test)] mod tests { … MIME byte tests }
├── service.rs        # MODIFIED — SmtpConfig gains `encoding`;
│                     #   send_test_mail{,_with_body} call build_message
├── worker.rs         # MODIFIED — send_mail_for_recipient loads bytes,
│                     #   then calls build_message; tests moved to send.rs
├── digest.rs         # UNCHANGED — still calls send_test_mail_with_body;
│                     #   inherits fix transitively
└── lib.rs            # MODIFIED — `pub mod send;`

docs/
├── audit/            # existing
└── OPERATIONS.md     # NEW — § "SMTP-Encoding umschalten (MAIL-04)"
```

### Pattern 1: Explicit sibling `SinglePart::builder` in both branches (D-09)

```rust
// Source: lettre 0.11.20 src/message/mimebody.rs:66-77 + src/message/header/content.rs:17-27
use lettre::message::header::{ContentTransferEncoding, ContentType};
use lettre::message::SinglePart;

pub enum MailEncoding {
    QuotedPrintable,
    EightBit,
}

fn build_text_part(body: &str, encoding: MailEncoding) -> SinglePart {
    let cte = match encoding {
        MailEncoding::QuotedPrintable => ContentTransferEncoding::QuotedPrintable,
        MailEncoding::EightBit => ContentTransferEncoding::EightBit,
    };
    SinglePart::builder()
        .header(ContentType::TEXT_PLAIN)          // sets `text/plain; charset=utf-8`
        .header(cte)                              // pre-sets CTE — body() will honor it
        .body(body.to_string())
}
```

**Note on `charset=utf-8`:** `ContentType::TEXT_PLAIN` is `"text/plain; charset=utf-8"` (verified — this is what `SinglePart::plain` already relies on at `mimebody.rs:116`, and today's worker uses the same). So we do NOT need to hand-craft the charset; picking `TEXT_PLAIN` is enough.

### Pattern 2: `build_message` signature (proposed)

```rust
// Source: composed from worker.rs:627-720 + D-01, D-02, D-06, D-07 constraints
use lettre::Message;
use std::sync::Arc;

pub struct LoadedAttachment {
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub bytes: Vec<u8>,
}

pub fn build_message(
    from: &str,                            // smtp_config.from
    to: &str,                              // recipient address
    subject: &str,
    body: &str,
    attachments: &[LoadedAttachment],      // empty for test-mail/digest
    in_reply_to: Option<&str>,             // None for test-mail/digest
    encoding: MailEncoding,                // from smtp_config.encoding
) -> Result<Message, MailServiceError> {
    use lettre::message::{Attachment, MultiPart};
    use lettre::message::header::ContentType;

    // D-06: consolidated address parsing
    let from_addr = from.parse().map_err(|e: lettre::address::AddressError| {
        MailServiceError::SmtpError(Arc::from(format!("Invalid from address: {}", e)))
    })?;
    let to_addr = to.parse().map_err(|e: lettre::address::AddressError| {
        MailServiceError::SmtpError(Arc::from(format!("Invalid to address: {}", e)))
    })?;

    let text_part = build_text_part(body, encoding);

    let mut builder = Message::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(subject)
        .message_id(None);

    if let Some(ref_id) = in_reply_to {
        let bracketed = format!("<{}>", ref_id);
        builder = builder.in_reply_to(bracketed.clone()).references(bracketed);
    }

    if attachments.is_empty() {
        builder.singlepart(text_part)
    } else {
        let mut multipart = MultiPart::mixed().singlepart(text_part);
        for att in attachments {
            let content_type = ContentType::parse(&att.mime_type)
                .unwrap_or_else(|_| ContentType::parse("application/octet-stream").unwrap());
            let attachment = Attachment::new(att.file_name.to_string())
                .body(att.bytes.clone(), content_type);
            multipart = multipart.singlepart(attachment);
        }
        builder.multipart(multipart)
    }
    .map_err(|e| MailServiceError::SmtpError(Arc::from(e.to_string())))
}
```

### Pattern 3: Config parsing with tolerant fallback (mirrors `smtp_tls`)

```rust
// Source: adapted from genossi_mail/src/service.rs:163-165 (existing smtp_tls pattern)
let encoding = match find("smtp_encoding").map(|e| e.value.as_ref()) {
    Some("8bit")            => MailEncoding::EightBit,
    Some("quoted-printable")=> MailEncoding::QuotedPrintable,
    Some("") | None         => MailEncoding::QuotedPrintable,   // safe default (MAIL-03)
    Some(other) => {
        tracing::warn!(
            value = %other,
            "Unknown smtp_encoding value — falling back to quoted-printable"
        );
        MailEncoding::QuotedPrintable
    }
};
```

**Why tolerant instead of hard-error:** The `smtp_tls` precedent (`service.rs:163-165`) treats missing/unknown gracefully; a typo in an operator config should not disable mail sending. This matches "Default bleibt quoted-printable" from D-08.

### Anti-Patterns to Avoid

- **Passing `bool` for encoding:** violates the project rule "Immer Enum statt Boolean" (documented in user memory `feedback_enum_not_boolean.md`). D-07 codifies this.
- **Threading `DocumentStorage` into `MailService`:** rejected in D-05. Loading stays in the worker (async), building stays sync (D-02/D-03).
- **Rebuilding `.parse()`/address-error handling per call site:** D-06 wants this consolidated inside `build_message`.
- **Setting `charset=utf-8` by hand as a raw string:** use `ContentType::TEXT_PLAIN`; lettre's typed header already carries the parameter (see `SinglePart::plain` implementation).
- **Persisting test-mail as a `MailJob`:** rejected Option E, D-05.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MIME multipart boundary generation | Manual `--boundary` strings | `MultiPart::mixed().singlepart(...)` | Boundary safety, RFC 2046 compliance |
| Content-Transfer-Encoding negotiation | Manual `=XX=` line breaking | Let lettre auto-QP OR set `ContentTransferEncoding::EightBit` explicitly | Correct edge-case handling (long lines, `Content-Length`, CRLF discipline) |
| Message-ID generation | Hand-crafted `<uuid@host>` string | `Message::builder().message_id(None)` (lettre generates one; already used at `worker.rs:662`) | Includes hostname, uniqueness guarantees |
| Address parsing | Split-by-`@` etc. | `str::parse::<Mailbox>()` (as done at `worker.rs:644,648`) | Handles quoted local-parts, IDN, comments |
| 8BITMIME automated capability probe | Custom SMTP dialog from Rust code | Operator runs `openssl s_client` once and documents result (D-12) | Dev cannot reach prod relay (network isolation); one-off check is cheaper than probe code + false-positive risk |

**Key insight:** Every capability we need already exists in lettre 0.11 as a public typed API. The phase is a refactor + one enum + one config key — NOT a design exercise.

## Runtime State Inventory

Not a rename/refactor of persisted identifiers. However, one subtle runtime item deserves an explicit "None":

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `SmtpConfig` is loaded fresh on every call from KV `config_entries`; no cached state | None |
| Live service config | `config_entries` table already holds `smtp_*` keys; **new** row `smtp_encoding` may be inserted post-deploy by operator via existing config UI — no schema migration needed (KV) | Operator inserts the new key AFTER 8BITMIME verified in prod; before that, absent key → default QP |
| OS-registered state | None — no systemd/cron work in this phase | None |
| Secrets/env vars | None — `smtp_encoding` is a public-ish operational toggle, not a secret; sits next to existing `smtp_host` etc. | None |
| Build artifacts | None — pure Rust refactor + doc file; no generated artifacts change | None |

## Common Pitfalls

### Pitfall 1: Losing `charset=utf-8` by using `Message::builder().body(...)`
**What goes wrong:** Umlauts render as mojibake on GMX Android (documented root cause of the whole phase, see comment at `worker.rs:652-655`).
**Why it happens:** `Message::builder().body("...")` emits `Content-Type: text/plain` WITHOUT the charset parameter. The three service.rs test-mail sites do this today (`service.rs:436, 479`).
**How to avoid:** Route ALL Message construction through `SinglePart::builder().header(ContentType::TEXT_PLAIN).body(...)`. D-01/D-04 enforce this.
**Warning signs:** Any surviving `Message::builder().<from/to/subject/body>()` chain in the mail crate after the phase. The exit criterion is that `grep -rn "\.body(" genossi_mail/src/` returns only `Attachment::body(...)` and the internal `build_message` body, NOT `Message::builder().body(...)`.

### Pitfall 2: 8BITMIME support not verified on relay
**What goes wrong:** Enabling `smtp_encoding=8bit` against a relay that does not advertise `250-8BITMIME` may cause `550` bounces or silent mangling (RFC 6152 says relays without 8BITMIME may downgrade the message to 7-bit and mangle bytes).
**Why it happens:** Modern relays generally support it, but not universally — provider-specific.
**How to avoid:** D-12: run `openssl s_client -starttls smtp -connect <relay>:<port>`, issue `EHLO <hostname>`, look for `250-8BITMIME`. Only then set `smtp_encoding=8bit`.
**Warning signs:** Rise in `MailServiceError::SmtpError` from `transport.send()`; recipient reports of mangled characters.

### Pitfall 3: Config value is `Arc<str>`, comparisons need care
**What goes wrong:** `find("smtp_encoding").unwrap().value` is `Arc<str>` (see `dao.rs` — same type as other config values). Matching against `Some("8bit")` requires `.as_ref()` or `&*`.
**Why it happens:** `ConfigEntry.value: Arc<str>` per project conventions.
**How to avoid:** Use `find("smtp_encoding").map(|e| e.value.as_ref())` (String pattern shown above); this compiles for `&str` match arms.
**Warning signs:** `error[E0308]: mismatched types … expected &str, found Arc<str>` at compile time — caught immediately by `cargo build`.

### Pitfall 4: Digest inherits the bug and it must be tested
**What goes wrong:** Digest (`digest.rs:170-187`) calls `send_test_mail_with_body`, so today it silently inherits the missing-charset bug. Post-fix, digest inherits the FIX for free — but no test currently proves this.
**How to avoid:** Add a MIME-byte test that specifically covers the "test-mail-with-body path" (D-10 explicitly mentions "Charset-Abdeckung für Test-Mail/Digest-Pfad wird ergänzt (heute ungetestet)"). Since all three paths converge on `build_message`, one test on `build_message(from, to, subj, body, &[], None, QuotedPrintable)` proves the point structurally.

### Pitfall 5: `Content-Transfer-Encoding: quoted-printable` may become `base64` for non-ASCII
**What goes wrong:** Existing worker tests at lines 1001-1006 assert `quoted-printable OR base64` — because lettre's auto-encoder picks base64 when quoted-printable would exceed line-length budgets.
**How to avoid:** Keep the OR-assertion in the QP-mode test (`quoted-printable || base64` — both are RFC-compliant non-7bit encodings). For the 8bit-mode test, assert `Content-Transfer-Encoding: 8bit` exactly (no OR) — we pinned it explicitly via header, so auto-selection is bypassed.
**Warning signs:** A flaky QP test because a slightly longer body flips into base64. Solve with `||`, not with a shorter body.

## Code Examples

### Example 1: QP mode assertion (mirrors existing pattern)

```rust
// Source: composed from genossi_mail/src/worker.rs:977-1007 (existing pattern) + D-10
#[test]
fn build_message_qp_has_utf8_charset_and_non_7bit_cte() {
    let email = build_message(
        "sender@example.com",
        "recipient@example.com",
        "Test",
        "Hallo Jürgen, schöne Grüße! ä ö ü ß",
        &[],
        None,
        MailEncoding::QuotedPrintable,
    )
    .expect("build_message must succeed");

    let text = String::from_utf8_lossy(&email.formatted()).to_string();

    assert!(text.contains("charset=utf-8"), "must declare charset=utf-8:\n{text}");
    assert!(
        text.contains("Content-Transfer-Encoding: quoted-printable")
            || text.contains("Content-Transfer-Encoding: base64"),
        "QP mode must emit QP or base64 CTE:\n{text}"
    );
    assert!(
        !text.contains("Content-Transfer-Encoding: 8bit"),
        "QP mode must NOT emit 8bit CTE:\n{text}"
    );
}
```

### Example 2: 8bit mode assertion (NEW — the byte-level guarantee for MAIL-02)

```rust
// Source: D-10 requirement (byte-exact 8bit CTE proof)
#[test]
fn build_message_8bit_has_utf8_charset_and_8bit_cte() {
    let email = build_message(
        "sender@example.com",
        "recipient@example.com",
        "Test",
        "Zeile eins mit ä ö ü ß — und eine sehr lange zweite Zeile ohne =-Softbreaks.",
        &[],
        None,
        MailEncoding::EightBit,
    )
    .expect("build_message must succeed");

    let text = String::from_utf8_lossy(&email.formatted()).to_string();

    assert!(text.contains("charset=utf-8"), "must declare charset=utf-8:\n{text}");
    assert!(
        text.contains("Content-Transfer-Encoding: 8bit"),
        "8bit mode must emit 8bit CTE exactly:\n{text}"
    );
    assert!(
        !text.contains("Content-Transfer-Encoding: quoted-printable"),
        "8bit mode must NOT emit QP CTE:\n{text}"
    );
    // Softbreak-free evidence: no bare `=\r\n` line-continuations in the body.
    assert!(
        !text.contains("=\r\n"),
        "8bit body must not carry QP soft line breaks:\n{text}"
    );
}
```

### Example 3: `load_smtp_config` unit test (fallback behavior)

```rust
// Source: pattern from genossi_mail/src/service.rs:517-550 + D-08 fallback requirement
#[tokio::test]
async fn load_smtp_config_defaults_encoding_to_qp_when_key_missing() {
    let mut cfg = MockConfigService::new();
    let entries = mock_smtp_config();          // helper at service.rs:517 — no smtp_encoding key
    cfg.expect_get_all().returning(move || Ok(Arc::from(entries.clone())));

    let smtp = load_smtp_config(&cfg).await.expect("must succeed");
    assert!(matches!(smtp.encoding, MailEncoding::QuotedPrintable));
}

#[tokio::test]
async fn load_smtp_config_reads_encoding_8bit_when_set() {
    let mut cfg = MockConfigService::new();
    let mut entries = mock_smtp_config();
    entries.push(ConfigEntry {
        key: Arc::from("smtp_encoding"),
        value: Arc::from("8bit"),
        value_type: Arc::from("string"),
    });
    cfg.expect_get_all().returning(move || Ok(Arc::from(entries.clone())));

    let smtp = load_smtp_config(&cfg).await.expect("must succeed");
    assert!(matches!(smtp.encoding, MailEncoding::EightBit));
}

#[tokio::test]
async fn load_smtp_config_falls_back_on_unknown_encoding_value() {
    let mut cfg = MockConfigService::new();
    let mut entries = mock_smtp_config();
    entries.push(ConfigEntry {
        key: Arc::from("smtp_encoding"),
        value: Arc::from("typo-nonsense"),
        value_type: Arc::from("string"),
    });
    cfg.expect_get_all().returning(move || Ok(Arc::from(entries.clone())));

    let smtp = load_smtp_config(&cfg).await.expect("must succeed");
    assert!(matches!(smtp.encoding, MailEncoding::QuotedPrintable));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Message::builder().body(...)` (no charset param) | `SinglePart::builder().header(ContentType::TEXT_PLAIN)…` for every plain-text mail | Since lettre 0.10 (2022+) | The correct-by-default pattern; worker already uses it (line 656) |
| Auto CTE selection only | Explicit `header(ContentTransferEncoding::EightBit)` | RFC 6152 (2011) 8BITMIME support in lettre since 0.10 | Enables MAIL-02 |
| Boolean flags for feature toggles | Enum with explicit variants | Project rule (user memory `feedback_enum_not_boolean.md`) | D-07 codifies |

**Deprecated/outdated:**
- Nothing lettre-related is deprecated in scope. lettre 0.11.20 is the current stable line.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | (none) | — | All API facts verified against vendored `lettre-0.11.20` source; all codebase facts verified via `grep -n` at the cited line numbers. |

**Empty by design.** Every load-bearing fact in this file was cross-checked against either the lettre source at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lettre-0.11.20/` or the actual codebase at the cited line numbers. No `[ASSUMED]` tags remain.

## Cascade Check for Callers

Repo-wide grep for `Message::builder()` in `.rs` files:

| Location | Purpose | Must route through `build_message`? |
|----------|---------|--------------------------------------|
| `genossi_mail/src/worker.rs:658` | `send_mail_for_recipient` — real bulk send | YES — this is the extraction source (D-01) |
| `genossi_mail/src/service.rs:421` | `send_test_mail` (Bug-Pfad) | YES (D-04) |
| `genossi_mail/src/service.rs:463` | `send_test_mail_with_body` (Bug-Pfad, digest also uses this) | YES (D-04) |
| `genossi_mail/src/service.rs:864` (doc comment mentioning `Message::builder()`) | Non-code — trait doc | No — just prose |
| `genossi_mail/src/worker.rs:985, 1035, 1079, 1111, 1263` | Existing worker tests (`plain_mail_body_has_utf8_charset`, `built_message_exposes_message_id_header`, `multipart_mail_body_has_utf8_charset`, `reply_mail_includes_in_reply_to_header`, `non_reply_mail_has_no_in_reply_to_header`) | YES — D-11 requires tests to call `build_message` instead of re-inlining. Recommend MOVING them into `send.rs` alongside the implementation. |

**Digest path** (`genossi_mail/src/digest.rs:174-186`) does not call `Message::builder()` directly — it delegates to `mail_service.send_test_mail_with_body(recipient, subject, body)`. Because `send_test_mail_with_body` gets rewired to `build_message` in this phase, digest inherits the fix transitively. No additional wiring needed at the digest site.

**Grep evidence:** `grep -rn "Message::builder()" --include="*.rs"` returns exactly the 6 non-doc hits above and 0 hits outside `genossi_mail/`. The rebrand surface is contained to one crate. No other subsystem sends mail via lettre.

**Also NOT in scope for `build_message`:**
- `Response::builder()` in `rest*.rs` files — that's `axum::response::Response`, not `lettre::Message`.
- `inbox_imap.rs` `fetch.body()` — IMAP inbound side; unrelated.
- `inbox.rs:1703, 1958` `Content-Transfer-Encoding` lines — those are test fixtures for inbound parsing, not outbound mail.

## Runbook Location Research

**Searched:**
- `find . -maxdepth 4 -type f \( -iname "*runbook*" -o -iname "*deploy*" -o -iname "RELAY*" -o -iname "OPS*" \)` — only hit is `deploy-binaries.sh` (a shell script), not a doc.
- `ls docs/` → contains only `docs/audit/` (audit-report templates + typst styles).
- `ls doc/` → CSV export dumps + `AUTHENTICATION.md` + `HTTP-PERIMETER.md`. Existing operator docs live at repo root as flat `.md` (`OIDC-CONFIG.md`) or in `doc/`.

**Recommendation:** Create a new file `docs/OPERATIONS.md` with a top-level § "SMTP-Encoding umschalten (MAIL-04)". Rationale:
- Aligns with existing `docs/audit/` sibling — same tree, operator-facing.
- Creates a landing spot for future ops runbooks (Phase 23+ will benefit).
- Alternatives: `doc/SMTP-8BITMIME.md` (single-topic file) is also viable; planner picks — D-12 grants discretion. If planner prefers a smaller footprint, a single-topic file avoids co-locating unrelated ops content.

**Content shape for D-12 (draft outline for the planner):**

```markdown
# Operations Runbook

## SMTP-Encoding umschalten (MAIL-04)

Der Default ist `quoted-printable` (Fallback, funktioniert überall).
`8bit` ist ein Opt-in und erfordert, dass der Produktivrelay das `8BITMIME`-
Feature per EHLO ankündigt. Aus der Dev-Umgebung ist der Prod-Relay NICHT
erreichbar (Netz-Isolation), deshalb muss der Betreiber diesen Check
ONE-SHOT im Prod-Netz durchführen.

### Schritt 1 — 8BITMIME am Relay verifizieren

```bash
openssl s_client -starttls smtp -connect <relay-host>:<port> -crlf
EHLO genossi.local
```

Erwartete Ausgabe enthält eine Zeile wie:

```
250-8BITMIME
```

Wenn diese Zeile FEHLT → `smtp_encoding=8bit` NICHT setzen (der Relay
könnte 8-bit-Bytes verstümmeln).

### Schritt 2 — Config-Toggle setzen

Nur wenn Schritt 1 grün ist:

Config-Key `smtp_encoding` = `8bit` in der Genossi-Config-UI setzen
(entspricht `config_entries`-Zeile). Kein Neustart nötig — die Config
wird pro Sendevorgang frisch geladen.

### Rollback

Config-Key `smtp_encoding` = `quoted-printable` (oder Key löschen —
Default greift automatisch).
```

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `lettre` crate | build_message | ✓ | 0.11.20 (workspace-pinned) | — |
| `mockall` | tests | ✓ | 0.13 | — |
| `tokio` test runtime | async tests for `load_smtp_config` | ✓ | 1.35+ | — |
| `openssl` CLI | operator's D-12 verify-in-prod step | out-of-scope for CI — operator concern | — | Any relay-diagnostics client (swaks, ncat) |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** None.

## Validation Architecture

Skipped — `.planning/config.json` has `workflow.nyquist_validation: false`.

## Security Domain

`security_enforcement` is not explicitly set to `false` in `.planning/config.json`, so this section is included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Phase does not touch auth. Test-mail endpoint is admin-only via existing REST permission guard (unchanged). |
| V3 Session Management | no | No session code change. |
| V4 Access Control | no | REST-layer permission checks for test-mail unchanged; digest recipients come from existing config path. |
| V5 Input Validation | yes | `smtp_encoding` config value validated via allow-list match (unknown → default). Body/subject already parameterized through minijinja render on upstream paths — not this phase. |
| V6 Cryptography | no | TLS transport (STARTTLS/TLS) driven by existing `smtp_tls` key — unchanged. |
| V7 Errors & Logging | yes | Encoding-mismatch/unknown values must log a warning without leaking secrets (never log full `SmtpConfig` — it has `pass`). |
| V8 Data Protection | partial | Test-mail body is passed by the caller; **privacy defense comment at service.rs:453-457** notes `to` MUST come from request body, NEVER a Member's stored email. This constraint is inherited untouched — `build_message` is a byte-mover, not a policy point. |
| V14 Configuration | yes | New KV config key `smtp_encoding` — pattern follows existing `smtp_tls`; no secret material. |

### Known Threat Patterns for lettre + SMTP

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Header injection via `subject`/`from`/`to` | Tampering | lettre's typed `Message::builder()` chain rejects raw CR/LF via `parse::<Mailbox>()` and `subject()` — same as today. `build_message` inherits this. |
| Address parsing errors leaking to logs | Info Disclosure | Existing error wraps: `format!("Invalid from address: {}", e)` — reveals malformed address only, not credentials. Unchanged. |
| Downgrade attack via unverified 8BITMIME | Tampering (bytes mangled by relay) | D-12: operator MUST verify via EHLO before flipping; default stays QP as safe fallback. |
| Config poisoning (rogue `smtp_encoding` value) | Tampering | Allow-list match with `tracing::warn!` fallback — no impact beyond staying on QP default. |
| Log leakage of `SmtpConfig.pass` | Info Disclosure | Existing `SmtpConfig` does NOT `impl Debug` in a way that leaks; do NOT add `#[derive(Debug)]` to `SmtpConfig` in this phase. If planner needs debug output, add manual `impl Debug` that redacts `pass`. |

## Sources

### Primary (HIGH confidence)
- **Codebase (grep-verified):**
  - `genossi_mail/src/worker.rs:627-720` — `send_mail_for_recipient` (extraction source)
  - `genossi_mail/src/worker.rs:977-1099` — existing MIME byte tests (pattern for D-10/D-11)
  - `genossi_mail/src/service.rs:118-181` — `SmtpConfig` + `load_smtp_config` (Config plumbing)
  - `genossi_mail/src/service.rs:415-488` — `send_test_mail` + `send_test_mail_with_body` (Bug paths + privacy comment)
  - `genossi_mail/src/service.rs:517-550` — `mock_smtp_config` helper (reuse for fallback tests)
  - `genossi_mail/src/digest.rs:150-190` — digest worker tick (transitive fix inheritor)
  - `genossi_mail/src/dao.rs:103` — `MailRecipientAttachment` struct (shape template for `LoadedAttachment`)
  - `Cargo.toml:60` — workspace lettre version pin: `"0.11"` with features
- **lettre 0.11.20 source (vendored):**
  - `src/message/mimebody.rs:40-118` — `SinglePartBuilder` API (`header`, `body`, `SinglePart::plain` = builder + TEXT_PLAIN + body)
  - `src/message/header/content.rs:17-27, 49-53, 62-66` — `ContentTransferEncoding` enum + string mapping
  - `src/message/mod.rs:26, 61, 113, 132, 471-472, 558` — Message builder + `singlepart`/`multipart` public API

### Secondary (MEDIUM confidence)
- **CONTEXT.md** (`.planning/phases/22-8bit-shared-mail-body-helper/22-CONTEXT.md`) — D-01..D-12 locked decisions.
- **ROADMAP.md** — Phase 22 goal + success criteria + Phase 23-25 boundaries.
- **REQUIREMENTS.md** — MAIL-01..05 wording + traceability + phase mapping.

### Tertiary (LOW confidence)
- None. No WebSearch was needed — everything was resolvable in-repo or from vendored source.

## Metadata

**Confidence breakdown:**
- Standard stack (lettre 0.11.20 API): HIGH — verified from vendored source.
- Architecture (three-caller convergence, digest inheritance): HIGH — grep-verified across the workspace.
- Pitfalls (charset bug root cause, QP-vs-base64 flakiness, 8BITMIME downgrade): HIGH — pitfalls 1, 3, 4, 5 are grep-verified; pitfall 2 is documented in D-12 and RFC 6152.
- Runbook location: MEDIUM — recommended, but planner has D-12 discretion.

**Research date:** 2026-07-02
**Valid until:** 2026-08-02 (30 days — lettre 0.11 is stable; code paths cited are unlikely to move within a month).
