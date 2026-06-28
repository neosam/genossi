---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-06
subsystem: frontend
tags: [dioxus, wasm, component-first, page-wiring, inbox, attachments]

# Dependency graph
requires:
  - phase: 19-05
    provides: "InboxAttachmentList component + InboundMailDetailTO.attachments field on frontend"
provides:
  - "inbox_page.rs detail-pane wired to InboxAttachmentList (single component invocation)"
  - "Phase 19 end-to-end UI path complete (Vorstand sees real attachments with Download/Preview)"
affects: [phase-19-checker-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Page-as-composer pattern: detail pane delegates entire attachment rendering to component; zero RSX iteration over Vec<InboundMailAttachmentTO> in the page file"
    - "Legacy-hint expression idiom: `attachments.is_empty() && has_attachments` triggers the component's legacy branch when backend reports attachment present but no rows exist (D-06 backfill couldn't recover bytes)"

key-files:
  created: []
  modified:
    - genossi-frontend/src/page/inbox_page.rs

key-decisions:
  - "Import added to existing inbox component import group as multi-line list (alphabetical): `use crate::component::inbox::{InboxAttachmentList, InboxMailListItem, InboxReplyForm, InboxStatusBadge};`. Avoids separate single-import line, keeps inbox-component imports grouped."
  - "Component invocation positioned inside the scrollable body column (parent div `flex-1 overflow-y-auto`) between the body `<pre>` and the assignment `<div class=\"border-t pt-2 mt-2\">`. Matches UI-SPEC §Page Integration exactly — attachments scroll with body, not pinned in flex-none header."
  - "MVP-hint deletion was the entire `if d.has_attachments { ... }` block from the flex-none header section (lines 331-335 in pre-modification file). The has_html_body hint at 336-340 stays unchanged — it is a separate concern (HTML-only body fallback notice)."

patterns-established:
  - "Component-First wiring: when a placeholder hint (like an MVP-amber-block) is replaced by a real component, the page delegates the entire UI surface — no inline iteration, no inline conditionals beyond the component's own boundary props. The page just passes data."

requirements-completed: []

# Metrics
duration: ~5min
completed: 2026-06-07
---

# Phase 19 Plan 06: Frontend Page Wiring Summary

**Inbox detail pane wired to `InboxAttachmentList` — MVP-amber-hint deleted, single component invocation inserted between `<pre>` body and assignment-section divider, Component-First principle enforced (zero inline RSX iteration in page file). WASM build green; manual smoke test pending checkpoint.**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-06-07T11:23:00Z (approx, plan loaded)
- **Completed:** 2026-06-07T11:28:00Z (Task 1; Task 2 checkpoint pending)
- **Tasks:** 1 of 2 (Task 2 is a `checkpoint:human-verify`)
- **Files modified:** 1

## Accomplishments

- **MVP-hint deletion:** The 5-line block `if d.has_attachments { div { class:"text-xs text-amber-700", "📎 Diese Mail enthält Anhänge (nicht anzeigbar im MVP)" } }` removed from the flex-none header section of the detail pane.
- **Import addition:** `InboxAttachmentList` added to the existing inbox-component import group (multi-line list, alphabetical with the existing three exports `InboxMailListItem`, `InboxReplyForm`, `InboxStatusBadge`).
- **Component invocation:** Single `InboxAttachmentList { mail_id, attachments, has_legacy_attachments }` call inserted inside the scrollable body column, between the body `<pre>` and the assignment `<div class="border-t pt-2 mt-2">` section.
- **Props derivation:**
  - `mail_id: d.id.clone()` — direct UUID-string clone from `InboundMailDetailTO`
  - `attachments: d.attachments.clone()` — `Vec<InboundMailAttachmentTO>` clone (the field was added in Plan 19-05 to the TO)
  - `has_legacy_attachments: d.attachments.is_empty() && d.has_attachments` — D-06 legacy-branch trigger
- **Component-First compliance:** `grep -cE "for .* in .*attachments"` returns 0 in `inbox_page.rs`. The page does NOT iterate, render `<img>`, render `<li>`, or render `<a download>` for attachments anywhere. The page is a pure composer; the component does all rendering.

## Task Commits

1. **Task 1: Delete MVP-hint + insert InboxAttachmentList call** — `26bc5e2` (feat)

**Task 2 (checkpoint:human-verify):** pending — see "Human Verification Required" below.

**Plan metadata commit:** pending Task 2 closure.

## Files Created/Modified

- `genossi-frontend/src/page/inbox_page.rs` — +9 / -6 LOC
  - Imports: replaced single-line `use crate::component::inbox::{...}` with multi-line list including `InboxAttachmentList`
  - Body section: deleted 5-line MVP-amber-hint `if d.has_attachments { ... }` block
  - Body section: inserted 5-line `InboxAttachmentList { ... }` component call between `<pre>` and assignment-section divider

## Decisions Made

- **Multi-line import group:** chose `use crate::component::inbox::{ InboxAttachmentList, InboxMailListItem, InboxReplyForm, InboxStatusBadge };` over a separate `use crate::component::inbox::InboxAttachmentList;` line. Keeps all inbox-component imports cohesive (alphabetical, single block); matches rustfmt's default group-imports behavior.
- **Component positioned in scrollable body, not in flex-none header:** UI-SPEC §Component Contract specifies the component goes "inside the scrollable body column ... AFTER the `<pre>` body and BEFORE the assignment section divider". This means attachments scroll with the body content, not get pinned in the meta-info header above. Confirmed in pre-existing file: the assignment-section already lives inside `div { class: "flex-1 overflow-y-auto flex flex-col gap-2 mt-2" }`, and the attachment section now sits in the same scroll container right before it.
- **The has_html_body hint (lines 336-340 in pre-mod file) stays:** that block is `if d.has_html_body && d.body_text.is_empty() { ... "Nur HTML-Inhalt vorhanden — im MVP nicht gerendert." }` — a separate concern about HTML-only mails without text body. Unrelated to attachments, NOT in plan scope.

## Deviations from Plan

None — plan executed exactly as written. All grep gates green, WASM build green on first try.

## Issues Encountered

- **`cargo check -p genossi-frontend` failed with "package ID specification did not match"** when run from the workspace root with the dashed package name. Resolved by running `cargo check --target wasm32-unknown-unknown` directly from the `genossi-frontend/` working directory (which the executor agent is already in per `cwd`). No code change, only verification-command adjustment.
- **Pre-existing modified files in working tree** (`genossi-frontend/assets/tailwind.css`, `genossi-frontend/rest-types/Cargo.lock`) carried over from prior sessions. NOT in scope of Plan 19-06 — left untouched. Only `inbox_page.rs` was staged + committed.

## Auth Gates

None — this plan is pure frontend page-wiring. No login flow, no OIDC config, no external secrets.

## User Setup Required

None — no external service configuration. The pre-existing `imap_*` configuration keys (Plans 19-01 through 19-04) must be set for a fully populated smoke test, but that's a Plan 19-02/19-04 dependency, not a Plan 19-06 requirement.

## Human Verification Required (Task 2 checkpoint:human-verify)

**Setup:**

1. Ensure a real inbox with mails is configured (`imap_*` config keys set). If not, manually seed a row via SQL:
   ```sql
   INSERT INTO inbound_mail_attachments (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized)
   VALUES (randomblob(16), <existing_mail_id_blob>, '2026-06-07T12:00:00.000Z', 'rechnung.pdf', 'application/pdf', 12345, 'inbound_mail_attachments/<mid>/<aid>', 0);
   ```
   Place the file manually under `$DOCUMENT_STORAGE_PATH/inbound_mail_attachments/<mid>/<aid>` (e.g. a small PDF/PNG).

**Start backend:**
```bash
DATABASE_URL=sqlite:genossi.db cargo run --bin genossi
```

Expected in log:
- `inbox_attachment_backfill: starting (N candidates)` (N ≥ 0)
- `inbox_attachment_backfill: done (Y persisted, Z skipped)`

**Start frontend:**
```bash
cd genossi-frontend && npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch &
dx serve --hot-reload
```

**In browser:**

1. Vorstand-Login via OIDC (or mock-auth feature-flag).
2. Open Inbox.
3. Select a mail with an attachment.
4. Below the body, verify:
   - Section header `📎 Anhänge (N)` in `text-sm font-semibold`
   - Per attachment a `<li>` row with filename + size + MIME-label
   - For `image/*`: thumbnail `<img>` (max-h-24), clickable → opens large in new tab
   - For `application/pdf`: blue Download button + secondary `Vorschau` link
   - For other MIME: only Download button
5. Click `Herunterladen` → browser starts native download with correct filename (Content-Disposition: attachment).
6. Click `Vorschau` on a PDF → opens inline in new tab (Content-Disposition: inline).
7. Filename edge case: a mail with Umlauts in filename (`Rückzahlung.pdf`) → Download filename in browser shows UTF-8 correctly.

**Empty/Legacy case verification:**

In DB: `UPDATE inbound_mails SET has_attachments = 1 WHERE id = <mail_ohne_attachment_rows>;`
- Frontend shows amber hint: "Anhang vor Phase 19 empfangen — bitte im Mail-Client öffnen"

**Oversized case verification:**

In DB:
```sql
INSERT INTO inbound_mail_attachments (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized)
VALUES (randomblob(16), <mid>, '2026-06-07T12:00:00.000Z', 'big.zip', 'application/zip', 12345678, NULL, 1);
```
- Frontend row shows amber hint: "Zu groß — bitte im Mail-Client öffnen" (NO Download button)

**Browser DevTools checks:**
- Network tab: Download request returns 200 + matching Content-Type + correct Content-Disposition
- Cross-Mail-IDOR: manually `https://localhost:3000/api/inbox/<wrong_mail_id>/attachments/<attachment_id>` → 404

**Resume signals:**
- Type `approved` if everything works. On findings:
  - "filename umlaut broken" → re-check backend http_util encoding
  - "preview not opening" → check browser PDF MIME handler + backend Content-Type
  - "oversized still shows download button" → check component branch (Plan 19-05 Task 2)
  - "legacy hint never appears" → check `has_legacy_attachments` expression in inbox_page
  - "404 on every download in dev" → Pitfall 8 (Cookie-Forwarding cross-origin); verify Dioxus.toml proxy setup

## Next Phase Readiness

- **Phase 19 end-to-end UI path is code-complete:** migration (19-01), persistence (19-02), REST endpoint (19-03), backfill worker (19-04), components (19-05), page wiring (19-06).
- **Pending:** Vorstand smoke test (Task 2 checkpoint) — once approved, Phase 19 closes with the checker UAT sign-off in `19-UI-SPEC.md` §Checker Sign-Off Dimensions.
- **No blockers identified** — frontend WASM build green, all grep gates green, Component-First principle satisfied.

## Self-Check: PASSED

- `grep -c "nicht anzeigbar im MVP" genossi-frontend/src/page/inbox_page.rs` → 0 (MVP hint deleted)
- `grep -c "InboxAttachmentList" genossi-frontend/src/page/inbox_page.rs` → 2 (1 import + 1 invocation)
- `grep -c "mail_id: d.id.clone()" genossi-frontend/src/page/inbox_page.rs` → 1
- `grep -c "has_legacy_attachments" genossi-frontend/src/page/inbox_page.rs` → 1
- `grep -c "d.attachments.is_empty() && d.has_attachments" genossi-frontend/src/page/inbox_page.rs` → 1
- `grep -cE "for .* in .*attachments" genossi-frontend/src/page/inbox_page.rs` → 0 (Component-First gate clean)
- `cargo check --target wasm32-unknown-unknown` exits 0 (warnings are pre-existing unused i18n keys, NOT errors)
- Commit `26bc5e2` exists: confirmed via `git rev-parse --short HEAD`
- Pre-existing modified files (`tailwind.css`, `rest-types/Cargo.lock`) left untouched — not scope of Plan 19-06

---
*Phase: 19-e-mail-anhaenge-anzeigen*
*Plan: 19-06-frontend-page-wiring*
*Status: Task 1 complete; Task 2 checkpoint:human-verify pending*
*Completed: 2026-06-07*
