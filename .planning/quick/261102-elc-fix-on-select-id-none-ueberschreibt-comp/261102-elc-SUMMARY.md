---
id: 261102-elc
type: quick
status: complete
date: 2026-11-02
description: "Fix: on_select_id(None) ueberschrieb die gerade gesetzte ComposeSource — Textfeld blieb beim Waehlen einer versendeten Mail leer"
subsystem: frontend-mail-compose
tags: [frontend, dioxus, mail, compose, bugfix, regression-test]
requires:
  - 261102-di4 (ComposeSource, SentMailSelection, allow_sent_mails/on_select_sent_job)
provides:
  - SelectorCallback (Callback-Folge einer Selector-Auswahl als Daten)
  - selector_callbacks (reine Entscheidungslogik des onchange-Handlers)
  - ComposeSource::apply (Quellen-Transition, produktiv und im Test dieselbe Funktion)
affects:
  - genossi-frontend/src/page/mail_page.rs
tech-stack:
  added: []
  patterns:
    - "Callback-Folge als Daten (Vec<SelectorCallback>) statt Kontrollfluss im Event-Handler — macht die Reihenfolge testbar"
    - "Transition-Funktion (ComposeSource::apply) wird von Produktion UND Test genutzt, statt das Verhalten im Test zu modellieren"
key-files:
  created: []
  modified:
    - genossi-frontend/src/component/mail_compose/template_selector.rs
    - genossi-frontend/src/component/mail_compose/mod.rs
    - genossi-frontend/src/page/mail_page.rs
decisions:
  - "Der on_select_id(None) im Job-Zweig entfaellt vollstaendig: ComposeSource::SentJob liefert per Konstruktion template_id() == None, der Aufruf war nicht nur schaedlich, sondern ueberfluessig."
  - "Die Entscheidungslogik des onchange-Handlers wandert in die reine selector_callbacks; der Handler ist nur noch ihr Adapter. Damit ist die Callback-REIHENFOLGE Gegenstand von Tests, nicht nur das Enum."
  - "ComposeSource::apply statt Test-seitigem Wiring-Modell: mail_page.rs fuehrt seine beiden Handler ueber dieselbe Funktion, die der Test faltet — Modell und Produktion koennen nicht auseinanderlaufen."
  - "SelectorCallback-Varianten heissen Body/Id/SentJob (nicht SelectBody/...), weil clippy::enum_variant_names das gemeinsame Select-Praefix anmahnt."
metrics:
  duration: ~25min
  tasks: 1
  files: 3
---

# Quick Task 261102-elc: `on_select_id(None)` ueberschrieb die gerade gesetzte `ComposeSource`

Die Job-Auswahl im Mail-Compose-Selector feuerte zwei Callbacks, von denen der
zweite die Wirkung des ersten sofort wieder loeschte — das Textfeld blieb leer,
obwohl Betreff und Text korrekt im Signal standen.

## Problem

`template_selector.rs` rief im Job-Zweig des `onchange`-Handlers:

```rust
on_select_sent_job.call(sel);
on_select_id.call(None);        // <-- der Fehler
```

In `mail_page.rs` setzt der erste Callback `compose_source =
ComposeSource::SentJob(job_id)`, der zweite setzt es auf `ComposeSource::None`.
Ergebnis: `editor_key() == "__no_source__"` — identisch zum Ausgangszustand.
Der `WysiwygEditor` seedet `innerHTML` ausschliesslich beim Mount und braucht
einen Key-Wechsel, um zu remounten; ohne Wechsel blieb er stehen und zeigte
leeren Text. `subject`, `body` und `body_html` waren korrekt gesetzt und
erreichten nur nie das DOM.

Das ist genau die Falle, die Quick 261102-di4 unter D-02 verhindern wollte: die
`ComposeSource`-Enum war richtig, die Verdrahtung hob sie auf.

## Fix

**1. Der schaedliche Aufruf entfaellt.** Er war zusaetzlich ueberfluessig:
`ComposeSource::SentJob` liefert per Konstruktion `template_id() == None`, der
Zweck des Resets (keine alte Vorlagen-Id als Versand-Parameter stehen lassen)
ist durch `compose_source.set(SentJob(..))` schon erfuellt.

**2. Die Entscheidungslogik ist jetzt rein.** Neue, Dioxus-freie Funktion

