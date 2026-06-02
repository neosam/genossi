---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 01
subsystem: foundation
tags: [phase-13, document-type, typst, template, letter, bundle, foundation]

requires:
  - phase: 10-massenmail-anbindung-template-variablen
    provides: "DocumentType::RepaymentMail-Variante als Pattern-Vorbild (Phase 10 D-09)"
  - phase: 11-export-pdf-csv
    provides: "DEFAULT_TEMPLATES + include_bytes!-Provisioning-Pattern (auszahlungsliste.typ)"
provides:
  - "DocumentType::RepaymentLetter-Variante mit allen 4 helper-Match-Arms (as_str, from_str, is_singleton, template_path)"
  - "Single-Letter Default-Template auszahlungs_anschreiben.typ mit exportierter render-letter(member, repayment, today)-Funktion (Single-Source-of-Truth)"
  - "Bundle-Wrapper-Template auszahlungs_anschreiben_bundle.typ — importiert render-letter, iteriert ueber recipients[] mit #pagebreak()"
  - "Zwei DEFAULT_TEMPLATES-Eintraege via include_bytes! — werden auf Startup ueber provision_defaults() im TEMPLATE_PATH bereitgestellt"
affects: [13-02, 13-03, 13-04, 13-05, 13-06, 13-07]

tech-stack:
  added: []
  patterns:
    - "Typst Single-Source-of-Truth via #let render-letter(...) als exportierte Funktion; Bundle macht #import statt Brief-Body-Duplikat (Drift-Schutz)"
    - "DEFAULT_TEMPLATES-Doppel-Eintrag (Single + Bundle) — Vorbild fuer kuenftige Letter-+Bundle-Templates"

key-files:
  created:
    - "templates/defaults/auszahlungs_anschreiben.typ"
    - "templates/defaults/auszahlungs_anschreiben_bundle.typ"
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-01-SUMMARY.md"
  modified:
    - "genossi_service/src/member_document.rs"
    - "genossi_service_impl/src/template_storage.rs"

key-decisions:
  - "D-13-08 angewendet: RepaymentLetter NICHT in is_singleton-matches!-Liste — Re-Generierung erlaubt"
  - "D-LETT-04 angewendet: template_path()-Match explizit per Variante (kein Wildcard), RepaymentLetter => None — Service nutzt hardcoded Pfade"
  - "Single-Source-of-Truth via Typst-Funktion: show: letter-simple.with(...) steht im render-letter-Funktions-Body. Typst erlaubt show-Rules im Block-Scope; das vermeidet Brief-Body-Duplikat im Bundle und macht Vorstands-Edits via /templates-Editor automatisch im Bundle wirksam"
  - "Bundle-Template enthaelt NUR Loop + Import; Drift-Schutz-Tests verifizieren dass 'Carolin Weidmann' und 'Mitgliedsnummer' NICHT im Bundle stehen"

patterns-established:
  - "TDD-RED-Then-GREEN pro DocumentType-Variante: erst Tests committen (compile-fail), dann Variante + Match-Arms ergaenzen"
  - "Drift-Schutz via Negativ-Assertion im Test: Bundle.contains(...) == false fuer alle Strings die nur im Single-Template leben duerfen"

requirements-completed: [BRIEF-01]

duration: 6min
completed: 2026-06-01
---

# Phase 13 Plan 01: Foundation — DocumentType::RepaymentLetter + Single+Bundle Templates Summary

**Neue `RepaymentLetter`-Variante im `DocumentType`-Enum plus zwei Default-Templates (`auszahlungs_anschreiben.typ` + `_bundle.typ`) registriert in `DEFAULT_TEMPLATES`; Single-Source-of-Truth-Vertrag via exportierter Typst-`render-letter`-Funktion mit Drift-Schutz-Tests.**

## Performance

- **Duration:** ~6 min (zwischen `40b577b` 23:33:18 und `63ae2ef` 23:39:24)
- **Tasks:** 2 (Task 1 TDD: RED + GREEN; Task 2: Templates + Registrierung)
- **Files modified:** 2
- **Files created:** 2
- **Commits:** 3

## Accomplishments

