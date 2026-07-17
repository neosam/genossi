# Phase 26: Editor-Formatierung vervollständigen - Context

**Gathered:** 2026-07-17
**Status:** Ready for planning

<domain>
## Phase Boundary

Der v1.4-WYSIWYG-Editor bekommt die **Round-Trip-Garantie** für Listen (`<ul>`/`<ol>`) und Überschriften (`<h2>`/`<h3>`, plus behaltenes `<h1>`), einen **mechanischen Grep-Gate** der `styleWithCSS=false` und Paste-Plain vor Regressionen schützt, sowie den **nachgeholten Vorstand-UAT-Smoke-Test** aus Phase 24 (12 alte Punkte + 4 neue).

**Überraschende Ausgangs-Erkenntnis beim Scouten:** Die geforderten Toolbar-Buttons (UL/OL/H2/H3, plus H1/¶/❝) sind **bereits alle in `wysiwyg_toolbar.rs` verdrahtet** — Phase 24 hat den vollen Umfang gebaut, die Spec-Nummerierung (EDIT-06..08) impliziert das aber. Was fehlt ist der **Beweis** dass Round-Trip funktioniert (kein Test), der **Grep-Gate** (EDIT-01/02 hatten keinen), und die **UAT-Deferral-Auflösung**.

**In scope:**
- E2E-Round-Trip-Test in `genossi_bin/tests/e2e_tests.rs` (Save→GET→Editor byte-identisch für UL/OL/H2/H3 Body).
- Unit-Tests in `genossi_mail/src/sanitize.rs` (`sanitize_preserves_ul_ol_h2_h3`, ein Test pro Element-Familie).
- Neuer Rust-Test in `genossi-frontend` der via `include_str!` das `wysiwyg_editor.rs` liest und `styleWithCSS`-Guard + `evt.prevent_default()` in `onpaste` mechanisch nachweist.
- `26-UAT-CHECKLIST.md` = Copy von `24-UAT-CHECKLIST.md` + neue Steps 13-16 (UL/OL/H2/H3 Round-Trip), Vorstand-Smoke-Termin.
- Ggf. Toolbar-Icon/Label-Politur (falls beim UAT auffällt).

**Out of scope (bewusst):**
- KEINE Änderung an `sanitize.rs`-Regeln (Default-Whitelist bleibt) — Konsequenz: Backward-Compat für v1.4-Templates folgt aus Nicht-Änderung.
- KEINE neuen Toolbar-Buttons (H1 bleibt drin, nicht rausnehmen; keine Text-Align, Font-Color).
- KEIN Bild-Support — Phase 27.
- KEINE Preview-Modi (Desktop/Mobile) — Phase 28.
- KEIN neuer Editor-Framework (execCommand bleibt, EDIT-02-Constraint aus Phase 24).

</domain>

<decisions>
## Implementation Decisions

### GA-1: H1-Button bleibt in der Toolbar (D-01)
- H1 ist heute in `wysiwyg_toolbar.rs` (siehe `formatBlock <h1>`-Handler). Spec (EDIT-08) fordert nur H2/H3 — aber H1 ist keine Regression, ammonia lässt `<h1>..<h6>` per Default durch, UAT-Checklist Phase 24 zählt „13 Buttons" inkl. H1.
- **Entscheidung:** H1 nicht entfernen. User: „Überschriften machen durchaus Sinn." Der Round-Trip-Test deckt zusätzlich `<h1>` mit ab.

### GA-2: Grep-Gate als Rust-`include_str!`-Test (D-02)
- Neuer Test in `genossi-frontend` (Ziel: `genossi-frontend/src/component/mail_compose/mod.rs` oder eigene `tests/wysiwyg_source_invariants.rs`), der:
  1. `include_str!("wysiwyg_editor.rs")` lädt.
  2. Asserted: `contains("exec_command_bool(&doc, \"styleWithCSS\", false)")` — der Guard existiert exakt.
  3. Asserted: `contains("evt.prevent_default()")` im Kontext der `onpaste`-Closure (Regex/Substring-Nachbarschaft: „onpaste" gefolgt von „prevent_default" innerhalb weniger Zeilen).
