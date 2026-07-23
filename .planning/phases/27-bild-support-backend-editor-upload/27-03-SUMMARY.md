---
phase: 27-bild-support-backend-editor-upload
plan: 03
subsystem: mail
tags: [lettre, multipart-related, cid, inline-image, base64-guard, mime, backward-compat]

# Dependency graph
requires:
  - phase: 27-bild-support-backend-editor-upload
    provides: "mail_asset entity (DAO + inline BLOB bytes) — 27-01"
  - phase: 27-bild-support-backend-editor-upload
    provides: "hardened ammonia <img data-genossi-asset-id> store-boundary contract — 27-02"
  - phase: 23-html-mail-sanitize
    provides: "build_message single MIME factory with multipart/alternative branch"
provides:
  - "rewrite_img_cids pure fn: sanitized <img data-genossi-asset-id=X> -> <img src=cid:asset-N@genossi> + de-duplicated Vec<AssetRef>"
  - "build_message multipart/related branch (mixed -> related -> alternative) with matching cid/Content-ID"
  - "25 MB base64-encoded wire-size guard (D-02) before MIME assembly"
  - "inline-image byte loading in the job-send worker + the test-mail service (IMG-07)"
  - "InlineImageByteLoader trait object (avoids a Dao generic on MailServiceImpl)"
affects: [27-04 editor-upload, phase-28 preview]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "lettre MultiPart::related() + Attachment::new_inline(cid) for inline images; cid string identical in HTML src and Content-ID header (Pitfall 6)"
    - "base64 wire-size guard via raw_len.div_ceil(3)*4 summed BEFORE assembly (D-02: SMTP SIZE applies to encoded message)"
    - "worker-only DAO generic (AS = MailAssetDao) instead of adding a Dao to MailServiceImpl (RESEARCH Anti-Pattern)"
    - "boxed InlineImageByteLoader trait object injected into MailServiceImpl for the test-mail path"
    - "img-stripped plain-text derivation (strip <img> before html2text) so no cid:/img leaks into the text part"

key-files:
  created: []
  modified:
    - genossi_mail/src/render.rs
    - genossi_mail/src/send.rs
    - genossi_mail/src/service.rs
    - genossi_mail/src/worker.rs
    - genossi_bin/src/lib.rs

key-decisions:
  - "cid scheme asset-{n}@genossi with per-mail sequential numbering per DISTINCT asset id; de-dup so one related part serves N <img> references (Pitfall 6)"
  - "25 MB limit is the BASE64-ENCODED wire size (D-02), not the raw payload; guard fires before address parsing / part building"
  - "images referenced but no HTML body -> BadRequest (cid refs live in HTML; never silently drop images)"
  - "missing/soft-deleted asset at send time -> log warning + skip that image (broken image beats a failed send, T-27-15)"
  - "test-mail loading via a boxed InlineImageByteLoader; worker loading via a worker-only MailAssetDao generic — MailServiceImpl gains no Dao type param (Anti-Pattern respected)"

patterns-established:
  - "rewrite_img_cids is a pure, unit-testable fn returning (rewritten_html, Vec<AssetRef>); empty Vec is the backward-compat signal"
  - "build_message's new inline_images param: empty slice runs the pre-change 4-branch matrix byte-identically (IMG-09)"

requirements-completed: [IMG-06, IMG-07, IMG-08, IMG-09]

coverage:
  - id: C1
    description: "rewrite_img_cids transforms sanitized image HTML to cid: refs, de-dups by asset id, collects referenced ids; plain text carries no image leakage"
    requirement: "IMG-06"
    verification:
      - kind: unit
        ref: "genossi_mail/src/render.rs::tests::rewrite_img_cids_single_image_produces_cid_and_ref"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/render.rs::tests::rewrite_img_cids_same_id_dedups_to_one_ref_both_imgs_same_cid"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/render.rs::tests::rewrite_img_cids_distinct_ids_get_sequential_cids"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/render.rs::tests::plain_from_html_strips_image_no_cid_no_img_leak"
        status: pass
    human_judgment: false
  - id: C2
    description: "build_message wraps alternative in related with matching cid/Content-ID; attachments -> mixed>related>alternative"
    requirement: "IMG-06"
    verification:
      - kind: unit
        ref: "genossi_mail/src/send.rs::tests::build_message_related_structure_matches_cid_and_content_id"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/send.rs::tests::build_message_mixed_wraps_related_when_attachments_present"
        status: pass
    human_judgment: false
  - id: C3
    description: "Base64-encoded total > 25 MB rejected BEFORE assembly (D-02 basis: raw < 25 MB but encoded > 25 MB is rejected)"
    requirement: "IMG-08"
    verification:
      - kind: unit
        ref: "genossi_mail/src/send.rs::tests::build_message_rejects_when_base64_encoded_size_exceeds_25mb"
        status: pass
    human_judgment: false
  - id: C4
    description: "Empty inline_images slice produces no related wrapper — byte-identical no-image path; every pre-existing send.rs test stays green"
    requirement: "IMG-09"
    verification:
      - kind: unit
        ref: "genossi_mail/src/send.rs::tests::build_message_empty_inline_images_is_byte_identical_no_related"
        status: pass
      - kind: regression
        ref: "genossi_mail/src/send.rs::tests (build_message_legacy_singlepart_text_unchanged, build_message_alternative_text_then_html_no_attachments, build_message_mixed_wraps_alternative_when_attach — only mechanical &[] arg added)"
        status: pass
    human_judgment: false
  - id: C5
    description: "Both the job-send worker and the test-mail service load referenced asset bytes and pass them to build_message; image-less body loads nothing"
    requirement: "IMG-07"
    verification:
      - kind: unit
        ref: "genossi_mail/src/service.rs::tests::send_test_mail_with_body_loads_asset_bytes_when_html_has_image"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/service.rs::tests::send_test_mail_with_body_no_image_does_not_load_assets"
        status: pass
    human_judgment: false
  - id: C6
    description: "Real-client inline-image rendering (Thunderbird/Outlook resolve cid:) — success criterion #3"
    requirement: "IMG-06"
    verification:
      - kind: manual
        ref: "UAT: send an image mail + test mail, confirm inline rendering in a real client"
        status: pending
    human_judgment: true

