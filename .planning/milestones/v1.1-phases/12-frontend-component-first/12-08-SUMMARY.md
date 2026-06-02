---
phase: 12-frontend-component-first
plan: 08
subsystem: ui
tags: [frontend, component, table, multi-select, filter, inline-edit, soft-delete, wave-5]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 — api::list/update/delete_repayment_entries + batch_toggle_repayment_status + RepaymentEntryTO + RepaymentEntryStatusTO + RepaymentPhaseTO + UpdateRepaymentEntryRequest + BatchStatusRequest"
  - phase: 12-frontend-component-first
    provides: "Plan 12-02 — RepaymentEntryStatusBadge + format_payout_eur"
  - phase: 12-frontend-component-first
    provides: "Plan 12-05 — Detail-Page mit EntriesTab-Stub 'TODO Plan 12-08'"
  - phase: 12-frontend-component-first
    provides: "Plan 12-07 — EditableShareCountCell + is_share_count_valid"
provides:
  - "#[component] pub fn RepaymentEntryList(phase, on_changed, on_add, on_paidout_request, on_mail_request, on_error) -> Element — UI-03 Kern-Component"
  - "4 pure-Helper-Funktionen (filter_entries_by_status, entry_counts_by_status, sort_entries_default, member_for_entry) + StatusFilter/StatusCounts Types"
  - "StatusFilterTab inline-#[component] fuer Tab-Strip-im-Tab Pattern (D-12)"
  - "Detail-Page EntriesTab ersetzt TODO-Stub aus Plan 12-05 mit Component-Mount + 3 EventHandler-Placeholder fuer 12-09/12-10/12-13"
affects:
  - "Plan 12-09 — Add-Entry-Modal: ersetzt on_add Toast-Placeholder mit echtem Modal-Open in der Detail-Page"
  - "Plan 12-10 — PaidOut-Confirm: ersetzt on_paidout_request Toast-Placeholder mit Confirm-Modal-Open"
  - "Plan 12-13 — Massenmail-Redirect: ersetzt on_mail_request Toast-Placeholder mit /mail?from=repayment-Navigation"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Multi-Select-Signal-Pattern (use_signal(|| Vec::<Uuid>::new) + write().push/retain) aus mail_page.rs als handgemachtes Pattern"
    - "Status-Filter-Tab-Strip-im-Tab via inline StatusFilterTab #[component] (D-12)"
    - "Client-Side-Join Member ↔ Entry via MEMBERS-Global-Signal (D-10) mit defensive '—'-Fallback bei Member-Mismatch (Pitfall 8)"
    - "readonly_mode-Conditional-Render (D-08) blendet Bulk-Buttons, Checkbox-Spalte, Inline-Edit und Trash-Action aus"
    - "Component-Vertrag via EventHandler-Props als Verdrahtungspunkte fuer Folge-Plans"
    - "Option A Test-Helper: alle 26 MemberTO-Felder explizit (kein Default-Spread; bewusste Compile-Time-Pflicht-Sync mit Backend-Schema)"

key-files:
  created:
    - "genossi-frontend/src/component/repayment_entry_list.rs (492 Zeilen: 4 Pure-Helper + StatusFilter/StatusCounts + RepaymentEntryList #[component] + StatusFilterTab inline-#[component] + 8 Unit-Tests)"
  modified:
    - "genossi-frontend/src/component/mod.rs (Phase-12-Plan-12-08-Block: pub mod + Re-Export von RepaymentEntryList, StatusFilter, StatusCounts und 4 pure-Helpers)"
    - "genossi-frontend/src/page/repayment_phase_details.rs (Plan-12-05 EntriesTab-Stub ersetzt durch RepaymentEntryList-Mount; Imports erweitert um RepaymentEntryList + RepaymentEntryTO)"

