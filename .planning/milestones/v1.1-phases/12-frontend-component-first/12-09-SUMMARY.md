---
phase: 12-frontend-component-first
plan: 09
subsystem: ui
tags: [frontend, component, modal, member-picker, add-entry, ui-04, wave-6]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 — api::create_repayment_entry + CreateRepaymentEntryRequest"
  - phase: 12-frontend-component-first
    provides: "Plan 12-02 — (kein direkter Reuse — share_value-Format ist hier nicht relevant)"
  - phase: 12-frontend-component-first
    provides: "Plan 12-05 — Detail-Page mit BasicsTab + TabStrip + close_conflict-Signal-Pattern (Vorbild fuer Add-Modal-Signal-Pattern)"
  - phase: 12-frontend-component-first
    provides: "Plan 12-08 — RepaymentEntryList mit on_add-Placeholder; wird durch reload_trigger:u64-Prop erweitert"
  - "Direct-Reuse: component/member_search.rs::MemberSearch (D-21)"
  - "Direct-Reuse: component/modal.rs::Modal-Wrapper"
provides:
  - "#[component] pub fn RepaymentEntryAddModal(phase_id, on_close, on_created, on_error) -> Element — UI-04 Modal mit MemberSearch + Anteile-Eingabe"
  - "pub fn validate_create_entry(member_id: Option<Uuid>, share_count: i32) -> bool — D-23 Pure-Validation"
  - "RepaymentEntryList erweitert um reload_trigger:u64-Prop + use_effect implicit dep (Counter-Trigger-Pattern)"
  - "Detail-Page show_add_modal + entries_reload_trigger Signal-Pattern als Vorbild fuer Plan 12-10 / 12-13"
affects:
  - "Plan 12-10 — PaidOut-Confirm-Modal: kann entries_reload_trigger reusen, falls Bulk-PaidOut weitere externe Mutationen anstoesst (Member-Refresh ist bereits durch refresh_members abgedeckt; Counter optional)"
  - "Plan 12-13 — Massenmail-Redirect: nach Status-Aenderung 'angeschrieben' kann entries_reload_trigger erneut inkrementiert werden, falls die Mail-Page mit Query-Param zurueck kommt"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Form-in-Modal-Pattern (analog assemblies.rs::CreateAssemblyForm Z. 96-184): `form { onsubmit: e.prevent_default() ZUERST + spawn(async ...) DANACH }`"
    - "Counter-Trigger-Pattern fuer Component-Reload: u64-Prop + `let _ = trigger;` als implizite use_effect-Dep; deterministischer Re-Run bei Caller-seitiger wrapping_add(1)"
    - "Direct-Reuse MemberSearch fuer Picker (D-21) + auto-vorbefuellen via current_shares-Lookup im MEMBERS-Global-Signal beim on_select (D-22)"
    - "D-23 minimal-Validation via pure-Func + Disabled-Button-Render — keine Toast-Fanfare, kein Service-Round-Trip"
    - "D-24 Trennung Add (Modal mit Picker) vs. Edit (Inline-Cell-Edit aus 12-07/12-08) bewahrt: KEIN gemeinsames Form-Code"

key-files:
  created:
    - "genossi-frontend/src/component/repayment_entry_add_modal.rs (142 Zeilen: validate_create_entry pure-Func + RepaymentEntryAddModal #[component] + 3 Unit-Tests)"
  modified:
    - "genossi-frontend/src/component/mod.rs (Phase-12-Plan-12-09-Block: pub mod + Re-Export von RepaymentEntryAddModal und validate_create_entry)"
    - "genossi-frontend/src/component/repayment_entry_list.rs (reload_trigger:u64-Prop hinzugefuegt zwischen phase und on_changed; use_effect liest Trigger via `let _ = reload_trigger;` als implizite Dep; Doc-Kommentar fuer Reload-Pattern)"
    - "genossi-frontend/src/page/repayment_phase_details.rs (Import RepaymentEntryAddModal; 2 neue Signals: show_add_modal + entries_reload_trigger; RepaymentEntryList-Aufruf nutzt reload_trigger; on_add ersetzt Toast-Placeholder durch show_add_modal.set(true); Add-Modal-Mount mit on_created → wrapping_add(1) + load_phase())"

