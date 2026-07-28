---
phase: 28-desktop-mobile-vorschau
plan: 04
subsystem: frontend
tags: [dioxus, props, signal, preview, grep-gate, privacy, call-site-wiring]

# Dependency graph
requires:
  - phase: 28-desktop-mobile-vorschau
    plan: 03
    provides: "`WysiwygEditor` mit den zwei `#[props(default)]`-Props `preview_member_id` und `repayment_phase_id`"
  - phase: 24-wysiwyg-frontend-editor
    provides: "Grep-Gate-Muster inkl. zweischichtiger Self-Reference-Abwehr, `#[props(default)]`-Migrationsmuster"
  - phase: 10
    provides: "`TemplatePreview` mit `POST /api/mail/preview`-Anbindung"
provides:
  - "`TemplatePreview` mit Pflicht-Prop `preview_member_id: Signal<Option<Uuid>>` (D-03) plus `value`-Bindung am Auswahlfeld"
  - "`TemplateTester` mit Pflicht-Prop `selected_member_id: Signal<Option<Uuid>>`, durchgereicht an `MemberSearch` und `TemplatePreview`"
  - "Genau eine Mitglieds-Auswahl je Seite in `mail_page.rs` und `mail_templates.rs`, die BEIDE Vorschauen speist"
  - "`reply_form.rs` nach D-03-Ausstiegsklausel verkabelt: `member_uuid_opt` direkt an die Device-Vorschau"
  - "4 neue Tests: 3 inhaltliche Grep-Gates + 1 Meta-Test des neuen Gate-Moduls"
affects: [28-05-uat]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pflicht-Prop statt `#[props(default)]`, wenn der Prop-Typ ein `Signal` ist: ein Default legt bei jedem Eltern-Render ein neues Signal im Eltern-Scope an, verliert Zustand und akkumuliert Signale (T-28-23). Zulaessig, weil alle Aufrufstellen im selben Plan mitgezogen werden"
    - "State-Lifting auf Call-Site-Ebene mit ZWEI Verbrauchern: die schreibende Component behaelt ihr Eingabefeld, die Page reicht dasselbe Signal zusaetzlich an einen zweiten, nur lesenden Verbraucher"
    - "Ausstiegsklausel call-site-weise statt global: wo der Wert bereits eindeutig bekannt ist, wird er direkt durchgereicht statt eine Auswahl hochzuziehen"
    - "Grep-Gate mit Fenster-Suche als Datenschutz-Invariante: Needle auf den Funktionsaufruf, Fenster von 200 Zeichen, Assertion auf das erwartete Argument"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/component/mail_compose/template_preview.rs"
    - "genossi-frontend/src/component/mail_compose/template_tester.rs"
    - "genossi-frontend/src/page/mail_page.rs"
    - "genossi-frontend/src/page/mail_templates.rs"
    - "genossi-frontend/src/component/inbox/reply_form.rs"

key-decisions:
  - "`mut preview_member_id: Signal<Option<Uuid>>` direkt in der Component-Signatur statt Rebinding im Rumpf: Dioxus' `#[component]`-Makro unterstuetzt `mut` an Props nachweislich (Vorbild `MailAttachmentPicker`), das spart eine Indirektion"
  - "Kommentar in `mail_templates.rs` umformuliert, damit das Wort `repayment_phase_id` dort nicht mehr vorkommt: das Akzeptanzkriterium ist ein Grep-Gate auf genau diesen Token und soll aussagekraeftig bleiben, statt durch eine blosse Erwaehnung im Kommentar entwertet zu werden"
  - "Negativ-Nachweis fuer `preview_member_id_is_a_prop_not_a_local_signal` in der SUBTILEN Variante (Prop bleibt, wird aber von einem lokalen Signal beschattet): das vollstaendige Entfernen des Props bricht den Build an den Aufrufstellen, der Test kaeme gar nicht zur Ausfuehrung. Die subtile Variante ist ausserdem die realistische Regression"
  - "Das lokale Vorschau-Signal in `reply_form.rs` startet als `None` und wird NICHT mit `member_uuid_opt` vorbelegt (T-28-24) — eine Vorbelegung waere eine unbeabsichtigte Verhaltensaenderung der dortigen `TemplatePreview`"

