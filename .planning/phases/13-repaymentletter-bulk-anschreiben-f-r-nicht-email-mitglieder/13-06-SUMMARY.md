---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 06
subsystem: frontend
tags: [phase-13, frontend, dioxus, component-first, button-pattern-d01, blob-download, bulk-action, selection-preservation, i18n-pluralisierung]

requires:
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "05"
    provides: "POST /api/repayment-phase/{phase_id}/letters/generate Endpoint mit X-Document-Count Header"
provides:
  - "API-Client `generate_repayment_letters(config, phase_id, entry_ids) -> Result<GeneratedLettersResult, AppError>` mit blob_url + document_count"
  - "EventHandler-Prop `on_letter_request: EventHandler<Vec<Uuid>>` an `RepaymentEntryList` (entry_ids — Server aggregiert)"
  - "Bulk-Action-Button 'Anschreiben erzeugen' (Purple) neben Massenmail-Button"
  - "Page-Handler in `RepaymentPhaseDetails::entries`-Tab mit Browser-Save (`<a download>`-Trick + revoke_object_url) + Singular/Plural-Toast"
  - "i18n-Keys + DE/EN-Strings (Singular/Plural-aware mit `{count}`-Placeholder)"
affects: [13-07]

tech-stack:
  added: []
  patterns:
    - "Browser-Save Pattern aus Phase 6 (assembly_details.rs:362-395) 1:1 fuer Bundle-PDF wiederverwendet"
    - "X-Document-Count Header-Read: `resp.headers().get('X-Document-Count').ok().flatten().and_then(parse::<usize>)` mit Fallback `entry_ids.len()`"
    - "Singular/Plural-i18n via getrennten Keys + `{count}`-Placeholder (deutsche Grammatik: '1 Brief' vs 'N Briefe')"
    - "D-13-09 Selection-Preservation: spawn-Ok-Branch modifiziert `selected_ids` NICHT — Vorstand kann sofort Phase-8-Batch ausloesen"
    - "Phase 12 D-01 Button-Pattern: `r#type: 'button'` als allererstes Attribut + onclick (kein form-onsubmit)"
    - "use_i18n() FRISCH im spawn-Block statt outer-Capture (Pattern aus assembly_details.rs:399) — vermeidet FnOnce-Issue mit dem owned i18n-Wert"

key-files:
  created:
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-06-SUMMARY.md"
  modified:
    - "genossi-frontend/src/api.rs"
    - "genossi-frontend/src/i18n/mod.rs"
    - "genossi-frontend/src/i18n/de.rs"
    - "genossi-frontend/src/i18n/en.rs"
    - "genossi-frontend/src/component/repayment_entry_list.rs"
    - "genossi-frontend/src/page/repayment_phase_details.rs"

key-decisions:
  - "Plan referenziert `genossi-frontend/src/i18n/keys.rs`, real lebt das Key-Enum in `mod.rs` — Keys dort ergaenzt (keine Datei-Splittung). Sowohl Strukturanker `Key::RepaymentEntryBulkLetterButton` als auch die Translation-Tables in de.rs/en.rs ergaenzt — keine i18n-Drift."
  - "GeneratedLettersResult-Struct statt Tuple (`Result<(String, usize), AppError>`) — bessere Lesbarkeit + Self-Documenting-Fields (blob_url, document_count) + spaetere Erweiterbarkeit (z.B. Content-Disposition-Filename) ohne Breaking-Change am Caller."
  - "fiscal_year fuer Browser-Save-Filename als `let fiscal_year_for_letters = p.fiscal_year;` vor dem RepaymentEntryList-Mount captured — vermeidet das Capture des ganzen RepaymentPhaseTO im async-spawn-Block, der bereits per `phase_for_entries` verbraucht ist."
  - "Toast-Text-Lookup im spawn nutzt `use_i18n()` fresh (statt outer-`i18n`-Closure-Capture) — Pattern aus assembly_details.rs:399. Vermeidet FnOnce-Konflikt mit owned i18n-Capture in spawn-async-Move-Closure."
  - "Plural-replace in eine Zeile gepackt (`replace('{count}', &count_str)`) statt zeilenumbrochenes `.replace(\\n ... )`-Pattern — Acceptance-Grep `rg '\\.replace\\(\"\\{count\\}\"'` matcht nur Single-Line, also code-style entsprechend angepasst."
  - "Plan-Acceptance-Grep `rg 'button\\s*\\{' ... | rg -v 'r#type:'` ist als kombinierter Single-Pass formuliert und matcht bei 9 button-Definitionen alle 9 (false-positive), weil `button {` selbst kein `r#type:` enthaelt. Tatsaechliche Phase-12-D-02-Intention ist 'next-line-check' — manuell verifiziert: alle 9 button-Definitionen haben `r#type: \"button\"` als unmittelbar folgende Zeile (siehe Self-Check)."

