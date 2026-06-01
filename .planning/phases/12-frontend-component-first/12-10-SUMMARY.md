---
phase: 12-frontend-component-first
plan: 10
subsystem: ui
tags: [frontend, component, modal, confirm, bulk-loop, audit, wave-7]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 — api::mark_repayment_entry_paid_out + RepaymentEntryTO + RepaymentEntryStatusTO + Phase-12 i18n-Keys (RepaymentEntryPaidOutConfirmTitle/Sum/Warn1-3/Button)"
  - phase: 12-frontend-component-first
    provides: "Plan 12-02 — format_payout_eur Pure-Helper"
  - phase: 12-frontend-component-first
    provides: "Plan 12-05 — Detail-Page mit BasicsTab + load_phase + ToastContainer + entries_reload_trigger Counter-Pattern"
  - phase: 12-frontend-component-first
    provides: "Plan 12-08 — RepaymentEntryList mit on_paidout_request: EventHandler<Vec<RepaymentEntryTO>> Placeholder"
provides:
  - "#[component] pub fn RepaymentEntryPaidOutConfirm(entries, share_value_cents, on_close, on_complete, on_error) — UI-05 Bulk-Confirm-Modal mit Sequential-Loop"
  - "pub fn sum_payout_amounts(&[RepaymentEntryTO], i64) -> i64 — D-16 Gesamt-Summe Pure-Helper"
  - "Detail-Page: paidout_modal_entries: Signal<Option<Vec<RepaymentEntryTO>>> + Modal-Mount + on_paidout_request-Verdrahtung"
affects:
  - "Plan 12-08 on_paidout_request Toast-Placeholder ersetzt durch echtes Modal-Open (verbleibender Toast-Stub aus 12-08: nur noch on_mail_request fuer Plan 12-13)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Sequential-Loop pro Entry (D-15) statt Backend-Bulk-Endpoint — Frontend-Atomarität-pro-Entry; Backend-Atomarität ist je-Entry-Transaction (Phase 9 PAYO-Cascade)"
    - "Pitfall 3 Mitigation: refresh_members().await nach Loop-Abschluss — current_shares-Cascade durch PaidOut-Cascade serverseitig invalidiert MEMBERS-Global-Signal"
    - "Caller-Discretion Summary-Toast: Component liefert (success, failure)-Tuple an on_complete; Detail-Page formuliert deutsche Meldung (deutsche Pluralisierung, Backend-Wording-Freiheit)"
    - "Modal-Mount via if-let Combo (paidout_modal_entries.is_some() && phase.is_some()) — share_value_cents braucht die Phase, Modal kann nur mit beiden gerendert werden"
    - "Counter-Bump + load_phase nach on_complete — Status-Spalten (PaidOut-Badge), version-UUIDs und current_shares-Sidebar bleiben in Sync"

key-files:
  created:
    - "genossi-frontend/src/component/repayment_entry_paidout_confirm.rs (212 Zeilen: sum_payout_amounts Pure-Helper + RepaymentEntryPaidOutConfirm #[component] + 5 Unit-Tests)"
  modified:
    - "genossi-frontend/src/component/mod.rs (Phase-12 Plan 12-10-Block: pub mod + Re-Export sum_payout_amounts + RepaymentEntryPaidOutConfirm)"
    - "genossi-frontend/src/page/repayment_phase_details.rs (paidout_modal_entries Signal + Modal-Mount + on_paidout_request Wiring — ersetzt Plan-12-08 Toast-Placeholder)"