patterns-established:
  - "Wenn ein Grep-Gate durch Entfernen eines Pflicht-Props nicht mehr ausfuehrbar ist, den Negativ-Nachweis ueber Beschattung fuehren — der Build bleibt gruen und der Gate beweist genau die stille Regression, gegen die er gerichtet ist"

requirements-completed: [PREV-02, PREV-03]

coverage:
  - id: D1
    description: "`TemplatePreview` bezieht die Mitglieds-Auswahl als Pflicht-Prop; ein Rueckfall auf ein lokales Signal wuerde die D-03-Kopplung lautlos aufheben und die Device-Vorschau dauerhaft auf der Hinweiszeile stehen lassen (T-28-20)"
    requirement: "PREV-02"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/template_preview.rs#preview_member_id_is_a_prop_not_a_local_signal"
        status: pass
    human_judgment: false
  - id: D2
    description: "Die Test-Mail-Adresse im `TemplateTester` stammt weiterhin ausschliesslich aus dem Test-Empfaenger-Feld, nie aus dem gewaehlten Mitglied — auch nachdem die Mitglieds-Auswahl von aussen gesteuert wird (T-28-19)"
    requirement: "PREV-02"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/template_tester.rs#test_mail_recipient_comes_from_test_address_only"
        status: pass
    human_judgment: false
  - id: D3
    description: "`TemplateTester` reicht sein Signal tatsaechlich an `TemplatePreview` weiter — sonst haette der Template-Editor wieder zwei konkurrierende Mitglieds-Auswahlen"
    requirement: "PREV-02"
    verification:
      - kind: unit
        ref: "genossi-frontend/src/component/mail_compose/template_tester.rs#selected_member_id_is_forwarded_to_preview"
        status: pass
    human_judgment: false
  - id: D4
    description: "Alle drei Call-Sites sind verkabelt, der Crate baut, und die bestehende Frontend-Suite bleibt vollstaendig gruen"
    requirement: "PREV-02, PREV-03"
    verification:
      - kind: command
        ref: "cd genossi-frontend && cargo build && cargo test && cargo clippy --all-targets"
        status: pass
    human_judgment: false
  - id: D5
    description: "Kein Backend-Regress durch die Frontend-Arbeit: die Backend-Suite bleibt exakt auf dem Ausgangsstand"
    requirement: "PREV-02"
    verification:
      - kind: command
        ref: "cargo test -p genossi_mail (279 passed) + cargo test -p genossi_bin --test e2e_tests (314 passed / 2 pre-existing failed)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Der Vorstand sieht je Seite genau eine Mitglieds-Auswahl und erkennt, dass sie beide Vorschauen steuert; im Template-Editor ist die frueher doppelte Auswahl verschwunden"
    requirement: "PREV-02"
    verification: []
    human_judgment: true
    rationale: "Die Verkabelung ist am Quelltext und ueber drei Grep-Gates nachgewiesen. Ob die verbleibende eine Auswahl fuer den Vorstand auch bedienbar und eindeutig WIRKT — insbesondere ob das `value`-gebundene Auswahlfeld der TemplatePreview und die MemberSearch des TemplateTester sichtbar synchron laufen — ist eine Beurteilungsfrage im laufenden Browser. Gehoert in die UAT in Plan 28-05."
  - id: D7
    description: "PREV-02 und PREV-03 sind im Browser wirksam: mit gewaehltem Mitglied rendert die Device-Vorschau die sanitisierte, variablen-aufgeloeste Fassung, und Bilder erscheinen darin"
    requirement: "PREV-02, PREV-03"
    verification: []
    human_judgment: true
    rationale: "Erst dieser Plan macht die Vorschau ueberhaupt renderbar (bis 28-03 war `preview_member_id` ueberall `None`). Ob die gerenderte Fassung inhaltlich korrekt aussieht und ob Bilder tatsaechlich laden, laesst sich nur gegen ein laufendes Backend mit echten Mitgliedsdaten beurteilen. Kern-UAT von Plan 28-05."