- `DocumentType::RepaymentLetter`-Variante ergaenzt; alle 5 Match-Arms (Enum + as_str + from_str + is_singleton-Kommentar + template_path) explizit per Variante (keine Wildcard) — verhindert dass kuenftige Enum-Erweiterungen versehentlich Stellen verpassen.
- 4 neue Tests in `member_document::tests` verifizieren `as_str`/`from_str`-Round-Trip, `is_singleton == false` (D-13-08), `template_path == None` (D-LETT-04) — alle 12 Tests im `member_document`-Modul gruen unter `--features utoipa`.
- Single-Letter-Template `auszahlungs_anschreiben.typ` (105 Zeilen) implementiert mit exportierter `render-letter(m, r, today)`-Funktion. Brief-Body enthaelt alle 4 D-13-06-Bausteine (Reference-Tabelle, Anrede + Auszahlungsbetrag, IBAN-Switch via `#if m.bank_account != none`, hardcoded Vorstands-Signatur) und KEINEN SEPA-Verwendungszweck (D-13-07).
- Bundle-Template `auszahlungs_anschreiben_bundle.typ` (17 Zeilen) macht ausschliesslich `#import "auszahlungs_anschreiben.typ": render-letter` + iteriert mit `#pagebreak()` zwischen Recipients. **Drift-Schutz-Greps gruen:** Bundle enthaelt NICHT "Carolin Weidmann", NICHT "Mitgliedsnummer" — Brief-Body-Strings leben nur im Single-Template.
- Zwei `DEFAULT_TEMPLATES`-Eintraege via `include_bytes!` ergaenzt → werden auf Startup ueber `provision_defaults()` im `TEMPLATE_PATH` bereitgestellt; UI-editierbar via existing `/templates`-Editor (D-13-05).
- 2 neue Tests in `template_storage::tests` verifizieren beide Eintraege existieren, sind non-empty, enthalten alle erwarteten Marker (letter-pro, sys.inputs, bank_account, Carolin Weidmann, render-letter, kein Verwendungszweck) und das Bundle erfuellt den Single-Source-of-Truth-Vertrag — alle 18 Tests im Modul gruen.

## Task Commits

1. **Task 1 RED: failing tests for DocumentType::RepaymentLetter** — `40b577b` (test)
2. **Task 1 GREEN: add DocumentType::RepaymentLetter variant** — `c1c46e9` (feat)
3. **Task 2: add RepaymentLetter single + bundle templates** — `63ae2ef` (feat)

_Note: Task 1 ist TDD mit RED→GREEN; kein REFACTOR-Commit noetig (Implementation bereits minimal-clean)._

## Files Created/Modified

- `genossi_service/src/member_document.rs` — `DocumentType::RepaymentLetter`-Variante + Match-Arms in 4 Helper-Funktionen + 4 neue Tests
- `genossi_service_impl/src/template_storage.rs` — 2 neue `DEFAULT_TEMPLATES`-Eintraege via `include_bytes!` + 2 neue Tests (Single + Bundle)
- `templates/defaults/auszahlungs_anschreiben.typ` — Neu: Single-Letter mit exportierter `render-letter`-Funktion + 4 Brief-Bausteinen (105 Zeilen)
- `templates/defaults/auszahlungs_anschreiben_bundle.typ` — Neu: Bundle-Wrapper, importiert `render-letter`, iteriert mit `#pagebreak()` (17 Zeilen)

## Decisions Made

- **`show: letter-simple.with(...)` INNERHALB von `render-letter` platziert** — der Plan erlaubte Fallback "Layout-Aufruf ausserhalb der Funktion" falls Typst-Scope-Probleme auftauchen. Typst 0.14 erlaubt `show`-Rules im Funktions-Body (wirken auf umgebendes Block-Scope der Funktion). Damit bleibt Subject ("Auszahlung deiner Anteile"), Sender, Recipient KOMPLETT in der `render-letter`-Funktion und das Bundle muss diese Layout-Details NICHT duplizieren. Drift-Schutz maximiert.
- **Parameter-Renaming `member`/`repayment` → `m`/`r` in `render-letter`** — vermeidet Shadowing mit den `#let member = ...`/`#let repayment = ...`-Top-Level-Bindings in der Single-Mode-Section. Aufruf-Site bleibt `render-letter(member, repayment, today)`. Funktionssemantisch identisch, lokal sauberer Scope.

## Deviations from Plan

None — plan executed exactly as written.

