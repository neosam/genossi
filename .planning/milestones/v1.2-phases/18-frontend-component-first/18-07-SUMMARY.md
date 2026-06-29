---
phase: 18-frontend-component-first
plan: 07
subsystem: frontend-page-integration
tags:
  - frontend
  - dioxus
  - page
  - integration
  - uat
dependency_graph:
  requires:
    - "18-01: rest-types DTOs (used transitively via MembershipAdjustModal)"
    - "18-02: ToastVariant + show_success_toast + SuccessToastContainer (Plan 02)"
    - "18-04: 46 i18n keys (MembershipAdjustButtonLabel, MembershipAdjustSuccess used here)"
    - "18-06: MembershipAdjustModal component (mounted here)"
  provides:
    - Phase-18 end-to-end UI integration on Member-Detail-Page
    - Persistent Manual-UAT document (18-MANUAL-UAT.md) for Vorstand walk-through
  affects:
    - Phase 18 closure (ROADMAP-SC-4: Button + i18n + ManualUAT erfuellt)
    - Future Member-Detail-Page extensions that need shared today/Toast infra
tech_stack:
  added: []
  patterns:
    - "Page integrates shared Component instead of inline RSX (Component-First)"
    - "Dioxus reload-bug mitigation (r#type=button + onclick) on new button only (C-18-CF-03)"
    - "Member-Snapshot via Clone passed into Modal (avoids Live-Signal-Drilling)"
    - "Zentrale today-Variable am Component-Top als Single-Source-of-Truth fuer Modal-Bounds; existierende Duplikate in :82-90/:138-148 bewusst nicht angefasst (L-7 Cross-File-Refactor-Risiko)"
    - "ToastContainer + SuccessToastContainer mit separaten Signal-Buckets aber geteiltem toast_counter (Plan-02-Pattern)"
    - "Modal-on_success refresht Member-Detail lokal (get_member + get_member_actions) und global (refresh_members)"
key_files:
  created:
    - .planning/phases/18-frontend-component-first/18-MANUAL-UAT.md
  modified:
    - genossi-frontend/src/page/member_details.rs
decisions:
  - "Button erscheint nur bei existierenden Mitgliedern (`if !is_new`), weil neue Members noch keine ID haben und das Modal eine konkrete Member-Identitaet braucht."
  - "Button wird im Header-Bereich neben dem Mail-Senden-Button gerendert, bewusst NICHT bei den unteren Action-Buttons (Speichern/Loeschen). Begruendung: Mitgliedschaft-anpassen ist eine Top-Level-Operation auf das Mitglied, keine Formular-Speicheraktion."
  - "Phase-18-Cleanup auf den bestehenden `Edit/Back`-Header-Button fuer reload-bug-Pattern wurde EXPLIZIT NICHT vorgenommen — der Plan begrenzt Scope auf den neuen Phase-18-Button. Folge-Cleanups sind separate Tickets."
  - "Existierende `error: Signal<Option<AppError>>` und `ErrorAlert`-Mount auf der Page bleiben fuer alle Nicht-Adjust-Operationen. Modal hat eigenen ErrorAlert intern (Plan 06 Pattern D-18-08). Damit ist `show_toast`-Helper auf dieser Page nicht noetig — wir importieren ihn nicht."
  - "ToastContainer + SuccessToastContainer werden als Plain-Signal an die Container-Props uebergeben (analog repayment_phases.rs:125), NICHT mit `.into()`. SuperInto-Trait kann das Signal nicht ohne Type-Annotation aufloesen (E0283); direkter Signal-Pass funktioniert."
metrics:
  duration_minutes: 5
  completed_date: 2026-06-07
  tasks_completed: 2
  files_created: 1
  files_modified: 1
  tests_added: 0
  tests_run: 13
requirements_addressed:
  - UI-01
  - UI-02
  - UI-03
  - UI-04
  - CANC-06
---

# Phase 18 Plan 07: Member-Detail-Page Integration + Manual-UAT — Summary

**Final Phase-18-Integration: Admin-only „Mitgliedschaft anpassen"-Button auf der Member-Detail-Page, gemeinsamer Modal-Mount mit zentraler today-Variable und Success-Toast-Container, plus persistente Manual-UAT-Anleitung mit 6 Browser-Test-Szenarien. Schliesst Phase 18 (ROADMAP SC-1..4) bis auf den manuellen UAT-Walk-Through, der als persistentes Artefakt fuer den Vorstand vorbereitet ist.**

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Button + Modal-Mount + ToastContainer + zentrale today-Variable | `dade39d8` | genossi-frontend/src/page/member_details.rs (+92 LOC) |
| 2 | Manual-UAT-Anleitung (6 Szenarien + Sign-Off) | `a817ab15` | .planning/phases/18-frontend-component-first/18-MANUAL-UAT.md (NEW, ~90 LOC) |

**Task 3 (`checkpoint:human-verify`)**: Auto-approved im Auto-Mode. Die UAT-Datei ist persistiert und steht fuer den Vorstand-Browser-Walk-Through bereit. Tatsaechlicher Sign-Off wird offline durch den Vorstand eingetragen (`☐ PASS` → `☑ PASS`).

