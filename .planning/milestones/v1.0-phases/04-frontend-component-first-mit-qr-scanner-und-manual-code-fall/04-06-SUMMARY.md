# Plan 04-06 — Summary

**Phase:** 04 (Frontend Component-First mit QR-Scanner + Manual-Code-Fallback)
**Plan:** 06 — Vorstand-spezifische Components + Tab/Toast-Extraction
**Wave:** 2 (parallel mit 04, 05; mod.rs-Update folgt in Wave 2.5 via Plan 06b)
**Depends on:** 03 (Frontend API surface + i18n keys)
**Status:** Done
**Completed:** 2026-05-05

---

## Was wurde gebaut

Sieben Vorstand-spezifische Components in `genossi-frontend/src/component/` für Plan 04-08 (`assemblies`-Page + `assembly_details`-Page mit 3-Tab-Layout). Tab-Pattern wurde aus `applications_page.rs:78-103` Component-First-konform extrahiert; Toast-Pattern aus `members.rs:49-62`. Zusätzlich `BasicsTab` mit W-01 Edit-Mode für `update_assembly` (Preparation-only) und Open/Close-Confirm-Modals — damit Plan 08 nur noch komponiert.

**B-01-Routing eingehalten:** Dieser Plan hat `genossi-frontend/src/component/mod.rs` NICHT modifiziert — Plan 06b serialisiert den `pub mod`/`pub use`-Append für alle in Wave 2 erzeugten Components.

---

## Components (7)

