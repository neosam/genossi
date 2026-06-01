---
phase: 12
plan: 12
subsystem: frontend
tags: [frontend, page, mail, query-params, redirect-target, issue-2-blocker-fix]
wave: 8
requires: [01, 11]
provides:
  - "parse_mail_query pure-helper in mail_page.rs (testbar nativ ohne web_sys)"
  - "ParsedMailContext struct (phase_id, member_ids) als pub für potenziellen Reuse in Plan 12-13"
  - "TemplateSelector erweitert um on_select_id-EventHandler (backward-compat via #[props(default)])"
  - "mail_page.rs erkennt ?phase_id=&members=&from=repayment Query-Params und befuellt die Compose-Form vor"
  - "selected_template_id-Signal als echter template_id-Lieferant fuer send_bulk_mail (Issue #2 BLOCKER-Fix)"
  - "TemplateVarButtons rendert Repayment-Vars nur im Repayment-Kontext (show_repayment_vars-Prop)"
affects:
  - genossi-frontend/Cargo.toml
  - genossi-frontend/src/component/mail_compose/template_selector.rs
  - genossi-frontend/src/page/mail_page.rs
tech-stack:
  added: []
  patterns:
    - "Pure-Func + #[cfg(test)] mod tests Pattern (analog member_search::filter_members) fuer parse_mail_query"
    - "Optional EventHandler via #[props(default)] fuer additive Component-API-Erweiterung (TemplateSelector::on_select_id)"
    - "web_sys::window().location().search() + manuelle String-Split fuer Query-Param-Parsing (RESEARCH Pitfall 4 Option 1 — minimal-invasiv, kein dioxus-router-Refactor)"
    - "Signal-driven template_id statt hardcoded None (Plan 12-01 Auto-Fix → Plan 12-12 echtes Signal)"
key-files:
  created: []
  modified:
    - genossi-frontend/Cargo.toml
    - genossi-frontend/src/component/mail_compose/template_selector.rs
    - genossi-frontend/src/page/mail_page.rs
decisions:
  - "parse_mail_query als pub-Funktion + ParsedMailContext als pub-Struct in mail_page.rs deklariert — testbar nativ; Plan 12-13 koennte sie via super::parse_mail_query oder crate::page::mail_page::parse_mail_query reusen"
  - "Manuelles String-Split statt web_sys::UrlSearchParams im parse_mail_query — entkoppelt Pure-Func vom WASM-Target, ermoeglicht cargo-test ohne wasm-bindgen-test (RESEARCH Pitfall 4 Decision)"
  - "TemplateSelector::on_select_id mit #[props(default)] — backward-compat, existing inbox/reply_form.rs Call-Site bleibt unveraendert (verifiziert via cargo build + grep)"
  - "TemplateSelector empty-Option-Reset feuert nun ZUSAETZLICH on_select_id.call(None) — Caller kann selected_template_id zuruecksetzen"
  - "send_bulk_mail-Argumente werden vor dem spawn(async) aus den Signals gelesen (template_id_owned + phase_id) — verhindert Signal-Read-im-async-Block"
  - "web-sys-Feature \"Location\" zu Cargo.toml hinzugefuegt — window.location().search() benoetigt dieses Feature explizit"
metrics:
  duration: ~10min
  completed: "2026-06-01T13:13:58Z"
  task-count: 3
  file-count: 3
  test-count-added: 7
  test-count-total: 188
  commits:
    - {sha: bf73573, type: test, task: 1, scope: "page/mail_page.rs (RED)"}
    - {sha: ce82a8b, type: feat, task: 1, scope: "page/mail_page.rs (GREEN)"}
    - {sha: 6436242, type: feat, task: 2, scope: "component/mail_compose/template_selector.rs"}
    - {sha: a1f99c5, type: feat, task: 3, scope: "Cargo.toml + page/mail_page.rs (5 distinkte Edits)"}
---

# Phase 12 Plan 12: Mail-Page Repayment-Kontext + Issue #2 BLOCKER-Fix Summary