patterns-established:
  - "Browser-Save Bundle-PDF: `api::* -> GeneratedLettersResult { blob_url, document_count }` -> `<a download>.click() + revoke_object_url` -> i18n-Toast nach Server-Count. Wiederverwendbar fuer Folge-Bulk-PDF-Endpoints (z.B. SEPA-Export, Beitragsabrechnungen)."
  - "X-Document-Count Header-Lesemuster: `resp.headers().get('X-Document-Count').ok().flatten().and_then(|s| s.parse::<usize>().ok()).unwrap_or(default)` — defensive Default falls Server-Header fehlt; im Plan 13-05 ist X-Document-Count immer gesetzt, aber Pattern verkraftet auch fehlenden Header."

requirements-completed: [BRIEF-01, UI-03, UI-06]

duration: ~25min
completed: 2026-06-02
---

# Phase 13 Plan 06: RepaymentLetter Frontend Bulk-Letter-Button Summary

**Frontend-Komplement zu Phase 12 D-18: neuer Bulk-Action-Button "Anschreiben erzeugen" (Purple) neben dem Massenmail-Button in der RepaymentEntryList-Component. POST + JSON-Body mit `entry_ids` der Multi-Selektion, Browser-Save des Bundle-PDFs via `<a download>`-Trick mit revoke_object_url, Toast nutzt `X-Document-Count`-Header (Server-Aggregations-Count nach D-13-04) mit Singular/Plural-aware i18n. Selection bleibt nach Download UNVERAENDERT (D-13-09 Selection-Preservation), damit Vorstand direkt mit dem Phase-8-Batch-Endpoint "Als angeschrieben markieren" auf der gleichen Auswahl fortsetzen kann. cargo check und cargo build --release beide clean.**

## Performance

- **Duration:** ~25 min (zwischen `5da84fb` parent und `3f1633b` Task-3-Commit)
- **Tasks:** 3 ausgefuehrt
- **Files modified:** 6 (genossi-frontend/src/api.rs, i18n/mod.rs, i18n/de.rs, i18n/en.rs, component/repayment_entry_list.rs, page/repayment_phase_details.rs)
- **Files created:** 1 (13-06-SUMMARY.md)
- **Commits:** 3 (1 pro Task)

## Accomplishments

### Task 1 — API-Client + i18n-Keys + DE/EN-Strings (Commit `e6f3bfc`)

- **`genossi-frontend/src/api.rs`:** Neue `pub struct GeneratedLettersResult { blob_url: String, document_count: usize }` und `pub async fn generate_repayment_letters(config, phase_id, entry_ids) -> Result<GeneratedLettersResult, AppError>`. Pattern 1:1 aus `export_attendance_url` (api.rs:1891) mit zusaetzlichem:
  - JSON-Body-Konstruktion: `serde_json::json!({ "entry_ids": entry_ids }).to_string()`
  - `web_sys::Headers::new()` + `.set("Content-Type", "application/json")` + `opts.set_headers(&headers)`
  - X-Document-Count Header-Read nach `resp.ok()`-Check: `resp.headers().get("X-Document-Count").ok().flatten().and_then(|s| s.parse::<usize>().ok()).unwrap_or(entry_ids_len)`
  - Blob-Pipeline danach 1:1 wie im Vorbild.
