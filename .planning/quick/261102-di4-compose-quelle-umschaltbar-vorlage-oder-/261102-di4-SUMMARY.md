---
id: 261102-di4
type: quick
status: complete
date: 2026-11-02
description: "Compose-Quelle umschaltbar: Vorlage oder versendete Mail als Textquelle"
subsystem: frontend-mail-compose
tags: [frontend, dioxus, mail, compose, i18n, component-first]
requires:
  - genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs (plain_to_html)
  - GET /api/mail/jobs (bestehend, unveraendert)
provides:
  - ComposeSource (Textquellen-Enum mit editor_key/template_id)
  - SentMailSelection (Callback-Nutzlast der Job-Quelle)
  - TemplateSelector-Props allow_sent_mails / on_select_sent_job
affects:
  - genossi-frontend/src/page/mail_page.rs
tech-stack:
  added: []
  patterns:
    - "Reine Helper + Unit-Tests im Kopf der Komponenten-Datei (Repo-Muster mail_jobs_list.rs)"
    - "Opt-in-Props mit #[props(default)] fuer Backward-Compat weiterer Aufrufstellen"
key-files:
  created: []
  modified:
    - genossi-frontend/src/api.rs
    - genossi-frontend/src/component/mail_compose/template_selector.rs
    - genossi-frontend/src/component/mail_compose/mod.rs
    - genossi-frontend/src/page/mail_page.rs
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
decisions:
  - "D-02 strukturell gelöst: ComposeSource ersetzt das doppelrollige selected_template_id-Signal; editor_key() bildet Vorlagen- UND Job-Quelle ab, template_id() liefert nur bei Template Some."
  - "D-04: body_html des Jobs wird direkt übernommen (serverseitig sanitisiert), Fallback plain_to_html."
  - "D-08: Job-Pfad hat eigenen Callback und hängt keinen Footer an."
  - "D-03: allow_sent_mails ist opt-in; bei false kein get_mail_jobs-Request."
metrics:
  duration: ~20min
  tasks: 3
  files: 7
  tests_added: 15
  completed: 2026-11-02
---

# Quick Task 261102-di4: Compose-Quelle umschaltbar — Vorlage oder versendete Mail

Im bestehenden Vorlagen-Selector der Mail-Seite kann der Vorstand jetzt zwischen
„Vorlage" und „Versendete Mail" als Textquelle umschalten und Betreff + Text
einer frueher versendeten Mail formatierungstreu uebernehmen — ohne zweiten
Knopf daneben und ohne Doppel-Footer, weil die alte Doppelrolle des
`selected_template_id`-Signals durch ein `ComposeSource`-Enum aufgeloest wurde.

## Was umgesetzt wurde

### Task 1 — `body_html` im Frontend-Spiegel von `MailJobTO` (Commit `319214e`)

- `genossi-frontend/src/api.rs`: `pub body_html: Option<String>` mit
  `#[serde(default, skip_serializing_if = "Option::is_none")]`, identisch
  attributiert wie `repayment_phase_id` und das Backend-Gegenstueck.
- 2 Serde-Tests (`test_mail_job_to_deserializes_without_body_html`,
  `..._with_body_html`) — Rueckwaerts-Kompatibilitaet zu Backends ohne das Feld
  ist abgedeckt.
- Kein Backend-Crate angefasst (`git diff --name-only | grep -v '^genossi-frontend/'`
  leer ueber alle drei Commits).

### Task 2 — Reine Helper-Schicht (Commit `e13f881`)

In `component/mail_compose/template_selector.rs`, oberhalb der
`#[component]`-Funktion (Repo-Muster aus `mail_jobs_list.rs`):

- `ComposeSource { None, Template(String), SentJob(String) }` mit
  `editor_key()` (`__no_source__` / `tpl:{id}` / `job:{id}`) und
  `template_id()` (nur `Template` → `Some`).