**One-liner:** Drei-Task-Plan, der (1) die pure-fn `parse_mail_query` mit 7 Unit-Tests in mail_page.rs etabliert, (2) den `TemplateSelector` um den `on_select_id`-Callback erweitert (additiv, backward-compat) und (3) fünf distinkte Edits in `mail_page.rs` macht, die Query-Param-Parsing, Pre-Selection der Empfaenger, Repayment-Var-Sichtbarkeit und — als Issue #2 BLOCKER-Fix — den echten `template_id`-Lieferanten via `selected_template_id`-Signal verdrahten.

## What Was Built

Drei Tasks, vier Commits (Task 1 in TDD mit RED + GREEN), ~10 min Dauer.

### Task 1: parse_mail_query Pure-Helper + Unit-Tests (TDD)

**RED (bf73573):** ParsedMailContext-Struct + parse_mail_query-Stub (`{ phase_id: None, member_ids: Vec::new() }`) + 7 Unit-Tests am Datei-Ende von mail_page.rs. `cargo test page::mail_page::tests` ergab 5 FAILED, 2 PASSED (die 2 PASS sind `parse_empty` und `parse_invalid_phase_id` — der Stub liefert exakt was die Tests fuer diese Cases erwarten).

**GREEN (ce82a8b):** Stub durch echten Parser ersetzt:
- `trim_start_matches('?')` macht das Parsing robust gegen Mit/Ohne-fuehrendes-`?`
- Iteration ueber `&`-getrennte Pairs, `splitn(2, '=')` pro Pair
- `phase_id`: `Uuid::parse_str(value).ok()` — bei invalid bleibt `None`
- `members`: `value.split(',').filter_map(|s| Uuid::parse_str(s.trim()).ok()).collect()` — invalide UUIDs werden defensiv rausgefiltert
- Unbekannte Keys (`from=repayment` etc.) werden ignoriert (kein Fehler)

Alle 7 Tests PASS:
- `parse_empty`, `parse_invalid_phase_id`, `parse_valid_phase_id`
- `parse_valid_members`, `parse_members_filters_invalid`
- `parse_combined` (alle drei Params zusammen)
- `parse_without_leading_question_mark`

`ParsedMailContext` ist `pub`, sodass Plan 12-13 (Redirect-Trigger in `RepaymentEntryList`) ggf. die gleiche Helper-Funktion reusen koennte (z.B. fuer URL-Konstruktion via Inverse-Funktion).

### Task 2: TemplateSelector on_select_id-Callback (6436242)

Issue #2 Root-Cause: Bestehender `TemplateSelector` rief nur `on_select(body: String)` — die Template-ID landete nirgends. Die Lösung ist ein zusaetzlicher EventHandler-Prop:

```rust
#[component]
pub fn TemplateSelector(
    on_select: EventHandler<String>,
    #[props(default)] on_select_id: EventHandler<Option<String>>,
) -> Element { ... }
```

**Backward-compat:** `#[props(default)]` macht `on_select_id` optional — der bestehende Aufrufer in `inbox/reply_form.rs` (Zeile 46) bleibt unveraendert kompilierbar (verifiziert via `cargo build`).

**Reset-Verhalten:** Wenn der User die leere Option "Vorlage waehlen..." waehlt, ruft der `onchange`-Handler nun `on_select_id.call(None)` auf. Das ist semantisch wichtig — der Caller (mail_page) kann damit seinen `selected_template_id`-Signal zuruecksetzen, was den naechsten `send_bulk_mail`-Aufruf wieder mit `template_id: None` durchfuehren laesst.

**Verworfene Alternative:** statt zwei separater Callbacks haette man `on_select` auf `EventHandler<(String, Option<String>)>` aendern koennen. Das haette aber die Signatur aller bestehenden Aufrufer gebrochen (reply_form.rs + mail_page.rs) und keinen klaren semantischen Gewinn gebracht.

### Task 3: 5 distinkte Edits in mail_page.rs + Cargo.toml (a1f99c5)

**Edit 1 — Zwei neue Signals** (nach `sending` ~Z. 57):
```rust
let mut repayment_phase_id = use_signal(|| Option::<Uuid>::None);
let mut selected_template_id = use_signal(|| Option::<String>::None);
```