# Metrics
duration: 14min
completed: 2026-07-28
status: complete
---

# Phase 28 Plan 04: Call-Site-Verkabelung Summary

**Die Mitglieds-Auswahl fuer die Vorschau ist aus `TemplatePreview` heraus auf die Call-Site-Ebene gehoben und speist dort BEIDE Verbraucher — die neue Device-Vorschau im `WysiwygEditor` und die unveraendert bestehende `TemplatePreview`; im Template-Editor verschwindet dadurch die frueher doppelte Auswahl, und die Datenschutzregel des `TemplateTester` ist zusaetzlich per Grep-Gate festgenagelt.**

## Performance

- **Duration:** ~14 min
- **Tasks:** 3/3
- **Commits:** 3 (`04c18d5`, `11d9feb`, `399c5a3`)
- **Dateien:** 0 neu, 5 modifiziert — kein Cargo-Manifest, keine neue Dependency (T-28-SC)

## (d) Endgueltige Testanzahl

`cd genossi-frontend && cargo test` → **331 passed, 0 failed** (Baseline aus Plan 28-03: 327 + 4 neue).

| Filter | Ergebnis |
|---|---|
| `cargo test preview_member_id_is_a_prop_not_a_local_signal` | 1 passed |
| `cargo test test_mail_recipient_comes_from_test_address_only` | 1 passed |
| `cargo test selected_member_id_is_forwarded_to_preview` | 1 passed |
| `cargo test grep_gate` | 19 passed (vorher 13 + die 4 neuen + 2 bestehende aus `template_preview.rs`) |

Der Plan hatte 3 neue Tests und mindestens 326 vorhergesagt. Es sind **4** geworden, weil das
neu angelegte `grep_gate_tests`-Modul in `template_tester.rs` zusaetzlich den vom Plan selbst
geforderten Meta-Test `production_region_excludes_test_module` traegt.

`cargo build` exit 0. `cargo clippy --all-targets` exit 0 — keine Warnung auf einer der neu
hinzugefuegten Zeilen (geprueft: die Treffer in `mail_page.rs` und `reply_form.rs` liegen
saemtlich auf vorbestehenden `use_signal`-Zeilen, nicht auf den Zeilen 71 / 44 / 102 mit den
neuen Signalen). `cargo fmt -- --check` meldet fuer die fuenf beruehrten Dateien nichts; die
einzige verbleibende Fundstelle im Crate ist die vorbestehende `api.rs`-Drift.

## (a) Endgueltige Signaturen

```rust
#[component]
pub fn TemplatePreview(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    #[props(default)] body_html: ReadOnlySignal<String>,
    member_ids: Vec<Uuid>,
    #[props(default)] repayment_phase_id: Option<Uuid>,
    mut preview_member_id: Signal<Option<Uuid>>,
) -> Element
```

```rust
#[component]
pub fn TemplateTester(
    subject: ReadOnlySignal<String>,
    body: ReadOnlySignal<String>,
    #[props(default)] body_html: ReadOnlySignal<String>,
    mut selected_member_id: Signal<Option<Uuid>>,
) -> Element
```

| Prop | Typ | Default | Semantik |
|---|---|---|---|
| `TemplatePreview.preview_member_id` | `Signal<Option<Uuid>>` | **keiner (Pflicht)** | schreibbar: das Auswahlfeld dieser Component schreibt hinein, die Page liest denselben Wert fuer die Device-Vorschau |
| `TemplateTester.selected_member_id` | `Signal<Option<Uuid>>` | **keiner (Pflicht)** | an `MemberSearch` (`selected_id` + `on_select`) und an `TemplatePreview` (`preview_member_id`) durchgereicht |

