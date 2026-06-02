---
phase: quick-260602-r2i
plan: 01
subsystem: templating
tags: [typst, minijinja, mail, repayment, pdf-generation]

requires:
  - phase: phase-10
    provides: merge_repayment_context helper (payout_amount/share_count/fiscal_year) und MailRestState::resolve_repayment_context-Trait
  - phase: phase-13
    provides: build_inputs_repayment_letter und _bundle (Single + Bundle Typst-Pipeline)
provides:
  - share_value (deutscher Euro-String "X,YZ") als 4. Variable im MiniJinja-Mail-Render-Context
  - r.share_value (deutscher Euro-String "X,YZ") als zusaetzliches Feld im repayment-JSON beider Typst-Letter-Templates (Single + Bundle)
  - Default-Single-Letter-Template zeigt "Hoehe pro Anteil: 120,00 €" sichtbar im Reference-Block
affects: [zukuenftige RepaymentPhase-Mail-Templates, Frontend-Template-Editor (zeigt share_value als verfuegbare Variable an)]

tech-stack:
  added: []
  patterns:
    - "Additive Erweiterung der Render-Pipelines ohne Breaking-Change fuer bestehende Templates"
    - "share_value-Format identisch zu payout_amount (deutsche Euro-Konvention 'X,YZ')"

key-files:
  created:
    - .planning/quick/260602-r2i-anteilswert-in-templates-verf-gbar-mache/260602-r2i-SUMMARY.md
    - .planning/quick/260602-r2i-anteilswert-in-templates-verf-gbar-mache/deferred-items.md
  modified:
    - genossi_service_impl/src/pdf_generation.rs
    - genossi_mail/src/template.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/rest.rs
    - genossi_bin/src/lib.rs
    - templates/auszahlungs_anschreiben.typ
    - templates/defaults/auszahlungs_anschreiben.typ

key-decisions:
  - "merge_repayment_context-Signatur erweitert (4-arg -> 5-arg) statt zweiter Helper-Funktion: konsistent zu Phase-10-D-04, vermeidet Doppel-Code im Worker und im Preview-REST-Pfad"
  - "MailRestState::resolve_repayment_context liefert 4-Tuple statt 3-Tuple (additiv: share_value zwischen share_count und fiscal_year einsortiert)"
  - "Single-Source-of-Truth-Drift-Schutz weiterhin aktiv: nur das Single-Letter-Template zeigt die neue Reference-Block-Zeile; das Bundle-Template iteriert via render-letter ueber recipients und uebernimmt die Zeile damit automatisch"
  - "format!-Konvention ohne .abs() — share_value > 0 per D-12-Constraint (Phase-7-Plan-01), identisches Pattern zu Phase-10-D-04"

patterns-established:
  - "TDD-RED-Commit fuer cross-cutting Signature-Aenderungen ist OK auch wenn der Test-Build noch nicht kompiliert — Compile-Fehler IST das RED-Signal"
  - "Underscore-Prefix entfernen, sobald ein zuvor unused Argument verwendet wird (`_phase` -> `phase`)"

requirements-completed: []

duration: ~25 min
completed: 2026-06-02
---

# Quick 260602-r2i: Anteilswert in Templates verfügbar machen — Summary

**Anteilswert (`RepaymentPhase.share_value`) ist jetzt als deutsche Euro-String-Variable `share_value` im Typst-Letter-Template (Single + Bundle) und im MiniJinja-Mail-Template verfuegbar; Default-Single-Letter zeigt "Hoehe pro Anteil: 120,00 €" sichtbar im Reference-Block.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-06-02T17:58Z
- **Tasks:** 3 (Task 1 Typst, Task 2 MiniJinja, Task 3 E2E-Verifikation)
- **Files modified:** 7 (5 Rust, 2 Typst)
- **LOC delta:** +242 / -22

## Accomplishments

- **Typst-Render-Pipeline:** `build_inputs_repayment_letter` und `build_inputs_repayment_letters_bundle` injizieren `share_value` als deutschen Euro-String "X,YZ" in den `repayment`-JSON-Slot (alle drei Slots beim Bundle: Recipients-Loop + First-Recipient-Compat + Empty-Bundle-Compat).
- **Default-Single-Letter-Template** zeigt sichtbar "Hoehe pro Anteil: #r.share_value €" zwischen "Anteile zur Auszahlung" und "Auszahlungsbetrag" — logische Reihenfolge: Stueck * Wert = Summe.
- **MiniJinja-Mail-Render-Pipeline:** `merge_repayment_context` erweitert von 4 auf 5 Argumente (share_value zwischen share_count und fiscal_year); Worker und Preview-REST-Pfad symmetrisch verdrahtet.
- **`MailRestState::resolve_repayment_context`-Trait** liefert 4-Tuple `(payout, share_count, share_value, fiscal_year)` statt vorher 3-Tuple; einzige Impl in `genossi_bin/src/lib.rs::RestStateImpl` entsprechend angepasst.