key-decisions:
  - "Pure-Helpers sind pub (component/mod.rs re-exportiert sie) — Plans 12-09 und 12-10 koennen sie reusen falls die Add/PaidOut-Modal-Listen ebenfalls Filtern/Sortieren brauchen"
  - "StatusFilterTab als inline-#[component] in derselben Datei (kein eigenes File) — Pattern wird nur von RepaymentEntryList genutzt; Component-First nicht verletzt, weil weder dupliziert noch ausser-Page geteilt"
  - "Single-Click-Header-Checkbox setzt ALLE sichtbar-gefilterten Entry-IDs auf selected_ids (kein Subset-Toggle wenn manche schon checked sind) — Pragmatic: bei Filter-Wechsel wird die Selection NICHT zurueckgesetzt (D-11 sagt nichts dazu); Vorstand kann dann den Header-Checkbox erneut nutzen, um die neue Filter-Sicht voll-auszuwaehlen"
  - "Delete-Confirm-Modal speichert nur Entry-ID-Signal (delete_confirm_for: Option<Uuid>) statt voller Entry-Daten — Modal-UI ist eine generische 'Eintrag loeschen?'-Frage ohne Detailtext (D-14 spezifiziert nur 'Confirm-Modal'; Plan 12-10 hat das vollere Listentabelle-Pattern fuer Bulk-PaidOut)"
  - "MEMBERS-Refresh + load_entries laufen PARALLEL im use_effect (zwei separate spawn-Tasks) statt sequenziell await — schnellere Erst-Render-Zeit, akzeptabler Trade-off (Loading-State zeigt mindestens entries-Loading; Member-Mismatch faellt defensiv auf '—' zurueck bis MEMBERS da ist)"
  - "MemberStatusTO::Normal statt MemberStatusTO::Active im Test-Helper — verifiziert via rest-types/src/lib.rs:145-148 (Variants sind Normal + FehlerhaftErfasst, KEINE Active-Variante); Plan-Wording referenzierte 'Active' was inkorrekt war"
  - "D-08 readonly_mode-Check via matches!(phase_status, RepaymentPhaseStatusTO::Closed) — Open und Preparation sind beide editierbar (Preparation wird durch Detail-Page bereits abgefangen; in der Praxis trifft die Component nur Open + Closed)"

patterns-established:
  - "Inline-Component-in-derselben-Datei (StatusFilterTab): Page-/Component-local sub-#[component] wenn die UI-Wiederverwendung auf eine einzige Caller-Datei begrenzt ist — verhindert namespace-Bloat in component/mod.rs"
  - "EventHandler-Props als Verdrahtungs-Vertrag fuer Folge-Plans: Component-Layer ist eigenstaendig, Folge-Plans verdrahten nur die Detail-Page-Caller-Seite — keine Component-Aenderung noetig"
  - "Test-Helper Option A (alle Felder explizit, kein Default-Spread) als bewusste Compile-Time-Sync mit Backend-Schema: neues Pflicht-Feld in MemberTO bricht den Compiler hier sofort; KEIN silent-Initialisierung-Risiko"

requirements-completed: [UI-03]

# Metrics
duration: ~7 min
completed: 2026-06-01T12:36:32Z
task-count: 2
file-count: 3
test-count-added: 8
test-count-total: 173
commits:
  - {sha: 8e7ce28, type: test, task: "1 RED", scope: "component/repayment_entry_list.rs (pure-helper stubs + 8 tests)"}
  - {sha: 4f84e0f, type: feat, task: "1 GREEN", scope: "component/repayment_entry_list.rs (pure-helper impls)"}
  - {sha: 4e68d7b, type: feat, task: 2, scope: "component/repayment_entry_list.rs + mod.rs + page/repayment_phase_details.rs"}
---

# Phase 12 Plan 08: RepaymentEntryList (UI-03) Summary

**One-liner:** UI-03 Kern-Component RepaymentEntryList als 7-Spalten-Multi-Select-Tabelle mit Status-Filter-Tabs, Inline-Cell-Edit fuer Anteile (via EditableShareCountCell), Soft-Delete-Trash + Confirm-Modal und 4 Header-Action-Buttons (Add + Mail + Markieren-Angeschrieben + Markieren-Ausbezahlt) — verdrahtet in der Detail-Page-EntriesTab als Ersatz fuer den TODO-Stub aus Plan 12-05; drei der vier Bulk-Aktionen liefern aktuell Toast-Placeholder, die Plans 12-09/12-10/12-13 mit echtem Modal/Redirect ersetzen.

## What Was Built

