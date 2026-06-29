---
phase: 18-frontend-component-first
reviewed: 2026-06-07T00:00:00Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - genossi-frontend/Cargo.lock
  - genossi-frontend/rest-types/Cargo.toml
  - genossi-frontend/rest-types/src/lib.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/component/fiscal_year_date_input.rs
  - genossi-frontend/src/component/member_search.rs
  - genossi-frontend/src/component/membership_adjust_modal.rs
  - genossi-frontend/src/component/mod.rs
  - genossi-frontend/src/component/toast.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/page/member_details.rs
findings:
  critical: 2
  warning: 9
  info: 6
  total: 17
status: issues_found
---

# Phase 18: Code Review Report

**Reviewed:** 2026-06-07
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Adversariale Sicht auf Phase 18 (Frontend-Component-First, MembershipAdjustModal +
FiscalYearDateInput + Phase-18-DTOs + Toast-Erweiterungen). Insgesamt ist das Modul
sauber strukturiert: Pure-Helpers sind testbar, i18n-Keys werden geprüft, Backend-
Mirrors sind dokumentiert. Die Hauptdefekte liegen in **state-leak zwischen Sub-Views**
(date_signal wird beim Verlassen der Sub-Choice nicht zurueckgesetzt, aber shares/recipient
schon — fuehrt zu inkonsistentem UX und potenziellen Wrong-Date-Submissions), **Side-Effects
in einer Render-Funktion** (`render_sub_choice` ruft `Signal::set` im Render-Pfad auf —
kann unter Dioxus zu Render-Loops / unnoetigen Reflows fuehren), und **falsch geboundete
Modal-Dispatch-Buttons** (Cancel-Sub-View nutzt die rote Button-Farbe — passt zu "Kuendigung",
aber Partial-Repayment / Transfer / Upgrade nutzen ebenfalls Bg-Red-600, was visuell falsch
ist und Vorstand suggeriert, jede Aktion sei destruktiv).

Zusatzlich gibt es eine ganze Reihe an Architektur-Befunden: ungenutzte i18n-Keys, ungenutzte
Imports (Toast-Variant), Inkonsistenz bei Validation (Sub-View "Partial-Repayment" rejected
`shares >= current` mit "use Cancel", Modal-Validation laesst aber gleichzeitig `shares <
current` zu — bedeutet exact equal ist im Validation-Block 2x identisch enthalten). Diese
Defekte sind nicht-blockierend, sollten aber vor Production-Cutover gefixt werden.

## Critical Issues

### CR-01: `date_signal` ueberlebt Sub-Choice-Wechsel — falsches Datum kann silently uebernommen werden

**File:** `genossi-frontend/src/component/membership_adjust_modal.rs:234-237` (kommentiert
"date_signal is NOT reset — persists across sub-view re-entry per CONTEXT.md Discretion")
und `:125` (Initialisierung mit `Some(today)`).

**Issue:** Der Sub-Choice-Renderer setzt explizit `shares_signal.set(1)` und
`recipient_id_signal.set(None)` zurueck, **nicht aber** `date_signal`. Konkretes Szenario:

1. User waehlt "Kuendigung", aendert das Datum auf einen Wert ausserhalb des Fiscal-Year-
   Bereichs (z.B. tippt 2027-12-30, dann editiert zu 2099-01-01 — wird nicht gefiltert weil
   `parse_date_input` jeden gueltigen Kalender-Tag akzeptiert und `value_set` direkt in der
   `oninput`-Closure passiert, BEVOR `is_valid_fiscal_year_date` greift).
2. User klickt "Zurueck", dann "Aufstockung".
3. Der Datepicker zeigt jetzt das von Sub-View 1 hinterlassene Datum `2099-01-01` — das ist
   logisch unzusammenhaengend mit Aufstockung und wuerde im Submit als Willensbekundungs-
   Datum 2099 an Backend gesendet (`is_valid` blockt zwar `submit`, aber der User sieht das
   alte Datum statt "today" und kann verwirrt sein, dass die Vorschau leer bleibt).

Schlimmer: Bei einer Sub-View, die kein FY-Bound erzwingt (siehe WR-04: `shares_now <= 0` und
`shares_now >= current` werden in Cancel-Sub-View NICHT geprueft, der greift nur auf das
Datum), kann ein "vom alten Sub-View hinterlassenes" Datum durchrutschen, ohne dass der User
es bemerkt.