## Task Commits

1. **Task 1 RED:** `0f1f70a` (test) — 4 neue Tests gegen `build_inputs_repayment_letter`/`_bundle`
2. **Task 1 GREEN:** `eca778b` (feat) — share_value in pdf_generation.rs + beide Letter-Templates
3. **Task 2 RED:** `6802501` (test) — 4 neue Tests + 2 angepasste bestehende fuer 5-arg-Signatur
4. **Task 2 GREEN:** `5c577d1` (feat) — merge_repayment_context-Signature, worker.rs, rest.rs, genossi_bin/lib.rs
5. **Task 3 Style:** `c77d2de` (style) — rustfmt-konformes Collapsing der share_value `format!`-Aufrufe

## Files Created/Modified

### Rust (5 Dateien)

- `genossi_service_impl/src/pdf_generation.rs` (+102/-13) — Single + Bundle Letter-Inputs, 4 neue Tests, `_phase` -> `phase`
- `genossi_mail/src/template.rs` (+93/-15) — `merge_repayment_context` 4→5-arg, Doc-Comment aktualisiert, 4 neue Tests, 2 bestehende Tests an 5-arg-Signatur angepasst
- `genossi_mail/src/worker.rs` (+7/-1) — Worker formatiert `share_value_str` aus `phase.share_value` und reicht ihn durch
- `genossi_mail/src/rest.rs` (+7/-4) — Trait `resolve_repayment_context` 3→4-Tuple, Preview-Handler destrukturiert 4-Tuple
- `genossi_bin/src/lib.rs` (+9/-4) — `RestStateImpl::resolve_repayment_context` baut `share_value_str` und liefert 4-Tuple

### Typst-Templates (2 Dateien)

- `templates/auszahlungs_anschreiben.typ` (+9/-1) — Doc-Block zur neuen Variable + sichtbare Reference-Block-Zeile
- `templates/defaults/auszahlungs_anschreiben.typ` (+9/-1) — synchron gehalten (include_bytes!-Quelle fuer DEFAULT_TEMPLATES)

## Neue Tests (8 + 2 angepasste)

### `genossi_service_impl::pdf_generation::tests` (4 neue)

- `test_build_inputs_repayment_letter_contains_share_value` — Format "120,00" bei `phase.share_value = 12000` Cent
- `test_build_inputs_repayment_letter_share_value_formatting` — Edge-Cases 100→"1,00", 9999→"99,99"
- `test_build_inputs_repayment_letters_bundle_contains_share_value` — share_value pro Recipient + compat-Top-Level
- `test_build_inputs_repayment_letters_bundle_empty_share_value` — Empty-Bundle traegt phase.share_value (nicht "0,00")

### `genossi_mail::template::tests` (4 neue + 2 angepasste)

- **Neu:** `test_merge_repayment_context_includes_share_value`
- **Neu:** `test_merge_preserves_base_context_fields_with_share_value`
- **Neu:** `test_share_value_missing_with_if_guard_renders_empty`
- **Neu:** `test_share_value_missing_without_guard_fails_strict`
- **Umbenannt + erweitert:** `test_merge_repayment_context_renders_all_three_vars` → `test_merge_repayment_context_renders_all_four_vars`
- **Angepasst (Signature):** `test_merge_preserves_base_context_fields` nutzt jetzt die 5-arg-Signatur

## Verifikation

- `cargo build --workspace`: OK
- `cargo test --workspace --lib`: **828 passed**, 0 failed (40+0+16+70+61+132+74+35+68+332)
- `cargo test -p genossi_service_impl --lib build_inputs_repayment`: **9/9 PASS**
- `cargo test -p genossi_mail --lib template`: **49/49 PASS** (inkl. 4 neuer share_value-Tests)
- `cargo test -p genossi_bin --test repayment_letter_e2e`: 6/7 PASS (1 pre-existing failure, siehe Deferred Issues)
- `cargo clippy -p genossi_service_impl -p genossi_mail -p genossi_bin -- -D warnings`: OK
- `cargo clippy --all-targets --all-features`: OK
- `rustfmt --check` auf den 5 geaenderten Rust-Files: 0 neue Drifts (pre-existing 5 Drifts in pdf_generation.rs ausserhalb meiner Aenderungen out-of-scope).