**Warum bewusst kein `#[props(default)]`** (T-28-23, entgegen dem sonstigen Projektmuster aus
Phase 24): Der Default-Wert eines `Signal` wuerde bei jedem Render der Elternkomponente ein
neues Signal im Eltern-Scope anlegen — der Zustand ginge bei jedem Render verloren und die
Signale wuerden akkumulieren. Pflicht-Props sind hier unproblematisch, weil alle vier
Aufrufstellen in diesem Plan liegen und mitgezogen wurden.

`mut` steht direkt in der Component-Signatur. Dioxus' `#[component]`-Makro unterstuetzt das
nachweislich — Vorbild ist `MailAttachmentPicker` (`mut selected_member_doc_ids: Signal<Vec<Uuid>>`).

### Verkabelung je Call-Site

| Datei | Signal | an `WysiwygEditor` | an Vorschau-Component |
|---|---|---|---|
| `page/mail_page.rs` | `preview_member_id` (neu) | `preview_member_id: *…read()` **und** `repayment_phase_id: *…read()` | `TemplatePreview.preview_member_id` (Signal selbst) |
| `page/mail_templates.rs` | `preview_member_id` (neu) | `preview_member_id: *…read()`, **kein** Rueckzahlungs-Kontext | `TemplateTester.selected_member_id` (Signal selbst) |
| `component/inbox/reply_form.rs` | `preview_member_id` (neu, nur fuer `TemplatePreview`) | `member_uuid_opt` **direkt** (Ausstiegsklausel) | `TemplatePreview.preview_member_id` (lokales Signal) |

## (b) Wie sich der Template-Editor jetzt bedient — relevant fuer die UAT in Plan 28-05

**Vorher** standen auf `mail_templates.rs` zwei verschachtelte Mitglieds-Auswahlen auf
derselben Seite: die `MemberSearch` im `TemplateTester` und, darin verschachtelt, das
`select`-Auswahlfeld der `TemplatePreview`. Beide hatten ein eigenes, unabhaengiges Signal.
Der Vorstand musste zweimal dasselbe Mitglied waehlen und konnte nicht erkennen, welche
Auswahl fuer welche Anzeige gilt.

**Jetzt** fuehren beide dasselbe Page-Signal:

1. Der Vorstand waehlt ein Mitglied in der `MemberSearch` des `TemplateTester`.
2. Dadurch wird `preview_member_id` der Page gesetzt.
3. Die `TemplatePreview` erscheint (ihre `is_some()`-Bedingung ist erfuellt) und ihr
   Auswahlfeld zeigt **dank der neuen `value`-Bindung** dasselbe Mitglied an, statt auf dem
   Platzhalter stehen zu bleiben.
4. Gleichzeitig bekommt die Device-Vorschau im `WysiwygEditor` denselben Wert und rendert im
   Desktop-/Mobile-Modus die variablen-aufgeloeste Fassung.

Umgekehrt gilt dasselbe: waehlt der Vorstand im Auswahlfeld der `TemplatePreview`, wandert
der Wert ueber dasselbe Signal zurueck in die `MemberSearch` und in die Device-Vorschau.
Genau diese Zweiwege-Kopplung ist der Grund fuer ein schreibbares `Signal` statt eines
`ReadOnlySignal`.

**Bewusst NICHT ergaenzt:** ein automatischer Vorschau-Trigger bei externer Aenderung des
Signals. Der Aktualisieren-Button der `TemplatePreview` ist bereits sichtbar, sobald ein
Mitglied gewaehlt ist; ein zusaetzlicher Effekt waere Scope-Creep und wuerde bei jeder
Member-Aenderung einen Request ausloesen.

**Wichtig fuer die UAT:** Der `key`-Bump auf dem `WysiwygEditor` ist unveraendert. Beim
Wechsel des bearbeiteten Templates wird der Editor remountet und der Vorschau-Modus faellt
auf `Bearbeiten` zurueck (T-28-18, in Plan 28-03 als bewusst akzeptiert dokumentiert). Die
Mitglieds-Auswahl ueberlebt den Remount hingegen, weil sie jetzt in der Page liegt.