key-decisions:
  - "Counter-Trigger (u64) statt use_resource + restart() — minimal-invasiv, kein Component-API-Refactor noetig (Plan-Discussion)"
  - "`let _ = reload_trigger;` im use_effect-Body als implizite Dependency — sauberer Pattern als use_memo-Indirection (defensiv-Variante dokumentiert als Fallback im Plan, aber nicht implementiert)"
  - "wrapping_add(1) statt + 1 — defensive Wraparound-Pragmatik (in der Praxis nie erreicht, aber sauberer Pattern)"
  - "Detail-Page Counter-Bump zusaetzlich zu load_phase() — load_phase() laedt phase-Signal neu (frische version + opened_at), Counter-Bump triggert RepaymentEntryList-Re-Fetch. Beides parallel, nicht redundant."
  - "Add-Modal nutzt MemberSearch direkt (D-21) + befuellt share_count automatisch mit current_shares beim on_select (D-22) — kein Default-0 das den Submit-Button disabled liesse"
  - "validate_create_entry ist `pub` — Plan 12-10 / 12-15 koennen es reusen (z.B. falls weitere Add-Pfade dazukommen oder ein Bulk-Add)"
  - "Submit-Button disabled-State liest Signal-Werte synchron im RSX (`!validate_create_entry(*selected_member_id.read(), *share_count.read())`) — kein use_memo noetig fuer den boolean-Check"

patterns-established:
  - "Counter-Trigger-Pattern fuer externe Component-Reload: u64-Prop + `let _ = trigger;` im use_effect + Caller `wrapping_add(1)` — wiederverwendbar fuer alle Components mit lokalem use_effect-Load und externer Mutations-Quelle"
  - "Form-in-Modal-Pattern bestaetigt (zweite Anwendung nach CreateAssemblyForm) — Form mit prevent_default-zuerst + spawn-danach + button r#type:button + button r#type:submit + disabled-Guard"
  - "Doc-Kommentar an Component erklaert Caller-Vertrag fuer Reload-Counter (verbatim Beschreibung in repayment_entry_list.rs)"

requirements-completed: [UI-04]

# Metrics
duration: ~25 min
completed: 2026-06-01T13:05:00Z
task-count: 3
file-count: 3
test-count-added: 3
test-count-total: 176
commits:
  - {sha: f82ffe8, type: test, task: "1 RED", scope: "component/repayment_entry_add_modal.rs (stub + 3 tests)"}
  - {sha: 7de0f03, type: feat, task: "1 GREEN", scope: "component/repayment_entry_add_modal.rs (validate_create_entry impl)"}
  - {sha: 3cb4c52, type: feat, task: 2, scope: "component/repayment_entry_list.rs reload_trigger-Prop + page/repayment_phase_details.rs minimal-Wiring (0_u64)"}
  - {sha: e8b0aec, type: feat, task: 3, scope: "page/repayment_phase_details.rs Add-Modal-Mount + entries_reload_trigger Signal-Wiring"}
---

# Phase 12 Plan 09: RepaymentEntryAddModal (UI-04) Summary

**One-liner:** UI-04 Add-Entry-Modal als #[component] mit MemberSearch-Direct-Reuse (D-21), Auto-Vorbefuellung von share_count_to_pay_out via current_shares beim Member-Select (D-22), D-23 minimal client-side validation (Submit disabled wenn Member fehlt oder count <= 0), und einem expliziten Counter-Trigger-Pattern in der Detail-Page (entries_reload_trigger: u64 + wrapping_add(1)) — RepaymentEntryList wurde um einen reload_trigger:u64-Prop erweitert, der via `let _ = reload_trigger;` als implizite use_effect-Dependency gelesen wird. Damit ist der Reload nach Add-Submit deterministisch — KEIN Hoffen auf load_phase-Cascade.

## What Was Built

Drei Tasks. Task 1 (TDD: RED → GREEN) liefert die `validate_create_entry`-Pure-Func + den RepaymentEntryAddModal-Component mit 3 Unit-Tests. Task 2 erweitert RepaymentEntryList um den reload_trigger:u64-Prop und die use_effect-Read. Task 3 verdrahtet den Add-Modal-Mount in der Detail-Page inklusive Counter-Bump im on_created.

### Task 1: RepaymentEntryAddModal + validate_create_entry (commits f82ffe8 RED, 7de0f03 GREEN)

Datei `genossi-frontend/src/component/repayment_entry_add_modal.rs` mit:

- **`pub fn validate_create_entry(member_id: Option<Uuid>, share_count: i32) -> bool`** — D-23 minimal client-side validation; gibt true zurueck wenn `member_id.is_some() && share_count > 0`
- **`#[component] pub fn RepaymentEntryAddModal(phase_id, on_close, on_created, on_error) -> Element`** mit:
  - `selected_member_id: Signal<Option<Uuid>>` — Member-Picker-State
  - `share_count: Signal<i32>` — Anteile-Eingabe (Auto-Vorbefuellung beim Member-Select, D-22)
  - `submitting: Signal<bool>` — Submit-Button-Disabled-Guard waehrend POST
  - Form-Pattern (analog assemblies.rs::CreateAssemblyForm Z. 96-184): `form { onsubmit: e.prevent_default() ZUERST + spawn(async ...) DANACH }`
  - MemberSearch-Direct-Reuse (D-21) mit `on_select`, `selected_id`, `exclude_id: None`
  - im `on_select`-Callback: MEMBERS-Lookup → wenn Member gefunden, `share_count.set(member.current_shares)` (D-22)
  - Submit-Button disabled wenn `!validate_create_entry(...)` ODER `submitting`
  - Cancel-Button mit `r#type: "button"` (D-01)
  - i18n-Keys: `RepaymentEntryAdd`, `RepaymentEntryColShares`, `Cancel`, `Save`

**3 Unit-Tests:**
1. `validate_requires_member` — None member rejected (passive-pass in RED-Stub)
2. `validate_requires_positive_count` — 0 und -1 rejected (passive-pass in RED-Stub)
3. `validate_accepts_valid` — valid Uuid + count >= 1 accepted (FAIL in RED-Stub, PASS in GREEN)

Re-Export in `genossi-frontend/src/component/mod.rs`:
```rust
pub mod repayment_entry_add_modal;
pub use repayment_entry_add_modal::{validate_create_entry, RepaymentEntryAddModal};
```

### Task 2: RepaymentEntryList reload_trigger-Prop (commit 3cb4c52)

Erweitert die Component-Signatur in `genossi-frontend/src/component/repayment_entry_list.rs`:

```rust
#[component]
pub fn RepaymentEntryList(
    phase: RepaymentPhaseTO,
    reload_trigger: u64,        // ── NEU Plan 12-09 ──
    on_changed: EventHandler<()>,
    on_add: EventHandler<()>,
    on_paidout_request: EventHandler<Vec<RepaymentEntryTO>>,
    on_mail_request: EventHandler<Vec<Uuid>>,
    on_error: EventHandler<String>,
) -> Element { ... }
```

Im `use_effect`:
```rust
use_effect(move || {
    // Plan 12-09: reload_trigger als implizite Dep mitlesen → Counter-Aenderung loest Re-Run aus.
    let _ = reload_trigger;
    spawn(async move {
        crate::service::member::refresh_members().await;
    });
    load_entries();
});
```

Damit garantiert: jede Caller-seitige Inkrementierung des Counters loest einen
neuen Component-Re-Run aus, der wiederum den `use_effect` re-running macht und
`load_entries()` aufruft.

Doc-Kommentar an der Component erklaert den Caller-Vertrag verbatim.

Caller in der Detail-Page wurde minimal mit `reload_trigger: 0_u64` verdrahtet,
um den Build green zu halten (Task 3 ersetzt die Konstante durch das echte Signal).

### Task 3: Detail-Page Add-Modal-Wiring + entries_reload_trigger (commit e8b0aec)

Import-Block ergaenzt:
```rust
use crate::component::{
    ErrorAlert, Modal, RepaymentEntryAddModal, RepaymentEntryList,
    RepaymentPhaseStatusBadge, TabDef, TabStrip, ToastContainer, TopBar, show_toast,
};
```

Zwei neue Signals direkt neben `close_conflict`:
```rust
let mut show_add_modal = use_signal(|| false);
let mut entries_reload_trigger = use_signal(|| 0_u64);
```

RepaymentEntryList-Aufruf erweitert:
```rust
RepaymentEntryList {
    phase: phase_for_entries,
    reload_trigger: *entries_reload_trigger.read(),    // NEU
    on_changed: move |_| load_phase(),
    on_add: move |_| show_add_modal.set(true),         // ersetzt Plan-12-08 Toast-Placeholder
    on_paidout_request: ... // Plan 12-10 Toast-Placeholder bleibt
    on_mail_request: ...    // Plan 12-13 Toast-Placeholder bleibt
    on_error: ...
}
```