## Datei-Aenderungen in member_details.rs

1. **Imports erweitert**:
   - `use crate::auth::RequirePrivilege;`
   - `use crate::component::{show_success_toast, MembershipAdjustModal, SuccessToastContainer, ToastContainer, ...}`
   - `use crate::service::member::{refresh_members, SELECTED_MEMBER_IDS};`

2. **Neue Signals** (im `MemberDetails`-Component-Body):
   - `show_adjust_modal: Signal<bool>`
   - `toast_messages: Signal<Vec<(u64, String)>>` (Error-Bucket, L-6 Mitigation)
   - `success_toast_messages: Signal<Vec<(u64, String)>>` (Success-Bucket)
   - `toast_counter: Signal<u64>` (geteilt zwischen beiden Buckets)

3. **Zentrale `today: time::Date` Variable** am Top-Scope:
   - Single-Source-of-Truth fuer Phase-18-Modal-Bounds
   - Berechnet via `js_sys::Date::new_0()` mit Fallback auf `2025-01-01`
   - Bestehende today-Duplikate in `:82-90` (join_date use_signal) und `:138-148` (action_date use_signal) bleiben unveraendert — L-7 Cross-File-Refactor-Risiko

4. **Admin-only Button** im Header (zwischen Mail-Button und Back-Button):
   ```rust
   if !is_new {
       RequirePrivilege {
           privilege: "admin",
           button {
               r#type: "button",  // Dioxus reload-bug mitigation
               class: "px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded text-sm font-medium min-h-[44px]",
               onclick: move |_| show_adjust_modal.set(true),
               "{i18n.t(Key::MembershipAdjustButtonLabel)}"
           }
       }
   }
   ```

5. **Conditional Modal-Mount** + Toast-Container am Ende des rsx-Baums:
   - Member-Snapshot via `member.read().clone()` (kein Live-Signal-Drilling)
   - `on_success` ruft `refresh_members()` UND lokal `api::get_member` + `api::get_member_actions`
   - `show_success_toast` mit Generic Success-Message `Key::MembershipAdjustSuccess`
   - Beide Toast-Container am Page-Root gemountet (`ToastContainer` + `SuccessToastContainer`)

## Acceptance Criteria Verification

| Criterion | Required | Found | Status |
|-----------|----------|-------|--------|
| `MembershipAdjustModal` mentions in page | >= 2 | 3 | PASS |
| `show_adjust_modal` mentions | >= 3 | 5 | PASS |
| `RequirePrivilege` mentions | >= 1 | 2 | PASS |
| `privilege: "admin"` | >= 1 | 1 | PASS |
| `show_success_toast` mentions | >= 1 | 2 | PASS |
| `SuccessToastContainer` mentions | >= 1 | 2 | PASS |
| `ToastContainer` mentions | >= 1 | 4 | PASS |
| `refresh_members` mentions | >= 1 | 2 | PASS |
| `MembershipAdjustButtonLabel` mentions | >= 1 | 1 | PASS |
| Zentrale `let today: time::Date` (NEU eingefuehrt) | == 1 | 1 | PASS |
| Modal-Mount nutzt `today: today` | == 1 | 1 | PASS |
| Dioxus reload-bug pattern (neuer Button) `r#type: "button"` | >= 1 | 1 | PASS |
| `cargo check --bin genossi-frontend` exit 0 | required | 0 | PASS |
| `cargo test --bin genossi-frontend page::member_details` (13 pre-existing tests) | green | 13/0/0 | PASS |
| `.planning/phases/18-frontend-component-first/18-MANUAL-UAT.md` exists | required | yes | PASS |
| `## UAT-Szenario` headers | >= 6 | 6 | PASS |
| Operation-Vokabular (Kuendigung/Teil-Rueckgabe/Uebertrag/Aufstockung) | >= 4 | 21 occurrences | PASS |
| Voll-Uebertrag-Anker | >= 1 | 2 | PASS |
| Sign-Off-Section | >= 1 | 1 | PASS |
| „Mitgliedschaft anpassen" Button-Label | >= 1 | 1 | PASS |

## Build & Test Gates

- `cargo check --bin genossi-frontend` — exit 0 (32 vorhandene unused-Key-Warnings, keine neuen Errors)
- `cargo test --bin genossi-frontend page::member_details` — **13 passed; 0 failed**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Initial ToastContainer-Mount via `.into()` triggerte E0283**

- **Found during:** Task 1, erster `cargo check`-Run
- **Issue:** Plan-Snippet hatte `ToastContainer { messages: toast_messages.into() }`, aber Dioxus' `SuperInto`-Trait kann ohne Type-Annotation nicht aufloesen — E0283 (`type annotations needed`).
- **Fix:** Direktes Signal-Pass `ToastContainer { messages: toast_messages }` (analog `repayment_phases.rs:125` und `assemblies.rs:91`).
- **Files modified:** `genossi-frontend/src/page/member_details.rs`
- **Commit:** `dade39d8`

**2. [Rule 3 - Blocking] `error_label`-Stub im Modal-on_success-Block hatte keinen Konsumenten**