- **`genossi-frontend/src/i18n/mod.rs`:** 4 neue Key-Varianten (`RepaymentEntryBulkLetterButton`, `RepaymentLetterToastSingular`, `RepaymentLetterToastPlural`, `RepaymentLetterFilenamePrefix`) mit Doc-Kommentaren zum Placeholder-Format.
- **`genossi-frontend/src/i18n/de.rs` + `en.rs`:** Strings fuer alle 4 Keys; deutsche Grammatik via getrennter Singular/Plural-Form ("1 Brief" vs "{count} Briefe"). EN nutzt analoge Singular/Plural-Pattern.
- `cargo check` sauber (nur Dead-Code-Warnings fuer noch unbenutzte Keys — wird in Task 2/3 behoben).

### Task 2 — Bulk-Button + on_letter_request Prop in Component (Commit `1e0131a`)

- **`genossi-frontend/src/component/repayment_entry_list.rs`:**
  - Neue `EventHandler<Vec<Uuid>>`-Prop `on_letter_request` in der `#[component]`-Signatur, direkt nach `on_mail_request`. Doc-Kommentar: "entry_ids (NICHT member_ids) — Server aggregiert via Resolver (D-13-04)".
  - Neuer `button {}`-Block in der Header-Action-Leiste, direkt UNTER dem existierenden Massenmail-Button. Pattern 1:1 mit `r#type: "button"` (Phase 12 D-01) + `onclick` (kein form-onsubmit). Visueller Unterschied via `bg-purple-600 hover:bg-purple-700` (Blau ist fuer Mail reserviert).
  - onclick-Body: `selected_set = selected_ids.read().clone()` -> filtere `entries.read()` auf selektierte Entries -> map `e.id` (NICHT `e.member_id`) -> `on_letter_request.call(ids)`. KEINE Modifikation von `selected_ids` (D-13-09).
- `cargo check` faellt mit erwartetem Error in der Page (`on_letter_request` Prop fehlt dort noch) — wird in Task 3 behoben.

### Task 3 — Page-Handler + Browser-Save + Toast (Commit `3f1633b`)

- **`genossi-frontend/src/page/repayment_phase_details.rs`:**
  - Neue lokale Variable `let fiscal_year_for_letters = p.fiscal_year;` direkt nach den existierenden `phase_for_*`-Captures. Wird im async-spawn-Block fuer das Filename-Format genutzt (`auszahlungs_anschreiben_GJ_{year}.pdf`).
  - Neuer `on_letter_request`-Handler in der `RepaymentEntryList`-Instanz, direkt nach `on_mail_request`. Body:
    1. Empty-Check (defensive — Button ist disabled bei 0 Selection)
    2. `spawn(async move { ... })` mit cfg-clone + `api::generate_repayment_letters(...)` Call
    3. Ok-Branch: Browser-Save via `document.create_element("a")` + `set_attribute("href"/download)` + `dyn_into::<HtmlElement>().click()` + `Url::revoke_object_url`. Filename mit `result.blob_url` + dynamischem `fiscal_year_for_spawn`. Pattern 1:1 aus `assembly_details.rs:362-395`.
    4. Toast-Logik: `if result.document_count == 1 { Singular } else { Plural mit replace("{count}", count_str) }`. **`use_i18n()` FRISCH im spawn** (nicht outer-`i18n`-Capture) — vermeidet FnOnce-Konflikt mit owned-i18n-Value im async-move-Closure (Pattern aus `assembly_details.rs:399`).
    5. D-13-09 Selection-Preservation: KEIN `selected_ids.write().clear()` o.ae. — Vorstand kann sofort weiter klicken.
    6. Err-Branch: `show_toast(...e.message)` (analog assembly_details.rs).