Die 4 Auto-Fix-Regeln waren nicht relevant:
- Rule 1 (Bug): keine Bugs — alle Tests gruen.
- Rule 2 (Missing Critical): keine fehlenden Sicherheits-/Korrektheits-Funktionen — Foundation-Plan ist rein Type-/Template-Definition; Security-relevante Logik (Permission-Funnel, Audit) kommt in Plans 02-06.
- Rule 3 (Blocking): kein Blocker — alle Dependencies (letter-pro 3.0.0, include_bytes!-Pattern) existieren.
- Rule 4 (Architectural): keine Architektur-Aenderung — Plan folgt etablierten Phase-10/11-Patterns.

**Pre-existing Build Issue (NICHT von dieser Aenderung verursacht):** `cargo build -p genossi_service` ohne Feature-Flag scheitert an `utoipa::ToSchema`-Imports in `auth_types.rs` — gleicher Fehler auf dem Parent-Commit `a320cd3` reproduziert. Workspace-default-Build verwendet implizit `--features utoipa`. Tests laufen mit `--features utoipa` sauber durch. Out-of-Scope fuer diesen Plan.

## Issues Encountered

None.

## Self-Check

```
=== Files exist ===
FOUND: /home/neosam/programming/rust/projects/genossi3/templates/defaults/auszahlungs_anschreiben.typ
FOUND: /home/neosam/programming/rust/projects/genossi3/templates/defaults/auszahlungs_anschreiben_bundle.typ
FOUND: /home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-01-SUMMARY.md

=== Commits exist ===
FOUND: 40b577b (RED)
FOUND: c1c46e9 (GREEN)
FOUND: 63ae2ef (Task 2)

=== Acceptance-Greps grün ===
- rg 'DocumentType::RepaymentLetter' genossi_service/src/member_document.rs: 8 matches (>=5 ✓)
- rg '"repayment_letter"' genossi_service/src/member_document.rs: 4 matches (>=2 ✓)
- rg 'test_repayment_letter_' genossi_service/src/member_document.rs: 4 matches (==4 ✓)
- rg 'DocumentType::RepaymentLetter\s*=>\s*None' genossi_service/src/member_document.rs: 1 match (>=1 ✓, template_path explizit)
- rg '#let render-letter' templates/defaults/auszahlungs_anschreiben.typ: 1
- rg 'letter-pro' templates/defaults/auszahlungs_anschreiben.typ: 1
- rg 'sys.inputs.at' templates/defaults/auszahlungs_anschreiben.typ: 3 (member, repayment, today)
- rg 'if m\.bank_account != none' templates/defaults/auszahlungs_anschreiben.typ: 1 (IBAN-Switch)
- rg 'Carolin Weidmann, Dina Beier und Simon Goller' templates/defaults/auszahlungs_anschreiben.typ: 1
- rg 'Verwendungszweck' templates/defaults/auszahlungs_anschreiben.typ: 0 (D-13-07 ✓)
- rg '#import "auszahlungs_anschreiben.typ": render-letter' templates/defaults/auszahlungs_anschreiben_bundle.typ: 1
- rg 'pagebreak\(\)' templates/defaults/auszahlungs_anschreiben_bundle.typ: 2 (im Code + im Kommentar)
- rg 'Carolin Weidmann' templates/defaults/auszahlungs_anschreiben_bundle.typ: 0 (Drift-Schutz ✓)
- rg 'Mitgliedsnummer' templates/defaults/auszahlungs_anschreiben_bundle.typ: 0 (Drift-Schutz ✓)
```

**Self-Check: PASSED**

## Next Plan Readiness

Plans 02-07 koennen jetzt:
- Auf `DocumentType::RepaymentLetter` zugreifen (Plan 02 RepaymentContextResolver, Plan 03 PdfGenerator-Methoden, Plan 04 Letter-Service, Plan 05 REST-Handler, Plan 06 Frontend, Plan 07 E2E-Tests).
- Templates ueber `template_base.join("auszahlungs_anschreiben.typ")` bzw. `_bundle.typ` laden — werden auf Startup provisioniert.
- Vorstand kann nach Startup beide Templates via `/templates`-Editor anpassen. Aenderungen am Single-Template wirken automatisch im Bundle, weil das Bundle `render-letter` importiert (Single-Source-of-Truth).

**Keine Blocker fuer Folge-Plans.**

---
*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-01*