```rust
pub fn selector_callbacks(
    sent_mode: bool,
    value: &str,
    jobs: &[MailJobTO],
    templates: &[MailTemplateTO],
) -> Vec<SelectorCallback>
```

liefert fuer eine Auswahl die vollstaendige, geordnete Folge der zu feuernden
Callbacks (`SelectorCallback::{Body, Id, SentJob}`, je eine Variante pro Prop).
Der `onchange`-Handler ist nur noch ihr Adapter: Folge berechnen, dann abspielen.
Nebeneffekt: die Read-Guards auf `jobs`/`templates` sind geschlossen, bevor der
erste Callback laeuft — vorher war das eine handgefuehrte Vorsichtsmassnahme im
Handler.

**3. Die Quellen-Transition ist eine Funktion, die Produktion und Test teilen.**
`ComposeSource::apply(&self, &SelectorCallback) -> ComposeSource` bildet einen
Callback auf die resultierende Quelle ab. `mail_page.rs` fuehrt **beide**
Handler (`on_select_id`, `on_select_sent_job`) darueber, statt wie vorher ein
`match` inline zu haben. Der Test faltet dieselbe Funktion ueber ganze Folgen.
Damit kann das, was der Test prueft, nicht still von dem abweichen, was im
Browser laeuft — der eigentliche Grund, warum der Bug durchkam.

**Verifiziert statt angenommen:** der Vorlagen-Zweig hatte das Problem **nicht**.
Dort laeuft `on_select` (setzt Body/Body-HTML, faesst `compose_source` nicht an)
vor `on_select_id(Some(id))` (setzt die Quelle). Die Reihenfolge ist unkritisch;
`select_body_leaves_the_source_untouched` pinnt diese Eigenschaft jetzt fest.
Der `None`-Zweig (Platzhalter-Option / unbekannte Id) behaelt sein Reset — dort
ist `ComposeSource::None` korrekt.

## Tests

Die Luecke war real: `editor_key_differs_between_two_sent_jobs` prueft die Enum
isoliert und blieb bei diesem Bug **gruen**. Acht neue Tests setzen eine Ebene
hoeher an, auf der Callback-Folge und der daraus resultierenden Quelle:

| Test | Was er festnagelt |
|---|---|
| `selecting_sent_mail_fires_only_the_job_callback` | **Der Regressionstest.** Eine Job-Auswahl ergibt genau `[SentJob(..)]` — kein nachgelagertes `Id(None)`. |
| `selecting_sent_mail_results_in_sent_job_source` | Dieselbe Eigenschaft als Wirkung: Quelle ist `SentJob(id)`, `editor_key() != "__no_source__"`, `template_id().is_none()`. |
| `two_sent_mails_in_a_row_yield_different_editor_keys` | Zwei Mails hintereinander ueber den Handler-Pfad (die zweite startet von der Quelle, die die erste hinterliess). |
| `selecting_template_still_sets_template_source_and_id` | Vorlagen-Pfad unveraendert: `[Body, Id(Some)]`, Quelle `Template(id)`, `template_id() == Some(id)`. |
| `template_after_sent_mail_overwrites_the_job_source` | Gegenrichtung: nach einer Job-Auswahl dreht eine Vorlagen-Auswahl die Quelle wieder zurueck. |
| `placeholder_option_resets_the_source_in_both_modes` | Platzhalter ergibt in beiden Modi `ComposeSource::None`. |
| `unknown_ids_keep_the_previous_behaviour` | Unbekannte Job-Id verhaelt sich wie der Platzhalter; unbekannte Vorlagen-Id aendert wie bisher nichts. |
| `select_body_leaves_the_source_untouched` | `Body` faesst die Quelle nicht an — macht die Reihenfolge im Vorlagen-Pfad unkritisch. |

### Rot-Nachweis (durchgefuehrt, nicht behauptet)

Der Fehler wurde temporaer wieder eingebaut (`SelectorCallback::Id(None)` hinter
`SentJob` in `selector_callbacks`) und die Suite gelaufen:

```
test result: FAILED. 361 passed; 3 failed
  selecting_sent_mail_fires_only_the_job_callback   FAILED
  selecting_sent_mail_results_in_sent_job_source    FAILED
  two_sent_mails_in_a_row_yield_different_editor_keys FAILED
```