key-decisions:
  - "sum_payout_amounts liefert Cent (i64), nicht Euros (f64) — konsistent mit format_payout_eur(share_count=1, total_sum_cents) als Format-Hack; vermeidet f64-Rundung in der Pipeline"
  - "Modal-Render fuegt Member-Lookup pro Row direkt via MEMBERS.read() ein — kein separater member_for_entry-Aufruf, weil das nur 4 Felder pro Row liest und der Component eh nur waehrend des Modal-Mounts lebt"
  - "on_complete-Callback-Signatur (usize, usize) als (success, failure) — Detail-Page-Discretion fuer Toast-Wording (deutsche Pluralisierung, Format-Freiheit; Backend-Wording bleibt im Plan 12-10 nicht hardcoded)"
  - "Cancel-Button + Endgueltig-markieren-Button beide mit `disabled: *submitting.read()` — verhindert doppel-Clicks waehrend des Sequential-Loops; Modal kann waehrend Loop nicht geschlossen werden (Vorstand muss warten)"
  - "Sequential-Loop nutzt borrowed `&entry` im for-Block (Iterator-Pattern via entries.iter()) — kein clone pro Entry; nur entries.clone() einmal vor dem spawn fuer ownership-into-async"
  - "TDD-Gate erfuellt: RED (d01ea95) Stub returns -1, GREEN (2d50023) implementiert sum, dann Component-Wiring (cba5a4e) nicht-TDD-markiert"
  - "Caller-Discretion auch fuer entry-Fehler-Toast-Wording: on_error.call(format!('Eintrag {}: {}', entry.id, e.message)) — Plan-Implementation prefixed entry.id im Toast, damit Vorstand bei Mehrfach-Fehlern weiss welcher Eintrag betroffen ist"

patterns-established:
  - "EventHandler-Tupel-Callback (usize, usize) als typed-Hand-Off statt zwei separate EventHandler<usize>-Props — reduziert Component-API-Oberflaeche bei minimaler Generizitaet"
  - "Sequential-Loop mit per-Entry-Toast (D-17) UND Summary-Toast (D-15) koexistieren — ToastContainer stapelt sie automatisch (toast.rs etabliert in Phase 4)"
  - "Pitfall-3-Mitigation als verpflichtende Loop-Abschluss-Aktion: refresh_members().await VOR on_complete.call() — Sidebar/anderen Pages sehen die neuen current_shares bevor der Summary-Toast erscheint"

requirements-completed: [UI-05]

# Metrics
duration: ~12 min
completed: 2026-06-01T13:08:30Z
task-count: 2
file-count: 3
test-count-added: 5
test-count-total: 181
commits:
  - {sha: d01ea95, type: test, task: "1 RED", scope: "component/repayment_entry_paidout_confirm.rs (sum_payout_amounts stub + 5 tests)"}
  - {sha: 2d50023, type: feat, task: "1 GREEN", scope: "component/repayment_entry_paidout_confirm.rs (sum_payout_amounts impl)"}
  - {sha: cba5a4e, type: feat, task: 2, scope: "component/repayment_entry_paidout_confirm.rs + mod.rs + page/repayment_phase_details.rs"}
---

# Phase 12 Plan 10: RepaymentEntryPaidOutConfirm (UI-05) Summary

**One-liner:** UI-05 PaidOut-Bulk-Confirm-Modal mit Listentabelle (Mitgl.-Nr./Name/Anteile/Betrag), Gesamtsumme, 3-Punkt-Warnliste in rot und rotem "Endgueltig markieren"-Button (D-16); Klick startet Sequential-Loop ueber api::mark_repayment_entry_paid_out pro Entry (D-15 Single-Endpoint-Backend), liefert pro Fehler einen Toast (D-17) und am Ende einen Summary-Toast plus refresh_members().await (Pitfall 3) — wired in der Detail-Page-EntriesTab als Ersatz fuer den `on_paidout_request`-Toast-Placeholder aus Plan 12-08.

## What Was Built

Zwei Tasks. Task 1 (TDD) liefert die `sum_payout_amounts`-Pure-Helper mit 5 Unit-Tests; Task 2 baut den vollstaendigen Modal-Component mit Listentabelle, Sequential-Loop, Pitfall-3-Mitigation und der Detail-Page-Integration.

### Task 1: sum_payout_amounts Pure-Helper + Tests (commits d01ea95 RED, 2d50023 GREEN)

Datei `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` mit:

- **`pub fn sum_payout_amounts(&[RepaymentEntryTO], share_value_cents: i64) -> i64`** — D-16 Gesamt-Summe in Cent: `entries.iter().map(|e| (e.share_count_to_pay_out as i64) * share_value_cents).sum()`.

**5 Unit-Tests:**
1. `sum_single_entry` — 1 Entry mit share_count=1, share_value=100 EUR → 10_000 Cent; 1 Entry mit share_count=5 → 50_000 Cent
2. `sum_multiple_entries` — 3 Entries (2,3,1 Anteile) → 60_000 Cent (= 600 EUR)
3. `sum_empty_returns_zero` — leere Liste → 0
4. `sum_zero_share_count_defensive` — Entry mit share_count=0 → 0 (sollte nicht vorkommen, aber defensiv)
5. `sum_realistic_phase_total` — 5 Eintraege mit je 3 Anteilen, 100 EUR pro Anteil → 150_000 Cent (= 1.500 EUR)

**RED-Gate:** Stub `pub fn sum_payout_amounts(...) -> i64 { -1 }` — alle 5 Tests failten mit `assertion left == right failed: left: -1, right: <expected>` (verifiziert lokal vor GREEN-Commit).

**GREEN-Gate:** Implementation via `.iter().map(...).sum()` — alle 5 Tests pass.

### Task 2: Full Component + Sequential-Loop + Detail-Page-Wire-up (commit cba5a4e)

**`#[component] pub fn RepaymentEntryPaidOutConfirm(entries, share_value_cents, on_close, on_complete, on_error) -> Element`** — UI-05 Bulk-Confirm-Modal.

**Render-Struktur:**
1. **Titel** in rot: `RepaymentEntryPaidOutConfirmTitle` ("Eintraege endgueltig als ausbezahlt markieren?")
2. **Listentabelle** (D-16, 4 Spalten):
   - Mitgl.-Nr. (aus MEMBERS-Join, "—" bei Mismatch)
   - Name (`{first_name} {last_name}`, "—" bei Mismatch)
   - Anteile (entry.share_count_to_pay_out)
   - Betrag (`format_payout_eur(share_count, share_value_cents)`)
3. **Gesamtsumme** rechts-aligned, bold: `RepaymentEntryPaidOutConfirmSum` + `format_payout_eur(1, total_sum_cents)` ("Summe: X,YY €")
4. **3-Punkt-Warnliste** in rot mit rot-100-Background (D-16):
   - `RepaymentEntryPaidOutConfirmWarn1` ("Diese Aktion ist final — kein Rueckweg moeglich.")
   - `RepaymentEntryPaidOutConfirmWarn2` ("Erzeugt einen Verkauf-Audit-Eintrag pro Mitglied.")
   - `RepaymentEntryPaidOutConfirmWarn3` ("Reduziert current_shares der betroffenen Mitglieder.")
5. **Buttons** (D-01, beide mit `r#type:`):
   - Cancel (grau, `disabled` waehrend Loop)
   - Endgueltig markieren (rot, `disabled` waehrend Loop) → startet Sequential-Loop

**Sequential-Loop-Logik:**
```rust
spawn(async move {
    let config = CONFIG.read().clone();
    let mut success_count = 0usize;
    let mut failure_count = 0usize;
    for entry in entries.iter() {
        match api::mark_repayment_entry_paid_out(&config, entry.id).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                failure_count += 1;
                on_error.call(format!("Eintrag {}: {}", entry.id, e.message));
            }
        }
    }
    crate::service::member::refresh_members().await;  // Pitfall 3
    on_complete.call((success_count, failure_count));
});
```

- **D-15 Single-Endpoint:** Backend hat keinen Bulk-PaidOut-Endpoint; Frontend orchestriert Sequential-Loop. Backend-Atomarität ist je-Entry-Transaction (Phase 9 PAYO-Cascade).
- **D-17 Per-Entry-Toast:** Bei Fehler eines Entries wird `on_error.call(format!("Eintrag {}: {}", entry.id, e.message))` mit der entry.id als Prefix gerufen — Vorstand kann Fehler-Eintraege identifizieren.
- **Pitfall 3:** `refresh_members().await` VOR `on_complete.call(...)` — die Sidebar/anderen Pages sehen die neuen `current_shares` bevor der Summary-Toast erscheint.
- **D-15 Summary-Toast:** `on_complete((success, failure))` triggert in der Detail-Page einen Summary-Toast.