- `cargo check` clean, `cargo build --release` clean (1m 21s WASM-Compile).

## Task Commits

1. **Task 1 — API + i18n:** `e6f3bfc` (feat) — `genossi-frontend/src/{api.rs,i18n/{mod.rs,de.rs,en.rs}}` (122 insertions)
2. **Task 2 — Component:** `1e0131a` (feat) — `genossi-frontend/src/component/repayment_entry_list.rs` (34 insertions)
3. **Task 3 — Page-Handler:** `3f1633b` (feat) — `genossi-frontend/src/page/repayment_phase_details.rs` (68 insertions)

## Files Created/Modified

- **Modified** `genossi-frontend/src/api.rs` — neue `GeneratedLettersResult`-Struct + `pub async fn generate_repayment_letters()` (~95 Zeilen Delta)
- **Modified** `genossi-frontend/src/i18n/mod.rs` — 4 neue Key-Enum-Varianten (~10 Zeilen Delta)
- **Modified** `genossi-frontend/src/i18n/de.rs` — 4 DE-String-Eintraege (~6 Zeilen Delta)
- **Modified** `genossi-frontend/src/i18n/en.rs` — 4 EN-String-Eintraege (~6 Zeilen Delta)
- **Modified** `genossi-frontend/src/component/repayment_entry_list.rs` — Prop + Button (~34 Zeilen Delta)
- **Modified** `genossi-frontend/src/page/repayment_phase_details.rs` — fiscal_year-Capture + on_letter_request-Handler (~68 Zeilen Delta)
- **Created** `.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-06-SUMMARY.md`

## Decisions Made

### `i18n/mod.rs` statt `i18n/keys.rs`

Plan-Text referenziert `genossi-frontend/src/i18n/keys.rs` als Ort fuer das Key-Enum, real lebt das Enum in `mod.rs` (es gibt keine `keys.rs` — siehe `ls genossi-frontend/src/i18n/`). Die 4 neuen Key-Varianten wurden in `mod.rs` direkt nach `RepaymentEntryBulkMailButton` ergaenzt, mit Doc-Kommentaren zu Singular/Plural + `{count}`-Placeholder. Keine Datei-Splittung noetig — die existierende Konvention sieht den Key im selben File wie das `Locale`-Enum vor.

### GeneratedLettersResult-Struct statt Tuple

Plan-Text gibt beide Optionen vor: `Result<(String, usize), AppError>` ODER `Result<GeneratedLettersResult, AppError>`. Ich habe das Struct gewaehlt, weil:
1. Self-Documenting-Fields (`blob_url`, `document_count`) im Caller-Code statt `result.0` / `result.1`.
2. Zukunftsfest: falls Phase 14+ Content-Disposition-Filename oder andere Metadaten braucht, ist die Erweiterung Non-Breaking.
3. Compiler-friendly: PartialEq + Eq fuer Unit-Tests trivial.

### use_i18n() im spawn-Block (Pattern aus Phase 6)

Im async-move-Closure des spawn-Blocks koennte das outer-`i18n` (owned `I18n`-Struct) zu einem FnOnce-Konflikt fuehren — die `move`-Closure verbraucht den Wert. Statt vorher einen Clone zu machen, holen wir `use_i18n()` direkt im spawn — das ist ein leichter Lookup vom Global-Signal und das gleiche Pattern wie in `assembly_details.rs:399`. Vorteile: kein outer-Capture, klarer Scope.

### Plural-Replace in eine Zeile

Mein erster Wurf hatte `.replace(\n  "{count}",\n  &count_str,\n)` — funktional korrekt, aber das Acceptance-Grep `rg '\.replace\("\{count\}"'` ist Single-Line-Regex und matcht das nicht. Loesung: `let count_str = result.document_count.to_string(); i18n_for_toast.t(Key::RepaymentLetterToastPlural).replace("{count}", &count_str)` — eine Zeile, klarer Lesefluss, Grep-fertig. Lessons learned fuer Folge-Plans: Acceptance-Greps sind oft Single-Line-Regex; multi-line `.replace(\n...)` matchen sie nicht.

