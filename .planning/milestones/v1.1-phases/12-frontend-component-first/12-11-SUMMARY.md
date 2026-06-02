---
phase: 12
plan: 11
subsystem: frontend
tags: [frontend, component, mail, template, variables]
wave: 8
requires: [12-01]
provides:
  - "TemplateVarButtons hat neuen Optional-Prop show_repayment_vars: bool (default=false)"
  - "REPAYMENT_VARS Konstante mit payout_amount/share_count/fiscal_year"
  - "Conditional Render-Block fuer drei orange-getoente Repayment-Variable-Buttons zwischen PRIMARY_VARS und Show-More-Toggle"
affects:
  - genossi-frontend/src/component/mail_compose/template_var_buttons.rs
tech-stack:
  added: []
  patterns:
    - "Optional-Prop via #[props(default)] fuer backward-compatible Erweiterung bestehender Components (Plan-PATTERNS §11)"
    - "i18n-Key-Lookup via match auf statischen Var-Namen (Mirror des PRIMARY_VARS-Loop-Patterns, aber mit Key statt label-tuple)"
    - "Visuelles Differential via Tailwind-Farbpalette (orange) statt blau/grau zur Kontext-Kennzeichnung"
key-files:
  created: []
  modified:
    - genossi-frontend/src/component/mail_compose/template_var_buttons.rs
decisions:
  - "Optional-Prop mit #[props(default)] gewaehlt statt eigene neue Component-Variante — 3 bestehende Callers (mail_page.rs, mail_templates.rs, inbox/reply_form.rs) bleiben funktional ohne Aenderung; Plan 12-12 wird mail_page.rs additiv um show_repayment_vars=repayment_phase_id.is_some() erweitern"
  - "Render-Position: zwischen PRIMARY_VARS-Loop und Show-More-Block (sichtbar ohne Mehr-Klick) — wichtig fuer Mail-Compose-Workflow im Repayment-Kontext, weil Vorstand die Vars direkt sieht"
  - "Visuelles Differential bg-orange-100/text-orange-800 (statt bg-blue-100/bg-gray-100) damit User den Kontext erkennt"
  - "i18n-Key-Lookup via match auf statischen Var-Namen statt parallel-iter mit Key-Array — kompakter und korrespondiert direkt zu den drei Plan-12-01-Keys (RepaymentTemplateVarPayoutAmount/ShareCount/FiscalYear)"
  - "_label_de in der for-Schleife unused weil i18n.t() das Label liefert; der const-Eintrag behaelt den deutschen String als Inline-Doku (analog PRIMARY_VARS/SECONDARY_VARS-Stil)"
metrics:
  duration: ~2min
  completed: "2026-06-01T13:04:32Z"
  task-count: 1
  file-count: 1
  test-count-added: 0
  test-count-total: 181
  commits:
    - {sha: 291e8c5, type: feat, task: 1, scope: "template_var_buttons.rs"}
---

# Phase 12 Plan 11: TemplateVarButtons-Erweiterung um Repayment-Vars (D-19) Summary

**One-liner:** TemplateVarButtons-Component erhaelt einen Optional-Prop `show_repayment_vars` (default=false) — bei true werden drei orange-getoente Buttons fuer `{{ payout_amount }}` / `{{ share_count }}` / `{{ fiscal_year }}` zwischen PRIMARY_VARS und Show-More-Toggle gerendert; alle drei bestehenden Callers bleiben backward-kompatibel und werden nicht angepasst.

## What Was Built

Ein Task. Plan-12-11 ist eine Single-File-Erweiterung von `genossi-frontend/src/component/mail_compose/template_var_buttons.rs` (39 Zeilen hinzugefuegt, 1 Zeile geaendert).

### Task 1: TemplateVarButtons mit show_repayment_vars Prop erweitern (commit 291e8c5)