**Detail-Page-Integration** (`genossi-frontend/src/page/repayment_phase_details.rs`):

1. **Import** erweitert:
```rust
use crate::component::{
    ErrorAlert, Modal, RepaymentEntryAddModal, RepaymentEntryList, RepaymentEntryPaidOutConfirm,
    RepaymentPhaseStatusBadge, TabDef, TabStrip, ToastContainer, TopBar, show_toast,
};
```

2. **Signal:**
```rust
let mut paidout_modal_entries = use_signal(|| Option::<Vec<RepaymentEntryTO>>::None);
```

3. **on_paidout_request-Wiring** (ersetzt Plan-12-08 Toast-Placeholder):
```rust
on_paidout_request: move |entries: Vec<RepaymentEntryTO>| {
    paidout_modal_entries.set(Some(entries));
},
```

4. **Modal-Mount** (nach show_add_modal-Modal, vor ToastContainer):
```rust
if let Some(entries_to_confirm) = paidout_modal_entries.read().clone() {
    if let Some(p) = phase.read().clone() {
        Modal {
            RepaymentEntryPaidOutConfirm {
                entries: entries_to_confirm,
                share_value_cents: p.share_value,
                on_close: move |_| paidout_modal_entries.set(None),
                on_complete: move |(success, failure): (usize, usize)| {
                    paidout_modal_entries.set(None);
                    let total = success + failure;
                    let msg = if failure == 0 {
                        format!("{success} Eintraege als ausbezahlt markiert.")
                    } else {
                        format!(
                            "{success} von {total} erfolgreich, {failure} fehlgeschlagen — siehe Status-Spalte."
                        )
                    };
                    show_toast(&mut toast_messages, &mut toast_counter, msg);
                    let current = *entries_reload_trigger.read();
                    entries_reload_trigger.set(current.wrapping_add(1));
                    load_phase();
                },
                on_error: move |msg: String| show_toast(&mut toast_messages, &mut toast_counter, msg),
            }
        }
    }
}
```

## Render-Path (Data Flow)

```
RepaymentEntryList (Wave 5):
  ↓ Vorstand selektiert Open/Contacted-Eintraege via Checkboxen
  ↓ klickt "Als ausbezahlt markieren" Header-Button
  ↓ on_paidout_request.call(selected_entries) — Vec<RepaymentEntryTO>

Detail-Page:
  ↓ paidout_modal_entries.set(Some(entries))
  ↓ Modal { RepaymentEntryPaidOutConfirm { entries, share_value_cents, ... } }

RepaymentEntryPaidOutConfirm:
  ↓ Render: Listentabelle + Summe + 3-Punkt-Warnliste + Cancel/Endgueltig-Buttons
  ↓ Vorstand klickt "Endgueltig markieren"
  ↓ submitting.set(true); spawn(async {
      Sequential-Loop:
        for entry in entries:
          api::mark_repayment_entry_paid_out(config, entry.id).await
          Ok → success_count += 1
          Err → failure_count += 1, on_error.call("Eintrag {id}: {msg}")  // D-17 Toast
      refresh_members().await                                              // Pitfall 3
      on_complete.call((success, failure))                                 // D-15 Summary-Toast
    })

Detail-Page on_complete:
  ↓ paidout_modal_entries.set(None) — Modal schliesst
  ↓ Summary-Toast (deutsche Pluralisierung)
  ↓ entries_reload_trigger.set(current.wrapping_add(1)) — RepaymentEntryList re-fetched
  ↓ load_phase() — Phase-Version + opened_at frisch
```

## How It Was Verified