**Edit 2 — Query-Param-Parsing use_effect** (nach refresh_members effect ~Z. 80):
```rust
use_effect(move || {
    if let Some(window) = web_sys::window() {
        if let Ok(search) = window.location().search() {
            let parsed = parse_mail_query(&search);
            if parsed.phase_id.is_some() { repayment_phase_id.set(parsed.phase_id); }
            if !parsed.member_ids.is_empty() { selected_member_ids.set(parsed.member_ids); }
        }
    }
});
```
Nutzt `window.location().search()` direkt — keine `web_sys::UrlSearchParams`-Indirektion noetig, weil `parse_mail_query` manuell String-Split macht. Defensive Bedingungen (`is_some()`, `!is_empty()`) verhindern, dass die Effects bei normalem `/mail`-Zugriff (ohne Query) bestehende State-Inits ueberschreiben.

**Edit 3 — TemplateVarButtons-Prop erweitert** (~Z. 386):
```rust
TemplateVarButtons {
    on_insert: move |var_text: String| { body.write().push_str(&var_text); },
    show_repayment_vars: repayment_phase_id.read().is_some(),
}
```
Plan 12-11 hat den Prop in `TemplateVarButtons` schon eingefuehrt; Plan 12-12 verdrahtet ihn jetzt mit dem Signal-Wert.

**Edit 4 — TemplateSelector mit on_select_id-Callback** (~Z. 396):
```rust
TemplateSelector {
    on_select: move |template_body: String| { /* unchanged */ },
    on_select_id: move |id: Option<String>| { selected_template_id.set(id); },
}
```

**Edit 5 — send_bulk_mail mit echten Werten** (~Z. 550-571):
```rust
let template_id_owned: Option<String> = selected_template_id.read().clone();
let phase_id = *repayment_phase_id.read();
spawn(async move {
    // ...
    let template_id: Option<&str> = template_id_owned.as_deref();
    match api::send_bulk_mail(
        &config, &recipients, &subj, &b, &att_ids, &static_ids,
        template_id,
        phase_id,
    ).await { ... }
});
```
Die Signal-Reads passieren VOR dem `spawn(async)` — sonst muesste der async-Block die Signals capturen, was bei Signal-Move-Semantics kompliziert wird.

**Edit 6 — web-sys-Feature "Location" in Cargo.toml**:
`window.location().search()` benoetigt das `Location`-Feature explizit. Vor Plan 12-12 hatte das Frontend nur `Window` aktiv, aber kein `Location` — das haette einen Compile-Error in Edit 2 verursacht. Pre-empfindlich gefixt via Cargo.toml-Erweiterung.

## How It Was Verified