Zwei Tasks. Task 1 (TDD) liefert die 4 Pure-Helper-Funktionen (filter, sort, count, join) mit 8 Unit-Tests; Task 2 baut den vollstaendigen Component mit allen 7 Spalten, Multi-Select-State-Pattern, Status-Filter-Tab-Strip-im-Tab, Inline-Cell-Edit-Pass-Through, Soft-Delete-Confirm-Modal, readonly_mode-Branches und der Detail-Page-Integration.

### Task 1: Pure-Helper-Funktionen + Unit-Tests (commits 8e7ce28 RED, 4f84e0f GREEN)

Datei `genossi-frontend/src/component/repayment_entry_list.rs` mit:

- **`enum StatusFilter`** — `All | Open | Contacted | PaidOut` (Copy + Eq)
- **`struct StatusCounts { all, open, contacted, paidout: usize }`** — Tuple-of-Counts fuer die Tab-Strip-Badges
- **`fn filter_entries_by_status(&[RepaymentEntryTO], StatusFilter) -> Vec<RepaymentEntryTO>`** — Client-Side-Filter (Backend liefert immer alle, Phase 8 D-10 + D-12)
- **`fn entry_counts_by_status(&[RepaymentEntryTO]) -> StatusCounts`** — Single-Pass-Count fuer die 4 Filter-Badges; leere Liste -> (0,0,0,0)
- **`fn member_for_entry(&RepaymentEntryTO, &[MemberTO]) -> Option<&MemberTO>`** — Client-Side-Join Member ↔ Entry via id-Match; None-Return bei Mismatch ist intentional fuer defensive UX (Pitfall 8)
- **`fn sort_entries_default(&[RepaymentEntryTO], &[MemberTO]) -> Vec<RepaymentEntryTO>`** — D-14 Mitgliedsnummer ASC + created ASC sekundaer; entries ohne Member-Match sortieren ans Ende (defensive)

**8 Unit-Tests:**
1. `filter_by_status_open` — Mix aus 3 Status, Filter Open liefert 2 zurueck
2. `filter_by_status_all_returns_all` — All-Filter laesst ALLE durch (auch PaidOut)
3. `counts_correct` — 4-Entries-Mix → (4, 2, 1, 1)
4. `counts_empty_returns_zeros` — leere Liste → (0, 0, 0, 0)
5. `member_for_entry_finds_match` — Some-Result bei valider member_id
6. `member_for_entry_returns_none_on_mismatch` — None bei unbekannter member_id
7. `sort_by_member_number_asc` — 3 Members mit Numbers 100/50/75 → sortierte Reihenfolge 50/75/100
8. `sort_entries_without_member_at_end` — Entry mit unknown_member_id sortiert hinter Entry mit known member_id

**Test-Helper-Pattern (Option A — Plan-Pflicht):** `make_member` listet ALLE 26 MemberTO-Felder explizit auf, kein `..Default::default()`-Spread. Backend hat KEIN `#[derive(Default)]` (verifiziert: 0 Treffer in `rest-types/src/lib.rs`). Bei Backend-Schema-Erweiterung bricht der Compiler hier sofort — bewusste Pflicht-Sync, keine silent-Initialisierung.

**Korrektur waehrend Implementation:** Plan-Wording referenzierte `MemberStatusTO::Active`. Diese Variante existiert NICHT in `rest-types/src/lib.rs:145-148`. Tatsaechliche Varianten sind `Normal` + `FehlerhaftErfasst`. Test-Helper nutzt `MemberStatusTO::Normal` (= harmlose, realistische Default-Annahme).

### Task 2: Full Component + Detail-Page-Wire-up (commit 4e68d7b)

**`#[component] pub fn RepaymentEntryList(phase, on_changed, on_add, on_paidout_request, on_mail_request, on_error) -> Element`** — UI-03 Kern.

**State:**
- `entries: Signal<Vec<RepaymentEntryTO>>` — geladen via `api::list_repayment_entries`
- `loading: Signal<bool>` — initial true, false nach Load
- `selected_ids: Signal<Vec<Uuid>>` — Multi-Select-State (write().push/retain pattern aus mail_page.rs)
- `status_filter: Signal<StatusFilter>` — Tab-Strip-im-Tab-State, default All
- `delete_confirm_for: Signal<Option<Uuid>>` — None = Modal hidden, Some(entry_id) = Modal sichtbar fuer dieses Entry