**Fix:** Auch `date_signal` zuruecksetzen, oder konsequent ein Reset-Flag fuer alle Felder
verwenden:

```rust
fn render_sub_choice(
    i18n: I18n,
    mut step: Signal<ModalStep>,
    mut date_signal: Signal<Option<time::Date>>,  // add this
    mut shares_signal: Signal<i32>,
    mut recipient_id_signal: Signal<Option<Uuid>>,
    today: time::Date,                            // add this
) -> Element {
    // Reset alle operation-spezifischen Felder + Datum auf "today".
    date_signal.set(Some(today));
    shares_signal.set(1);
    recipient_id_signal.set(None);
    // …
}
```

### CR-02: Side-Effect (`Signal::set`) im Render-Pfad von `render_sub_choice` (potenzieller Render-Loop)

**File:** `genossi-frontend/src/component/membership_adjust_modal.rs:228-237`

**Issue:** `render_sub_choice` wird vom Parent (`MembershipAdjustModal`) bei jedem Render
aufgerufen, weil das `match step.read()` in der RSX-Branch eingebettet ist. Innerhalb wird
**unbedingt** `shares_signal.set(1)` und `recipient_id_signal.set(None)` aufgerufen. Das
hat zwei Konsequenzen:

1. **Re-Render-Loop-Risiko:** Wenn der User in der Sub-Choice landet und ein anderer
   Signal-Write (z.B. der Error-Dismiss in `:158`) einen Re-Render triggert, wird in
   jedem Render-Cycle erneut `shares_signal.set(1)` aufgerufen. Dioxus Signals haben
   keine integrierte Equality-Short-Circuit-Guarantee fuer Re-Subscriber: wenn die Reads
   des Signals woanders im Tree subscriben, kann das einen weiteren Re-Render anstossen
   — quasi-loop bis Stabilitaet.
2. **User-Datenverlust:** Nehmen wir an, der User landet versehentlich auf der Sub-Choice
   (z.B. weil der "Zurueck"-Button im Cancel-Sub-View geklickt wurde, um nur kurz die
   Beschreibungen nachzulesen). In dem Moment werden `shares_signal` und `recipient_id_signal`
   silently zurueckgesetzt. Wenn der User dann auf die ursprueglich gewaehlte Sub-View
   zurueckkehrt, ist sein vorher eingegebener Wert weg — ohne Warnung. Das ist genau die
   gleiche UX-Falle wie Phase 4's Helper-Token-Card-Bug, die im Memory dokumentiert ist.

**Fix:** Side-Effects gehoeren in den onclick-Handler, nicht in den Render-Pfad. Der Reset
sollte beim Klick auf "Zurueck" oder beim ersten Wechsel in die Sub-Choice einmal passieren,
nicht bei jedem Re-Render:

```rust
// In den onclick-Handlern der "Zurueck"-Buttons (cancel/partial/transfer/upgrade):
onclick: move |_| {
    shares_signal.set(1);
    recipient_id_signal.set(None);
    date_signal.set(Some(today));
    step.set(ModalStep::SubChoice);
},

// render_sub_choice ist dann reine Render-Funktion ohne side effects:
fn render_sub_choice(i18n: I18n, mut step: Signal<ModalStep>) -> Element {
    // … nur rsx, kein .set(...)
}
```

## Warnings

### WR-01: Cancel-Sub-View "Bereich" pruefte nur das Datum, nicht das Min/Max — `parse_date_input` umgeht HTML-`min/max`

**File:** `genossi-frontend/src/component/fiscal_year_date_input.rs:82-91`

**Issue:** Die `oninput`-Closure des FiscalYearDateInput parsed jeden Wert mit
`parse_date_input` und ruft `value.set(Some(d))` + `on_change.call(d)` auf, BEVOR `is_valid_
fiscal_year_date` greift. Die `min`/`max`-HTML-Attribute werden in der Praxis nicht von
allen Browsern (insbesondere mobilen Safari/Firefox) strikt validiert — sie sind nur ein
UX-Hint, kein Hard-Block. D.h. der User kann mit Datumspicker oder Tipp-Eingabe Werte
ausserhalb des erlaubten Bereichs setzen; der rote Warntext erscheint zwar, aber die
`on_change`-Event-Handler in den Sub-Views (z.B. `:367`) schreiben den Wert trotzdem in
`date_signal`. Wenn die Sub-View dann den Submit ueber den `is_valid`-Check fuehrt, ist
zwar gesperrt — aber: 