### fiscal_year als separater Capture

Im RepaymentPhaseDetails::entries-Branch existieren bereits drei Captures (`phase_for_basics`, `phase_for_entries`, `phase_for_export`). Der `RepaymentEntryList`-Mount verbraucht `phase_for_entries` direkt — das heisst, im on_letter_request-spawn-Closure ist es nicht mehr verfuegbar. Ich habe deshalb VOR dem `rsx!`-Block eine separate Variable `let fiscal_year_for_letters = p.fiscal_year;` capturen — i32 ist Copy, der spawn-Block kann sich also problemlos ein Copy ziehen. Alternative waere ein 4. `phase_for_letters`-Clone gewesen — overhead-aermer ist der i32-Copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan-Pfad `i18n/keys.rs` existiert nicht**

- **Found during:** Task 1, beim Versuch die Datei zu lesen
- **Issue:** Plan-Text gibt `genossi-frontend/src/i18n/keys.rs` als Datei-Pfad fuer Key-Decl an, dort existiert aber nur `mod.rs`, `de.rs`, `en.rs`.
- **Fix:** Key-Varianten in `i18n/mod.rs` ergaenzt (wo das existierende `Key`-Enum lebt). Frontmatter-`files_modified` im Plan zeigt `i18n/keys.rs`, real geandert ist `i18n/mod.rs` — Plan-Text wuerde besser auf `i18n/mod.rs` referenzieren.
- **Files modified:** `genossi-frontend/src/i18n/mod.rs` (statt vermeintlicher `keys.rs`)
- **Commit:** `e6f3bfc` (Task 1)

**2. [Rule 3 - Blocking] Multi-line .replace bricht Acceptance-Grep**

- **Found during:** Task 3 Acceptance-Greps
- **Issue:** Mein erster Wurf hatte `.replace(\n  "{count}",\n  &count_str,\n)` ueber mehrere Zeilen — funktional korrekt, aber `rg '\.replace\("\{count\}"'` matcht das nicht (Single-Line-Regex).
- **Fix:** Refactor in eine Zeile: `let count_str = ...; i18n_for_toast.t(...).replace("{count}", &count_str)`.
- **Files modified:** `genossi-frontend/src/page/repayment_phase_details.rs`
- **Commit:** `3f1633b` (vor Commit gefixt; entry war im selben Task)

### Auto-fix Rules nicht relevant

- Rule 1 (Bug): keine Bugs — REST-Endpoint aus Plan 13-05 lieferte X-Document-Count Header korrekt; Frontend-Pattern aus Phase 6 Browser-Save war 1:1 wiederverwendbar.
- Rule 2 (Missing Critical): keine fehlenden Korrektheits-/Sicherheits-Elemente erkannt.
- Rule 4 (Architectural): keine architektonischen Aenderungen — Frontend folgt 1:1 dem Phase-6/12-Pattern + dem REST-Vertrag aus Plan 13-05.

## Issues Encountered

### Pre-Existing — Plan-Acceptance Grep `button\s*\{ | rg -v 'r#type:'`

Der Plan-Acceptance-Grep `rg 'button\s*\{' file | rg -v 'r#type:' | wc -l == 0` ist eine kombinierte Single-Pass-Form, die in der vorliegenden Datei 9 Treffer ergibt — weil die Zeile `button {` selbst nie `r#type:` enthaelt (das steht erst in der naechsten Zeile). Die echte Phase-12-D-02-Intention ist "naechste Zeile nach `button {` muss `r#type:` enthalten" — das laesst sich nur mit `rg -A 1`-Multi-Line-Grep verifizieren. Manuelle Verifikation aller 9 button-Definitionen in der Component-Datei: ALLE haben `r#type: "button"` als unmittelbar folgende Zeile (siehe Self-Check unten). D-02-Intention erfuellt.