**Use-Effect:** Parallel `crate::service::member::refresh_members().await` UND `load_entries()` — beide in separaten spawn-Tasks (schnellere Erst-Render-Zeit; Member-Mismatch-Fallback auf '—' ist defensive UX bis MEMBERS da ist; Pitfall 8).

**Render-Struktur:**
1. **Status-Filter-Tab-Strip-im-Tab** (D-12) via inline `StatusFilterTab #[component]` — 4 Tabs (Alle/Offen/Angeschrieben/Ausbezahlt) mit Count-Badges
2. **Header-Action-Leiste** (D-11, nur wenn `!readonly_mode`):
   - Add-Button (immer aktiv) → `on_add.call(())`
   - Mail-Button mit Count-Badge → `on_mail_request.call(selected_ids)`; disabled bei 0 Selection
   - "Als angeschrieben markieren"-Button mit Count-Badge → POST `/api/repayment-entry/batch-status` mit `target_status=Contacted`; bei Success Selection leeren + on_changed
   - "Als ausbezahlt markieren"-Button (rot) mit Count-Badge → `on_paidout_request.call(selected_entries)` (Plan 12-10 oeffnet Confirm-Modal)
3. **Tabelle** (D-10, 7 Spalten):
   - Checkbox-Spalte (Header "Alle auswaehlen" + Per-Row, nur wenn `!readonly_mode`)
   - Mitgliedsnummer (aus MEMBERS-Join, "—" bei Mismatch)
   - Name (`{first_name} {last_name}`, "—" bei Mismatch)
   - Anteile via `EditableShareCountCell` (nur wenn `!readonly_mode && !is_paidout`; sonst plain `span`)
   - Betrag via `format_payout_eur(entry.share_count_to_pay_out, share_value)`
   - IBAN (`bank_account` aus Member, "—" bei None oder leer)
   - Status via `RepaymentEntryStatusBadge`
   - Actions: Trash-Icon (`!readonly_mode && !is_paidout`) → setzt `delete_confirm_for`
4. **Empty-State-Branches:**
   - Loading → "{i18n.t(Key::Loading)}"
   - Sorted leer + Filter==All + entries leer → `RepaymentEntryEmptyAutoFill`
   - Sorted leer + Filter != All ODER Entries hat Items → `RepaymentEntryEmptyFilter`
5. **Delete-Confirm-Modal** (D-14) — Cancel + roter Delete-Button; Delete-Button ruft `api::delete_repayment_entry` → on_changed.call(())

**Inline-Cell-Edit-Save-Path:**
```rust
EditableShareCountCell {
    value: entry_share_count,
    disabled: false,
    on_save: move |new_count: i32| {
        let Some(version) = entry_version else { on_error.call("..."); return; };
        let req = UpdateRepaymentEntryRequest {
            share_count_to_pay_out: Some(new_count),
            status: None,
            version,
        };
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::update_repayment_entry(&config, entry_id, &req).await {
                Ok(_) => on_changed.call(()),
                Err(e) => on_error.call(e.message),
            }
        });
    },
}
```

**readonly_mode (D-08):** Aktiviert bei `matches!(phase_status, RepaymentPhaseStatusTO::Closed)`. Verhalten:
- Header-Action-Leiste komplett ausgeblendet (kein Add, keine Bulk-Buttons)
- Checkbox-Spalte (thead `th` + tbody `td`) ausgeblendet
- Anteile-Zelle rendert als plain `span` (kein EditableShareCountCell-Mount)
- Actions-Spalte ausgeblendet (kein Trash-Icon)
- Tab-Strip-Filter bleibt aktiv (read-only-View kann gefiltert werden)

**Detail-Page-Integration** (`genossi-frontend/src/page/repayment_phase_details.rs`):

