---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-05
slug: frontend-components
type: execute
wave: 4
depends_on: [19-03]
files_modified:
  - genossi-frontend/src/util/mod.rs
  - genossi-frontend/src/util/format.rs
  - genossi-frontend/src/component/inbox/attachment_list.rs
  - genossi-frontend/src/component/inbox/attachment_list_item.rs
  - genossi-frontend/src/component/inbox/mod.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/main.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "Two new components live under `genossi-frontend/src/component/inbox/`: `InboxAttachmentList` (header + list) and `InboxAttachmentListItem` (row + actions) — Component-First (D-13)"
    - "List component hides itself when `attachments.is_empty() && !has_legacy_attachments` (UI-SPEC §Component Contract)"
    - "Legacy-empty case renders amber hint with `InboxAttachmentsEmptyLegacy` copy (D-06)"
    - "Oversized rows render amber `InboxAttachmentsOversized` label + size — NO download/preview buttons (D-02, D-11)"
    - "Image rows show `<img>` thumbnail wrapped in `<a target=\"_blank\" rel=\"noopener\">` pointing at the inline-disposition URL (D-12, T-08)"
    - "PDF rows show primary `Herunterladen` `<a download>` PLUS secondary `Vorschau` `<a target=\"_blank\" rel=\"noopener\">` to inline URL (D-12)"
    - "All other MIME rows show ONLY primary `Herunterladen` `<a download>` (D-12)"
    - "All action elements are `<a>` anchors, never `<button onclick>` — avoids Dioxus button-reload bug (memory `feedback_dioxus_button_type.md`)"
    - "Seven i18n keys exist in `Key` enum AND in both `de.rs` and `en.rs` (D-14, frontend-CLAUDE.md two-locale rule)"
    - "`format_size` integer-math util in `src/util/format.rs` with 4 unit tests covering B/KB/MB/GB ranges (UI-SPEC §Formatting & States)"
    - "Filename is rendered as RSX text-content (Dioxus auto-escapes) — never as raw HTML (T-05 XSS mitigation)"
  artifacts:
    - path: "genossi-frontend/src/util/format.rs"
      provides: "format_size(u64) -> String"
      contains: "pub fn format_size"
    - path: "genossi-frontend/src/util/mod.rs"
      provides: "Module registry for util/"
      contains: "pub mod format"
    - path: "genossi-frontend/src/component/inbox/attachment_list.rs"
      provides: "InboxAttachmentList component"
      contains: "pub fn InboxAttachmentList"
    - path: "genossi-frontend/src/component/inbox/attachment_list_item.rs"
      provides: "InboxAttachmentListItem component + glyph_for_mime/short_mime helpers"
      contains: "pub fn InboxAttachmentListItem"
    - path: "genossi-frontend/src/component/inbox/mod.rs"
      provides: "Registry for two new components"
      contains: "pub use attachment_list"
    - path: "genossi-frontend/src/i18n/mod.rs"
      provides: "7 new Key variants"
      contains: "InboxAttachmentsHeader"
    - path: "genossi-frontend/src/i18n/de.rs"
      provides: "7 German translations"
      contains: "InboxAttachmentsHeader"
    - path: "genossi-frontend/src/i18n/en.rs"
      provides: "7 English translations"
      contains: "InboxAttachmentsHeader"
  key_links:
    - from: "InboxAttachmentList"
      to: "InboxAttachmentListItem"
      via: "iteration in RSX for-loop"
      pattern: "for att in attachments"
    - from: "InboxAttachmentListItem"
      to: "format_size util"
      via: "import + call with attachment.size_bytes"
      pattern: "format_size\\("
    - from: "Both components"
      to: "i18n Key enum"
      via: "i18n.t(Key::InboxAttachments…)"
      pattern: "Key::InboxAttachments"

---

<objective>
Lege die zwei Dioxus-Components + die `format_size`-Util + alle 7 i18n-Keys an.
Page-Wiring kommt in Plan 19-06.