**`reply_form.rs`** verhaelt sich absichtlich anders: dort ist genau ein Mitglied bekannt und
es gibt keine Auswahl, die mit einer zweiten konkurrieren koennte. Die Device-Vorschau bekommt
`member_uuid_opt` direkt; es wurde **keine** zusaetzliche Auswahl eingebaut. Die dortige
`TemplatePreview` behaelt ihr eigenes, mit `None` startendes lokales Signal und verhaelt sich
damit exakt wie vor Phase 28 (T-28-24). Ist kein Mitglied zugeordnet, zeigt die Device-Vorschau
die Hinweiszeile aus Plan 28-03 statt eines leeren Rahmens.

## (c) Ergebnis des Backend-e2e-Laufs

`cargo build` (Workspace-Root) exit 0. `cargo test -p genossi_mail` → **279 passed, 0 failed**.

`cargo test -p genossi_bin --test e2e_tests` → **314 passed, 2 failed** — exakt die in der
Wave-Uebergabe genannte Baseline. Beide Fehlschlaege sind die dokumentierten Pre-existing
Failures aus Phase 22/24 (`cfa3794`), im Wortlaut unveraendert:

```
---- preview_body_html_round_trips_to_response stdout ----
thread 'preview_body_html_round_trips_to_response' panicked at
genossi_bin/tests/e2e_tests.rs:14961:5:
assertion `left == right` failed: plain body must render member first_name
  left: "Hallo **Max**"
 right: "Hallo Max"

---- test_mail_preview_repayment_no_entries_does_not_default_to_one stdout ----
thread 'test_mail_preview_repayment_no_entries_does_not_default_to_one' panicked at
genossi_bin/tests/e2e_tests.rs:14628:44:
errors must be array
```

Der Plan erwartete beim Filter `preview` **einen** Fehlschlag; es sind **zwei**, weil der
Filter `preview` auf beide dokumentierten Pre-existing Failures passt (die Wave-Uebergabe
listet sie ausdruecklich beide). Kein Regress: die Gesamtzahl 314/2 stimmt exakt mit der
Ausgangslage ueberein.

## Negativ-Nachweise der neuen Gates

Beide inhaltlichen Gates wurden aktiv gegengeprueft; die Aenderungen wurden unmittelbar
zurueckgenommen und die beiden Dateien stimmen byte-genau mit dem committeten Stand ueberein.

**Gate 1 — `preview_member_id_is_a_prop_not_a_local_signal`.** Die naheliegende Variante
(Prop entfernen und wieder ein `use_signal` anlegen) bricht den Build an den drei
Aufrufstellen, der Test kaeme gar nicht zur Ausfuehrung. Der Nachweis wurde deshalb in der
**subtilen** Variante gefuehrt, die zugleich die realistische stille Regression ist: das Prop
bleibt in der Signatur, wird im Rumpf aber von einem lokalen Signal beschattet. Der Crate baut
weiter, die Page setzt weiterhin ihr Signal — und die Vorschau reagiert nie darauf.

```
thread 'component::mail_compose::template_preview::grep_gate_tests::preview_member_id_is_a_prop_not_a_local_signal'
panicked at src/component/mail_compose/template_preview.rs:307:9:
Grep gate FAILED: in template_preview.rs wird wieder ein LOKALES Member-Signal angelegt.
Das hebt die D-03-Kopplung lautlos auf: das Auswahlfeld zeigt ein Mitglied, die
Device-Vorschau bleibt dauerhaft bei der Hinweiszeile, weil sie ein anderes Signal liest.
Die Auswahl gehoert in die Call-Site und wird von dort hereingereicht.

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out
```

**Gate 2 — `test_mail_recipient_comes_from_test_address_only` (T-28-19, Datenschutz).**
Das erste Argument des Sende-Aufrufs testweise von der Test-Adresse auf eine andere Variable
umgestellt:

```
thread 'component::mail_compose::template_tester::grep_gate_tests::test_mail_recipient_comes_from_test_address_only'
panicked at src/component/mail_compose/template_tester.rs:270:9:
Grep gate FAILED: die Empfaengeradresse der Test-Mail stammt nicht mehr aus dem
Test-Adress-Feld. DATENSCHUTZREGEL (Genossi, Datensparsamkeit): die Test-Mail darf niemals
an die Adresse des gewaehlten Mitglieds gehen — das Mitglied liefert ausschliesslich die
Template-Variablen ueber seine Id. Fenster hinter dem Sende-Aufruf (erste 200 Zeichen):
send_test_mail_with_template(
                                &config, &member_addr, &subj, &bdy, &mid_str,
                            )
                            .await

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 330 filtered out
```

Damit ist belegt, dass beide Gates genau die Eigenschaft pruefen, fuer die sie da sind, und
nicht bloss zufaellig mitlaufen.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blockierend] `use uuid::Uuid;` in `mail_templates.rs` ergaenzt**

- **Gefunden waehrend:** Task 2
- **Problem:** Die Datei hatte bis dahin keinen `Uuid`-Import; das neue
  `use_signal(|| None::<Uuid>)` liess sich ohne ihn nicht uebersetzen.
- **Fix:** `use uuid::Uuid;` unter `use dioxus::prelude::*;`, analog zu `reply_form.rs`.
- **Datei:** `genossi-frontend/src/page/mail_templates.rs`
- **Commit:** `11d9feb`

**2. [Rule 1 - Bug] `Ref::map` beschattet `Option::map` bei der `value`-Bindung**

- **Gefunden waehrend:** Task 1
- **Problem:** `preview_member_id.read().map(|id| id.to_string())` uebersetzt nicht —
  `Signal::read()` liefert ein `Ref<Option<Uuid>>`, und `.map` trifft `Ref::map` statt
  `Option::map` (`E0599: the method to_string exists for struct Ref<'_, Option<Uuid>>, but
  its trait bounds were not satisfied`).
- **Fix:** Explizit dereferenzieren: `(*preview_member_id.read()).map(…)`. Begruendung als
  Kommentar an der Stelle hinterlegt, damit es nicht spaeter „vereinfacht" wird.
- **Datei:** `genossi-frontend/src/component/mail_compose/template_preview.rs`
- **Commit:** `04c18d5`

**3. [Rule 3 - Blockierend] Gezieltes `rustfmt` statt crate-weitem `cargo fmt`**

- **Gefunden waehrend:** Task 1, 2 und 3
- **Problem:** Ein `cargo fmt` ueber den Crate haette zugleich die vorbestehende Drift in
  `src/api.rs:405` mitveraendert und damit eine unbeteiligte Datei in den Plan-Diff gezogen —
  was das repo-spezifische Git-Protokoll ausdruecklich verbietet. Identische Abweichung wie in
  den Plaenen 28-02 (Abweichung 2) und 28-03 (Abweichung 3).
- **Fix:** `rustfmt --edition 2021 <datei>` auf den jeweils beruehrten Dateien. Danach ist
  `cargo fmt -- --check` fuer alle fuenf sauber.
- **Nachkontrolle:** Suite nach jedem Formatieren erneut gruen — insbesondere die Grep-Gates,
  deren Needles auf exakte Byte-Sequenzen zielen.

**4. [Rule 1 - Bug] Kommentar in `mail_templates.rs` umformuliert, um das eigene Grep-Gate
nicht zu entwerten**

- **Gefunden waehrend:** Task 2
- **Problem:** Das Akzeptanzkriterium `grep -c 'repayment_phase_id' mail_templates.rs` = 0 ist
  ein Grep-Gate darauf, dass dieses Prop auf der Template-Seite nicht gesetzt wird. Mein
  erklaerender Kommentar nannte den Token woertlich und lieferte 1 statt 0 — der Gate haette
  ab sofort auch dann angeschlagen, wenn das Prop weiterhin korrekt ungesetzt bleibt, und
  waere damit als Signal wertlos geworden.