```bash
# Task 1 done criteria (alle 7 Tests PASS)
$ cd genossi-frontend && cargo test --bin genossi-frontend page::mail_page::tests
running 7 tests
test page::mail_page::tests::parse_empty ... ok
test page::mail_page::tests::parse_combined ... ok
test page::mail_page::tests::parse_invalid_phase_id ... ok
test page::mail_page::tests::parse_valid_members ... ok
test page::mail_page::tests::parse_valid_phase_id ... ok
test page::mail_page::tests::parse_members_filters_invalid ... ok
test page::mail_page::tests::parse_without_leading_question_mark ... ok
test result: ok. 7 passed; 0 failed

# Task 2 done criteria
$ rg "on_select_id" genossi-frontend/src/component/mail_compose/template_selector.rs | wc -l
4   # >= 2: Prop-Definition + on_select_id.call(None) + on_select_id.call(Some(...)) + Doc-Hinweis

# Task 3 done criteria
$ rg "let mut repayment_phase_id" genossi-frontend/src/page/mail_page.rs
    let mut repayment_phase_id = use_signal(|| Option::<Uuid>::None);                  # = 1

$ rg "let mut selected_template_id" genossi-frontend/src/page/mail_page.rs
    let mut selected_template_id = use_signal(|| Option::<String>::None);              # = 1

$ rg "show_repayment_vars" genossi-frontend/src/page/mail_page.rs
                                show_repayment_vars: repayment_phase_id.read().is_some(),  # = 1

$ rg "on_select_id" genossi-frontend/src/page/mail_page.rs | wc -l
3   # >= 1: Doc-Hinweis + on_select_id-Callback-Definition + Sub-Doc

$ rg "parse_mail_query" genossi-frontend/src/page/mail_page.rs | wc -l
9   # >= 2: 1 Aufruf in use_effect + 1 pub-Definition + 7 Tests

$ rg "api::send_bulk_mail" genossi-frontend/src/page/mail_page.rs | wc -l
1   # = 1

# Issue #2 BLOCKER-Acceptance
$ rg "template_id:\s*None" genossi-frontend/src/page/mail_page.rs | wc -l
0   # = 0 (kein hardcoded None mehr)

$ rg "let template_id:\s*Option<&str>\s*=\s*None" genossi-frontend/src/page/mail_page.rs | wc -l
0   # = 0 (alter Auto-Fix-Default beseitigt)

# D-01 Button-Gate auf mail_page.rs
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' genossi-frontend/src/page/mail_page.rs | grep -v 'r#type:' | grep -c 'button {'
0   # konstant 0

# Overall
$ cargo build -p genossi-frontend
warning: `genossi-frontend` (bin "genossi-frontend") generated 23 warnings ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 21.71s

$ cargo test --bin genossi-frontend
test result: ok. 188 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

188 Tests (181 vor Plan 12-12 + 7 neu) — alle gruen.

## Decisions Made

**parse_mail_query in mail_page.rs (statt eigene Datei):**
Pure-Func + Tests sitzen am Ende von mail_page.rs (~80 LOC inkl. Tests). Plan-Discretion: hat KEIN eigenes Modul bekommen, weil (a) der Helper aktuell nur von `mail_page::use_effect` aufgerufen wird und (b) `ParsedMailContext` Mail-spezifisch ist (nicht generisch). Falls Plan 12-13 die Inverse-Funktion (URL-Construction aus phase_id+members) braucht, kann sie als zweite pub-fn am gleichen Ort leben.

**Defensive Filterung statt Hard-Fail bei invaliden UUIDs (members-list):**
Bei `?members=u1,invalid,u2` wird `invalid` rausgefiltert, statt das gesamte Parsing fehlschlagen zu lassen. Begruendung: Plan 12-13 wird die URLs konstruieren — invalide UUIDs sollten nie auftreten, aber wenn doch (Copy-Paste-Fehler, Browser-Bookmark), ist es freundlicher, die validen IDs zu uebernehmen als eine leere Selektion.

**use_effect VOR Spawn-Block fuer Cargo-Test-Stabilitaet (Edit 5):**
Der `send_bulk_mail`-Block liest die Signals (`selected_template_id.read()`, `repayment_phase_id.read()`) BEVOR der `spawn(async move {...})` startet. Das ist sicherer, weil Dioxus-Signal-Capturing in async-Blocks beim Re-Render zu Race-Conditions fuehren kann.

**web-sys "Location" Feature additiv hinzugefuegt:**
Cargo.toml hatte bereits `Window` und `Url` aktiv, aber nicht `Location`. Letzteres ist die Rueckgabe von `window.location()` und braucht das Feature explizit. Pre-empfindlich gefixt; kein Auto-Fix waehrend des Builds (haette einen Round-Trip gekostet).

**TemplateSelector::on_select_id Reset-Semantik:**
Wenn der User das leere Default-Option waehlt, ruft die Component on_select_id.call(None) auf. Das ist eine semantische Verbesserung: vorher konnte der Caller nicht wissen, ob der User die Vorlage abgewaehlt hat (kein Callback fuer "reset"). Jetzt kann mail_page selected_template_id auf None zuruecksetzen, was beim nachsten Senden den template_id-Backend-Arg wieder leer macht.

## Deviations from Plan

**None — plan executed exactly as written.**

Eine **kleine Praezisierung** waehrend der Ausfuehrung: Plan-Wording sagte "Aufruf-Zeilen ~Z. 519". Das war eine Schaetzung; der echte Aufruf liegt bei Z. 560 nach den vorherigen Edits, die zusaetzliche Zeilen einfuegen. Die Strukturzeilen sind eindeutig und die Aenderung ist semantisch identisch zur Plan-Spec.

**Cargo.toml `Location`-Feature ergaenzt (war im Plan als "verify before" markiert):**
Das Plan-Wording sagte "verifiziere via grep ... `Window` und `Location`". Verifikation ergab: `Location` war nicht aktiv. Edit zur Cargo.toml hinzugefuegt (Plan-konform — Plan sagte: "Fallback wenn `Window/Location` nicht aktiv: zu Cargo.toml ergaenzen").

## Known Stubs

**None.**

Alle Edits sind voll funktionsfaehig:
- `parse_mail_query` parst real und ist via cargo-test getestet
- `TemplateSelector::on_select_id` ist kein No-op — beim onchange-Event wird der Callback wirklich gerufen
- `selected_template_id` Signal wird beim Template-Wechsel wirklich beschrieben und beim Senden wirklich gelesen
- `repayment_phase_id` Signal wird bei Mount-mit-Query-Params wirklich gesetzt und beim Senden wirklich uebermittelt

Plan 12-13 wird die Komplettkette schliessen, indem die `RepaymentEntryList`-Bulk-Mail-Aktion den Redirect zu `/mail?from=repayment&phase_id=...&members=...` triggert.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: query-param-trust | genossi-frontend/src/page/mail_page.rs | `parse_mail_query` akzeptiert beliebige Strings aus `window.location().search()`. Sicherheits-Surface ist minimal (alle Werte werden Uuid-validiert), aber der Trust-Boundary ist Plan 12-13's Verantwortung: nur intern-konstruierte Redirect-URLs duerfen die Page mit Pre-Selection aufrufen. Backend-Validation (Phase 10) ist Backstop: `repayment_phase_id` muss eine echte Phase referenzieren, sonst fehlt der Template-Resolver-Kontext. |

## Self-Check: PASSED

Verified all claims against the actual repo:
- ✓ `genossi-frontend/src/page/mail_page.rs` enthaelt `parse_mail_query` als `pub fn` + `ParsedMailContext` als `pub struct`
- ✓ `genossi-frontend/src/page/mail_page.rs` `tests`-Modul hat 7 Test-Funktionen (alle gruen)
- ✓ `genossi-frontend/src/component/mail_compose/template_selector.rs` hat `on_select_id: EventHandler<Option<String>>` als Prop mit `#[props(default)]`
- ✓ `genossi-frontend/src/component/mail_compose/template_selector.rs` ruft `on_select_id.call(None)` bei Reset und `on_select_id.call(Some(tpl.id.clone()))` bei Template-Wahl
- ✓ `genossi-frontend/src/page/mail_page.rs` hat die zwei neuen Signals (`repayment_phase_id`, `selected_template_id`)
- ✓ `genossi-frontend/src/page/mail_page.rs` hat den neuen `use_effect` mit `web_sys::window().location().search()` + `parse_mail_query`
- ✓ `genossi-frontend/src/page/mail_page.rs::TemplateVarButtons`-Aufruf hat `show_repayment_vars: repayment_phase_id.read().is_some()`
- ✓ `genossi-frontend/src/page/mail_page.rs::TemplateSelector`-Aufruf hat `on_select_id`-Callback
- ✓ `genossi-frontend/src/page/mail_page.rs::api::send_bulk_mail`-Aufruf nutzt `template_id` (aus selected_template_id) und `phase_id` (aus repayment_phase_id), KEIN hardcoded `None`
- ✓ `genossi-frontend/Cargo.toml` hat `Location` in web-sys features
- ✓ `rg "template_id: None" genossi-frontend/src/page/mail_page.rs` → 0 Treffer (Issue #2 BLOCKER-Fix verifiziert)
- ✓ Commits exist: bf73573, ce82a8b, 6436242, a1f99c5
- ✓ `cargo build -p genossi-frontend` (via `cd genossi-frontend && cargo build`) exits 0
- ✓ `cargo test --bin genossi-frontend` exits 0 mit 188 passing tests (181 + 7 new)
- ✓ D-01 button-gate auf mail_page.rs: 0 untyped buttons