Im `"entries"`-Tab-Body-Branch wurde der Plan-12-05-Stub
```rust
_ => rsx! { div { "TODO Plan 12-08: RepaymentEntryList für phase_id={...}" } }
```
ersetzt mit dem RepaymentEntryList-Mount:
```rust
_ => rsx! {
    RepaymentEntryList {
        phase: phase_for_entries,
        on_changed: move |_| load_phase(),
        on_add: move |_| show_toast(&mut toast_messages, &mut toast_counter, "Add-Modal kommt in Plan 12-09".into()),
        on_paidout_request: move |_entries: Vec<RepaymentEntryTO>| show_toast(&mut toast_messages, &mut toast_counter, "PaidOut-Confirm kommt in Plan 12-10".into()),
        on_mail_request: move |_ids: Vec<uuid::Uuid>| show_toast(&mut toast_messages, &mut toast_counter, "Mail-Redirect kommt in Plan 12-13".into()),
        on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
    }
},
```
Die drei `on_add`/`on_paidout_request`/`on_mail_request`-Toast-Placeholder sind die Verdrahtungs-Punkte fuer 12-09/12-10/12-13.

## Render-Path (Data Flow)

```
RepaymentEntryList (mount)
  ↓ use_effect:
    spawn → refresh_members()
    + load_entries() (spawn → api::list_repayment_entries)
  ↓ entries.set(...)
  ↓ Render-Tree:
    Status-Filter-Tabs (4× StatusFilterTab mit Count-Badges)
    Header-Action-Leiste (if !readonly_mode):
      Add → on_add.call(())
      Mail → on_mail_request.call(selected_ids)
      Mark-Contacted → spawn → api::batch_toggle_repayment_status → on_changed
      Mark-PaidOut → on_paidout_request.call(selected_entries)
    Tabelle (if !loading && !sorted.is_empty()):
      thead (Checkbox-all-Toggle + 6/7 column-headers, abhaengig von readonly_mode)
      tbody (Row-Loop):
        Per-Row-Checkbox → selected_ids.write().push/retain
        Member-Join (member_for_entry → "—" fallback)
        EditableShareCountCell (if !readonly_mode && !is_paidout):
          → on_save → spawn → api::update_repayment_entry → on_changed
        format_payout_eur(share_count, share_value) → "60,00 €"
        RepaymentEntryStatusBadge { status: entry_status }
        Trash-Icon (if !readonly_mode && !is_paidout) → delete_confirm_for.set(Some(id))
    Empty-State-Branch (if sorted.is_empty()):
      → AutoFill-Text oder Filter-Text
    Delete-Confirm-Modal (if delete_confirm_for.is_some()):
      Cancel → delete_confirm_for.set(None)
      Delete → spawn → api::delete_repayment_entry → on_changed
```

## How It Was Verified

```bash
# Task 1 RED
$ cd genossi-frontend && cargo test --bin genossi-frontend component::repayment_entry_list::tests
test result: FAILED. 2 passed; 6 failed; 0 ignored

# Task 1 GREEN
$ cd genossi-frontend && cargo test --bin genossi-frontend component::repayment_entry_list::tests
test result: ok. 8 passed; 0 failed

# Task 2 build
$ cd genossi-frontend && cargo build --bin genossi-frontend
warning: ... 24 warnings (unused i18n keys; will be consumed by 12-09..12-15)
    Finished `dev` profile in 0.29s

# Task 2 full test suite
$ cd genossi-frontend && cargo test --bin genossi-frontend
test result: ok. 173 passed; 0 failed; 0 ignored; 0 measured

# D-01 Button-Gate (zero buttons without r#type:)
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' \
    genossi-frontend/src/component/repayment_entry_list.rs \
    genossi-frontend/src/page/repayment_phase_details.rs \
  | grep -v 'r#type:' | grep -c 'button {'
0

# Done-criteria greps
$ rg "TODO Plan 12-08" genossi-frontend/src/page/repayment_phase_details.rs
(0 Treffer — Stub ersetzt)

$ rg "RepaymentEntryList \{" genossi-frontend/src/page/repayment_phase_details.rs
1 Treffer

$ rg "EditableShareCountCell \{" genossi-frontend/src/component/repayment_entry_list.rs
1 Treffer

$ rg "RepaymentEntryStatusBadge \{" genossi-frontend/src/component/repayment_entry_list.rs
1 Treffer

$ rg "format_payout_eur\(" genossi-frontend/src/component/repayment_entry_list.rs
1 Treffer

$ rg "api::list_repayment_entries|api::update_repayment_entry|api::delete_repayment_entry|api::batch_toggle_repayment_status" \
    genossi-frontend/src/component/repayment_entry_list.rs
4 distinct Treffer

$ rg "\.\.Default::default\(\)" genossi-frontend/src/component/repayment_entry_list.rs
1 Treffer (Doc-Kommentar, kein code-spread)

$ rg "impl Default for MemberTO|#\[derive\([^)]*Default" genossi-frontend/rest-types/src/lib.rs
0 Treffer (Backend bleibt unveraendert — Option B verboten)
```