- `SentMailSelection { job_id, subject, body, body_html }`.
- `SENT_MAIL_SOURCE_LIMIT = 20`, `COMPOSE_SOURCE_NONE_KEY`.
- `sent_mail_sources()` — filtert `sent_count > 0` und nicht-leeren Body,
  sortiert selbst absteigend nach `created`, schneidet auf `limit`.
- `job_body_html()` — bevorzugt `body_html`, Fallback `plain_to_html(body)`.
- `sent_mail_option_label()` — locale-frei, Aufrufer liefert das formatierte
  Datum.
- 13 Unit-Tests, darunter die beiden Pins
  `editor_key_differs_between_two_sent_jobs` und
  `template_id_is_none_for_sent_job_and_none_source` (unverändert in Stärke,
  wie im Plan gefordert).
- `mod.rs`: additiver Re-Export `{ComposeSource, SentMailSelection, TemplateSelector}`.

### Task 3 — Umschaltung verdrahtet + `mail_page` umgestellt (Commit `c951869`)

- **i18n:** 4 neue `Key`-Varianten (`MailSourceLabel`, `MailSourceTemplate`,
  `MailSourceSentMail`, `MailSourceSentMailSelect`) in `mod.rs` und je ein Arm
  in `de.rs` + `en.rs` (exhaustive Matches erzwingen beides).
- **`TemplateSelector`:** zwei neue `#[props(default)]`-Props
  (`allow_sent_mails: bool`, `on_select_sent_job: EventHandler<SentMailSelection>`),
  Signale `jobs` + `sent_mode`, zweiter `use_effect`, der bei
  `allow_sent_mails == false` **sofort returnt** (kein `get_mail_jobs`-Request).
  Im RSX nur bei `allow_sent_mails` eine Zwei-Knopf-Umschaltung
  (`r#type: "button"` + `onclick`, nie `form`/`onsubmit`), sonst unverändert das
  bisherige `"Vorlage"`-Label. Das `<select>` hat zwei Betriebsarten; der
  Vorlagen-Zweig ist wörtlich der alte Code. Der Job-Zweig ruft
  `on_select_sent_job` **plus** `on_select_id.call(None)` und bewusst **nicht**
  `on_select` (D-08).
- **`mail_page.rs`:** `selected_template_id` → `compose_source`
  (`ComposeSource`); `editor_key` aus `compose_source.read().editor_key()`;
  `template_id_owned` aus `.template_id()`; neuer `on_select_sent_job`-Handler
  setzt `subject`/`body`/`body_html` (letzteres direkt aus `sel.body_html`) und
  `ComposeSource::SentJob(job_id)`, ohne Footer; nach erfolgreichem Versand
  zusätzlich `compose_source.set(ComposeSource::None)`.
- Die Umschaltung lebt vollständig in der Komponente — `mail_page.rs` enthält
  kein Selector-RSX (Component-First).

## Verification

Alle Gates aus dem Plan, in der Nix-Devshell ausgeführt:

| Gate | Ergebnis |
|---|---|
| `cargo test --manifest-path genossi-frontend/Cargo.toml` (volle Suite) | 356 passed, 0 failed + 1 passed (`no_form_submit_regression`) |
| `cargo clippy --manifest-path genossi-frontend/Cargo.toml --all-targets` | keine Fehler, **keine** neue Warnung in `template_selector.rs`; die verbleibenden Treffer in `mail_page.rs` (`redundant closure`, `mut` an `repayment_phase_id`) sind vorbestehend |
| `rustfmt --edition 2021 --check` (die 7 Dateien) | clean |
| `git diff --name-only \| grep -c 'inbox/reply_form.rs\|application_compose.rs'` | `0` |
| `grep -c 'compose_source' page/mail_page.rs` | `6` (≥ 5 gefordert) |
| `grep -v '^\s*//' page/mail_page.rs \| grep -c selected_template_id` | `0` |
| `git diff --name-only \| grep -v '^genossi-frontend/' \| grep -v '^\.planning/'` | leer (kein Backend-Crate berührt) |