- **Found during:** Task 1, Refactor
- **Issue:** Plan-Snippet enthielt einen `error_label`-String-Hilfsvariablenstub, der nicht verwendet wurde. Wuerde `unused_variable`-Warning ohne Effekt produzieren.
- **Fix:** Variable entfernt; Error-Pfad wird stattdessen vom Modal intern via ErrorAlert behandelt (Plan-06-D-18-08-Pattern). Damit erfuellt diese Page das D-18-08-Contract: Error bleibt im Modal, Success-Toast geht auf die Page.
- **Files modified:** `genossi-frontend/src/page/member_details.rs`
- **Commit:** `dade39d8`

**3. [Rule 3 - Blocking] `show_toast`-Helper im Import war unbenoetigt**

- **Found during:** Task 1, Cleanup
- **Issue:** Der Plan listete `show_toast` im Import-Block, aber die Page ruft ihn nicht auf (Error-Pfad ist Modal-intern). Import-Listing waere unused.
- **Fix:** `show_toast` aus dem `use crate::component::{...}`-Block entfernt. `ToastContainer` selbst bleibt gemountet (L-6 Mitigation fuer kuenftige Error-Toast-Erweiterungen).
- **Files modified:** `genossi-frontend/src/page/member_details.rs`
- **Commit:** `dade39d8`

## UAT-Sign-Off-Resultat

**Status:** `pending-signoff`. Datei `.planning/phases/18-frontend-component-first/18-MANUAL-UAT.md` enthaelt 6 Szenarien + Sign-Off-Checkliste. Tatsaechliche Browser-Walk-Through-Durchfuehrung erfolgt manuell durch den Vorstand und wird offline in der Datei bestaetigt.

## Phase 18 — Plan-Liste (komplett)

```
.planning/phases/18-frontend-component-first/
├── 18-01-PLAN.md → SUMMARY (Wave 1 — Frontend rest-types DTOs)
├── 18-02-PLAN.md → SUMMARY (Wave 1 — Toast variant + show_success_toast)
├── 18-03-PLAN.md → SUMMARY (Wave 1 — MemberSearch members_override prop)
├── 18-04-PLAN.md → SUMMARY (Wave 1 — FiscalYearDateInput + 46 i18n keys)
├── 18-05-PLAN.md → SUMMARY (Wave 1 — 5 API client functions)
├── 18-06-PLAN.md → SUMMARY (Wave 2 — MembershipAdjustModal component)
└── 18-07-PLAN.md → THIS SUMMARY (Wave 3 — Page integration + Manual UAT)
```

## Roadmap-Update-Vorschlag

ROADMAP.md Phase 18 Plans: alle 7 Plans markieren als `[x]`. Status der Phase 18 wechselt von `In Progress` zu `Complete (UAT pending)` bis Vorstand-Sign-Off in `18-MANUAL-UAT.md`. Sobald Vorstand `**Ergebnis:** ☑ PASS` einträgt, ist die Phase fuer `/gsd-verify-phase 18` bereit.

## Requirements adressiert

Alle 5 Phase-18-Requirements voll abgedeckt:

- **UI-01** (Single-Button auf Member-Detail) — Admin-only Button hier integriert
- **UI-02** (Modal als shared Component) — Mount + Re-Use aus Plan 06
- **UI-03** (4 flat Sub-Choice) — In Modal von Plan 06 enthalten, hier konsumiert
- **UI-04** (Vorschau-Section pro Sub-View) — In Modal von Plan 06 enthalten, hier konsumiert
- **CANC-06** (Live-Preview als Confirm fuer Kuendigung) — Im Modal als Pattern etabliert; auf der Page wird Modal mit echtem Member-Snapshot mountet

## Known Stubs

Keine. Alle Pfade sind end-to-end gewired:
- Button-Click → Signal-Toggle → Modal-Mount
- Modal-Internals (Sub-Choice + 4 Sub-Views + API-Calls) liefern realen Result
- on_success → refresh_members + lokaler Refresh + Success-Toast

## Threat Flags

Keine neuen Threat-Flaechen ueber das in Plan-PLAN-frontmatter dokumentierte STRIDE-Register hinaus. Der RequirePrivilege-Gate auf den Button + Backend-ADMIN_PRIVILEGE-Funnel decken Spoofing/Tampering ab; Member-Snapshot in-memory kapselt PII; on_success-Flow ruft nur autorisierte Endpoints.

## Self-Check: PASSED

Files created:
- FOUND: `/home/neosam/programming/rust/projects/genossi3/.planning/phases/18-frontend-component-first/18-MANUAL-UAT.md`

Files modified:
- FOUND: `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/member_details.rs`

Commits in jj log:
- FOUND: `dade39d8` (Task 1 — page integration)
- FOUND: `a817ab15` (Task 2 — Manual UAT artifact)

Verifications:
- `cargo check --bin genossi-frontend` — exit 0
- `cargo test --bin genossi-frontend page::member_details` — 13 passed; 0 failed
- All grep-based acceptance criteria PASS (s. Table above)

---
*Phase: 18-frontend-component-first*
*Plan: 07*
*Completed: 2026-06-07*