- **Fix:** Kommentar auf „Das optionale Rueckzahlungs-Kontext-Prop wird hier bewusst NICHT
  gesetzt …" umformuliert. Die Begruendung bleibt vollstaendig erhalten, der Token kommt in
  der Datei nicht mehr vor.
- **Datei:** `genossi-frontend/src/page/mail_templates.rs`
- **Commit:** `11d9feb`

### Abweichungen bei Akzeptanzkriterien (Formulierung, nicht Substanz)

**5. Task 1 — `cargo build 2>&1 | grep -c "missing field"` ergibt 0 statt einer positiven Zahl**

Dioxus meldet ein fehlendes Pflicht-Prop nicht als „missing field", sondern ueber seinen
Typestate-Builder als `error[E0061]: this method takes 1 argument but 0 arguments were
supplied`, verortet auf dem `rsx!`-Block der Aufrufstelle. Die Absicht des Kriteriums — nach
Task 1 brechen ausschliesslich die Aufrufstellen, nicht die beiden geaenderten Dateien — wurde
stattdessen direkt geprueft: nach Task 1 exakt 3 Fehler, alle in `reply_form.rs`,
`mail_page.rs` und `mail_templates.rs`, keiner in `template_preview.rs` oder
`template_tester.rs`.

**6. Task 2 — `cargo build 2>&1 | grep -c 'mail_page.rs\|mail_templates.rs'` ergibt 1 statt 0**

Der eine Treffer ist eine **vorbestehende Warnung**, kein Fehler:
`mail_page.rs:74: variable does not need to be mutable` fuer
`let mut repayment_phase_id = use_signal(…)`. Das Signal wird nirgends geschrieben (verifiziert:
`grep -c 'repayment_phase_id.set\|repayment_phase_id.write'` = 0 bereits im Ausgangsstand), die
Warnung lag also schon vor diesem Plan an. Auf Fehler eingeschraenkt ergibt das Kriterium 0.
Bewusst nicht gefixt — vorbestehend und ausserhalb der Scope Boundary.

**7. Task 3 — `cargo test -p genossi_bin --test e2e_tests preview` zeigt 2 statt 1 Fehlschlag**

Der Filter `preview` trifft beide dokumentierten Pre-existing Failures. Siehe Abschnitt (c);
die Gesamtzahl 314/2 entspricht exakt der Baseline.

**8. Testanzahl 331 statt der vorhergesagten „mindestens 326"**

4 neue Tests statt 3, weil das neue `grep_gate_tests`-Modul in `template_tester.rs` den vom
Plan selbst geforderten Meta-Test `production_region_excludes_test_module` mitbringt.

### Bewusst NICHT gefixt (Scope Boundary)

- **`cargo fmt`-Drift in `genossi-frontend/src/api.rs:405`** — vorbestehend, unberuehrte Datei,
  kein Phase-28-Bezug. Bereits als `deferred-items.md` Punkt 5 ausgelagert.
- **Vorbestehende `unused mut`- und `redundant closure`-Warnungen** in `mail_page.rs`,
  `reply_form.rs` und weiteren Dateien — keine liegt auf einer in diesem Plan hinzugefuegten
  Zeile (geprueft gegen die neuen Signal-Zeilen 71 / 44 / 102).
- **`genossi-frontend/Cargo.lock`** — wird bei jedem Build wegen des datumsbasierten
  Dev-Version-Strings neu geschrieben. In keinem Commit gestaged, im Arbeitsverzeichnis dirty
  gelassen. Es wurde kein Paket installiert (T-28-SC).
- **Die beiden vorbestehenden Backend-Testfehlschlaege** — dieser Plan fasst kein Backend-File
  an. Bereits als `deferred-items.md` Punkt 1 und 2 erfasst.

## Authentication Gates

Keine.

## Known Stubs