- `on_change.call(d)` wird trotz Out-of-Range gefeuert. Das kann zu unerwartetem Verhalten
  fuehren, wenn der `on_change`-Handler des Callers Seiteneffekte hat (z.B. eine externe
  Preview generieren).
- Die Sub-Views koennten ein Datum lesen, dass `is_valid_fiscal_year_date == false` ergibt;
  ihre `preview_text`-Logik ist resilient (gibt empty String), aber inkonsistent (das Feld
  zeigt rot, aber das Signal haelt einen "ungueltigen" Wert).

**Fix:** Out-of-Range-Werte sollten gar nicht ins Signal gelangen:

```rust
oninput: move |e| {
    let s = e.value();
    if s.is_empty() {
        value.set(None);
    } else if let Some(d) = parse_date_input(&s) {
        if is_valid_fiscal_year_date(d, today) {
            value.set(Some(d));
            on_change.call(d);
        } else {
            // Signal trotzdem setzen, aber on_change.call(d) NICHT — Caller
            // soll nicht mit Out-of-Range-Werten konfrontiert werden.
            value.set(Some(d));
        }
    }
}
```

### WR-02: Cancel-Sub-View nutzt `bg-red-600` (destruktive Farbe) — semantisch korrekt nur fuer Kuendigung

**File:** `genossi-frontend/src/component/membership_adjust_modal.rs:391, 553, 796, 938`

**Issue:** Die Submit-Buttons aller 4 Sub-Views (`Cancel`, `PartialRepayment`, `Transfer`,
`Upgrade`) verwenden alle `class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white …"`.
Rot ist im Tailwind/Material-Design eine **destruktive** Action-Farbe; sie passt zu
"Kuendigung" (= Mitgliedschaft beenden), ist aber falsch fuer:

- **Aufstockung** (Member zeichnet zusaetzliche Anteile — das ist eine konstruktive, positive
  Aktion).
- **Uebertrag** (Anteile wechseln den Inhaber — neutral, nicht destruktiv).
- **Teil-Rueckgabe** (Member gibt Anteile zurueck — neutral mit leichtem destruktiven
  Touch, aber nicht in der Schaerfe von "Mitgliedschaft beenden").

Vorstand wird visuell suggeriert, jede Anpassung sei destruktiv, was zu Klick-Hesitation
fuehrt und mittelbar das DOM-Pattern verletzt, das anderswo in der App konsistent verwendet
wird (`bg-blue-600` fuer positive/neutrale Actions, siehe `member_details.rs:436` —
"Mitgliedschaft anpassen"-Button ist blau).

**Fix:**
- Cancel-Sub-View: `bg-red-600` ist korrekt.
- PartialRepayment-Sub-View: `bg-orange-600` (Warn-Farbe, weil Auszahlung trotzdem ein
  Geld-Ausgang ist).
- Transfer-Sub-View: `bg-blue-600` (neutral) oder `bg-orange-600` bei Voll-Uebertrag.
- Upgrade-Sub-View: `bg-blue-600` (positiv).

### WR-03: `is_valid` in PartialRepayment akzeptiert `shares_now < current` — Equality-Branch ungeprueft

**File:** `genossi-frontend/src/component/membership_adjust_modal.rs:489-491`

```rust
let is_valid = date_for_submit.map_or(false, |d| is_valid_fiscal_year_date(d, today))
    && shares_now >= 1
    && shares_now < current;
```