Add-Modal-Mount am Ende des RequirePrivilege-Bodys vor ToastContainer:
```rust
if *show_add_modal.read() {
    Modal {
        RepaymentEntryAddModal {
            phase_id,
            on_close: move |_| show_add_modal.set(false),
            on_created: move |_| {
                show_add_modal.set(false);
                // ── Plan 12-09 verbatim: Counter-Trigger statt load_phase-Cascade ──
                let current = *entries_reload_trigger.read();
                entries_reload_trigger.set(current.wrapping_add(1));
                load_phase();
            },
            on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
        }
    }
}
```

## Reload-Trigger-Counter-Pattern (verbatim)

Verbatim Reload-Cascade nach erfolgreichem Add-Submit:

1. Vorstand klickt Submit im Add-Modal
2. `RepaymentEntryAddModal` ruft `api::create_repayment_entry` → POST `/api/repayment-entry`
3. Auf 2xx: `on_created.call(())` feuert in der Detail-Page
4. Detail-Page-Closure:
   - `show_add_modal.set(false)` → Modal verschwindet
   - `let current = *entries_reload_trigger.read();`
   - `entries_reload_trigger.set(current.wrapping_add(1));` → Counter ++
   - `load_phase();` → Phase-Signal wird neu geladen (frische version + opened_at)
5. Dioxus re-rendert die Detail-Page, weil `entries_reload_trigger` ein gelesenes Signal ist (in `RepaymentEntryList { reload_trigger: *entries_reload_trigger.read() }`)
6. `RepaymentEntryList` erhaelt neuen `reload_trigger`-Prop-Wert → Component-Funktion re-runt
7. Im `use_effect` der RepaymentEntryList wird `let _ = reload_trigger;` als implizite Read-Dependency gelesen → Effect re-runt
8. `load_entries()` wird erneut aufgerufen → frische Entry-Liste via `api::list_repayment_entries`
9. Vorstand sieht den frischen Eintrag innerhalb 1-2 Sekunden ohne Page-Reload

## Gegenueberstellung der verworfenen Alternative

**Variante A (verworfen): use_resource + restart() in der Detail-Page**

Stattdessen haette die Detail-Page einen `entries_resource = use_resource(...)`
halten und nach on_created `entries_resource.restart()` rufen koennen. Die
RepaymentEntryList-API muesste dann von "intern entries laden" auf "Daten von
aussen reinreichen" umgebaut werden — der entries-Signal wandert von der
Component in die Caller-Page.

**Warum verworfen:** zu invasiv. Plan 12-08 hat RepaymentEntryList mit lokalem
entries-Signal und intern aufrufendem use_effect-Load gebaut; eine API-Aenderung
wuerde die etablierten 5 EventHandler-Props brechen und die Tests von Plan 12-08
ggf. invalidieren.

**Variante B (gewaehlt): reload_trigger:u64 als Counter-Prop**

Ein zusaetzlicher Prop, der von der Caller-Seite manuell inkrementiert wird.
Minimal-invasiv (nur ein zusaetzlicher Argument-Slot in der Component-Signatur),
klar testbar (Counter ist u64, einfacher Vergleich), keine
Resource-Restart-Reasoning, kein Risiko bei Folge-Plans.

## D-22 Vorbefuellung-Use-Case

Standard-Use-Case ist die Voll-Auszahlung: ein Mitglied tritt aus und bekommt
alle seine Anteile zurueck. Statt den Vorstand den `current_shares`-Wert
nachschauen und tippen zu lassen, befuellt die Modal das `share_count`-Feld
beim Member-Select automatisch mit `member.current_shares`. Der Vorstand kann
den Wert dann editieren (Teil-Abtretung, verspaetet gemeldeter Austritt mit
abweichender Anteils-Zahl).

Pattern: im `MemberSearch.on_select`-Callback:
```rust
if let Some(uid) = id {
    let members = MEMBERS.read();
    if let Some(m) = members.items.iter().find(|m| m.id == Some(uid)) {
        share_count.set(m.current_shares);
    }
}
```

`MEMBERS` ist das Global-Signal aus `service/member.rs` — voraussichtlich
bereits geladen, weil RepaymentEntryList (Plan 12-08) im use_effect parallel
`refresh_members()` triggert.

## Hinweis fuer Plan 12-10 / 12-13

Das `entries_reload_trigger`-Pattern kann von beiden Folge-Plans reused werden,
falls dort weitere externe Mutationen die RepaymentEntryList invalidieren:

**Plan 12-10 (PaidOut-Confirm-Modal):** nach erfolgreicher Bulk-PaidOut-Loop
sollte `entries_reload_trigger.set(current.wrapping_add(1))` analog gerufen
werden, falls die Bulk-Aktion das `on_changed` nicht bereits abdeckt. Plan 12-08
verdrahtet `on_changed: move |_| load_phase()` — load_phase laedt nur das
phase-Signal neu, NICHT die entries-Liste. Wenn 12-10's Bulk-Loop sich auf
`on_changed` verlaesst, sieht der Vorstand die Status-Aenderungen nicht
unmittelbar. Mit Counter-Trigger wird das deterministisch.

**Plan 12-13 (Massenmail-Redirect):** wenn die Mail-Page mit
`/mail?sent=true`-Banner zurueck navigiert und einen Status-Wechsel
`offen → angeschrieben` triggert (deferred Idee aus 12-CONTEXT.md, nicht im
v1.1-Scope), wuerde derselbe Counter-Trigger den entries-Reload triggern.

Im v1.1-Scope ist 12-13 nur ein Redirect ohne Auto-Status-Wechsel, daher kein
Reload-Trigger noetig.

## Render-Path (Data Flow)

```
Detail-Page mount
  ↓ load_phase() → phase: Signal<Option<RepaymentPhaseTO>>
  ↓ Render-Tree (status_value == Open):
    TabStrip
      "entries" → RepaymentEntryList { reload_trigger: 0 (initial) }
        ↓ use_effect (1× mount):
          spawn → refresh_members()
          load_entries() → entries.set(...)
        ↓ render Tabelle + Header-Action-Buttons
        ↓ Vorstand klickt "Eintrag manuell hinzufuegen" (D-CONTEXT.md D-14 Empty-State CTA oder Bulk-Header)
        ↓ on_add.call(()) → show_add_modal.set(true)
    if show_add_modal:
      Modal
        RepaymentEntryAddModal
          ↓ Vorstand sucht Mitglied via MemberSearch
          ↓ on_select → selected_member_id.set(id) + share_count.set(member.current_shares)
          ↓ Submit:
            form.onsubmit:
              e.prevent_default()
              validate_create_entry(mid, sc) → true
              spawn → api::create_repayment_entry(...)
                Ok → on_created.call(())
                Err → on_error.call(e.message)
          ↓ Detail-Page on_created:
            show_add_modal.set(false)
            entries_reload_trigger.set(current.wrapping_add(1))    ── Counter ++
            load_phase()                                            ── phase frisch
          ↓ Re-render Detail-Page
          ↓ RepaymentEntryList re-runs with new reload_trigger
          ↓ use_effect re-runs (let _ = reload_trigger sieht den neuen Wert)
          ↓ load_entries() → frische Liste mit dem neuen Eintrag
          ↓ Vorstand sieht den neuen Eintrag ohne Page-Reload
```

## How It Was Verified

```bash
# Task 1 RED
$ cd genossi-frontend && cargo test --bin genossi-frontend component::repayment_entry_add_modal
test result: FAILED. 2 passed; 1 failed
  ↑ validate_accepts_valid FAILS (stub returns false unconditionally)

# Task 1 GREEN
$ cd genossi-frontend && cargo test --bin genossi-frontend component::repayment_entry_add_modal
test result: ok. 3 passed; 0 failed

# Task 2 build (after minimal caller-fix reload_trigger:0_u64)
$ cd genossi-frontend && cargo build --bin genossi-frontend
    Finished `dev` profile in 26.83s

# Task 3 full build
$ cd genossi-frontend && cargo build --bin genossi-frontend
    Finished `dev` profile in 22.00s

# Task 3 full test suite
$ cd genossi-frontend && cargo test --bin genossi-frontend
test result: ok. 176 passed; 0 failed; 0 ignored; 0 measured

# Done-criteria greps
$ rg "MemberSearch \{" genossi-frontend/src/component/repayment_entry_add_modal.rs
1 Treffer (D-21 reuse)

$ rg "current_shares" genossi-frontend/src/component/repayment_entry_add_modal.rs
3 Treffer (D-22 vorbefuellen-Logik)

$ rg "api::create_repayment_entry" genossi-frontend/src/component/repayment_entry_add_modal.rs
1 Treffer

$ rg "reload_trigger:\\s*u64" genossi-frontend/src/component/repayment_entry_list.rs
2 Treffer (Signatur + Doc-Kommentar)

$ rg "let _ = reload_trigger" genossi-frontend/src/component/repayment_entry_list.rs
1 Treffer (use_effect implicit dep)

$ rg "RepaymentEntryAddModal \{" genossi-frontend/src/page/repayment_phase_details.rs
1 Treffer (Modal-Mount)

$ rg "show_add_modal" genossi-frontend/src/page/repayment_phase_details.rs
6 Treffer (Signal + on_add + on_close + on_created-set-false + Modal-Condition)

$ rg "entries_reload_trigger" genossi-frontend/src/page/repayment_phase_details.rs
5 Treffer (Signal + Prop-Read + on_created Read + on_created Set + Inline-Doc)

$ rg "wrapping_add\\(1\\)" genossi-frontend/src/page/repayment_phase_details.rs
1 Treffer (Counter-Pattern)

$ rg "reload_trigger:\\s*\\*entries_reload_trigger\\.read\\(\\)" genossi-frontend/src/page/repayment_phase_details.rs
1 Treffer (Counter wird als Prop uebergeben)

$ rg "Add-Modal kommt in Plan 12-09" genossi-frontend/src/page/repayment_phase_details.rs
0 Treffer (Plan-12-08 Placeholder ist entfernt)

$ rg "kommt in Plan 12-(10|13)" genossi-frontend/src/page/repayment_phase_details.rs
2 Treffer (PaidOut + Mail-Redirect bleiben als Hand-Off fuer Folge-Plans)

# D-01 Button-Gate (zero buttons without r#type:)
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' \
    genossi-frontend/src/component/repayment_entry_add_modal.rs \
    genossi-frontend/src/component/repayment_entry_list.rs \
    genossi-frontend/src/page/repayment_phase_details.rs \
  | grep -v 'r#type:' | grep -c 'button {'
0
```