Keine. Alle drei Call-Sites sind vollstaendig verkabelt; `preview_member_id` ist nirgends mehr
hartkodiert `None`. Der `None`-Fall (kein Mitglied gewaehlt bzw. kein Mitglied zugeordnet) ist
ein vollstaendig implementierter, getesteter Pfad mit eigener Benutzerfuehrung
(`MailEditorModeSelectMember`).

## Threat Flags

Keine neue Angriffsflaeche ausserhalb des Threat Models. Die fuenf `mitigate`-Dispositionen
dieses Plans sind umgesetzt und belegt:

| Threat | Umsetzung | Beweis |
|---|---|---|
| T-28-19 (Test-Mail-Empfaenger) | Versand-Handler unveraendert; Empfaenger allein aus dem Test-Adress-Feld; Modul-Doc um Phase-28-Hinweis ergaenzt, drei Verteidigungsschichten intakt | `test_mail_recipient_comes_from_test_address_only` + Negativ-Nachweis; `grep -c 'Privacy defense layers'` = 1; `grep -c 'is_valid_test_address(&addr)'` = 1 |
| T-28-20 (Rueckfall auf lokales Signal) | Pflicht-Prop statt `use_signal` | `preview_member_id_is_a_prop_not_a_local_signal` + subtiler Negativ-Nachweis |
| T-28-22 (Submit-Guards) | alle drei Guards unangetastet | `git diff -U0` der drei Dateien enthaelt **null** Zeilen mit `get_element_by_id` |
| T-28-23 (Signal-Akkumulation) | kein `#[props(default)]` an den beiden Signal-Props; alle vier Aufrufstellen im selben Plan mitgezogen | Signaturen oben; `cargo build` exit 0 |
| T-28-24 (Verhaltensaenderung im Reply-Formular) | lokales Vorschau-Signal startet als `None`, nicht mit `member_uuid_opt` vorbelegt | Kommentar an der Deklaration; `TemplatePreview`-Bedingung und `member_ids`-Konstruktion unveraendert |
| T-28-21 (Member-Id im Request) | `accept` — bestehender, session-gated `/api/mail/preview`, kein neues Feld | keine Aenderung an `api.rs` |
| T-28-SC (Package-Legitimacy) | kein Install, kein Manifest im Diff | `git diff --name-only` listet genau die fuenf Quelldateien |

## Offene Punkte fuer die UAT (Plan 28-05)

Ab jetzt sinnvoll durchfuehrbar — bis 28-03 war `preview_member_id` ueberall `None`.

- Mitglied waehlen und pruefen, dass Device-Vorschau **und** `TemplatePreview` dasselbe
  Mitglied zeigen (auf beiden Seiten).
- Im Template-Editor: dass wirklich nur noch **eine** Auswahl sichtbar ist und das
  Auswahlfeld der `TemplatePreview` dank `value`-Bindung mitzieht.
- Bilder in der Vorschau (PREV-03).
- Auf `mail_page.rs`: Repayment-Variablen loesen auch in der Device-Vorschau auf.
- Auf `mail_templates.rs`: ein Template mit Repayment-Platzhaltern soll den Render-Fehler
  sichtbar im roten Fehler-Block zeigen (bewusst kein Rueckzahlungs-Kontext auf dieser Seite).
- Im Reply-Formular: Device-Vorschau ohne zusaetzliche Auswahl; ohne zugeordnetes Mitglied
  erscheint die Hinweiszeile.
- Weiterhin offen aus 28-03: optische Hervorhebung des aktiven Modus-Buttons, kein Flackern
  beim Wechsel Desktop ↔ Mobile, Zeilenumbrueche im `text/plain`-Teil nach Vorschau → Senden.

## Self-Check: PASSED

Alle fuenf behaupteten Dateien existieren auf der Platte, alle drei Commit-Hashes (`04c18d5`,
`11d9feb`, `399c5a3`) sind in `git log` auffindbar, und `git diff --name-only` gegen den
Ausgangsstand listet genau diese fuenf Dateien.