# Metrics
duration: 40min
completed: 2026-07-23
status: complete
---

# Phase 27 Plan 03: CID Renderer + multipart/related Send Path Summary

**Inline-image embedding for the mail send path: a pure `rewrite_img_cids` transform (`<img data-genossi-asset-id=X>` -> `<img src="cid:asset-N@genossi">`), a `build_message` `multipart/related` branch with matching cid/Content-ID, a 25 MB base64-wire-size guard, and asset-byte loading wired into both the job-send worker and the Vorstand test-mail — with the no-image path kept byte-identical (IMG-09).**

## Performance

- **Duration:** ~40 min
- **Tasks:** 3 completed
- **Files modified:** 5

## Accomplishments
- Added `rewrite_img_cids` — a pure, unit-testable function that rewrites sanitized image HTML to `cid:` references, assigns per-mail sequential CIDs per DISTINCT asset id (de-dup so one `related` part serves N `<img>`), leaves malformed ids untouched, and returns the referenced asset ids for byte loading. No-image input returns the HTML unchanged + an empty Vec (the backward-compat signal).
- Hardened the plain-text derivation: `plain_from_html` now strips `<img>` before html2text so neither `cid:` nor `img` leaks into the text part (T-27-13).
- Extended `build_message` with an `inline_images` parameter that, when non-empty, builds `multipart/related` (wrapped in `multipart/mixed` if document attachments are present) using `Attachment::new_inline(cid)` so the `Content-ID: <asset-1@genossi>` header exactly matches the `src="cid:asset-1@genossi"` in the HTML (Pitfall 6). When empty, the pre-change 4-branch matrix runs byte-identically (IMG-09).
- Added the IMG-08 guard on the BASE64-ENCODED wire size (D-02) that rejects `> 25 MB` before any assembly — proven by a test where the raw payload is under 25 MB but its base64 encoding exceeds it (a raw-byte guard would wrongly accept).
- Wired asset-byte loading into both send paths: the worker gained a worker-only `MailAssetDao` generic (mirrors `attachment_dao`), and the test-mail service received a boxed `InlineImageByteLoader` trait object — so `MailServiceImpl`'s generic list gained NO new Dao type param (RESEARCH Anti-Pattern respected).

## Task Commits

1. **Task 1: rewrite_img_cids pure fn + img-stripped plain derivation** - `291797a` (feat)
2. **Task 2: build_message multipart/related branch + 25 MB base64 guard** - `43e79b5` (feat)
3. **Task 3: wire inline-image byte loading into worker + test-mail (IMG-07)** - `7ee6337` (feat)

## Files Created/Modified
- `genossi_mail/src/render.rs` - `rewrite_img_cids` + `AssetRef` struct + `strip_img_tags`/`extract_asset_id`/`utf8_char_len` helpers; `plain_from_html` now strips `<img>`; 7 new tests.
- `genossi_mail/src/send.rs` - `LoadedInlineImage` struct + `base64_encoded_len`/`MAX_ENCODED_MAIL_BYTES`; extended `build_message` with the `inline_images` param, the related branch, and the 25 MB base64 guard; mechanical `&[]` at every existing test call site; 5 new tests.
- `genossi_mail/src/service.rs` - `InlineImageByteLoader` trait; optional `image_loader` field + `with_image_loader` builder on `MailServiceImpl`; `send_test_mail_with_body` runs `rewrite_img_cids` + loads bytes; 2 new tests.
- `genossi_mail/src/worker.rs` - `MailAssetDao` generic (`AS`, appended last) on `start_mail_worker`; `send_mail_for_recipient` loads inline-image bytes in a read tx and forwards them; call site updated.
- `genossi_bin/src/lib.rs` - shared `mail_asset_dao` Arc for the REST service + test-mail loader + worker; `MailAssetImageLoader` impl; `worker_mail_asset_dao` field; worker spawn passes the 16th positional arg.