**Schritt 1 — Neue REPAYMENT_VARS Konstante** (nach SECONDARY_VARS):
```rust
/// Phase-12 (D-19) Repayment-spezifische Variablen. Werden nur bei
/// `show_repayment_vars=true` gerendert. Aufgeloest backend-seitig in
/// Phase 10 D-05/D-13 (minijinja Strict-Mode mit `{% if X is defined %}`).
const REPAYMENT_VARS: &[(&str, &str)] = &[
    ("payout_amount", "Auszahlbetrag"),
    ("share_count", "Anteile"),
    ("fiscal_year", "Geschaeftsjahr"),
];
```

**Schritt 2 — Signatur-Erweiterung:**
```rust
#[component]
pub fn TemplateVarButtons(
    on_insert: EventHandler<String>,
    #[props(default)] show_repayment_vars: bool,
) -> Element {
```

`#[props(default)] show_repayment_vars: bool` ist die Standard-Dioxus-API fuer ein optional-mit-default-false-Prop. Alle bestehenden Callers, die das Prop nicht setzen, bekommen automatisch `false`. Verifiziert: cargo build kompiliert ohne Aenderungen in mail_page.rs:375-379, mail_templates.rs:237 und inbox/reply_form.rs:56.

**Schritt 3 — Conditional Render-Block** (eingefuegt direkt nach PRIMARY_VARS-Loop und vor `if *show_more.read()`-Block):
```rust
// Phase-12 D-19: Repayment-Vars nur wenn show_repayment_vars=true
if show_repayment_vars {
    for (var_name, _label_de) in REPAYMENT_VARS.iter() {
        {
            let vn = var_name.to_string();
            let i18n_key = match *var_name {
                "payout_amount" => Key::RepaymentTemplateVarPayoutAmount,
                "share_count" => Key::RepaymentTemplateVarShareCount,
                "fiscal_year" => Key::RepaymentTemplateVarFiscalYear,
                _ => Key::RepaymentTemplateVarPayoutAmount, // unreachable defensive
            };
            let label = i18n.t(i18n_key).to_string();
            rsx! {
                button {
                    class: "bg-orange-100 hover:bg-orange-200 text-orange-800 px-2 py-1 rounded text-xs font-mono",
                    r#type: "button",
                    title: "{var_name}",
                    onclick: move |_| {
                        on_insert.call(format!("{{{{ {} }}}}", vn));
                    },
                    "{label}"
                }
            }
        }
    }
}
```

### Edit-Position

- **Konkret:** Zwischen dem schliessenden `}` des `for (var_name, label) in PRIMARY_VARS.iter() { ... }`-Loops und dem `if *show_more.read() {` Block.
- **User-Wahrnehmung:** Im Mail-Compose-Modus sieht der User PRIMARY_VARS (blau), dann REPAYMENT_VARS (orange, nur bei `show_repayment_vars=true`), dann optional SECONDARY_VARS (grau, nach `Mehr`-Klick).
- **Rationale:** Repayment-Vars sind im Repayment-Workflow kontext-zentral; sie zwischen Sekundaer-Vars zu verstecken (hinter `Mehr`) wuerde sie verbergen. Direkt sichtbar zu rendern macht den Klick-Flow `Mail an N ausgewaehlte -> Variable einfuegen -> Senden` einen Schritt kuerzer.

### Visuelles Differential

| Var-Block | Hintergrund | Text | Hover | Kontext |
|-----------|-------------|------|-------|---------|
| PRIMARY_VARS | bg-blue-100 | text-blue-800 | bg-blue-200 | Standard-Mitglieder-Vars (first_name, last_name, member_number, ...) |
| **REPAYMENT_VARS (D-19)** | **bg-orange-100** | **text-orange-800** | **bg-orange-200** | **Repayment-spezifisch — Backend loest pro Empfaenger auf (Phase 10 D-05/D-13)** |
| SECONDARY_VARS | bg-gray-100 | text-gray-700 | bg-gray-200 | Optional (Adresse, Bankverbindung, ...) — hinter "Mehr"-Toggle |