Purpose: Component-First-Prinzip + UI-SPEC §Action Matrix exakt umsetzen.
Plan 19-06 darf danach NUR noch einen einzigen Component-Aufruf in
`inbox_page.rs` einsetzen.

Output: Sieben Dateien (1 util-mod, 1 util-format, 2 components, 1 component
registry, 3 i18n files) + main.rs Modul-Deklaration.

**Scope rationale (accepted borderline):** Plan deckt drei zusammenhängende
Frontend-Schichten in 9 Files ab — Util + i18n + TO in Task 1 (~4 Files),
Components + Registry + Page-Modul-Decl in Task 2 (~5 Files). Akzeptiert als
one-plan-scope, weil Task 1 (i18n-Keys, `format_size`, `InboundMailAttachmentTO`)
ohne Task 2 (Components, die diese konsumieren) nicht in der UI testbar ist.
Ein Split würde Task 1 mit einer leeren `cargo check`-Verifikation hinterlassen
und Plan 19-06 müsste auf zwei separate Vorgänger-Plans warten. Cleaner als
one-plan.
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-UI-SPEC.md
@genossi-frontend/CLAUDE.md

<interfaces>
<!-- Pre-extracted from analog files + RESEARCH/PATTERNS Code Examples. -->

From `genossi-frontend/src/component/inbox/mail_list_item.rs` (whole file, style baseline):
```rust
use dioxus::prelude::*;
use super::InboxStatusBadge;

#[component]
pub fn InboxMailListItem(
    subject: String,
    /* … other props … */
    on_click: EventHandler<()>,
) -> Element { /* … RSX with Tailwind classes … */ }
```

From `genossi-frontend/src/component/inbox/mod.rs` (whole file):
```rust
pub mod mail_list_item;
pub mod reply_form;
pub mod status_badge;

pub use mail_list_item::InboxMailListItem;
pub use reply_form::InboxReplyForm;
pub use status_badge::InboxStatusBadge;
```

From `genossi-frontend/src/i18n/mod.rs:504-505` (positional anchor for inbox keys):
```rust
OpenInboxCount,
OpenInboxNone,
```

From `genossi-frontend/src/i18n/de.rs:436-437` (translation pattern):
```rust
Key::OpenInboxCount => "{} offene Mails".into(),
Key::OpenInboxNone => "Keine offenen Mails".into(),
```

`Config` and `CONFIG` global state — accessible via `crate::service::config::CONFIG` (URL prefix `cfg.backend`).