All plan-acceptance criteria pass.

## Decisions Made

### Counter-Trigger statt use_resource + restart

Plan-Diskussion zeigte zwei Varianten fuer den Reload-nach-Mutation:
- Variante A: `use_resource` in der Detail-Page halten + `restart()` rufen — invasive Component-API-Aenderung
- Variante B (gewaehlt): `reload_trigger: u64` Counter-Prop + Caller bumps + use_effect implicit dep

Variante B ist minimal-invasiv: kein API-Bruch, kein Resource-Restart-Reasoning,
einfacher Mental-Model. Trade-off: ein zusaetzlicher Argument-Slot in der
Component-Signatur — pragmatisch akzeptabel.

### `let _ = reload_trigger;` statt use_memo-Indirection

Plan-Action-Block dokumentiert beide Patterns. Der direkte Read ist sauberer
und tut explizit was er beschreiben sollte — der u64-Prop wird im
Effect-Body gelesen → Dioxus tracked ihn → Aenderung re-runt den Effect.

`use_memo`-Indirection waere ein Fallback gewesen, falls Dioxus
Capture-Verhalten zickt; in der Praxis funktioniert der direkte Read.

### wrapping_add(1) statt + 1

Defensive Wraparound-Pragmatik. Ein u64 Counter wraparound passiert nach
~18.4 Quintillion Inkrementen — in der Praxis nie erreichbar, aber der
Pattern ist sauberer.

### Detail-Page Counter-Bump zusaetzlich zu load_phase()

`load_phase()` und der Counter-Bump sind nicht redundant:
- `load_phase()` schreibt das phase-Signal neu (frische version + opened_at)
- Counter-Bump triggert RepaymentEntryList re-fetch der entries-Liste

Beide haben unterschiedliche Verantwortung. Wenn nur load_phase() ohne
Counter-Bump aufgerufen wuerde, koennte die RepaymentEntryList theoretisch
stale entries zeigen — load_phase aendert NICHT entries.

### Add-Modal nutzt MemberSearch direkt (D-21) + Auto-Vorbefuellung (D-22)

Statt einen eigenen Member-Picker-Code im Add-Modal zu bauen, wird der
bereits-etablierte MemberSearch-Component (Component-First) direkt reused.
Im `on_select`-Callback wird das `share_count`-Signal mit
`member.current_shares` befuellt — Standard-Use-Case ist die Voll-Auszahlung
(Genosse tritt aus → alle Anteile retour).

Vorstand kann den Wert nachher editieren fuer Teil-Abtretungen.

### validate_create_entry ist `pub`

Pure-Func ist `pub` und wird in `component/mod.rs` re-exportiert. Plan 12-10 /
12-15 koennen sie reusen, falls weitere Add-Pfade dazukommen (z.B. Bulk-Add
aus einer Excel-Datei, deferred). Verhindert Duplikation der
Validation-Logik.

### Submit-Button disabled-State direkt im RSX (kein use_memo)