```bash
# Task 1 RED
$ cd genossi-frontend && cargo test --bin genossi-frontend component::repayment_entry_paidout_confirm::tests
test result: FAILED. 0 passed; 5 failed

# Task 1 GREEN
$ cd genossi-frontend && cargo test --bin genossi-frontend component::repayment_entry_paidout_confirm::tests
test result: ok. 5 passed; 0 failed

# Task 2 build
$ cd genossi-frontend && cargo build --bin genossi-frontend
warning: ... 23 warnings (unused i18n keys; will be consumed by 12-13/12-14)
    Finished `dev` profile in 28.08s

# Task 2 full test suite
$ cd genossi-frontend && cargo test --bin genossi-frontend
test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured

# D-01 Button-Gate (zero buttons without r#type:)
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' \
    genossi-frontend/src/component/repayment_entry_paidout_confirm.rs \
    genossi-frontend/src/page/repayment_phase_details.rs \
  | grep -v 'r#type:' | grep -c 'button {'
0

# Done-criteria greps
$ rg "api::mark_repayment_entry_paid_out" genossi-frontend/src/component/repayment_entry_paidout_confirm.rs | wc -l
2  # 1 doc-comment + 1 actual call

$ rg "refresh_members" genossi-frontend/src/component/repayment_entry_paidout_confirm.rs | wc -l
3  # 1 module-doc + 1 doc-comment + 1 actual call

$ rg "RepaymentEntryPaidOutConfirm \{" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
1

$ rg "kommt in Plan 12-10" genossi-frontend/src/page/repayment_phase_details.rs
(empty — Toast-Placeholder ersetzt)
```

All plan-acceptance criteria pass.

## Decisions Made

### Cent-Pipeline statt Float-Konvertierung in sum_payout_amounts

`sum_payout_amounts` liefert i64-Cent statt f64-Euros. Begruendung: Konsistent mit `format_payout_eur(share_count, share_value_cents)`-Signatur (Plan 12-02). Im Modal-Footer wird das Total via `format_payout_eur(1, total_sum)` ausgegeben — der `share_count=1`-Trick nutzt die existierende Formatierungs-Logik (`total_cents = 1 * total_sum_cents`). Kein f64-Roundtrip in der Pipeline; keine Drift-Risiken.

### Member-Lookup pro Row inline (kein member_for_entry-Aufruf)

In der Listentabelle wird MEMBERS.read() einmal pro Render geladen, dann pro Row via `members_state.items.iter().find(...)`. Plan-12-08 hat `member_for_entry()` als pure-Helper, aber dieser fuegt eine zusaetzliche Abstraktionsschicht ohne Mehrwert hinzu — die Modal-Liste rendert hoechstens ~20 Eintraege (Vorstand-Schmerzgrenze laut CONTEXT.md), und die find-Iteration ist trivial. Plan 12-10 nutzt deshalb den direkten Inline-Lookup statt member_for_entry. Vorteil: keine zusaetzliche Pure-Helper-Sync mit der Caller-API.

### on_complete als (usize, usize)-Tupel statt zwei separate EventHandler

Component-API liefert `EventHandler<(usize, usize)>` als (success, failure). Alternative: zwei separate `EventHandler<usize>`-Props (on_success_count + on_failure_count). Tupel-Signatur reduziert die Component-API-Oberflaeche und gibt der Detail-Page die volle Wording-Discretion fuer den Summary-Toast (deutsche Pluralisierung kann nicht generisch im Component formuliert werden).

### Cancel-Button waehrend Sequential-Loop disabled

Beide Buttons (Cancel + Endgueltig markieren) sind `disabled: *submitting.read()` waehrend des Loops. Begruendung: ein Cancel-mitten-im-Loop wuerde die Atomic-Pro-Entry-Garantie verletzen (bereits committed Eintraege bleiben PaidOut, abgebrochene bleiben Open/Contacted) und Vorstand koennte den Zwischenstand misinterpretieren. Sicherer: warten bis Summary-Toast.

### Caller-Discretion fuer entry-Fehler-Toast-Wording

