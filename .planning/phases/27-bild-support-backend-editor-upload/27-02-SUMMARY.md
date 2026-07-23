---
phase: 27-bild-support-backend-editor-upload
plan: 02
subsystem: mail
tags: [ammonia, sanitize, html, security, img, store-boundary]

# Dependency graph
requires:
  - phase: 23-html-mail-sanitize
    provides: "sanitize_html store-boundary choke-point (wired into create_job / template create+update / send_test_mail_with_body)"
  - phase: 26-wysiwyg-lists-headings
    provides: "ammonia default list/heading survival guarantees (must stay green)"
provides:
  - "hardened sanitize_html backed by a custom ammonia::Builder (cached via OnceLock) — permissive ammonia::clean() removed from the production path"
  - "the <img data-genossi-asset-id=\"X\"> persisted-HTML contract (external/data: src stripped, svg dropped)"
affects: [27-03 cid-renderer, 27-04 editor-upload]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cached custom ammonia::Builder via std::sync::OnceLock (Builder construction is not free) starting from Builder::default() and tightening only the <img> rule"
    - "Store-boundary <img> allowlist: only data-genossi-asset-id survives; src/srcset/alt/width/height/loading stripped, data: scheme forbidden, <svg> dropped by absence from the default tag set"

key-files:
  created: []
  modified:
    - genossi_mail/src/sanitize.rs

key-decisions:
  - "add_tag_attributes(\"img\", &[\"data-genossi-asset-id\"]) is sufficient in ammonia 4.1.3 to whitelist the data-* attribute — the Pitfall-2 fallback (add_generic_attribute_prefixes) was NOT needed (Assumption A2 confirmed)"
  - "Builder cached in a OnceLock<ammonia::Builder<'static>> and reused per call rather than rebuilt per sanitize (construction cost avoided)"
  - "SVG stays stripped by NOT adding <svg> to the tag set — no explicit removal needed since it is absent from ammonia's default"

requirements-completed: [IMG-05]

coverage:
  - id: S1
    description: "<img> restricted to data-genossi-asset-id; external http src, data: URI, and svg all stripped at the store boundary"
    requirement: "IMG-05"
    verification:
      - kind: unit
        ref: "genossi_mail/src/sanitize.rs::tests::sanitize_preserves_img_data_genossi_asset_id"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/sanitize.rs::tests::sanitize_strips_external_http_img_src_keeps_asset_id"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/sanitize.rs::tests::sanitize_strips_data_uri_img_src"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/sanitize.rs::tests::sanitize_strips_svg"
        status: pass
    human_judgment: false
  - id: S2
    description: "Phase 23/26 backward-compat: script/event/scheme strip, Jinja survival, list/heading survival all remain green after the <img> rule change"
    requirement: "IMG-05"
    verification:
      - kind: unit
        ref: "genossi_mail/src/sanitize.rs::tests (sanitize_strips_script_tag, sanitize_strips_event_handlers, sanitize_strips_dangerous_url_schemes, sanitize_preserves_jinja_placeholder_in_text_content, sanitize_preserves_unordered_list, sanitize_preserves_ordered_list, sanitize_preserves_headings_h1_h2_h3)"
        status: pass
    human_judgment: false

# Metrics
duration: 10min
completed: 2026-07-23
status: complete
---

# Phase 27 Plan 02: ammonia `<img>` Hardening Summary

**Replaced the permissive `ammonia::clean()` default with a cached custom `ammonia::Builder` (OnceLock) that restricts `<img>` to the single `data-genossi-asset-id` attribute — stripping external `src`, `data:` URIs and `<svg>` at the store boundary while keeping every Phase 23/26 sanitizer guarantee intact.**

## Performance

- **Duration:** ~10 min
- **Tasks:** 1 completed
- **Files modified:** 1

## Accomplishments
- Rewrote `sanitize_html` to build a custom `ammonia::Builder` once (cached in a `OnceLock<ammonia::Builder<'static>>`, since Builder construction is not free) instead of calling the permissive `ammonia::clean()`.
- Tightened the `<img>` rule: `rm_tag_attributes("img", &["src", "srcset", "alt", "width", "height", "loading"])` + `add_tag_attributes("img", &["data-genossi-asset-id"])` + `rm_url_schemes(&["data"])`. `<svg>` stays dropped (absent from the default tag set).
- Confirmed Assumption A2 / resolved Pitfall 2 empirically: in ammonia 4.1.3 the `data-genossi-asset-id` survival test passes with `add_tag_attributes` — the `add_generic_attribute_prefixes` fallback was NOT required.
- Added 4 new img round-trip tests and kept all 7 existing Phase 23/26 sanitize tests plus the other `genossi_mail` store-boundary tests green.
- Updated the module doc comment to reflect that the sanitizer is no longer the permissive default.

## Task Commits

1. **Task 1: Custom ammonia Builder restricting img to data-genossi-asset-id** - `6f0769f` (feat)

_TDD note: Task 1 was `tdd="true"`. The 4 new img tests plus the pre-existing survival tests are the RED→GREEN proof; the survival test for `data-genossi-asset-id` was the arbiter that decided `add_tag_attributes` (not the fallback lever) is correct for ammonia 4.1.3. Implementation and tests were authored together and verified green in a single commit._

## Files Created/Modified
- `genossi_mail/src/sanitize.rs` - `builder()` (cached custom `ammonia::Builder` via `OnceLock`) backing an unchanged-signature `sanitize_html`; 4 new img tests; updated module doc.

## Decisions Made
- `add_tag_attributes("img", &["data-genossi-asset-id"])` suffices in ammonia 4.1.3 — Pitfall-2 fallback avoided.
- Public signature `pub fn sanitize_html(html: &str) -> String` kept unchanged, so the four Phase 23 store-boundary call sites need no edits.
- `<svg>` is dropped implicitly (not in ammonia's default tag set) — no explicit removal.

## Deviations from Plan

None - plan executed exactly as written. No auto-fixes required (Rules 1-4 not triggered).

## Issues Encountered

None. The colocated jj+git index was rebuilt via `git read-tree HEAD` before staging and the tree verified via `git write-tree` (46 top-level entries, CLAUDE.md present) before the commit, per the plan 27-01 gotcha — no index desynchronization occurred this time.

## Verification
- `cargo test -p genossi_mail sanitize` — 16 passed (4 new img tests + all pre-existing Phase 23/26 script/event/scheme/Jinja/list/heading tests + store-boundary call-site tests).
- `cargo test -p genossi_mail` — 265 passed, 0 failed (no regressions elsewhere in the crate).
- `grep -n "ammonia::clean(" genossi_mail/src/sanitize.rs | grep -v '^\s*//'` — no non-comment matches (permissive default gone from the production path).

## Threat Model Coverage
- T-27-06 (SVG-as-image XSS) — `sanitize_strips_svg` proves `<svg>` dropped.
- T-27-07 (external http src SSRF / tracking pixel) — `sanitize_strips_external_http_img_src_keeps_asset_id` proves src removed, asset-id kept.
- T-27-08 (data: URI exfiltration) — `sanitize_strips_data_uri_img_src` proves `data:` absent.
- T-27-09 (stored resolvable src) — only `data-genossi-asset-id` persisted; `src` injected downstream (27-03 cid: / 27-04 /bytes).
- T-27-10 (list/heading regression) — all Phase 26 survival tests kept green.

## Self-Check: PASSED

- Modified file present: `genossi_mail/src/sanitize.rs` (in HEAD tree).
- Task commit present: `6f0769f`.