**Issue:** Das ist sehr nahe an einem Off-By-One/Inconsistency-Fall: in `inline_error_text`
(`:460`) wird `shares_now >= current` als Fehler markiert (mit Suggestion "Use Cancel for
full"). Aber `is_valid` benutzt `shares_now < current` — wenn `current == 5` und `shares_now
== 5`, dann ist `inline_error_text = Some(...)` UND `is_valid = false`. Das ist konsistent.

ABER: das Backend `partial_repayment` schreibt einen RepaymentEntry und reduziert
`current_shares` (siehe `api.rs:2887-2908`). Wenn der Server `shares >= current` akzeptiert,
soll auf Backend-Ebene auf Austritt umgeleitet werden — der Frontend-Check ist eine
DEFENSIVE-IN-DEPTH-Spiegelung. Das ist OK, aber: **was passiert, wenn `current` zwischen-
zeitlich vom Backend updated wurde (z.B. wegen einer parallelen Action)**? Die Frontend-
Snapshot-`current` ist stale; ein Submit mit `shares_now == 4, snapshot_current == 5` koennte
am Backend mit `shares >= server_current = 4` zu einem `Conflict` fuehren, der nirgends
sauber gehandled wird (nur generisches `error_signal.set(e)` ohne 409-spezifische Recovery).

**Fix:** Bei 409 (Optimistic-Locking-Fail oder Validation-Konflikt) sollte die Member-Snapshot
neu geladen und der Form-State refreshed werden, z.B.:

```rust
Err(e) => {
    submitting.set(false);
    if e.status == Some(409) {
        // Re-fetch member + refresh
        let new_member = api::get_member(&config, id).await;
        // ... reopen sub-view with refreshed shares
    }
    error_signal.set(Some(e));
}
```

### WR-04: Cancel-Sub-View ignoriert `current_shares == 0` — Voll-bereits-leer-Member kann fehlerhaft "gekuendigt" werden

**File:** `genossi-frontend/src/component/membership_adjust_modal.rs:342-346, 393-411`

**Issue:** Die Cancel-Submit-Logik validiert nur `is_valid_fiscal_year_date(d, today)`,
nicht aber Member-State. Wenn ein Member bereits ausgetreten ist (`status != Normal`, oder
`current_shares == 0`, oder `exit_date != None`), zeigt das Modal trotzdem den "Kuendigung
auslösen"-Button (er ist enabled). Der Submit landet am Backend; das Backend hat seine
eigenen Checks (`cancel_membership` returns 4xx oder 409), aber der User sieht den Fehler
erst nach dem Klick — schlechte UX.

Schlimmer: der Modal-Caller (`member_details.rs:431`) zeigt den "Mitgliedschaft anpassen"-
Button bei JEDEM existierenden Member, ohne Filter auf `status` oder `exit_date`. D.h. ein
ausgetretener Member bekommt den Button gezeigt — das eroeffnet eine UX-Falle.

**Fix:**
1. Im Caller (`member_details.rs:431`): Button nur zeigen, wenn `member.is_active(&today)` ist.
2. Im Modal-Sub-View Cancel: zusaetzliche `is_valid`-Bedingung
   `member.is_active(&today) && member.current_shares > 0`.

### WR-05: Doppelte `today`-Berechnung in `member_details.rs` mit divergierender Fehler-Recovery

**File:** `genossi-frontend/src/page/member_details.rs:86-94, 142-152, 180-191`

**Issue:** Der Code berechnet `today` **dreimal** in unterschiedlichen Closures, und die
Fehler-Recovery-Pfade weichen voneinander ab:

- `:86-94`: Bei Month-Parse-Fehler -> `try_into().unwrap_or(Month::January)` (Fallback auf
  Januar) und bei Date-Konstruktor-Fehler -> Fallback `2025-01-01`.
- `:142-152`: Selbes Pattern.
- `:180-191`: **Anderer** Pfad — explizites `match time::Month::try_from(month_idx)`, bei
  Err Fallback auf `2025-01-01`. Die `unwrap_or` der ersten Variante fuehrt zu "today wird
  zu Januar 1 des aktuellen Jahres" (mit potentially-correct Tag), aber die dritte Variante
  faellt auf `2025-01-01` zurueck — das ergibt drei unterschiedliche `today`-Werte fuer
  drei verschiedene Komponenten der Page.

In der Praxis wird `try_from` fuer Month-Indices `1-12` immer Ok zurueckgeben (JS
`get_month()` liefert `0-11`, +1 ergibt `1-12`), aber:

1. Wenn ein **Browser-Bug** oder **JS-Zeitkrise** `get_month()` mit `12` zurueckgibt
   (theoretisch impossible, aber defensive Code soll nicht divergieren), dann liefert
   `try_from(13)` Err — und `today` ist je nach Stelle entweder `Jan 1 current_year`
   ODER `Jan 1 2025`.
2. Code-Duplikation ist ein **Memory-Eintrag** (`feedback_component_first.md` — "Always use
   reusable components, never duplicate UI logic"). Das ist hier nicht UI-RSX, aber
   immerhin duplizierte Date-Konstruktor-Logik.

**Fix:** Eine Helper-Funktion `fn today_or_fallback() -> time::Date` (Modul-level) extrahieren
und an allen drei Stellen aufrufen. Bonus: Konsistenter Fallback (entweder ueberall
`2025-01-01` oder ueberall den ersten Tag des aktuellen Jahres).

### WR-06: i18n-Keys `MembershipAdjustPartialRepaymentAutoCreateHint` + `…SuccessAutoCreate` definiert aber NIE verwendet — Dead-Code

**File:** Keys-Definition in `genossi-frontend/src/i18n/mod.rs:743, 747`; DE-Translation
`genossi-frontend/src/i18n/de.rs:660, 663`; EN-Translation `genossi-frontend/src/i18n/en.rs:653,
656`. **KEIN** Aufruf in `membership_adjust_modal.rs`, `member_details.rs` oder anderswo
im Repo.

**Issue:** Die Phase-18-PLAN-Spec sieht offenbar einen "⚠ Auszahlungsphase FY{fiscal_year}
wird automatisch angelegt"-Hint in der Partial-Repayment-Vorschau und einen abweichenden
Success-Toast vor (`…SuccessAutoCreate` mit `{fiscal_year}`-Substitution), aber das Modal
implementiert das nicht: in `render_partial_sub_view` wird beim Submit lediglich
`on_success.call(())` aufgerufen, was im Caller `member_details.rs:1494` immer den generischen
`MembershipAdjustSuccess`-Toast triggert — egal ob `phase` in der Response Some oder None war.

Die `PartialRepaymentResponseTO` (in `rest-types/src/lib.rs:617`) enthaelt explizit ein
optionales `phase`-Feld fuer den Auto-Create-Branch — aber dieses Feld wird nie ausgelesen
im Frontend. Konsequenz: 

- User sieht keinen Hinweis, dass eine neue Phase angelegt wurde -> Erwartungs-Verletzung.
- Code-Pfad (Backend-Auto-Create) ist unsichtbar gegenueber User -> wirkt wie ein "stiller"
  Side-Effect.

**Fix:** In `render_partial_sub_view` die `inline_error_text` durch einen `auto_create_hint`
ersetzen, wenn ein bestimmtes Kriterium gegeben ist (z.B. Backend-Response `phase = Some(...)`,
oder Frontend-Heuristik `compute_effective_date_mirror(d)` gibt eine Phase im Future zurueck).
Im Submit-Callback die Response (Result<PartialRepaymentResponseTO>) auswerten und je nach
`phase.is_some()` einen anderen Success-Toast-Key feuern.

### WR-07: `ToastVariant` definiert und exportiert, aber NIE verwendet — Dead Code

**File:** `genossi-frontend/src/component/toast.rs:17-21`, Export in
`genossi-frontend/src/component/mod.rs:141`.

**Issue:** Der Enum `ToastVariant { Success, Error }` ist definiert und tests-only verwendet
(`toast.rs:112-124`), aber kein Konsument im Production-Code referenziert ihn. Die zwei
Container (`ToastContainer` rot, `SuccessToastContainer` gruen) werden ueber **separate
Signal-Buckets** unterschieden, nicht ueber den Variant. Damit ist `ToastVariant` ein
ungenutztes Public-API-Element.

**Fix:** Entweder:
1. Loeschen (`pub enum ToastVariant` + Re-Export) — die Container koennen weiterhin via
   Separate-Buckets unterschieden werden.
2. Oder die Container per Variant-Prop unifizieren:
   ```rust
   #[component]
   pub fn Toast(messages: …, variant: ToastVariant) -> Element { … }
   ```
   und dann SuccessToastContainer/ToastContainer als Thin-Wrappers behalten — aber das
   waere eine API-Vereinfachung, kein Bug-Fix.

### WR-08: `member_search.rs` deutsche Hardcoded-Strings statt i18n

**File:** `genossi-frontend/src/component/member_search.rs:102`

**Issue:** Der `placeholder` des Such-Inputs ist hardcodet als
`"Name oder Nummer suchen..."`. Englische Locale wird damit ignoriert; ein User mit
EN-Locale sieht den deutschen Text. Das verletzt die i18n-Convention der App:

```rust
placeholder: "Name oder Nummer suchen...",
```

Andere Components in dem Verzeichnis lokalisieren saubsr via `i18n.t(Key::…)`. Der
Memory-Eintrag (`feedback_component_first.md`) deckt zwar nur Komponenten-Duplikation
ab, aber konsistente i18n ist Best-Practice.

**Fix:** Einen Key `Key::MemberSearchPlaceholder` einfuehren (in `i18n/mod.rs` + `de.rs` +
`en.rs`) und in `member_search.rs:102` via `let i18n = use_i18n();` + `placeholder: "{i18n.t(Key::MemberSearchPlaceholder)}"`
einsetzen.

### WR-09: `iso8601_date::serialize/deserialize` benutzen `time::format_description::parse(...).unwrap()` — Panic-Pfad in Hot-Path

**File:** `genossi-frontend/rest-types/src/lib.rs:54, 69, 87, 97`

**Issue:** Vier `format_description::parse("[year]-[month]-[day]").unwrap()`-Aufrufe in der
Serialisierungs/Deserialisierungs-Logik. Das `parse` einer statischen Format-String kann
zwar nur fehlschlagen, wenn der String selbst syntaktisch falsch ist (was hier konstant ist,
also de-facto unmoeglich), aber:

1. Bei jedem Serialize/Deserialize-Aufruf wird der Format-String **neu geparst**. Das ist
   ein wiederholter Aufruf von `parse` mit allocations — fuer eine Liste mit 1000 Members
   = 1000+ Parse-Aufrufe. Ineffizient.
2. `unwrap` in Library-Code ist ein Smell. Wenn die `time`-Crate-API sich aendert und das
   Format-String-Schema fragmentiert, wird die ganze WASM-App in der naechsten Major-Version
   sterben — anstelle eines klaren Compile-Errors.

**Fix:** Konstantes `Lazy` oder `once_cell`-Pattern fuer das parsed Format:

```rust
use std::sync::OnceLock;
fn date_format() -> &'static [time::format_description::FormatItem<'static>] {
    static F: OnceLock<Vec<time::format_description::FormatItem<'static>>> = OnceLock::new();
    F.get_or_init(|| {
        time::format_description::parse("[year]-[month]-[day]").unwrap()
    })
}
```

Aber besser: `time` bietet `format_description::well_known::Iso8601` — verwenden statt
selbst geparstem Format (siehe `iso8601_datetime`-Modul oberhalb, das es korrekt macht).

## Info

### IN-01: `genossi-frontend/Cargo.lock` ist im Review-Scope — Lock-Files sollten gefiltert werden

**File:** `genossi-frontend/Cargo.lock`

**Issue:** Cargo.lock ist im `files`-Block enthalten. Lock-Files sind generierte Artefakte
und sollten typischerweise ausgeschlossen werden. In der Anweisung im Workflow steht
"`package-lock.json`, `yarn.lock`, `Gemfile.lock`, `poetry.lock`" — Cargo.lock fehlt in der
Default-Filter-Liste. Sollte ergaenzt werden.

**Fix:** Workflow- oder Agent-Config um `Cargo.lock` erweitern.

### IN-02: `MembershipAdjust*Success` Keys (4 Variants) NIE verwendet — Dead i18n

**File:** `genossi-frontend/src/i18n/mod.rs:735, 745, 759, 767`
(`MembershipAdjustCancelSuccess`, `MembershipAdjustPartialRepaymentSuccess`,
`MembershipAdjustTransferSuccess`, `MembershipAdjustUpgradeSuccess`).

**Issue:** Diese 4 operation-spezifischen Success-Texts sind definiert, aber kein Sub-View
nutzt sie. Stattdessen feuert `member_details.rs:1487/1510` immer den generischen
`MembershipAdjustSuccess`-Toast. Konsequenz: User bekommt fuer Kuendigung, Aufstockung,
Teil-Rueckgabe und Uebertrag exakt denselben Toast-Text ("Anpassung wurde erfolgreich
gespeichert."), obwohl die spezifischen Texte feiner sind ("Kuendigung wurde ausgeloest",
"Aufstockung wurde eingetragen", etc.).

**Fix:** `on_success`-EventHandler im Modal koennte einen `op_kind`-Parameter mitbringen,
damit der Caller den richtigen Toast-Key auswaehlt. Alternativ: per-Sub-View den Toast
direkt aus dem Modal abfeuern statt im Caller.

### IN-03: `MembershipAdjustTransferRecipientLoadError` Key gelesen, aber Variable nicht verwendet

**File:** `genossi-frontend/src/component/membership_adjust_modal.rs:602-604`

**Issue:** `recipient_load_err` wird via `i18n.t(...)` gelesen — und Zeile 752 nutzt ihn in
einem `rsx!`-String:
```rust
Some(Err(_)) => rsx! { p { class: "text-sm text-red-600", "{recipient_load_err}" } },
```
OK, der **wird** verwendet. Mein Verdacht war falsch — die `let _ = recipient_load_err` Sorge
ist Unbegruendet. Nur als Hinweis: das `_` als Discard erkennt der Rust-Compiler korrekt,
aber das Pattern `Some(Err(_))` ohne Capture der `AppError` wirft die nuetzliche Detail-
Information weg (z.B. waere es nett, in der Detail-Konsole zu sehen, ob 401/403/500 — fuer
Bug-Triage):

```rust
Some(Err(e)) => {
    let err_msg = recipient_load_err.clone();
    rsx! { 
        p { class: "text-sm text-red-600", "{err_msg}" } 
        // Optional: kleiner Debug-Hint fuer Vorstand:
        if let Some(status) = e.status {
            span { class: "text-xs text-gray-500", "(HTTP {status})" }
        }
    }
}
```

### IN-04: Magische Konstante `150` (Timeout fuer Dropdown-Click) — sollte benannt sein

**File:** `genossi-frontend/src/component/member_search.rs:76`

**Issue:** `gloo_timers::future::TimeoutFuture::new(150).await;` — die Zahl 150 ist ohne
Kontext und Erklaerung. Code-Comment darueber sagt "Small delay to allow click events on
dropdown items to fire", erklaert aber nicht, warum 150ms gewaehlt sind (vs. z.B. 100ms
oder 200ms).

**Fix:** Konstante mit Begruendung:
```rust
const DROPDOWN_CLICK_GRACE_MS: u32 = 150;  // > Browser-Click-Event-Propagation (60-100ms)
```

### IN-05: `AppError::is_request() && status.is_none()` Branch koennte verwirrt sein bei wasm32-Targets

**File:** `genossi-frontend/src/api.rs:60-64`

**Issue:** Der Code-Comment in `:54-55` erklaert detailliert, dass `is_decode`, `is_timeout`,
`is_request`, `is_redirect`, `is_status` "in beiden Targets stabil sind". Der Kommentar in
`:61-64` ergaenzt, dass `is_request()` ohne Status sowohl Build-Fehler als auch Network-Layer-
Fehler erfasst. Das ist kognitiv komplex — und im wasm32-Target ist das Backend `reqwest::wasm`
mit zT. anderen Branches. Eine kleine Smoke-Test-Funktion oder ein `#[cfg(test)]`-Test, der
verifiziert, dass das Mapping fuer realistische Errors die richtige Message liefert, waere
hilfreich.

**Fix:** Tests fuer `AppError::from(reqwest::Error)` ergaenzen (besonders fuer
`is_request()`-Branch).

### IN-06: `format_date_input` und `parse_date_input` doppelt in `fiscal_year_date_input.rs` und `member_details.rs`

**File:** `genossi-frontend/src/component/fiscal_year_date_input.rs:17-32` UND
`genossi-frontend/src/page/member_details.rs:34-36, 67-77`.

**Issue:** Phase 18 Plan kommentiert "minimal duplication per PATTERNS L-7" — also bewusste
Entscheidung. Aber: jetzt existieren zwei nicht-zentralisierte Date-Format-Helpers. Wenn
in 3 Monaten ein zusaetzlicher Edge-Case (z.B. `2026-2-3` ohne Zero-Padding) auftritt, muss
das an zwei Stellen gepatcht werden. Memory-Eintrag (`feedback_component_first.md`)
empfiehlt explizit ein Extract-into-shared-module-Pattern.

**Fix:** Beide Helpers in `src/util/date.rs` (oder bei den i18n-Helpers in `i18n/mod.rs`)
zentralisieren. Beide Call-Sites importieren von dort. Das eliminiert Drift-Risiko.

---

_Reviewed: 2026-06-07T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