## Decisions Made
- CID scheme `asset-{n}@genossi` with per-mail sequential numbering; de-dup by asset id (one `related` part serves N references).
- 25 MB limit interpreted as base64-encoded wire size (D-02), guarded before assembly.
- Images with no HTML body -> `BadRequest` (never silently drop images).
- Missing/soft-deleted asset at send time -> warning + skip (T-27-15).
- Test-mail uses a boxed loader; worker uses a worker-only DAO generic — no Dao added to `MailServiceImpl`.

## Deviations from Plan

None affecting scope. Rules 1-4 not triggered. The one mechanical adjustment beyond the plan text: `send.rs`'s existing production call sites in `worker.rs`/`service.rs` also required the mechanical `&[]` inline_images arg in Task 2 to keep the crate compiling between commits (they receive real image loading in Task 3). This is a compile-continuity insertion, not a behavior change.

## Issues Encountered

**Worker argument order.** The initial placement of the new `mail_asset_dao` parameter BEFORE `repayment_context_resolver` in `start_mail_worker` mismatched the call site (which passes RCR as the existing 15th arg), causing E0277 trait-bound errors (the compiler bound the resolver against `MailAssetDao`). Fixed by appending `mail_asset_dao` LAST so every existing positional arg keeps its order (Rule 3 blocking fix, resolved inline).

**Colocated jj + git index.** Rebuilt the index via `git read-tree HEAD` before staging each task and verified the tree via `git write-tree` (46 top-level entries, CLAUDE.md + Cargo.toml present) before every commit, per the plan 27-01/02 gotcha. No index desynchronization occurred.

**Out-of-scope rustfmt drift.** `cargo fmt -p genossi_mail -p genossi_bin` reformatted pre-existing rustfmt drift in two UNRELATED files (`genossi_mail/src/sanitize.rs`, `genossi_bin/tests/membership_adjust_e2e.rs`). Per the SCOPE BOUNDARY rule these were restored to their HEAD content (via `git show HEAD:<path> >` redirect, since `git checkout` is forbidden in this repo) and excluded from the commits — only the 5 plan files were staged.

## Deferred Issues

The two pre-existing e2e failures documented in `deferred-items.md` (`preview_body_html_round_trips_to_response`, `test_mail_preview_repayment_no_entries_does_not_default_to_one`) remain red on the full-workspace run (`310 passed; 2 failed`). Both are the Markdown `**bold**` leak in html2text's `plain_from_html` and a repayment-preview aggregation issue — NOT image support. Per the sequential-execution note this plan did not expand scope to chase them; the img-stripping change in `plain_from_html` does not touch the bold-leak path (the tolerant `plain_from_html_bold_becomes_markdown_stars` test stays green).

## Threat Model Coverage
- T-27-11 (DoS oversized mail) — 25 MB base64 guard fires before assembly; `build_message_rejects_when_base64_encoded_size_exceeds_25mb`.
- T-27-12 (CID/Content-ID mismatch) — identical cid string in HTML + `Attachment::new_inline`; `build_message_related_structure_matches_cid_and_content_id` asserts both.
- T-27-13 (image leaking into plain text) — `plain_from_html` strips `<img>`; `plain_from_html_strips_image_no_cid_no_img_leak`.
- T-27-14 (backward-compat regression) — empty `inline_images` runs the unchanged 4-branch matrix; every pre-existing send.rs test kept green.
- T-27-15 (missing asset at send) — warning + skip in both send paths (accepted disposition).

## Verification
- `cargo test -p genossi_mail` — 279 passed, 0 failed (rewrite_img_cids, related structure/cid match, base64 25 MB guard, test-mail loader wiring, and all pre-existing send/service/worker/render tests).
- `cargo build --workspace` — success (worker generic + test-mail loader + DI wiring compile end to end).
- `cargo test --workspace` — 310 passed, 2 failed; the 2 failures are the pre-existing deferred mail-preview/render issues (e2e_tests.rs:14961 / :14628), unrelated to this plan.
- `grep MailAssetDao genossi_mail/src/service.rs` — only a doc-comment reference; MailServiceImpl's generic list gained NO Dao type param (Anti-Pattern respected).
- IMG-09 byte-identity: `git diff` on send.rs shows only argument additions inside the pre-existing tests — no assertion changed.

## Self-Check: PASSED