Der Orange-Ton ist farbpsychologisch in Tailwind-UI eine Warn-/Akzent-Farbe und signalisiert "kontext-getrieben, nicht universell verwendbar". Bei normalem Mitglieder-Mailing (`show_repayment_vars=false`) verschwindet der gesamte Block — User sieht ein konsistentes blau/grau-Interface ohne Verwirrung.

### Backward-Compat via `#[props(default)]` verteidigt

Alle drei bestehenden Aufrufer wurden NICHT angepasst und kompilieren weiterhin:
- `genossi-frontend/src/page/mail_page.rs:375-379` — Plan 12-12 wird hier `show_repayment_vars: repayment_phase_id.read().is_some()` ergaenzen
- `genossi-frontend/src/page/mail_templates.rs:237` — Bleibt unveraendert (Template-Editor, kein Repayment-Kontext)
- `genossi-frontend/src/component/inbox/reply_form.rs:56` — Bleibt unveraendert (Inbox-Reply, kein Repayment-Kontext)

Verifiziert per `cargo build -p genossi-frontend` exit 0 und 181 Tests gruen — kein Caller bricht.

## How It Was Verified

```bash
# Done-Criteria
$ cd genossi-frontend && cargo build
warning: `genossi-frontend` (bin "genossi-frontend") generated 23 warnings ...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.75s
$ echo "exit=$?"
exit=0

$ cd genossi-frontend && cargo test
test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ rg "REPAYMENT_VARS" genossi-frontend/src/component/mail_compose/template_var_buttons.rs | wc -l
2   # Definition + Verwendung

$ rg "show_repayment_vars" genossi-frontend/src/component/mail_compose/template_var_buttons.rs | wc -l
4   # Doc-Kommentar + Prop + Branch-Kommentar + if-Branch

$ rg "#\[props\(default\)\] show_repayment_vars: bool" genossi-frontend/src/component/mail_compose/template_var_buttons.rs | wc -l
1

$ rg "Key::RepaymentTemplateVarPayoutAmount|Key::RepaymentTemplateVarShareCount|Key::RepaymentTemplateVarFiscalYear" genossi-frontend/src/component/mail_compose/template_var_buttons.rs | wc -l
4   # 3 Keys + 1 defensive fallback im match-arm

# D-01 Button-Grep-Gate (alle button-Tags haben r#type:)
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' \
    genossi-frontend/src/component/mail_compose/template_var_buttons.rs \
  | grep -v 'r#type:' | grep -c 'button {'
0
```

Build- und Test-Erfolg unveraendert (181 Tests, +0 neue Tests in diesem Plan — pure Strukturerweiterung ohne neue Logik-Helper). Plan 12-12 wird die Verdrahtung und ggf. Integrations-Tests einbringen.

## Decisions Made

**`#[props(default)]` statt `Option<bool>`:**
Dioxus 0.6.3 unterstuetzt `#[props(default)]` direkt fuer primitive Defaults — backward-compatible Optional-Prop ohne `Option`-Wrapping noetig. Pattern-Anker im Codebase: `member_search.rs::MemberSearch` nutzt `#[props(optional)]` fuer `Option<...>`-Felder; hier ist `bool` mit Default-`false` semantisch passender als `Option<bool>`.

**i18n-Key-Lookup via match-auf-Var-Name statt parallele Key-Liste:**
Variante 1 (match): `match *var_name { "payout_amount" => Key::RepaymentTemplateVarPayoutAmount, ... }`. Variante 2 (parallel-list): `const REPAYMENT_VARS: &[(&str, &str, Key)]` mit i18n-Key als drittes Tupel-Element. Variante 1 gewaehlt, weil:
1. `Key`-Enum ist `#[derive(Copy)]` aber non-const — kann nicht in einem `const &[...]` stehen
2. Der match ist explizit-lesbar und der defensive `_ => ...`-Arm dokumentiert die Vollstaendigkeit
3. Korrespondiert visuell zur bestehenden Var-Listen-Konstruktion (`vn = var_name.to_string()`-Pattern uebertragen)