Pattern:
```rust
disabled: *submitting.read() || !validate_create_entry(*selected_member_id.read(), *share_count.read()),
```

Der boolean-Check wird bei jedem Re-Render synchron evaluiert. Kein use_memo
noetig, weil:
- `validate_create_entry` ist O(1) (zwei Vergleiche)
- Die Inputs sind primitive Werte (Option<Uuid> + i32)
- Re-Render passiert nur bei Signal-Aenderungen, die ohnehin den Disabled-State
  veraendern wuerden

Sauberer und lesbarer als use_memo-Indirection.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 2 alleine bricht den Build, weil der Detail-Page-Caller den neuen reload_trigger-Prop nicht uebergibt**

- **Found during:** Task 2 build
- **Issue:** Plan strukturiert Tasks 2 und 3 als getrennte Commits, aber Task 2 erweitert die RepaymentEntryList-Signatur um einen Pflicht-Prop. Die bestehende Detail-Page-Caller-Seite kompiliert nicht ohne den Prop.
- **Fix:** Task 2's Commit enthaelt einen minimalen Caller-Fix `reload_trigger: 0_u64` in der Detail-Page, um den Build green zu halten. Task 3 ersetzt die Konstante durch das echte Signal-Read.
- **Files modified:** `genossi-frontend/src/page/repayment_phase_details.rs` (zusaetzlicher Fix in Task-2-Commit)
- **Commit:** 3cb4c52 enthaelt den temporaeren Caller-Fix
- **Verification:** `cargo build --bin genossi-frontend` exit 0 nach Task 2