All plan-acceptance criteria pass.

## Decisions Made

### Pure-Helpers sind `pub` (component/mod.rs re-exportiert sie)

Die 4 Pure-Funcs sind public und werden in `component/mod.rs` re-exportiert. Plans 12-09 (Add-Modal-Liste) und 12-10 (PaidOut-Confirm-Listentabelle) koennen sie reusen, falls dort ebenfalls Member-Mismatch-Defensive-Joins oder Sort-Logik gebraucht werden. Verhindert Duplikation, falls zwei UI-Pfade dieselbe Sortier-Strategie brauchen.

### StatusFilterTab als inline-`#[component]` in derselben Datei

Statt eines eigenen Files (`component/status_filter_tab.rs`) lebt der Sub-Component inline. Pattern: wenn ein Sub-`#[component]` nur von einer einzigen Caller-Datei genutzt wird, ist inline-Definition akzeptabel (analog zu `BasicsTab` in `page/repayment_phase_details.rs`, Plan 12-05 D). Component-First-Prinzip ist nicht verletzt: Wenn ein zweiter Caller spaeter dieselbe Tab-Styling braucht, kann ein Extract-Refactor folgen (Plan-Discretion).

### Single-Click-Header-Checkbox = "select all visible"

Header-Checkbox setzt ALLE gefiltert-sichtbaren Entry-IDs auf `selected_ids` (bzw. leert die Liste wenn alle schon checked sind). Bei Filter-Wechsel wird die Selection NICHT zurueckgesetzt — Vorstand kann gefiltert auswaehlen ("Alle 'offene' selektieren") und dann den Filter wechseln, um die Selection zu sehen. Pragmatic; D-11 spezifiziert nichts zur Filter-Selection-Interaktion.

### Delete-Confirm-Modal speichert nur Entry-ID

`delete_confirm_for: Signal<Option<Uuid>>` speichert nur die ID, nicht die volle Entry-Daten. Modal-UI ist eine generische "Eintrag loeschen?"-Frage (i18n `RepaymentEntryDeleteConfirm`). Plan 12-10 hat das vollere Listentabelle-Pattern fuer Bulk-PaidOut (D-16); fuer Single-Entry-Delete reicht das simplere Modal. Reduziert State-Komplexitaet.

### MEMBERS-Refresh + Entries-Load PARALLEL

Im `use_effect` werden zwei separate `spawn(async move { ... })`-Tasks gestartet:
```rust
use_effect(move || {
    spawn(async move { crate::service::member::refresh_members().await; });
    load_entries();  // spawnt selbst intern
});
```
Schnellere Erst-Render-Zeit als sequenziell. Trade-off: wenn MEMBERS noch nicht da ist, faellt Member-Mismatch defensiv auf '—' zurueck (Pitfall 8 von RESEARCH). Nach MEMBERS-Refresh re-rendert der Component automatisch (GlobalSignal-Subscription).

### D-08 readonly_mode-Check nur `Closed`, nicht `Preparation`

`readonly_mode = matches!(phase_status, RepaymentPhaseStatusTO::Closed)`. In der Praxis trifft der Component nur Open + Closed, weil Preparation-Phasen in der Detail-Page schon vor dem RepaymentEntryList-Mount durch den Hinweis-Text "Phase noch nicht geoeffnet" abgefangen werden (D-06, Plan 12-05). Im Fall einer Race-Condition (Phase-Status-Wechsel waehrend laufender Component) bleibt der Open-Pfad aktiv — kein UI-Bug, weil Backend dann selber 404/409 liefern wuerde.

### MemberStatusTO::Normal statt ::Active (Plan-Wording-Korrektur)