| # | Component | Datei | Zeilen | Key-Decision |
|---|-----------|-------|--------|--------------|
| 1 | `AssemblyStatusBadge` | `assembly_status_badge.rs` | 81 | 1:1 Pattern aus `application_list.rs:7-27`, andere Enum/Farben (gray/green/blue für Preparation/Open/Closed laut UI-SPEC §Status-Badge palette) — als eigene Component verpackt (D-13), weil 2x reused (Liste + Detail-Header). Tests für 3 Farb-Mappings + konsistente Pill-Klassen. |
| 2 | `AssemblyListRow` | `assembly_list_row.rs` | 36 | Card-Style statt Table, mobile-first, `<Link>` zu `Route::AssemblyDetails { id }` (Forward-Reference auf Plan 07 — Plan-Spezifikation explizit erlaubt). |
| 3 | `TabStrip` (+ `TabDef`) | `tab_strip.rs` | 78 | Generischer Tab-Component mit `tabs: Vec<TabDef>`, `active_key: String`, `on_change: EventHandler<String>`, `children: Element` (Body-Slot). `print:hidden` auf Strip; Body druckt mit. role=tab. **`applications_page.rs` wurde NICHT migriert** — Plan-Anweisung „leave inline copy" (default keep blast-radius small); Plan 08 nutzt ausschließlich den neuen TabStrip; spätere Refactor-Phase kann applications_page migrieren. |
| 4 | `Toast` (`ToastContainer` + `show_toast`) | `toast.rs` | 47 | `show_toast`-Helper unverändert von `members.rs:49-62` (5s Auto-Remove via `gloo_timers::TimeoutFuture`); kein Context-Provider — Caller hält `Signal<Vec<(u64, String)>>` + `Signal<u64>` lokal und reicht via `&mut Signal` an `show_toast`. ToastContainer ist ein dünner View-Component, der `messages: ReadOnlySignal<Vec<(u64, String)>>` rendert (bottom-center mobile, top-right desktop). **`members.rs` wurde NICHT migriert** (gleiche Begründung wie #3). |
| 5 | `TokenRow` | `token_row.rs` | 96 | Row in der Tokens-Liste mit Status-Badge (yellow/green/gray für Open/Used/Revoked), Memo, eingelöst-am, Revoke-Button (nur wenn Open) + Confirm-Modal. Ruft `api::revoke_helper_token` und propagiert via `on_changed`/`on_error` an Parent. |
| 6 | `CreateTokenForm` | `create_token_form.rs` | 70 | Inline-Form (Memo-Input + Submit) für `<Modal>`-Wrap durch Caller. Validierung: leeres Memo → on_error mit i18n-String. Submit ruft `api::create_helper_token` und propagiert `HelperTokenCreateResponseTO` via `on_created` (Plan 08 nutzt das, um QrCard nach erfolgreichem Create einmalig anzuzeigen). i18n-Borrow-Workaround: Key vorab via `let memo_required_msg = i18n.t(...).to_string()` aufgelöst, weil `I18n` nicht Copy ist und in mehreren Closures genutzt wird. |
| 7 | `BasicsTab` | `basics_tab.rs` | 257 | Stamm-Daten-Display mit ReadOnly/Edit-Toggle (lokales `BasicsMode`-Enum). Edit nur in `Preparation` (D-08). Edit-Form: Name + Datum (datetime-local) + Ort + Save/Cancel; Submit ruft `api::update_assembly` (W-01: D-22). Außerdem GV-öffnen-/-schließen-Buttons mit Confirm-Modals (Open im `Preparation`, Close im `Open`); rufen `open_assembly`/`close_assembly`. Confirm-Dialogs sind hier zentralisiert, damit Plan 08 sie nicht inline duplizieren muss. |

**Total:** 665 Zeilen Component-Code.

---

## Key Decisions

### Migration: applications_page + members.rs NICHT migriert
Plan-Anweisung explizit: „extract only — keep blast-radius small". `applications_page.rs:78-103` behält seine inline-Tabs; `members.rs:49-62` behält seine inline-`show_toast`-Funktion. Plan 08 (assembly_details, assemblies) und Plan 09 (helper_attendance) nutzen ausschließlich die neuen Components. Spätere Refactor-Phase (out-of-scope hier) kann diese beiden Pages auf die neuen Components migrieren.

### Toast-Pattern: kein Context-Provider
`show_toast` ist eine Free-Function, kein Hook/Context-Provider. Caller hält `toast_messages: Signal<Vec<(u64, String)>>` und `toast_counter: Signal<u64>` lokal und reicht via `&mut Signal` durch. Begründung: 1:1 Pattern-Erhaltung von `members.rs:49-62` für niedrigste Refactor-Surface; ToastContainer ist View-only. Ein globaler Toast-Service wäre Phase-5-Erweiterung.

### Forward-Reference auf Route::AssemblyDetails
`AssemblyListRow` nutzt `Route::AssemblyDetails { id }`, das Plan 07 erst anlegt. Plan-Spezifikation explizit erlaubt diese Forward-Reference. Da `mod.rs` die neuen Component-Module nicht deklariert, werden sie aktuell nicht compiliert — `cargo check` ist grün. Sobald Plan 06b mod.rs ergänzt UND Plan 07 die Route hinzugefügt hat, compilieren beide Module zusammen.

### W-01 BasicsTab Edit-Mode (D-22 update_assembly)
Edit-Mode-Toggle nur sichtbar im Status `Preparation` (D-08). Nach `Open` ist update verbandskonform unerwünscht (Stamm-Daten der eröffneten GV bleiben fix). Falls Backend trotzdem 200 zurückgibt: das ist Plan-9-Refinement.

---

## Tests

| File | Test count | Decken ab |
|------|-----------|-----------|
| `assembly_status_badge.rs` | 4 | Pure-Funktionen `status_badge_class` für 3 Farb-Mappings + konsistente Pill-Class-Konvention |
| `tab_strip.rs` | 2 | `TabDef` Clone + Equality-Discriminierung |
| `basics_tab.rs` | 1 | `BasicsMode` Copy/Eq |

**7 Tests** insgesamt für reine Logik. Render-Tests sind in der Codebase nicht etabliert (kein `wasm-bindgen-test`-Setup); Component-Render-Verifikation erfolgt in Phase-5-Generalprobe (siehe 04-CONTEXT.md §Test-Strategie).

**Hinweis:** Tests laufen erst NACH Plan 06b (mod.rs-Append) — solange das Modul nicht in `pub mod`-Liste steht, werden auch die Tests nicht compiliert. `cargo check`/`cargo test` ist im Worktree-Endzustand für die genossi-frontend-bin grün, weil die nicht-deklarierten Component-Files schlicht ignoriert werden. Plan 06b aktiviert sie.

---

## Verification

- `cargo check -p genossi-frontend` (im genossi-frontend-Verzeichnis) — grün vor jedem Commit
- Plan-Anker `grep`-Verifikation:
  - `pub fn AssemblyStatusBadge` ✓ (assembly_status_badge.rs)
  - `bg-gray-100 text-gray-800`, `bg-green-100 text-green-800`, `bg-blue-100 text-blue-800` ✓
  - `pub fn AssemblyListRow` ✓ + `Route::AssemblyDetails` ✓
  - `pub fn TabStrip` + `pub struct TabDef` + `border-b-2 border-blue-600` + `print:hidden` ✓
  - `pub fn ToastContainer` + `pub fn show_toast` + `TimeoutFuture::new(5_000)` ✓
  - `pub fn TokenRow` + `revoke_helper_token` ✓
  - `pub fn CreateTokenForm` + `create_helper_token` ✓
  - `pub fn BasicsTab` + `update_assembly` + `BasicsMode::Edit` ✓
- `component/mod.rs` NICHT modifiziert in den 04-06-Commits (verifiziert via `git log -- genossi-frontend/src/component/mod.rs` zeigt keine `04-06`-Commits).

---

## Commits

| Task | Commit-SHA | Message |
|------|-----------|---------|
| Task 1 | `3881e65` | `feat(04-06): AssemblyStatusBadge + AssemblyListRow components` |
| Task 2 | `024d716` | `feat(04-06): TabStrip + Toast components extracted (Component-First)` |
| Task 3 | `98aa32a` | `feat(04-06): TokenRow + CreateTokenForm + BasicsTab (W-04 Component-Extraction + W-01 update_assembly Edit-Mode)` |

---

## Files modified

```
genossi-frontend/src/component/assembly_status_badge.rs    (new, 81 lines)
genossi-frontend/src/component/assembly_list_row.rs        (new, 36 lines)
genossi-frontend/src/component/tab_strip.rs                (new, 78 lines)
genossi-frontend/src/component/toast.rs                    (new, 47 lines)
genossi-frontend/src/component/token_row.rs                (new, 96 lines)
genossi-frontend/src/component/create_token_form.rs        (new, 70 lines)
genossi-frontend/src/component/basics_tab.rs               (new, 257 lines)
```

**Nicht modifiziert (B-01-Routing):** `genossi-frontend/src/component/mod.rs` — Plan 06b appended dort die `pub mod`/`pub use`-Zeilen für alle in Wave 2 erzeugten Components in einem einzigen Schritt.

---

## Hand-off zu Plan 08 (assembly_details / assemblies)

Plan 08 importiert nach Plan 06b:

```rust
use crate::component::{
    AssemblyStatusBadge, AssemblyListRow, TabStrip, TabDef,
    ToastContainer, show_toast, TokenRow, CreateTokenForm, BasicsTab,
    Modal, // bereits exportiert
};
```

Pages komponieren nur — keine inline-RSX-Duplikate von Status-Badges, Tab-Strips, Toasts, Token-Rows, Create-Forms, Basics-Tabs.

---

## Deviations

- **mod.rs Update:** ausgelagert an Plan 06b (B-01-Routing serialisiert Wave-2-Module).
- **Tests:** nur Pure-Function-Tests; Render-Tests fehlen mangels `wasm-bindgen-test`-Setup im Projekt (siehe 04-CONTEXT.md). Phase-5-Generalprobe ist die finale Render-Verifikation.
- **applications_page.rs / members.rs:** **nicht migriert** — Plan-Anweisung explizit „leave inline copies".
- **i18n-Borrow-Workaround in CreateTokenForm:** `let memo_required_msg = i18n.t(Key::HelperTokenMemo).to_string()` vorab aufgelöst, weil `I18n` nicht Copy ist; sonst Compile-Error E0382 in der `onsubmit`-Closure.

---

*Phase: 4 — Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback*
*Plan: 06 — Vorstand-Components + Tab/Toast-Extraction*
*Authored: 2026-05-05*