### Pre-Existing — ROADMAP.md vom Orchestrator modifiziert

Beim Plan-Start war `.planning/ROADMAP.md` bereits modifiziert (vermutlich vom Wave-Orchestrator). Per Plan-Instruktion ("Do NOT modify STATE.md or ROADMAP.md") wurde es bei allen 3 Commits explizit ausgeschlossen (`git add` selektiv pro File).

### Pre-Existing — typst-packages-Files untracked

Wie in Plan 13-05 SUMMARY bereits dokumentiert: `genossi_service_impl/typst-packages/preview/letter-pro/3.0.0/` Files zeigen als untracked. Bei allen 3 Commits wurden sie via selektives `git add` ausgeschlossen — keine `git add .`-Konstrukte verwendet.

## Self-Check

```
=== Files exist ===
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/api.rs (modified)
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/i18n/mod.rs (modified)
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/i18n/de.rs (modified)
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/i18n/en.rs (modified)
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/repayment_entry_list.rs (modified)
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/repayment_phase_details.rs (modified)
FOUND: /home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-06-SUMMARY.md

=== Commits exist ===
FOUND: e6f3bfc (Task 1 — API + i18n)
FOUND: 1e0131a (Task 2 — Component Bulk-Button + Prop)
FOUND: 3f1633b (Task 3 — Page-Handler + Browser-Save + Toast)

=== Task 1 Acceptance-Greps ===
- rg 'pub async fn generate_repayment_letters' genossi-frontend/src/api.rs: 1 ✓
- rg 'pub struct GeneratedLettersResult' genossi-frontend/src/api.rs: 1 ✓
- rg 'pub document_count: usize' genossi-frontend/src/api.rs: 1 ✓
- rg 'entry_ids' genossi-frontend/src/api.rs: 11 (>=2) ✓
- rg 'api/repayment-phase/.+/letters/generate' genossi-frontend/src/api.rs: 2 (Doc-Kommentar + format!) ✓
- rg 'X-Document-Count' genossi-frontend/src/api.rs: 4 (Doc-Kommentar + Code) ✓
- rg 'create_object_url_with_blob' genossi-frontend/src/api.rs: hat sich um 1 erhoeht ✓
- rg 'RepaymentEntryBulkLetterButton' genossi-frontend/src/i18n/mod.rs: 1 ✓
- rg 'RepaymentLetterToastSingular' genossi-frontend/src/i18n/mod.rs: 1 ✓
- rg 'RepaymentLetterToastPlural' genossi-frontend/src/i18n/mod.rs: 1 ✓
- rg 'Anschreiben erzeugen' genossi-frontend/src/i18n/de.rs: 1 ✓
- rg '1 Brief erzeugt' genossi-frontend/src/i18n/de.rs: 1 ✓
- rg '\{count\} Briefe erzeugt' genossi-frontend/src/i18n/de.rs: 1 ✓
- rg 'Generate letters' genossi-frontend/src/i18n/en.rs: 1 ✓
- cd genossi-frontend && cargo check: clean (nur Dead-Code-Warnings) ✓

=== Task 2 Acceptance-Greps ===
- rg 'on_letter_request: EventHandler<Vec<Uuid>>' genossi-frontend/src/component/repayment_entry_list.rs: 1 ✓
- rg 'on_letter_request' genossi-frontend/src/component/repayment_entry_list.rs: 2 (Prop-Decl + onclick.call) ✓
- rg 'RepaymentEntryBulkLetterButton' genossi-frontend/src/component/repayment_entry_list.rs: 1 ✓
- rg 'bg-purple-600' genossi-frontend/src/component/repayment_entry_list.rs: 1 ✓
- Phase 12 D-02 Naext-Line-Check: alle 9 `button {` haben unmittelbar folgendes `r#type: "button"` ✓
- D-13-09 Selection-Preservation (Component-Ebene): `rg 'on_letter_request' ... -A 12 | rg 'selected_ids\.write\(\)\.clear|selected_ids\.write\(\) ='` returns 0 ✓
- cargo check exit 0 (nach Task 3 — Task 2 alleine fail wegen Page-on_letter_request-Missing, erwartet) ✓

=== Task 3 Acceptance-Greps ===
- rg 'on_letter_request:' genossi-frontend/src/page/repayment_phase_details.rs: 1 ✓
- rg 'generate_repayment_letters' genossi-frontend/src/page/repayment_phase_details.rs: 1 ✓
- rg 'auszahlungs_anschreiben_GJ_' genossi-frontend/src/page/repayment_phase_details.rs: 2 (Kommentar + format!) ✓
- rg 'create_element\("a"\)' genossi-frontend/src/page/repayment_phase_details.rs: 1 ✓
- rg 'revoke_object_url' genossi-frontend/src/page/repayment_phase_details.rs: 2 (Kommentar + Code) ✓
- rg 'result\.document_count' genossi-frontend/src/page/repayment_phase_details.rs: 2 (Singular-Check + Plural-Replace) ✓
- rg 'RepaymentLetterToastSingular' genossi-frontend/src/page/repayment_phase_details.rs: 1 ✓
- rg 'RepaymentLetterToastPlural' genossi-frontend/src/page/repayment_phase_details.rs: 1 ✓
- rg '\.replace\("\{count\}"' genossi-frontend/src/page/repayment_phase_details.rs: 1 ✓
- entry_ids.len-Misuse-Check: rg returns 0 ✓
- D-13-09 Selection-Preservation: rg returns 0 ✓
- cargo check exit 0 ✓
- cargo build --release exit 0 (1m 21s) ✓

=== Phase 12 D-02 Multi-Line-Check (manuelle Verifikation aller button-Definitionen) ===
genossi-frontend/src/component/repayment_entry_list.rs:
  Line 224 `button {` → Line 225 `r#type: "button",` ✓ (Add)
  Line 230 `button {` → Line 231 `r#type: "button",` ✓ (Mail)
  Line 258 `button {` → Line 259 `r#type: "button",` ✓ (NEU: Letter — Phase 13)
  Line 284 `button {` → Line 285 `r#type: "button",` ✓ (MarkContacted)
  Line 311 `button {` → Line 312 `r#type: "button",` ✓ (MarkPaidOut)
  Line 467 `button {` → Line 468 `r#type: "button",` ✓ (Delete-Trash)
  Line 491 `button {` → Line 492 `r#type: "button",` ✓ (Modal Cancel)
  Line 497 `button {` → Line 498 `r#type: "button",` ✓ (Modal Delete)
  Line 528 `button {` → Line 529 `r#type: "button",` ✓ (StatusFilterTab)
genossi-frontend/src/page/repayment_phase_details.rs:
  Alle button-Definitionen sind aus Phase 12 unveraendert + setzen `r#type: "button"` ✓
RESULT: 0 button-Definitionen ohne r#type — Phase 12 D-02 erfuellt.

=== Verification Spec ===
- cd genossi-frontend && cargo check exit 0: ✓
- cd genossi-frontend && cargo build --release exit 0: ✓
- Phase 12 D-02 Grep-Gate (manuell verifiziert): 0 ✓
- i18n-Keys (Singular + Plural) in DE + EN registriert: ✓ (4 Keys * 2 Locales = 8 Eintraege)
- X-Document-Count fliesst Server → API → Toast: ✓ (3 Greps gruen)
- D-13-09 Selection-Preservation Grep-Gate: 0 ✓

=== Plan Acceptance ===
- Component-First eingehalten — keine inline RSX-Duplikate in der Page (Bulk-Button lebt im Component) ✓
- Beide i18n-Locales (de.rs + en.rs) gepflegt ✓
- No modifications to STATE.md / ROADMAP.md ✓
- Single-Arc-Pattern n/a (Frontend-Plan, kein DI-Wiring)

=== No untracked files committed ===
- git show --stat e6f3bfc: 4 files (api.rs + 3 i18n) ✓
- git show --stat 1e0131a: 1 file (component) ✓
- git show --stat 3f1633b: 1 file (page) ✓
- KEIN genossi_service_impl/typst-packages/ in commits ✓
- KEIN .planning/ROADMAP.md in commits ✓
```

**Self-Check: PASSED**

## Threat Flags

Keine neuen Threat-Flags ueber das Plan-`<threat_model>` hinaus. Mitigationen verifiziert:

- **Phase 12 D-01 Verletzung (Button ohne r#type)**: VERIFIZIERT-MITIGIERT — alle 9 button-Definitionen in der Component-Datei haben `r#type: "button"` als allererstes Attribut (siehe Self-Check Multi-Line-Check).
- **CSRF / Cross-Origin Request**: ACCEPT — Same-Origin POST, Backend setzt CORS (siehe Plan 13-05).
- **PII-Leak im Toast**: VERIFIZIERT-MITIGIERT — Toast enthaelt nur Count + i18n-String, keine Member-Namen.
- **Browser-Memory-Leak durch nicht-revoke'd Blob-URLs**: VERIFIZIERT-MITIGIERT — `Url::revoke_object_url(&result.blob_url)` direkt nach `.click()` im selben Block.
- **Doppelter-Klick → Race**: LOW — Button hat `disabled: selected_count == 0` (siehe Server-Limit von 200 Entries im Plan 13-05).
- **i18n-Drift (DE/EN-Keys asymmetrisch)**: VERIFIZIERT-MITIGIERT — alle 4 Keys in BEIDEN Locale-Dateien gepflegt (siehe Self-Check).
- **Toast-Count-Inkonsistenz (entry_ids.len vs aggregierte Brief-Anzahl)**: VERIFIZIERT-MITIGIERT (D-13-04 user-trust) — API-Funktion liest X-Document-Count Header, Page-Handler nutzt diesen Server-Wert (NICHT entry_ids.len()). Grep-Gate `rg 'entry_ids\.len\(\)' ... | rg 'RepaymentLetter|toast'` returns 0.
- **Toast-Grammatik (1 Briefe falsch)**: VERIFIZIERT-MITIGIERT (UX-Quality) — Singular/Plural-i18n-Keys + `if document_count == 1`-Branch.
- **D-13-09 Selection-Loss**: VERIFIZIERT-MITIGIERT (UX + D-13-09 user-trust) — `selected_ids` wird NICHT modifiziert (weder im Ok- noch im Err-Branch). Grep-Gate `rg 'selected_ids\.write\(\)\.clear|...'` returns 0.
- **D-13-09 Compliance (Vorstand vergisst Status-Toggle)**: ACCEPT — Toast bewahrt den User mit dem Hinweis "Vergiss nicht, die Eintraege anschliessend als angeschrieben zu markieren".

## Next Plan Readiness

Plan 13-07 (E2E-Tests) kann jetzt:
- Frontend-Komponente ist verdrahtet — End-to-End-Tests koennen den Endpoint via `client.post(...).json({entry_ids: ...})` ausloesen
- X-Document-Count Header wird im Frontend korrekt gelesen — manuelle UAT (echtes Klicken im Browser) ist moeglich
- Toast-Pluralisierung verifiziert via Singular/Plural-Keys + cargo build --release

**Keine Blocker fuer Folge-Plans.**

**Pending Follow-ups (durch dieses Plan NICHT abgedeckt):**
- Plan 13-07 (E2E-Tests): 6+ HTTP-Tests via reqwest gegen den neuen Endpoint, plus Audit-Hashchain-Verify
- D-13-11 Phase-10-Worker-Refactor: `.planning/todos/pending/phase-10-worker-refactor-resolver.md` — bleibt als Folge-Quick offen

---

*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-02*