- Läuft mit `cargo test` (kein extra Husky, keine CI-Infra). Versioniert im Code, User: „(b)".
- **Warum:** EDIT-09 verlangt einen Gate „analog EDIT-01/02" — den gab es aber nie als Skript. Wir bauen das Muster **jetzt** neu, im Rust-Test-Framework statt Shell — passt zu vorhandener Test-Kultur, kein neuer Tooling-Wildwuchs.

### GA-3: Round-Trip via Unit-Test + E2E-Test (D-03)
- **Unit-Tests** in `genossi_mail/src/sanitize.rs`:
  - `sanitize_preserves_unordered_list` — `<ul><li>a</li><li>b</li></ul>` überlebt.
  - `sanitize_preserves_ordered_list` — `<ol><li>1</li><li>2</li></ol>` überlebt.
  - `sanitize_preserves_headings_h1_h2_h3` — alle drei Tags überleben.
- **Ein E2E-Test** in `genossi_bin/tests/e2e_tests.rs` analog zu `bulk_mail_body_html_sanitized_and_persisted`:
  - `template_body_html_with_lists_and_headings_round_trips` — POST Template mit `body_html: "<h2>Titel</h2><ul><li>a</li></ul><ol><li>b</li></ol><h3>Sub</h3>"`, GET zurück, assert alle Tags/Text erhalten (ammonia-normalisiert, byte-exakt wo möglich).
- **E2E-Suite läuft heute** — 308 Tests grün beim Scouten, 1 Pre-existing Fail (`test_mail_preview_repayment_no_entries_does_not_default_to_one` aus Phase 22, dokumentiert in STATE.md). Kein E2E-Blocker für Phase 26.
- User: „(c) — aber funktionieren e2e tests aktuell?" → verifiziert: ja.

### GA-4: Backward-Compat aus Nicht-Änderung (D-04)
- `sanitize.rs` wird in Phase 26 **nicht angefasst** (keine neuen `Builder`-Regeln, keine Attribut-Änderungen). Damit ist Success-Criterion #5 („bestehende v1.4-Templates rendern byte-identisch") als Konsequenz erfüllt — keine dedizierten Snapshot-Tests.
- **Ausstiegs-Klausel:** Wenn die neuen Unit-Tests (D-03) unerwartet zeigen, dass ammonia z. B. `<h1>` doch nicht durchlässt oder Attribute stripped, muss der Planner nachschieben (Snapshot-Test oder Ammonia-Config-Anpassung). Aktuelle Erwartung: alles ok, weil Ammonia-Default laut Doku `<h1>..<h6>, <ul>, <ol>, <li>` erlaubt.
- User: „(c)".

### GA-5: UAT-Checklist = Copy von Phase 24 + neue Steps (D-05)
- `26-UAT-CHECKLIST.md` = Copy von `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md` (Steps 1-12) + 4 neue Steps 13-16 für UL/OL/H2/H3 (Editor → Toolbar-Klick → innerHTML in DevTools → Save → Reload → innerHTML byte-identisch).
- **Setup-Sektion** aktualisieren auf 2026-07 (Skill `run-rust-backend-and-frontend`).
- Ein Sign-Off-Termin für alle 16 Punkte.
- User: „(a)".

### GA-6: UAT ist Ship-Gate, nicht Merge-Gate (D-06)
- User: „Wir benutzen jj. Das ist in Git in der Regel detached. Mach dir da keinen Stress."
- Interpretation: keine harte Merge-Blockade (jj-Changes sind eh WIP-detached, kein PR-Gate); der Sinn der Phase (UAT-Nachhol) bleibt aber gültig.
- **Entscheidung:** UAT MUSS vor **v1.5-Milestone-Close** (`/gsd-complete-milestone`) abgehakt sein — die Milestone-Audit-Skill prüft das. Innerhalb der Phase ist Code-fertig = Code-Reviews + automatische Tests grün. UAT läuft parallel/nachgelagert.
- **Konsequenz für Verifier:** Phase-VERIFICATION.md dokumentiert UAT-Status als „Pending Sign-Off" wenn Smoke noch nicht durch ist; kein Fail-Gate innerhalb der Phase.