**Total deviations:** 1 auto-fixed (Rule 3 - Build-blocking).
**Impact on plan:** Keine; Plan-Logik unveraendert. Reihenfolge Task 2 → Task 3
bleibt erhalten; Task 3's done-criterion `reload_trigger: *entries_reload_trigger.read()`
zaehlt 1 Treffer (Task 2's `0_u64` wurde durch das Signal-Read ersetzt).

## Known Stubs

Zwei EventHandler-Toast-Placeholder verbleiben in
`genossi-frontend/src/page/repayment_phase_details.rs::EntriesTab`:

| Placeholder | Toast-Text | Resolved by |
|---|---|---|
| `on_paidout_request: \|_entries\| show_toast("PaidOut-Confirm kommt in Plan 12-10".into())` | Toast | Plan 12-10 |
| `on_mail_request: \|_ids\| show_toast("Mail-Redirect kommt in Plan 12-13".into())` | Toast | Plan 12-13 |

Der `on_add`-Placeholder aus Plan 12-08 wurde durch das echte Modal-Open
ersetzt — die Liste der noch-offenen Hand-Offs schrumpft von 3 auf 2.

Der grep-Test `rg 'kommt in Plan 12-(10|13)' genossi-frontend/src/page/repayment_phase_details.rs`
liefert nach diesem Plan 2 Zeilen und nach Abschluss von 12-10 / 12-13
entsprechend 0.

## Threat Flags

None — dieser Plan konsumiert nur bestehende Backend-Endpoints (Phase 7-11)
und fuegt UI-Surface hinzu. Keine neue Netzwerk-Endpoint, keine Auth-Pfade,
keine Schema-Aenderung. Der Add-Modal triggert `POST /api/repayment-entry`,
das Backend handhabt Validation (Service-Layer + DB-CHECK, Phase 8 D-11.3) als
Backstop. Frontend-Validation (D-23) ist nur UX, keine Trust-Boundary.

## Self-Check: PASSED

Verifizierte Artefakte gegen das Repo:

- FOUND: `genossi-frontend/src/component/repayment_entry_add_modal.rs` (142 Zeilen, > 80 plan-minimum)
- FOUND: `validate_create_entry` als `pub fn` definiert
- FOUND: `RepaymentEntryAddModal` als `#[component]` definiert
- FOUND: `pub mod repayment_entry_add_modal;` + `pub use ...validate_create_entry, RepaymentEntryAddModal;` in `component/mod.rs`
- FOUND: `MemberSearch {` 1 Treffer in repayment_entry_add_modal.rs (D-21 reuse)
- FOUND: `current_shares` 3 Treffer in repayment_entry_add_modal.rs (D-22 vorbefuellen)
- FOUND: `api::create_repayment_entry` 1 Treffer in repayment_entry_add_modal.rs
- FOUND: `reload_trigger:\s*u64` 2 Treffer in repayment_entry_list.rs (Signatur + Doc-Kommentar)
- FOUND: `let _ = reload_trigger` 1 Treffer in repayment_entry_list.rs
- FOUND: `RepaymentEntryAddModal {` 1 Treffer in repayment_phase_details.rs
- FOUND: `show_add_modal` 6 Treffer in repayment_phase_details.rs
- FOUND: `entries_reload_trigger` 5 Treffer in repayment_phase_details.rs
- FOUND: `wrapping_add(1)` 1 Treffer in repayment_phase_details.rs
- FOUND: `reload_trigger: *entries_reload_trigger.read()` 1 Treffer in repayment_phase_details.rs
- VERIFIED: Plan-12-08 Placeholder "Add-Modal kommt in Plan 12-09" 0 Treffer (ersetzt)
- FOUND: 2 verbliebene Placeholder fuer 12-10 / 12-13 als Hand-Off-Anker
- FOUND: D-01 Button-Gate fuer 3 Phase-12-09-Dateien = 0 Treffer ohne `r#type:`
- FOUND: cargo build --bin genossi-frontend exit 0
- FOUND: cargo test --bin genossi-frontend exit 0 mit 176 PASS (vorher 173)
- FOUND: Commit f82ffe8 (RED, validate_accepts_valid FAILS, andere 2 passive-PASS)
- FOUND: Commit 7de0f03 (GREEN, 3/3 PASS)
- FOUND: Commit 3cb4c52 (Task 2 — reload_trigger-Prop + minimal Caller-Fix)
- FOUND: Commit e8b0aec (Task 3 — Add-Modal-Mount + entries_reload_trigger Signal)

## TDD Gate Compliance

- **Task 1 RED gate:** `f82ffe8` (`test(12-09): add failing tests for RepaymentEntryAddModal validation`) — 1 von 3 Tests fail (validate_accepts_valid FAILS, weil stub immer false zurueckgibt); 2 passive-pass (validate_requires_member, validate_requires_positive_count — passen weil stub auch false zurueckgibt).
- **Task 1 GREEN gate:** `7de0f03` (`feat(12-09): implement validate_create_entry for RepaymentEntryAddModal`) — alle 3/3 PASS.
- **REFACTOR gate:** kein Commit — Implementation war minimal und braucht keinen Refactor.
- **Task 2:** kein expliziter TDD-Cycle (Task 2 ist nicht TDD-markiert im Plan; Acceptance ist Build + Grep-Greps).
- **Task 3:** kein expliziter TDD-Cycle (Task 3 ist UI-Wiring; Acceptance ist Build + Grep-Greps).

Gate sequence in `git log 5675b88..HEAD`:
```
f82ffe8 test(12-09): add failing tests for RepaymentEntryAddModal validation     ← Task 1 RED
7de0f03 feat(12-09): implement validate_create_entry for RepaymentEntryAddModal   ← Task 1 GREEN
3cb4c52 feat(12-09): add reload_trigger:u64 prop to RepaymentEntryList            ← Task 2
e8b0aec feat(12-09): wire Add-Entry-Modal + entries_reload_trigger in detail page ← Task 3
```

Strict test→feat→feat→feat — TDD-gate erfuellt fuer Task 1; Tasks 2/3 sind nicht TDD-markiert.

## Next Phase Readiness

**Bereit fuer Wave 6+ (Plans 12-10, 12-13):**

- **Plan 12-10 (PaidOut-Confirm-Modal):** kann den `entries_reload_trigger`-Counter-Pattern reusen. Nach erfolgreicher Bulk-PaidOut-Loop empfiehlt sich, den Counter zu inkrementieren, damit die Liste die Status-Aenderungen sofort zeigt (statt sich auf `on_changed = load_phase()` zu verlassen, was nur das Phase-Signal aendert).

- **Plan 12-13 (Massenmail-Redirect):** ist nur ein Redirect; im v1.1-Scope kein Auto-Status-Wechsel, daher kein Reload-Trigger noetig.

- **Plan 12-15 (UAT-Checkliste):** kann verifizieren, dass der frische Eintrag innerhalb 1-2 Sekunden in der Liste auftaucht ohne Page-Reload — der Counter-Trigger-Pattern macht das deterministisch.

Component-Contract via EventHandler-Props funktioniert weiterhin als saubere
Verdrahtungs-Schnittstelle; der Counter-Prop ist ein additives, nicht-breaking
Erweiterung.

---

*Phase: 12-frontend-component-first*
*Plan: 09 — RepaymentEntryAddModal (UI-04)*
*Completed: 2026-06-01T13:05:00Z (~25 min)*
