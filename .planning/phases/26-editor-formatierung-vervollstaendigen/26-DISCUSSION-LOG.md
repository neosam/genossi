# Phase 26 Discussion Log

**Date:** 2026-07-17
**Format:** Freitext-Chat (Memory `feedback_discuss_in_chat_form`)

## Scouting-Erkenntnisse (vor der Diskussion)

Beim Lesen von `wysiwyg_toolbar.rs`, `wysiwyg_editor.rs`, `sanitize.rs` und `24-UAT-CHECKLIST.md` kamen 4 Überraschungen heraus:

1. Die Toolbar zeigt bereits alle geforderten Buttons (13 Stück inkl. UL/OL/H1/H2/H3) — Phase 24 hat den vollen Umfang vorweggenommen.
2. Ammonia-Default lässt `<ul>/<ol>/<li>/<h1..h6>` durch (laut Kommentar), aber **kein Test** beweist das.
3. Kein Round-Trip-E2E-Test für Listen/Headings vorhanden.
4. Der Grep-Gate aus EDIT-09 („analog EDIT-01/02") existiert bisher **gar nicht** — er ist neu zu bauen.

Konsequenz: Phase 26 ist eine **Test-/Verifikations-/UAT-Phase**, keine „Toolbar-Erweiterungs-Phase" wie der ROADMAP-Titel klingen ließ.

## Gray Areas — Q&A

### GA-1: H1-Button behalten oder rausnehmen?
- Vorschlag: (b) behalten. Ammonia lässt durch, funktioniert, kein Schaden.
- **User:** „Ja, Überschriften machen durchaus Sinn."
- **Entscheidung:** H1 bleibt (D-01).

### GA-2: Grep-Gate wo/wie?
- Optionen: (a) `.cargo-husky` pre-commit / (b) Rust-Test mit `include_str!` / (c) GitHub-Actions Step.
- Empfehlung: (b) — versioniert im Code, kein neuer Tooling-Bedarf.
- **User:** „(b)".
- **Entscheidung:** Rust-`include_str!`-Test (D-02).

### GA-3: Round-Trip-Test-Umfang?
- Optionen: (a) nur Unit / (b) nur E2E / (c) beides.
- Empfehlung: (c).
- **User:** „(c) — aber funktionieren e2e tests aktuell?"
- **Verifikation:** `cargo test --test e2e_tests` → 308 passed, 1 failed (`test_mail_preview_repayment_no_entries_does_not_default_to_one`, dokumentiert als Pre-existing failure aus Phase 22 in STATE.md und in 24-UAT-CHECKLIST). Kein Phase-26-Blocker.
- **Entscheidung:** Unit + E2E (D-03), E2E-Suite grün abgesehen vom bekannten Pre-existing failure.

### GA-4: Backward-Compat-Test für v1.4-Templates?
- Optionen: (a) Snapshot / (b) include_str!-Fixture / (c) nichts (Nicht-Änderung von sanitize.rs impliziert Bytre-identisch).
- Empfehlung: (c).
- **User:** „(c)".
- **Entscheidung:** Nicht-Änderung erfüllt Success-Criterion #5 (D-04); Ausstiegs-Klausel falls Unit-Tests unerwartet failen.

### GA-5: UAT-Checklist Reuse oder neu?
- Optionen: (a) Copy von Phase 24 + neue Steps 13-16 / (b) nur neue Steps / (c) Verweis auf Phase 24 + neue Steps.
- Empfehlung: (a) — ein Dokument, ein Termin, komplettes Sign-Off.
- **User:** „(a)".
- **Entscheidung:** 26-UAT-CHECKLIST.md = Copy von 24 + Steps 13-16 für UL/OL/H2/H3 (D-05).

### GA-6: UAT hart als Merge-Gate oder weich?
- Optionen: (a) hart (Merge blockiert bis UAT durch) / (b) weich (deferred wie Phase 24).
- Empfehlung: (a) — sonst wiederholen wir das Muster, das die Phase abschaffen soll.
- **User:** „Wir benutzen jj. Das ist in Git in der Regel detached. Mach dir da keinen Stress."
- Interpretation: jj-WIP-Changes sind ohnehin nicht wie klassische Git-Branches gated; kein künstlicher Merge-Blocker aufbauen. Der Sinn der Phase (UAT nachholen) bleibt aber gültig.
- **Entscheidung:** UAT ist **Ship-Gate vor v1.5-Milestone-Close** (`/gsd-complete-milestone`), nicht Merge-Gate innerhalb der Phase. Verifier dokumentiert UAT-Status als „Pending Sign-Off" (D-06).

## Nicht angesprochen (schon entschieden aus Memory/Konventionen)

- Kein neuer Editor-Framework — Phase-24-Constraint EDIT-02.
- Kein Bild-Support — Phase 27.
- Kein Preview-Modi — Phase 28.
- `r#type: "button"` + onclick — bereits so gebaut (`feedback_dioxus_button_type`).
- i18n zweisprachig (de.rs + en.rs) — Keys existieren bereits (`MailEditorUnorderedList` etc.).
- jj statt git für Commits — Projekt-Standard (`feedback_use_jj_not_git`).

## Deferred (Aufkommen während Diskussion)

Keine — Diskussion blieb im Scope. Alle scope-erweiternden Ideen (Bild, Preview, mehr Buttons, Icon-Politur) sind als „später" markiert.

## Claude's Discretion (an den Planner delegiert)

- Exakte Datei/Modul-Location für den Grep-Gate-Test.
- Regex-Nachbarschaftsheuristik im Grep-Gate (wie eng darf „prevent_default" bei „onpaste" stehen?).
- Ammonia-Normalisierungs-Toleranz im E2E-Test (byte-exakt vs. „enthält alle Tags").
- Icon-/Label-Politur beim UAT (falls sichtbar auffällt).