InboundMailAttachmentTO will be added to `genossi-frontend/src/api.rs` in Plan 19-06. For Plan 19-05, we MUST also add the TO definition here OR define it locally in the component. To keep wiring clean: add the TO to `api.rs` as part of THIS plan (Plan 19-06 only handles DetailTO field extension + page wiring). See action Step 6 below.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Util format module + i18n keys + InboundMailAttachmentTO</name>
  <files>
    genossi-frontend/src/util/mod.rs,
    genossi-frontend/src/util/format.rs,
    genossi-frontend/src/main.rs,
    genossi-frontend/src/i18n/mod.rs,
    genossi-frontend/src/i18n/de.rs,
    genossi-frontend/src/i18n/en.rs,
    genossi-frontend/src/api.rs
  </files>
  <read_first>
    - genossi-frontend/src/main.rs (search for existing `pub mod i18n;` / `pub mod component;` to find module-decl block)
    - genossi-frontend/src/i18n/mod.rs (Key enum at :45-100+ and around :504-505 for inbox-key positional anchor)
    - genossi-frontend/src/i18n/de.rs:425-450 (translations for OpenInboxCount/OpenInboxNone — line 436-437)
    - genossi-frontend/src/i18n/en.rs (find OpenInboxCount/OpenInboxNone — same positional anchor)
    - genossi-frontend/src/api.rs:1349-1420 (existing InboundMailDetailTO at :1364-1378 — add the new TO ABOVE it)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-UI-SPEC.md §Copywriting Contract (exact De/En strings)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Size Formatter (lines 992-1010 — verbatim implementation)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §15, §16, §17, §18, §19, §20
  </read_first>
  <behavior>
    - New file `src/util/mod.rs` with single line `pub mod format;`
    - New file `src/util/format.rs` with `pub fn format_size(bytes: u64) -> String` using integer-math (no `{:.1}` floats) per UI-SPEC §Formatting & States
    - 4 unit tests in `format.rs::tests`: bytes < KB, KB range integer, MB range one-decimal, GB range one-decimal
    - `src/main.rs` (or `lib.rs` — whichever declares modules) gains `pub mod util;` in module-decl block (alphabetical position)
    - `i18n/mod.rs` Key enum gains 7 variants in inbox-key region (after `OpenInboxNone`):
      InboxAttachmentsHeader, InboxAttachmentsDownload, InboxAttachmentsPreview,
      InboxAttachmentsEmptyLegacy, InboxAttachmentsOversized,
      InboxAttachmentsDownloadError, InboxAttachmentsImageAltPrefix
    - `i18n/de.rs` gains 7 De translations (exact strings from UI-SPEC §Copywriting Contract)
    - `i18n/en.rs` gains 7 En translations
    - `src/api.rs` gains struct `InboundMailAttachmentTO { id, file_name, mime_type, size_bytes, oversized }` matching backend D-07, and `InboundMailDetailTO` gains `pub attachments: Vec<InboundMailAttachmentTO>` field for cross-plan API compatibility. (Plan 19-06 reads/uses this field; defining it here keeps Plan 19-05's components compilable.)
  </behavior>
  <action>
    **Step 1 — Create `genossi-frontend/src/util/mod.rs`** with verbatim contents:
    ```rust
    pub mod format;
    ```

    **Step 2 — Create `genossi-frontend/src/util/format.rs`** with verbatim contents (from RESEARCH §Size Formatter lines 992-1010 + the test block from PATTERNS.md §15):
    ```rust
    /// Format a byte count into a human-readable string.
    /// Integer-math to avoid floating rounding surprises.
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;
        const GB: u64 = 1024 * 1024 * 1024;
        if bytes < KB { format!("{} B", bytes) }
        else if bytes < MB { format!("{} KB", bytes / KB) }
        else if bytes < GB {
            let tenths = bytes * 10 / MB;
            format!("{}.{} MB", tenths / 10, tenths % 10)
        } else {
            let tenths = bytes * 10 / GB;
            format!("{}.{} GB", tenths / 10, tenths % 10)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn bytes_under_kb() {
            assert_eq!(format_size(42), "42 B");
            assert_eq!(format_size(999), "999 B");
        }
        #[test]
        fn kb_range_integer() {
            assert_eq!(format_size(12 * 1024), "12 KB");
            assert_eq!(format_size(1023 * 1024), "1023 KB");
        }
        #[test]
        fn mb_range_one_decimal() {
            assert_eq!(format_size(1_468_006), "1.4 MB");
        }
        #[test]
        fn gb_range_one_decimal() {
            let b: u64 = 12 * 1024 * 1024 * 1024 / 10;
            assert_eq!(format_size(b), "1.2 GB");
        }
    }
    ```

    **Step 3 — Update `genossi-frontend/src/main.rs`**: find the block where modules are declared (e.g. `pub mod i18n;`, `pub mod component;`, etc.). Add `pub mod util;` in alphabetical position (after `pub mod service;` or wherever `u` belongs alphabetically). If frontend has a `lib.rs` that also lists modules, add it there too.

    **Step 4 — Update `genossi-frontend/src/i18n/mod.rs`**: locate `Key` enum around `:45-100+`. Find the inbox-related cluster (positional anchor: `OpenInboxCount, OpenInboxNone` at `:504-505`). Insert the 7 new variants AFTER `OpenInboxNone`:
    ```rust
    InboxAttachmentsHeader,
    InboxAttachmentsDownload,
    InboxAttachmentsPreview,
    InboxAttachmentsEmptyLegacy,
    InboxAttachmentsOversized,
    InboxAttachmentsDownloadError,
    InboxAttachmentsImageAltPrefix,
    ```

    **Step 5 — Update `genossi-frontend/src/i18n/de.rs`**: locate the match-arm pattern at `:436-437` for `OpenInboxCount`/`OpenInboxNone`. Add 7 new arms (exact De copy from UI-SPEC §Copywriting Contract):
    ```rust
    Key::InboxAttachmentsHeader => "Anhänge".into(),
    Key::InboxAttachmentsDownload => "Herunterladen".into(),
    Key::InboxAttachmentsPreview => "Vorschau".into(),
    Key::InboxAttachmentsEmptyLegacy => "Anhang vor Phase 19 empfangen — bitte im Mail-Client öffnen".into(),
    Key::InboxAttachmentsOversized => "Zu groß — bitte im Mail-Client öffnen".into(),
    Key::InboxAttachmentsDownloadError => "Anhang konnte nicht geladen werden — bitte erneut versuchen".into(),
    Key::InboxAttachmentsImageAltPrefix => "Vorschau für".into(),
    ```

    (Note: The `{size}` placeholder in `Oversized` is composed CLIENT-side in the component via `format!("{} ({})", i18n.t(...), size_str)` — the i18n value here is the base copy without interpolation. Same pattern for `{file_name}` in `ImageAltPrefix`.)

    **Step 6 — Update `genossi-frontend/src/i18n/en.rs`**: same positional anchor as `de.rs`. Add 7 new arms (exact En copy from UI-SPEC):
    ```rust
    Key::InboxAttachmentsHeader => "Attachments".into(),
    Key::InboxAttachmentsDownload => "Download".into(),
    Key::InboxAttachmentsPreview => "Preview".into(),
    Key::InboxAttachmentsEmptyLegacy => "Attachment received before Phase 19 — open in your mail client".into(),
    Key::InboxAttachmentsOversized => "Too large — open in your mail client".into(),
    Key::InboxAttachmentsDownloadError => "Could not load attachment — please try again".into(),
    Key::InboxAttachmentsImageAltPrefix => "Preview of".into(),
    ```

    **Step 7 — Extend `genossi-frontend/src/api.rs`**: locate `InboundMailDetailTO` at `:1364-1378`. Add the new TO IMMEDIATELY ABOVE it:
    ```rust
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct InboundMailAttachmentTO {
        pub id: String,
        pub file_name: String,
        pub mime_type: String,
        pub size_bytes: i64,
        pub oversized: bool,
    }
    ```
    Then add the new field to `InboundMailDetailTO` (after `has_attachments: bool,`):
    ```rust
    #[serde(default)]
    pub attachments: Vec<InboundMailAttachmentTO>,
    ```
    The `#[serde(default)]` allows the frontend to deserialize older backend responses (defensive — backend already adds the field in Plan 19-03, but defensive default protects against deploy-skew).
  </action>
  <verify>
    <automated>cargo test -p genossi-frontend util::format::tests 2>&amp;1 | tee /tmp/19-05-task1.log; grep -q "test result: ok. 4 passed" /tmp/19-05-task1.log &amp;&amp; grep -q "InboxAttachmentsHeader" genossi-frontend/src/i18n/mod.rs &amp;&amp; grep -q "Anhänge" genossi-frontend/src/i18n/de.rs &amp;&amp; grep -q "Attachments" genossi-frontend/src/i18n/en.rs &amp;&amp; grep -q "pub struct InboundMailAttachmentTO" genossi-frontend/src/api.rs</automated>
  </verify>
  <acceptance_criteria>
    - File `genossi-frontend/src/util/format.rs` exists with `pub fn format_size`
    - File `genossi-frontend/src/util/mod.rs` exists with `pub mod format;`
    - `grep -c "pub mod util;" genossi-frontend/src/main.rs` returns 1 (or in lib.rs if that's where modules live)
    - `grep -c "InboxAttachmentsHeader\\|InboxAttachmentsDownload\\|InboxAttachmentsPreview\\|InboxAttachmentsEmptyLegacy\\|InboxAttachmentsOversized\\|InboxAttachmentsDownloadError\\|InboxAttachmentsImageAltPrefix" genossi-frontend/src/i18n/mod.rs` returns ≥ 7
    - `grep -c "InboxAttachmentsHeader\\|InboxAttachmentsDownload\\|InboxAttachmentsPreview\\|InboxAttachmentsEmptyLegacy\\|InboxAttachmentsOversized\\|InboxAttachmentsDownloadError\\|InboxAttachmentsImageAltPrefix" genossi-frontend/src/i18n/de.rs` returns ≥ 7
    - `grep -c "InboxAttachmentsHeader\\|InboxAttachmentsDownload\\|InboxAttachmentsPreview\\|InboxAttachmentsEmptyLegacy\\|InboxAttachmentsOversized\\|InboxAttachmentsDownloadError\\|InboxAttachmentsImageAltPrefix" genossi-frontend/src/i18n/en.rs` returns ≥ 7
    - `grep -F "Anhänge" genossi-frontend/src/i18n/de.rs` matches (positive check on De copy)
    - `grep -F "Attachments" genossi-frontend/src/i18n/en.rs` matches (positive check on En copy)
    - `grep -c "pub struct InboundMailAttachmentTO" genossi-frontend/src/api.rs` returns 1
    - `grep -c "pub attachments: Vec<InboundMailAttachmentTO>" genossi-frontend/src/api.rs` returns 1
    - `cargo test -p genossi-frontend util::format::tests` exits 0 (4 tests pass)
    - `cargo check -p genossi-frontend` exits 0
  </acceptance_criteria>
  <done>
    Util + i18n + TO-Extension fertig — Components in Task 2 können auf alles zugreifen.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: InboxAttachmentList + InboxAttachmentListItem components + registry</name>
  <files>
    genossi-frontend/src/component/inbox/attachment_list.rs,
    genossi-frontend/src/component/inbox/attachment_list_item.rs,
    genossi-frontend/src/component/inbox/mod.rs
  </files>
  <read_first>
    - genossi-frontend/src/component/inbox/mod.rs (whole file, 7 lines — registry pattern)
    - genossi-frontend/src/component/inbox/mail_list_item.rs (whole file, 43 lines — style baseline, Tailwind class conventions, #[component] macro use)
    - genossi-frontend/src/api.rs (InboundMailAttachmentTO from Task 1)
    - genossi-frontend/src/service/config.rs (or wherever CONFIG lives — verify import path `crate::service::config::CONFIG`)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-UI-SPEC.md §Component Contract + §Action Matrix + §Glyph Table
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Code Examples → Frontend Component Skeleton (lines 837-988 — verbatim)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §10, §11, §12
  </read_first>
  <behavior>
    - `attachment_list.rs` exports `pub fn InboxAttachmentList(mail_id: String, attachments: Vec<InboundMailAttachmentTO>, has_legacy_attachments: bool) -> Element`
    - List component returns empty `rsx! { }` (no section rendered at all) when `attachments.is_empty() && !has_legacy_attachments` (UI-SPEC §Component Contract)
    - List wrapper: `div { class: "border-t pt-2 mt-3 flex flex-col gap-2" }` per UI-SPEC §Spacing Scale (lg+)
    - Header: `div { class: "text-sm font-semibold" }` containing `span { aria_hidden: "true", "📎 " }` followed by `{i18n.t(Key::InboxAttachmentsHeader)} ({attachments.len()})`
    - Legacy branch: when `attachments.is_empty() && has_legacy_attachments`, render `div { class: "text-xs text-amber-700", "{i18n.t(Key::InboxAttachmentsEmptyLegacy)}" }`
    - Non-empty branch: `ul { class: "flex flex-col gap-2", for att in attachments.iter().cloned() { InboxAttachmentListItem { mail_id: mail_id.clone(), attachment: att } } }`
    - `attachment_list_item.rs` exports `pub fn InboxAttachmentListItem(mail_id: String, attachment: InboundMailAttachmentTO) -> Element`
    - Reads `CONFIG.read().clone()` once, builds `download_url = "{cfg.backend}/api/inbox/{mail_id}/attachments/{attachment.id}"` and `inline_url = "{download_url}?disposition=inline"`
    - Calls `format_size(attachment.size_bytes.max(0) as u64)` from `crate::util::format`
    - **Oversized branch** (early return when `attachment.oversized == true`):
      ```rust
      li { class: "p-3 border rounded bg-white flex items-center gap-3",
          span { aria_hidden: "true", "📎" }
          div { class: "flex flex-col flex-1",
              span { class: "text-sm", "{attachment.file_name}" }
              span { class: "text-xs text-amber-700",
                  {format!("{} ({})", i18n.t(Key::InboxAttachmentsOversized), size_str)}
              }
          }
      }
      ```
    - **Non-oversized layout** (li outer with class `p-3 border rounded bg-white flex items-center gap-3`):
      1. Leading visual:
         - If `attachment.mime_type.starts_with("image/")`: anchor `<a href="{inline_url}" target="_blank" rel="noopener">` wrapping `<img src="{inline_url}" alt="{i18n.t(Key::InboxAttachmentsImageAltPrefix)} {attachment.file_name}" class="max-h-24 max-w-32 object-contain rounded border" loading="lazy" />`
         - Else: `span { aria_hidden: "true", {glyph_for_mime(&attachment.mime_type)} }`
      2. Metadata column: `div { class: "flex flex-col flex-1 min-w-0", span { class: "text-sm truncate", title: "{attachment.file_name}", "{attachment.file_name}" } span { class: "text-xs text-gray-500", "{size_str} · {short_mime(&attachment.mime_type)}" } }`
      3. Action column: `div { class: "flex gap-2 ml-auto" }` containing:
         - Primary `<a href="{download_url}" download="{attachment.file_name}" class="px-3 py-1.5 bg-blue-500 hover:bg-blue-600 text-white text-sm rounded">{i18n.t(Key::InboxAttachmentsDownload)}</a>`
         - If `mime_type == "application/pdf"`: also `<a href="{inline_url}" target="_blank" rel="noopener" class="px-3 py-1.5 text-blue-600 hover:underline text-sm">{i18n.t(Key::InboxAttachmentsPreview)}</a>`
    - Helper fns `glyph_for_mime(&str) -> &'static str` and `short_mime(&str) -> &'static str` per UI-SPEC §Glyph Table (private to the component file)
    - `mod.rs` declares both new modules and re-exports both components
    - Every action element is `<a>` — NO `<button onclick>` (memory `feedback_dioxus_button_type.md` page-reload bug)
    - `rel="noopener"` on every `target="_blank"` (T-08 open-redirect mitigation)
    - `loading="lazy"` on the image — verify compiles via `cargo check` (Pitfall 7)
  </behavior>
  <action>
    **Step 1 — Create `attachment_list.rs`** using the verbatim skeleton from RESEARCH lines 837-879 (§Code Examples → Frontend Component Skeleton). Key points the executor must keep exactly:
    - `use crate::api::InboundMailAttachmentTO;`
    - `use crate::i18n::{use_i18n, Key};`
    - `use super::InboxAttachmentListItem;`
    - `#[component]` macro on the fn
    - Three props in the specified order
    - Early-return `rsx! { }` when both attachments empty AND no legacy hint
    - Header uses `text-sm font-semibold` (UI-SPEC §Typography)
    - Wrapper div uses `border-t pt-2 mt-3 flex flex-col gap-2`
    - List ul uses `flex flex-col gap-2`
    - Legacy hint div uses `text-xs text-amber-700`

    **Step 2 — Create `attachment_list_item.rs`** using the verbatim skeleton from RESEARCH lines 882-987. Key points the executor must keep exactly:
    - All imports: `dioxus::prelude::*`, `crate::api::InboundMailAttachmentTO`, `crate::i18n::{use_i18n, Key}`, `crate::service::config::CONFIG`, `crate::util::format::format_size`
    - URL formation: `download_url = format!("{}/api/inbox/{}/attachments/{}", cfg.backend, mail_id, attachment.id)`, `inline_url = format!("{}?disposition=inline", download_url)`
    - `size_str = format_size(attachment.size_bytes.max(0) as u64)`
    - Oversized early-return branch (verbatim)
    - Image branch wraps `<img>` in `<a target="_blank" rel="noopener">` — pointing at inline URL
    - PDF: primary `Herunterladen` `<a download>` + secondary `Vorschau` `<a target="_blank" rel="noopener">`
    - All other MIME: only primary `Herunterladen` `<a download>`
    - `glyph_for_mime` and `short_mime` helpers verbatim from RESEARCH lines 971-988 (PDF=📄, image=🖼️, zip/tar/gz=🗜️, msword/wordprocessingml=📝, ms-excel/spreadsheetml=📊, text=📃, else=📎; short labels PDF/Bild/Word/Excel/Datei)

    **Step 3 — Update `genossi-frontend/src/component/inbox/mod.rs`**. The existing file is 7 lines (3 `pub mod` + 3 `pub use`). Replace with (preserving order — add new modules alphabetically before existing ones):
    ```rust
    pub mod attachment_list;
    pub mod attachment_list_item;
    pub mod mail_list_item;
    pub mod reply_form;
    pub mod status_badge;

    pub use attachment_list::InboxAttachmentList;
    pub use attachment_list_item::InboxAttachmentListItem;
    pub use mail_list_item::InboxMailListItem;
    pub use reply_form::InboxReplyForm;
    pub use status_badge::InboxStatusBadge;
    ```

    **Step 4 — Build the WASM frontend** to verify `loading: "lazy"` and `aria_hidden: "true"` attributes are accepted by Dioxus 0.6 RSX (Pitfall 7):
    ```bash
    cargo check -p genossi-frontend --target wasm32-unknown-unknown
    ```
    If `loading: "lazy"` is rejected by Dioxus 0.6 (rare), drop ONLY that attribute — lazy-loading is a perf optimization, not a functional requirement. Do NOT remove `aria_hidden` or `target`/`rel`.

    Anti-patterns to AVOID (locked by UI-SPEC §Anti-Patterns):
    - NO `<button onclick>` for download/preview (use `<a>` per `feedback_dioxus_button_type.md`)
    - NO inline preview for `text/html` or `text/plain` (only `image/*` and `application/pdf`)
    - NO hardcoded copy (always via `i18n.t(Key::…)`)
    - NO icon-library import (Unicode glyphs only — `📎`, `📄`, `🖼️`, `🗜️`, `📝`, `📊`, `📃`)
    - NO color outside the existing inbox palette
    - NO `aria-live` / toast for download success (browser-native UX is the signal)
  </action>
  <verify>
    <automated>cargo check -p genossi-frontend --target wasm32-unknown-unknown 2>&amp;1 | tee /tmp/19-05-task2-check.log; ! grep -q "^error" /tmp/19-05-task2-check.log &amp;&amp; test -f genossi-frontend/src/component/inbox/attachment_list.rs &amp;&amp; test -f genossi-frontend/src/component/inbox/attachment_list_item.rs &amp;&amp; grep -q "pub use attachment_list::InboxAttachmentList" genossi-frontend/src/component/inbox/mod.rs &amp;&amp; grep -q "pub use attachment_list_item::InboxAttachmentListItem" genossi-frontend/src/component/inbox/mod.rs</automated>
  </verify>
  <acceptance_criteria>
    - File `genossi-frontend/src/component/inbox/attachment_list.rs` exists with `pub fn InboxAttachmentList`
    - File `genossi-frontend/src/component/inbox/attachment_list_item.rs` exists with `pub fn InboxAttachmentListItem`
    - `grep -c "#\[component\]" genossi-frontend/src/component/inbox/attachment_list.rs` returns 1
    - `grep -c "#\[component\]" genossi-frontend/src/component/inbox/attachment_list_item.rs` returns 1
    - `grep -c 'target: "_blank"' genossi-frontend/src/component/inbox/attachment_list_item.rs` returns ≥ 1 (image/PDF preview anchors)
    - `grep -c 'rel: "noopener"' genossi-frontend/src/component/inbox/attachment_list_item.rs` returns ≥ 1 (T-08 mitigation — every target=_blank must have it)
    - `grep -c "onclick" genossi-frontend/src/component/inbox/attachment_list_item.rs` returns 0 (button-reload-bug avoidance — NO onclick handlers)
    - `grep -c "button" genossi-frontend/src/component/inbox/attachment_list_item.rs` returns 0 (no `<button>` elements for actions)
    - `grep -c "Key::InboxAttachments" genossi-frontend/src/component/inbox/attachment_list.rs genossi-frontend/src/component/inbox/attachment_list_item.rs` returns ≥ 6 (all 7 keys used somewhere; allow 6 because ImageAltPrefix may only be in item file)
    - `grep -c "bg-blue-500\|hover:bg-blue-600" genossi-frontend/src/component/inbox/attachment_list_item.rs` returns ≥ 1 (primary button color from UI-SPEC §Color)
    - `grep -c "text-amber-700" genossi-frontend/src/component/inbox/attachment_list_item.rs` returns ≥ 1 (oversized hint color)
    - `grep -c "pub use attachment_list::InboxAttachmentList" genossi-frontend/src/component/inbox/mod.rs` returns 1
    - `grep -c "pub use attachment_list_item::InboxAttachmentListItem" genossi-frontend/src/component/inbox/mod.rs` returns 1
    - `cargo check -p genossi-frontend --target wasm32-unknown-unknown` exits 0
  </acceptance_criteria>
  <done>
    Beide Components vorhanden + registriert, WASM-Build grün, kein `<button onclick>`-Pfad, `rel="noopener"` auf jedem `target="_blank"`, alle Aktionen über `<a>`.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Backend response → Dioxus RSX | Untrusted filename string flows into RSX text content |
| Image URL → browser DOM | `<img src>` and `<a href>` use backend-side same-origin URLs |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-05 | Stored XSS via filename in RSX | InboxAttachmentListItem | mitigate | Filename appears as RSX text-content (`"{attachment.file_name}"`) which Dioxus auto-escapes to text — never as raw HTML attribute value that could break out of context. Filename ALSO flows into `<img alt>` and `<a download>` — both browser-side auto-escaped by Dioxus's attribute serializer. NO `dangerous_inner_html` use anywhere. |
| T-08 | Open-redirect via target="_blank" | InboxAttachmentListItem | mitigate | Every `target: "_blank"` anchor carries `rel: "noopener"` (verified by grep gate). Prevents `window.opener` access from the inline preview tab. |

(T-01, T-02, T-03, T-04, T-06, T-07 owned by other plans.)
</threat_model>

<verification>
- `cargo test -p genossi-frontend util::format::tests` exits 0 (4 tests)
- `cargo check -p genossi-frontend --target wasm32-unknown-unknown` exits 0
- `grep -c "rel: \"noopener\"" genossi-frontend/src/component/inbox/attachment_list_item.rs` ≥ 1 (T-08 gate)
- `grep -c "onclick" genossi-frontend/src/component/inbox/attachment_list_item.rs` == 0 (button-reload-bug gate)
- 7 i18n keys appear in mod.rs + de.rs + en.rs (D-14)
- Two-locale rule satisfied — no other locale file edited
</verification>

<success_criteria>
- Components live under `src/component/inbox/` per Component-First (D-13)
- All copy goes through i18n with both locales populated (D-14)
- Anchor-driven actions only (memory `feedback_dioxus_button_type.md`)
- `format_size` integer-math with 4 passing unit tests
- WASM build green
- Plan 19-06 has everything it needs to do the page-wiring
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-05-SUMMARY.md`
</output>