Index-Hygiene (jj+git-Desync-Falle): vor jedem `git add` wurde
`git read-tree HEAD` ausgeführt und danach `git diff --cached --name-only`
geprüft — pro Commit waren exakt die erwarteten Dateien gestaged (1 / 2 / 5).
Kein Commit enthält Löschungen (`git diff --diff-filter=D HEAD~1 HEAD` je leer).

## Deviations from Plan

Keine funktionalen Abweichungen. Drei kleine, bewusst gewählte
Umsetzungsdetails innerhalb des Plan-Rahmens:

1. **`SentMailSelection` wird vor dem Callback aus dem Signal geklont**
   (`jobs.read().iter().find(..).map(..)` → `match`), statt den Callback
   innerhalb des `if let Some(job) = jobs.read()...`-Borrows zu rufen. Grund:
   kein offener Read-Guard auf `jobs`, während der Aufrufer Signale schreibt.
   Verhalten identisch zur Plan-Beschreibung.
2. **Button-Klassen werden vor dem `rsx!` berechnet** (`tpl_btn_class` /
   `sent_btn_class` aus drei `const`-Fragmenten) statt als Inline-`if` im
   Attribut — robuster gegenüber der RSX-Attribut-Syntax, gleiche
   Tailwind-Klassen wie im Plan (`bg-blue-600 text-white` aktiv,
   `bg-white text-gray-700 border` inaktiv).
3. **`in_sent_mode` ist `allow_sent_mails && *sent_mode.read()`** — der
   Job-Modus ist damit auch bei einem hypothetisch gesetzten `sent_mode` nicht
   erreichbar, wenn die Quelle nicht erlaubt ist (zusätzliche Absicherung von
   D-03, keine Verhaltensänderung für die bestehenden Aufrufstellen).

## Out of scope / Deferred

- **`genossi-frontend/Cargo.lock` ist nach dem ersten `cargo test` dirty**
  (`version = "2026.251.1-dev"` → `"2026.251.2-dev"`). Das ist ein Nachlauf des
  Release-Commits `add29c5` („Set version to 2026.251.2-dev"), der
  `Cargo.toml` bumpt, ohne den Frontend-Lockfile neu zu generieren — nicht Teil
  dieses Quicks und deshalb **nicht gestaged**. Die Datei bleibt im Arbeitsbaum
  modifiziert; der nächste Release-Lauf oder ein Einzeiler-Commit kann sie
  nachziehen.
- Unversionierte Alt-Pfade (`.worktrees/`) wurden nicht angefasst.
- Plangemäß nicht umgesetzt: Anhänge übernehmen (D-05), Suche in der Job-Liste
  (D-07), `rendered_body` als Quelle (D-01 — bewusst verworfen).

## Known Stubs

Keine. Alle UI-Pfade sind an echte Datenquellen verdrahtet
(`api::get_mail_jobs` → `sent_mail_sources` → `<option>` → `on_select_sent_job`).

## Offene manuelle Abnahme (Browser)

Die 7 UAT-Schritte aus dem Plan sind automatisiert nicht abdeckbar (Dioxus-WASM,
Editor-Remount, Server-Render der Platzhalter). Besonders relevant:

4. **Trap-Check D-02:** Job A wählen, direkt danach Job B → der Editor muss Bs
   Text zeigen. Der Unit-Test `editor_key_differs_between_two_sent_jobs` pinnt
   die notwendige Bedingung (verschiedene Keys), der tatsächliche Remount ist
   nur im Browser sichtbar.
6. Versand mit Job-Quelle an ein Testmitglied → Platzhalter müssen für *dieses*
   Mitglied auflösen (D-01).
7. `/inbox`-Antwortformular und Antragsteller-Compose zeigen **keine**
   Umschaltung und laden keine Job-Liste (D-03).

## Self-Check: PASSED

- Alle 7 geänderten Dateien existieren auf Platte.
- Alle drei Task-Commits sind in `git log` vorhanden: `319214e`, `e13f881`,
  `c951869`.
- Volle Frontend-Suite nach dem letzten Commit erneut grün (357 Tests).