Fehlermeldung des Regressionstests:

```
left:  [SentJob(SentMailSelection { job_id: "job-a", ... }), Id(None)]
right: [SentJob(SentMailSelection { job_id: "job-a", ... })]
```

Und — der Beleg fuer die Testluecke — `editor_key_differs_between_two_sent_jobs`
blieb bei wieder eingebautem Fehler **gruen**. Danach wurde der Fix
zurueckgebaut und die Suite erneut gelaufen: `364 passed; 0 failed`. Der
Rot-Nachweis wurde nach der clippy-getriebenen Varianten-Umbenennung ein zweites
Mal durchgefuehrt, damit er die final committete Fassung belegt.

## Gates

| Gate | Ergebnis |
|---|---|
| `cargo test --manifest-path genossi-frontend/Cargo.toml` (voll) | **364 passed, 0 failed** (+ 1 Integrationstest) — vorher 356, also +8 |
| `cargo clippy --manifest-path genossi-frontend/Cargo.toml --all-targets` | 0 errors; 170 Warnungen = Baseline, **keine** in den drei geaenderten Dateien |
| `rustfmt --edition 2021` (gezielt, drei Dateien) | clean, kein workspace-weites `cargo fmt` |
| `reply_form.rs` / `application_compose.rs` | 0 Zeilen Diff |
| Nicht-Frontend-Diff | leer — kein Backend-Crate angefasst |
| `genossi-frontend/Cargo.lock` | vorbestehend modifiziert, **nicht** gestaged |

Zwischenschritt: die erste Fassung erzeugte zwei neue clippy-Warnungen
(`enum_variant_names` wegen des gemeinsamen `Select`-Praefix, und `dead_code`
fuer die Falt-Hilfe `compose_source_after`, die nur Tests brauchen). Beide
behoben: Varianten heissen `Body`/`Id`/`SentJob`, die Falt-Hilfe lebt im
Testmodul. Sie faltet ausschliesslich die produktive `apply` und enthaelt keine
eigene Logik — es wird also nichts simuliert, was produktiv anders laeuft.

## Abweichungen vom Auftrag

Keine inhaltlichen. Der Auftrag liess offen, ob ein `SelectionOutcome`-Enum oder
ein einfacherer Weg genommen wird; gewaehlt wurde die Callback-**Folge** als
`Vec<SelectorCallback>` statt eines Outcome-Enums mit einer Variante pro Fall.
Begruendung: ein Outcome-Enum macht „zwei Callbacks, von denen der zweite den
ersten aufhebt" strukturell unmoeglich, aber damit auch untestbar — der Test
koennte nicht mehr zeigen, dass die Folge richtig ist, nur dass das Enum
existiert. Als Folge ist die Reihenfolge ein Wert, den `assert_eq!` direkt
vergleicht, und der Rot-Nachweis oben ist genau dieser Vergleich. Zusaetzlich
(ueber den Auftrag hinaus, zwei Zeilen in `mail_page.rs`) fuehrt die Seite ihre
Transitionen ueber `ComposeSource::apply`, damit das Getestete und das Laufende
dieselbe Funktion sind.

## Manuelle Abnahme (offen — nur im Browser sichtbar)

Backend `:3000` und `dx serve` liefen waehrend der Arbeit; `dx serve` hat den
Fix automatisch neu gebaut. Zu pruefen auf `/mail`:

1. Mitglieder waehlen → auf „Versendete Mail" umschalten → Mail A waehlen:
   Betreff und Text stehen jetzt im Editor (vorher: leer).
2. Direkt danach Mail B waehlen → der Editor zeigt Bs Text.
3. Zurueck auf „Vorlage", eine Vorlage waehlen → unveraendertes Verhalten,
   `template_id` wird beim Versand wieder mitgesendet.
4. `/inbox` Antwort-Formular und Antragsteller-Compose → unveraendert.

## Self-Check: PASSED

- `genossi-frontend/src/component/mail_compose/template_selector.rs` — FOUND
- `genossi-frontend/src/component/mail_compose/mod.rs` — FOUND
- `genossi-frontend/src/page/mail_page.rs` — FOUND
- Commit `4405c7c` — FOUND in `git log`
- Keine Datei-Loeschungen im Commit, keine neuen untracked Dateien