### Claude's Discretion
- Exakte Datei/Modul-Location für den Grep-Gate-Test (`genossi-frontend/tests/wysiwyg_source_invariants.rs` vs. inline im `mod.rs`).
- Regex/Substring-Nachbarschafts-Heuristik im Grep-Gate (wie eng darf „prevent_default" bei „onpaste" stehen?).
- Ammonia-Normalisierungs-Toleranz im E2E-Test (byte-exakt vs. „enthält alle Tags").
- Icon-/Label-Politur beim UAT (falls sichtbar auffällt).
- Ob H2-Icon größer als H3-Icon dargestellt wird (kosmetisch, aktuell alle gleich).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — EDIT-06..10 (Zeilen suchen nach `EDIT-06`), Phase-26-Zuordnung
- `.planning/ROADMAP.md` §"Phase 26: Editor-Formatierung vervollständigen" — Goal, Success Criteria (5 Punkte), Depends-on Phase 25

### Vorbild aus Phase 24 (hartes Reuse)
- `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-CONTEXT.md` — Editor-Design, Toolbar-Constraints (D-05 gegen ammonia-Whitelist), Paste-Plain (D-07), Link-Modal (D-06)
- `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md` — **VORLAGE für 26-UAT-CHECKLIST.md**; Hard-Fail-Gates 3/4/5 werden übernommen
- `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-RESEARCH.md` — Pitfalls (styleWithCSS-Persistenz, Selection-Range-Verlust, Signal-Sync-Lag)

### Zu prüfende / zu testende Frontend-Stellen
- `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` — Ziel des Grep-Gates (styleWithCSS-Guard + onpaste-prevent_default)
- `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` — 13 Buttons vorhanden (B/I/U/S/UL/OL/H1/H2/H3/¶/❝/🔗/⊘), execCommand `insertUnorderedList`, `insertOrderedList`, `formatBlock <h2>/<h3>`
- `genossi-frontend/src/i18n/de.rs` + `en.rs` + `mod.rs` — Keys `MailEditorUnorderedList`, `MailEditorOrderedList`, `MailEditorHeading2`, `MailEditorHeading3` bereits vorhanden; falls neue Toolbar-Labels nötig, BEIDE Locales.

### Zu prüfende / zu testende Backend-Stellen
- `genossi_mail/src/sanitize.rs` — ammonia-Wrapper, aktuelle Tests: script/onclick/javascript:/data:/Jinja. **Fehlt:** UL/OL/H2/H3-Tests → D-03.
- `genossi_bin/tests/e2e_tests.rs` — `bulk_mail_body_html_sanitized_and_persisted` (~Zeile 14655) als Vorbild für den neuen Round-Trip-Test.
- `genossi_mail/src/rest_templates.rs` — Template-CRUD-Endpoints (POST/PUT/GET), `body_html`-Feld läuft durch `sanitize_html` bevor Persistenz.

### Projekt-/Frontend-Konventionen
- `CLAUDE.md` (Root) — Component-First, jj statt git (Memory)
- `genossi-frontend/CLAUDE.md` — Component-First-Prinzip, i18n zweisprachig (de.rs + en.rs), `Locale`-Enum hat nur `En` und `De`

### Bekannter Test-Ausfall (nicht Phase 26)
- `.planning/STATE.md` — dokumentiert `test_mail_preview_repayment_no_entries_does_not_default_to_one` als Pre-existing failure aus Phase 22; NICHT Phase-26-Regression.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`wysiwyg_toolbar.rs`** — komplette 13-Button-Toolbar bereits vorhanden inkl. UL/OL/H1/H2/H3. Phase 26 fasst diese Datei voraussichtlich NICHT an (nur Test-Nachweis + evtl. Icon-Cosmetik).
- **`wysiwyg_editor.rs`** — `styleWithCSS=false`-Guard in `onmounted`-Closure (Zeile ~80), `evt.prevent_default()` in `onpaste` (Zeile ~105) bereits richtig verdrahtet. Ziel des Grep-Gates: diese Stellen mechanisch schützen.
- **`genossi_mail/src/sanitize.rs::sanitize_html`** — 4 bestehende Unit-Tests (script/onclick/URL/Jinja); Phase 26 erweitert um 3 (UL/OL/Headings).
- **`bulk_mail_body_html_sanitized_and_persisted`** (`e2e_tests.rs` ~14655) — Vorlage für neuen Round-Trip-E2E-Test (POST `/api/mail/…` mit body_html, GET zurück, sanitize-assert).
- **`24-UAT-CHECKLIST.md`** — Copy-Vorlage für 26-UAT-CHECKLIST.md; Setup-Sektion identisch (Skill `run-rust-backend-and-frontend`), Steps 1-12 unverändert übernehmen.

### Established Patterns
- **Component-First** (Memory `feedback_component_first`): keine neue Component nötig — reine Test-/Verifikations-Phase.
- **jj statt git** (Memory `feedback_use_jj_not_git`): Commits via `jj commit -m …`.
- **Discuss/Feedback im Chat** (Memory `feedback_discuss_in_chat_form`): keine AskUserQuestion-Popups im Downstream (research/plan).
- **Enum statt Boolean** (Memory `feedback_enum_not_boolean`): kein bool-Toggle in Sanitize-Config; ammonia-Default bleibt (D-04).
- **Dioxus Button-Reload-Bug** (Memory `feedback_dioxus_button_type`): bereits als `r#type: "button"` in Toolbar umgesetzt.
- **`include_str!`-basierte Source-Invariant-Tests** — Muster **neu** in dieser Phase (D-02); ist Vorbild für zukünftige Grep-Gates.

### Integration Points
- **Grep-Gate-Test** (`genossi-frontend/tests/wysiwyg_source_invariants.rs` oder inline): läuft mit `cargo test --bin genossi-frontend` oder `cargo test -p …-frontend`; frontend-target ist `wasm32-unknown-unknown`, aber ein reiner String-Test läuft auch native.
- **Sanitize-Unit-Tests** (`genossi_mail/src/sanitize.rs`): `cargo test -p genossi_mail --lib`; 252 baseline → 255 nach Phase 26.
- **E2E-Round-Trip** (`genossi_bin/tests/e2e_tests.rs`): `cargo test --test e2e_tests`; 308 baseline (1 Pre-existing failure ignorieren) → 309.
- **UAT-Smoke-Setup**: Backend `cargo run --features mock_auth --bin genossi` (:3000), Frontend `dx serve` (:8080), Skill `run-rust-backend-and-frontend`. **Warnung:** Dev-DB enthält echte Mitglieder-E-Mails (Memory `reference_frontend_smoke_test_setup`) — Send-Button im Smoke NICHT klicken.

</code_context>

<specifics>
## Specific Ideas

- **User-Entscheidung H1:** „Überschriften machen durchaus Sinn." → H1 bleibt drin (D-01).
- **User-Entscheidung Grep-Gate:** „(b)" → Rust-Test mit `include_str!`, nicht Husky-Hook (D-02).
- **User-Entscheidung Round-Trip:** „(c) — aber funktionieren e2e tests aktuell?" → Unit + E2E, E2E-Suite verifiziert grün beim Discuss (D-03).
- **User-Entscheidung Backward-Compat:** „(c)" → Nicht-Änderung von sanitize.rs erfüllt Success-Criterion #5 (D-04).
- **User-Entscheidung UAT-Checklist:** „(a)" → Copy von 24 + Steps 13-16 (D-05).
- **User-Entscheidung Merge-Gate:** „Wir benutzen jj. … Mach dir da keinen Stress." → UAT als Ship-Gate vor Milestone-Close, nicht als Merge-Blocker in der Phase (D-06).

</specifics>

<deferred>
## Deferred Ideas

- Mehr Toolbar-Buttons (Text-Align, Text-Color, Font-Size, Font-Family) — eigene Zukunfts-Phase, nicht v1.5.
- Icon-Politur (SVG-Icons statt Text „B/I/U/S/1./•/H1/H2/H3/¶/❝/🔗/⊘") — kosmetisch, nur wenn beim UAT als störend auffällt.
- Snapshot-Test für existierende v1.4-Templates (fallback für D-04, falls Unit-Tests unerwartet failen).
- H2 visuell größer als H3 in der Toolbar — kosmetisch.
- Bild-Upload → **Phase 27** (v1.5).
- Desktop/Mobile-Preview → **Phase 28** (v1.5).

</deferred>

---

*Phase: 26-editor-formatierung-vervollstaendigen*
*Context gathered: 2026-07-17*