Plan referenzierte `MemberStatusTO::Active` als Default fuer den Test-Helper. Diese Variante existiert NICHT in `rest-types/src/lib.rs:145-148`. Tatsaechlich definiert sind `Normal` + `FehlerhaftErfasst`. Test-Helper nutzt `MemberStatusTO::Normal` (= realistische default-Annahme, "Mitglied ist gueltig erfasst"). Keine Backend-Aenderung noetig.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan-Wording referenzierte nicht-existierende MemberStatusTO::Active-Variante**

- **Found during:** Task 1 Implementation (vor RED-Commit)
- **Issue:** Plan-Action-Block enthielt `status: MemberStatusTO::Active` im Test-Helper `make_member`. Diese Variante existiert nicht; `rest-types/src/lib.rs:145-148` definiert nur `Normal` + `FehlerhaftErfasst`. Compile waere mit E0599 fehlgeschlagen.
- **Fix:** `MemberStatusTO::Active` → `MemberStatusTO::Normal` im Test-Helper (= realistische default-Annahme, "Mitglied ist gueltig erfasst").
- **Files modified:** `genossi-frontend/src/component/repayment_entry_list.rs` (test module)
- **Verification:** `cargo test component::repayment_entry_list::tests` zeigt 8 PASS — Test-Helper kompiliert sauber.
- **Committed in:** 8e7ce28 (RED) und 4f84e0f (GREEN)

**Total deviations:** 1 auto-fixed (Rule 1 - Plan-Wording-Bug).
**Impact on plan:** Keine; Plan-Pflicht-Sync (Option A) blieb unveraendert (alle 26 Felder explizit, kein Default-Spread). Backend bleibt unveraendert (Option B weiter verboten).

## Known Stubs

Die drei EventHandler-Toast-Placeholder in `genossi-frontend/src/page/repayment_phase_details.rs::EntriesTab` sind absichtliche Hand-Offs:

| Placeholder | i18n-Toast | Resolved by |
|---|---|---|
| `on_add: \|_\| show_toast("Add-Modal kommt in Plan 12-09".into())` | Toast | Plan 12-09 |
| `on_paidout_request: \|_entries\| show_toast("PaidOut-Confirm kommt in Plan 12-10".into())` | Toast | Plan 12-10 |
| `on_mail_request: \|_ids\| show_toast("Mail-Redirect kommt in Plan 12-13".into())` | Toast | Plan 12-13 |

Diese sind KEINE Bugs — sie sind die explizite Verdrahtungs-Schnittstelle, die der Component-Contract vorsieht. Die Component selbst ist voll funktional fuer alle Lese-/Schreib-Operationen, die NICHT diese 3 Handler benoetigen (Filter, Sort, Inline-Edit, Soft-Delete, Bulk-Mark-Contacted).

Der grep-Test `rg 'kommt in Plan 12-(09|10|13)' genossi-frontend/src/page/repayment_phase_details.rs` sollte nach diesem Plan 3 Zeilen liefern und nach Abschluss von 12-09/12-10/12-13 entsprechend abnehmen.

## Threat Flags

None — dieser Plan konsumiert nur bestehende Backend-Endpoints (Phase 7-11) und fuegt UI hinzu. Keine neue Netzwerk-Surface, keine Auth-Pfaede, keine Schema-Aenderungen. Soft-Delete via `DELETE /api/repayment-entry/{id}` ist Backend-Standard (Phase 8 ENTR-05); Frontend ruft das Pattern, Backend mapped intern auf PUT mit `deleted`-Timestamp.

## Self-Check: PASSED

Verifizierte Artefakte gegen das Repo:

- ✓ `genossi-frontend/src/component/repayment_entry_list.rs` existiert (492 Zeilen, > 300 plan-minimum)
- ✓ `pub mod repayment_entry_list` + `pub use ...RepaymentEntryList, StatusFilter, StatusCounts, plus 4 Helpers` in `component/mod.rs`
- ✓ `#[component] pub fn RepaymentEntryList(phase, on_changed, on_add, on_paidout_request, on_mail_request, on_error)` definiert
- ✓ `#[component] fn StatusFilterTab(label, is_selected, on_click)` inline-definiert
- ✓ 4 pure-Funcs (filter_entries_by_status, entry_counts_by_status, member_for_entry, sort_entries_default) public
- ✓ `genossi-frontend/src/page/repayment_phase_details.rs` zeigt `TODO Plan 12-08` 0× und `RepaymentEntryList {` 1×
- ✓ Imports erweitert um `RepaymentEntryTO` und `RepaymentEntryList`
- ✓ D-01 Button-Gate fuer beide Dateien = 0 (Multi-Line-Grep)
- ✓ `..Default::default()` nicht im Code (nur in Doc-Kommentar erwaehnt)
- ✓ Backend `MemberTO` bleibt ohne `#[derive(Default)]` (verifiziert: 0 Treffer)
- ✓ Commit `8e7ce28` (RED, 8 Tests fail/passive)
- ✓ Commit `4f84e0f` (GREEN, 8/8 PASS)
- ✓ Commit `4e68d7b` (Task 2 feat — Component + Detail-Page-Wire-up)
- ✓ `cargo build --bin genossi-frontend` exit 0
- ✓ `cargo test --bin genossi-frontend` exit 0 mit 173 passing (vorher 165)

## TDD Gate Compliance

- **Task 1 RED gate:** `8e7ce28` (`test(12-08): add failing tests for repayment_entry_list pure helpers`) — 6 von 8 Tests fail (Stubs returnen `Vec::new()` / None / Zero-Counts); 2 passive-pass (`counts_empty_returns_zeros`, `member_for_entry_returns_none_on_mismatch`) — meaningful-behavior ist RED.
- **Task 1 GREEN gate:** `4f84e0f` (`feat(12-08): implement repayment_entry_list pure helpers`) — alle 8/8 PASS.
- **REFACTOR gate:** kein Commit — Implementation war minimal und matched die Codebase-Konvention.
- **Task 2:** kein expliziter RED (Task 2 ist nicht TDD-markiert im Plan; Acceptance ist Build + Reuse-Greps). Plan 12-08 Task 2 ist UI-Wiring, kein Pure-Logic-Refactor.

Gate sequence in `git log 4b20acb..HEAD`:
```
8e7ce28 test(12-08): add failing tests for repayment_entry_list pure helpers  ← Task 1 RED
4f84e0f feat(12-08): implement repayment_entry_list pure helpers              ← Task 1 GREEN
4e68d7b feat(12-08): wire RepaymentEntryList component + integrate into detail page  ← Task 2
```

Strict test→feat→feat — TDD-gate erfuellt fuer Task 1.

## Next Phase Readiness

**Bereit fuer Wave 5+ (parallele Plans 12-09, 12-10, 12-13):**

- **Plan 12-09 (Add-Entry-Modal):** ersetzt `on_add`-Placeholder in `page/repayment_phase_details.rs` mit echtem Modal-Open-Signal; nutzt `MemberSearch` direkt (D-21) + `parse_euro_to_cents` aus Plan 12-02 (D-23 minimal-Validation) + `api::create_repayment_entry`.
- **Plan 12-10 (PaidOut-Confirm-Modal):** ersetzt `on_paidout_request`-Placeholder mit echtem Confirm-Modal-Open; sequential-Loop ueber `api::mark_repayment_entry_paid_out` pro Entry; nach Loop einmaliger `crate::service::member::refresh_members().await` (Pitfall 3) + `on_changed`. Bulk-Confirm-Modal D-16 mit Listentabelle + Summe + 3-Punkt-Warnliste.
- **Plan 12-13 (Massenmail-Redirect):** ersetzt `on_mail_request`-Placeholder mit `navigator.push(Route::MailPage)` + Query-Param-Encoding (`?from=repayment&phase_id=...&members=...`); `/mail`-Page-Erweiterung parsed Query-Params und befuellt `selected_member_ids` + `repayment_phase_id`-Signal vor; `send_bulk_mail`-Aufruf mit Phase-12-Felder gefuellt (vorbereitet in Plan 12-01).

Alle drei Plans aendern AUSSCHLIESSLICH die Detail-Page-Caller-Seite und ggf. neue Files; die RepaymentEntryList-Component selbst bleibt unveraendert. Component-Contract via EventHandler-Props funktioniert als saubere Verdrahtungs-Schnittstelle.

---

*Phase: 12-frontend-component-first*
*Plan: 08 — RepaymentEntryList (UI-03)*
*Completed: 2026-06-01T12:36:32Z (~7 min)*