## Decisions Made

Siehe Frontmatter — alle Plan-Decisions wurden 1:1 umgesetzt (5-arg-Signatur, 4-Tuple, Single-Source-of-Truth, kein .abs()).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Style] rustfmt-konformes Collapsing der share_value `format!`-Aufrufe**

- **Found during:** Task 3 (Workspace-Verifikation)
- **Issue:** Plan-Text hatte das `format!` ueber mehrere Zeilen vorgegeben; rustfmt 1.93 collapsed das auf eine Zeile.
- **Fix:** Einzeilige `format!("{},{:02}", phase.share_value / 100, phase.share_value % 100)` in `pdf_generation.rs` (zwei Stellen — Single + Bundle).
- **Files modified:** `genossi_service_impl/src/pdf_generation.rs`
- **Verification:** `rustfmt --check` zaehlt jetzt 5 Drifts (= Anzahl im Base-Commit), 0 von r2i hinzugefuegt.
- **Committed in:** `c77d2de`

**Total deviations:** 1 auto-fixed (Style/Cleanup).
**Impact on plan:** Keine; rein kosmetisch. Plan-Inhalte alle in der originalen Form umgesetzt.

## Deferred Issues

Siehe [deferred-items.md](./deferred-items.md):

- **Pre-existing E2E failure** `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` im Letter-E2E-Suite: bereits am Base-Commit `36d79a5` rot — durch frisches Klonen + Run gegen Base verifiziert. Nicht durch r2i verursacht; gehoert vermutlich zur Folgearbeit von Quick 260602-q9l (idempotent regeneration). Out-of-scope per Scope-Boundary; nicht in r2i fixierbar.

## Issues Encountered

- **Worktree-Branch-Reset während Debugging:** Beim Versuch, das pre-existing E2E-Failure am Base-Commit zu reproduzieren, habe ich versehentlich `git checkout 36d79a5 -- <paths>` benutzt — das hat meine Working-Tree-Aenderungen ueberschrieben (Commits blieben aber intakt). Sofort via `git checkout HEAD -- <paths>` restauriert. Stash @{0} (ROADMAP.md-Merge-Conflict-Mess) gedroppt, weil es nichts mit r2i zu tun hatte. Kein Datenverlust.

## Threat Flags

Keine — alle r2i-Aenderungen fallen unter den im Plan dokumentierten `threat_model` (T-r2i-01 bis T-r2i-05). Keine neuen Trust-Boundaries; `share_value` ist deterministisch aus i64-Cents formatiert (kein User-Input) und damit weder Injection- noch Tampering-Surface.

## Known Stubs

Keine. Alle 7 betroffenen Dateien tragen produktive Logik; alle 8 neuen Tests asserten konkrete Werte; keine TODOs hinzugefuegt.

## Hinweis fuer den Vorstand

Im Default-Anschreiben erscheint ab jetzt automatisch die Zeile **"Hoehe pro Anteil: 120,00 €"** (Wert dynamisch aus der jeweiligen Phase). In Custom-Templates kann der Vorstand:

- in Typst-Letter-Templates: `#r.share_value €`
- in Mail-Templates: `{{ share_value }}` (bei optionaler Verwendung mit `{% if share_value is defined %}`-Guard)

beliebig platzieren. Alte Templates ohne diese Variable funktionieren unveraendert weiter.

## Self-Check: PASSED

Verifiziert:

- Alle 7 modifizierten Dateien existieren (`ls` per Tool oben).
- Alle 5 Commits sind im `git log` zwischen Base und HEAD sichtbar (`0f1f70a`, `eca778b`, `6802501`, `5c577d1`, `c77d2de`).
- `cargo test -p genossi_mail --lib template`: 49/49 ✓
- `cargo test -p genossi_service_impl --lib build_inputs_repayment`: 9/9 ✓
- `cargo test --workspace --lib`: 828/828 ✓
- `cargo clippy --all-targets --all-features`: clean ✓

---
*Quick: 260602-r2i*
*Completed: 2026-06-02*