Pro Entry-Fehler: `on_error.call(format!("Eintrag {}: {}", entry.id, e.message))`. Die entry.id im Prefix erlaubt Vorstand bei Mehrfach-Fehlern die betroffenen Eintraege zu identifizieren. Alternativ haette der Component nur `e.message` weitergeben koennen und Detail-Page den Prefix selbst formulieren — aber das wuerde zwei Stellen (Component + Detail-Page) zur Wartung des Fehler-Wordings zwingen. Component-Internal-Format ist akzeptabel hier.

### Counter-Bump + load_phase nach on_complete (analog Plan 12-09)

Pattern aus Plan 12-09 reused: `entries_reload_trigger.set(current.wrapping_add(1))` triggert RepaymentEntryList Re-Fetch (Status-Spalten zeigen PaidOut + version-UUIDs aktuell), `load_phase()` aktualisiert die Phase-version. Beide noetig, weil Backend nach Cascade neue version-UUIDs auf Phase + Member + Entry vergibt.

## Deviations from Plan

**None — plan executed exactly as written.**

Die Plan-Acceptance-Tests, die laut Plan-Frontmatter `must_haves.truths` und Done-Criteria, wurden alle exakt verifiziert:
- Neuer Component `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` mit Bulk-Confirm-Modal (D-15, D-16): ✓
- Listentabelle der ausgewaehlten Eintraege + Gesamtsumme + 3-Punkt-Warnliste + roter Button (D-16): ✓
- Pure-Func `sum_payout_amounts` mit 5 Unit-Tests: ✓
- Sequential-Loop pro entry_id mit POST /api/repayment-entry/{id}/mark-paid-out (D-15): ✓
- Per-Fehler ein on_error-Call (D-17): ✓
- Summary-Toast via on_complete.call((success, failure)) (D-15): ✓
- refresh_members().await nach Loop (Pitfall 3): ✓
- Detail-Page haelt `show_paidout_modal: Option<Vec<RepaymentEntryTO>>` — Plan-Wording sagte `show_paidout_modal`, tatsaechlicher Name ist `paidout_modal_entries` (klarer als Plan-Wording, weil das Signal Vec haelt, nicht Bool); semantisch identisch
- D-01 Button-Pattern: roter "Endgueltig markieren"-Button + grauer "Abbrechen"-Button, beide mit r#type: ✓
- cargo build exit 0: ✓
- cargo test exit 0 (181 grün): ✓
- D-01 Grep-Gate beide Dateien = 0: ✓

## Known Stubs

**Verbleibender Toast-Placeholder in Detail-Page:** `on_mail_request: move |_ids| show_toast(&mut toast_messages, &mut toast_counter, "Mail-Redirect kommt in Plan 12-13".into())` — wird durch Plan 12-13 (Massenmail-Redirect) ersetzt. Das ist KEIN Bug aus diesem Plan; es ist der einzige verbleibende Verdrahtungs-Hand-Off aus Plan 12-08, der durch Plan 12-13 abgeschlossen wird.

Der `on_paidout_request`-Toast-Placeholder aus Plan 12-08 ist durch dieses Plan 12-10 vollstaendig ersetzt (grep `"PaidOut-Confirm kommt in Plan 12-10"` liefert 0).

## Threat Flags

None — dieser Plan konsumiert nur bestehende Backend-Endpoints (Phase 9 `POST /api/repayment-entry/{id}/mark-paid-out`) und fuegt UI hinzu. Keine neue Netzwerk-Surface, keine Auth-Pfaede, keine Schema-Aenderungen. Sequential-Loop ist client-side Orchestrierung von Server-side-Atomarität.

## Self-Check: PASSED

Verifizierte Artefakte gegen das Repo:

- ✓ `genossi-frontend/src/component/repayment_entry_paidout_confirm.rs` exists (212 Zeilen, > 120 plan-minimum)
- ✓ `pub mod repayment_entry_paidout_confirm` + `pub use ... sum_payout_amounts, RepaymentEntryPaidOutConfirm` in `component/mod.rs`
- ✓ `#[component] pub fn RepaymentEntryPaidOutConfirm(entries, share_value_cents, on_close, on_complete, on_error)` definiert
- ✓ `pub fn sum_payout_amounts(&[RepaymentEntryTO], i64) -> i64` definiert
- ✓ `genossi-frontend/src/page/repayment_phase_details.rs` zeigt `"PaidOut-Confirm kommt in Plan 12-10"` 0× und `RepaymentEntryPaidOutConfirm {` 1×
- ✓ Imports erweitert um `RepaymentEntryPaidOutConfirm`
- ✓ `paidout_modal_entries: Signal<Option<Vec<RepaymentEntryTO>>>` definiert
- ✓ Modal-Mount-Block (if let Some(entries) && if let Some(p) = phase.read()) korrekt
- ✓ D-01 Button-Gate fuer beide Dateien = 0 (Multi-Line-Grep)
- ✓ `refresh_members` 1 Call-Site + 2 doc-comments
- ✓ `api::mark_repayment_entry_paid_out` 1 Call-Site + 1 doc-comment
- ✓ Commit `d01ea95` (RED, 5 Tests fail)
- ✓ Commit `2d50023` (GREEN, 5/5 PASS)
- ✓ Commit `cba5a4e` (Task 2 feat — Component + Detail-Page-Wire-up)
- ✓ `cargo build --bin genossi-frontend` exit 0
- ✓ `cargo test --bin genossi-frontend` exit 0 mit 181 passing (vorher 176)

## TDD Gate Compliance

- **Task 1 RED gate:** `d01ea95` (`test(12-10): add failing tests for sum_payout_amounts`) — 5/5 Tests fail (Stub returnt -1).
- **Task 1 GREEN gate:** `2d50023` (`feat(12-10): implement sum_payout_amounts pure-helper`) — alle 5/5 PASS.
- **REFACTOR gate:** kein Commit — Implementation war minimal (3 Zeilen iter/map/sum) und brauchte kein Cleanup.
- **Task 2:** kein expliziter RED (Task 2 ist nicht TDD-markiert im Plan; Acceptance ist Build + Reuse-Greps + D-01-Gate). Plan 12-10 Task 2 ist UI-Wiring, kein Pure-Logic-Refactor.

Gate sequence in `git log 99aac01..HEAD`:
```
d01ea95 test(12-10): add failing tests for sum_payout_amounts                  ← Task 1 RED
2d50023 feat(12-10): implement sum_payout_amounts pure-helper                  ← Task 1 GREEN
cba5a4e feat(12-10): wire RepaymentEntryPaidOutConfirm modal + detail page     ← Task 2
```

Strict test→feat→feat — TDD-gate erfuellt fuer Task 1.

## Next Phase Readiness

**Wave 7 Plan 12-10 ist abgeschlossen.** UI-05 ist voll funktional:
- Vorstand selektiert Open/Contacted-Eintraege in der Liste
- klickt "Als ausbezahlt markieren" Header-Button
- sieht Modal mit Listentabelle + Summe + 3-Punkt-Warnung + rotem Button
- klickt "Endgueltig markieren"
- Sequential-Loop laeuft, pro Fehler ein Toast
- am Ende Summary-Toast + Sidebar-Refresh (current_shares aktualisiert)

**Verbleibender Stub aus Plan 12-08 EntriesTab:**
- Plan 12-13 (Massenmail-Redirect): ersetzt `on_mail_request`-Placeholder mit `navigator.push(Route::MailPage)` + Query-Param-Encoding (`?from=repayment&phase_id=...&members=...`).

Nach Plan 12-13 sind alle drei `RepaymentEntryList`-EventHandler-Placeholder durch echte Logik ersetzt (Add: 12-09 ✓, PaidOut: 12-10 ✓, Mail: 12-13 pending).

---

*Phase: 12-frontend-component-first*
*Plan: 10 — RepaymentEntryPaidOutConfirm (UI-05)*
*Completed: 2026-06-01T13:08:30Z (~12 min)*