**Render-Position direkt nach PRIMARY_VARS (nicht versteckt hinter Show-More):**
Plan-CONTEXT D-19 sagt "Repayment-Var-Buttons erscheinen NUR bei repayment_phase_id-Kontext" — das wuerde auch "innerhalb des Show-More-Blocks" beduten koennen. Aber: Im Repayment-Workflow ist der Var-Einsatz der zentrale Mehrwert, NICHT optional. Direkt sichtbar zu rendern (zwischen PRIMARY_VARS und Show-More) ist die UX-bessere Wahl. Plan-PATTERNS §11 (Z. 717-737) bestaetigt diese Position explizit ("PLUS Bedingung", was als Insertion zwischen PRIMARY_VARS-Iter und show_more-Branch zu lesen ist).

**Orange-Farbpalette (bg-orange-100/text-orange-800):**
Tailwind hat keine "Default"-Akzentfarbe fuer kontext-getriebene Vars. Blau (PRIMARY_VARS) und Grau (SECONDARY_VARS) sind belegt. Orange ist farbpsychologisch eine Warn-/Akzent-Farbe in Tailwind-UI-Konventionen und signalisiert "spezifischer Kontext". Plan-PLAN gibt explizit `bg-orange-100, text-orange-800` als visuelles Differential vor — uebernommen 1:1.

## Deviations from Plan

**None — plan executed exactly as written.**

Die Plan-Acceptance-Tests wurden alle exakt verifiziert:
- REPAYMENT_VARS Konstante mit 3 Eintraegen (payout_amount/share_count/fiscal_year): correct
- Signatur erweitert um `#[props(default)] show_repayment_vars: bool`: correct
- Conditional-Block zwischen PRIMARY_VARS-Loop und Show-More-Block eingefuegt: correct
- Orange Tailwind-Klassen: correct
- D-01 Button-Grep-Gate: 0 Treffer (alle 3 neuen button-Tags haben r#type:)
- Build + Tests gruen
- Backward-Compat via default-prop: verifiziert durch unveraenderte 3 bestehende Caller

Anmerkung: Die im PLAN.md spezifizierte deutsche Variable-Beschriftung "Geschaeftsjahr" (mit ae-Umlaut-Ersatz im Plan-Wording) wurde in den const-Eintrag uebernommen. Die echte deutsche Uebersetzung ("Geschäftsjahr" mit Umlaut) lebt in `i18n/de.rs:585` und wird via `i18n.t()` zur Render-Zeit aufgeloest — der const-Inline-Label dient nur als Code-Doku (analog `_label_de`-Naming, das den Wert explizit als "nur Doku, nicht verwendet" markiert).

## Known Stubs

**None.**

Plan 12-11 ist eine reine Erweiterung. Die `show_repayment_vars`-Bedingung wird in Plan 12-12 in mail_page.rs verdrahtet (`show_repayment_vars: repayment_phase_id.read().is_some()`). Bis dahin ist der Default `false` korrekt und intentional — kein Stub, sondern eine bewusste compile-able Foundation fuer eine spaetere Verdrahtung.

## Self-Check: PASSED

Verified all claims against the actual repo:
- File exists: `genossi-frontend/src/component/mail_compose/template_var_buttons.rs`
- Commit exists: `291e8c5` (verified via `git log --oneline --all | grep 291e8c5`)
- `REPAYMENT_VARS` constant present with 3 entries
- `show_repayment_vars: bool` prop with `#[props(default)]` annotation
- Three `Key::RepaymentTemplateVar*` references + 1 defensive fallback in match
- D-01 button-grep-gate: 0 buttons without `r#type:` in the file
- `cargo build` (frontend) exits 0
- `cargo test` (frontend) exits 0 with 181 passing tests
- Three existing callers (mail_page.rs:375, mail_templates.rs:237, inbox/reply_form.rs:56) remain unchanged and compile (backward-compat via default-prop)
