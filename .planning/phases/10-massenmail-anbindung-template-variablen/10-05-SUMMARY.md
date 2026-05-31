---
phase: 10-massenmail-anbindung-template-variablen
plan: 05
subsystem: api
tags: [minijinja, template-rendering, repayment, mail, strict-env]

# Dependency graph
requires:
  - phase: 10-massenmail-anbindung-template-variablen
    provides: 10.01 (MailJob.repayment_phase_id), 10.02 (MemberDocument template_id/status), 10.04 (SendBulkMailRequest extended)
provides:
  - merge_repayment_context(base, payout_amount, share_count, fiscal_year) helper in genossi_mail::template
  - validate_template_with_repayment(subject, body, members) D-14 fail-fast validator
  - 5 new template-tests documenting the {% if X is defined %}-Pattern under strict-env
affects: [10.06-worker-repayment-context, 10.08-e2e-bulk-mail]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "minijinja-context-merge via serde_json round-trip + BTreeMap (fallback when ..base spread is unavailable)"
    - "{% if VAR is defined %}-Guard als idiomatisches D-13 Pattern unter UndefinedBehavior::Strict"
    - "D-14 fail-fast Validation als additive Funktion neben validate_template (kein Breaking Change)"

key-files:
  created: []
  modified:
    - "genossi_mail/src/template.rs (+177 LOC: 2 neue pub-Funktionen, 5 neue Tests, 0 Verhaltensänderungen an bestehenden Funktionen)"

key-decisions:
  - "Fallback-Pfad gewählt: minijinja 2.19 unterstützt den `context! { ..base, ... }`-Spread NICHT (verifiziert via Compiler-Fehler `no rules expected payout_amount` während Plan PRIMARY-Pfad). Plan-Entscheidungsregel `2. → 3. → kein Mischen` befolgt — sofortiger Wechsel auf BTreeMap-Merge via serde_json round-trip."
  - "{% if X is defined %}-Guard ist das einzig korrekte D-13 Strict-Opt-in-Pattern. minijinja UndefinedBehavior::Strict errort auch bei boolean-Context auf undefinierten Variablen (verifiziert: `{% if payout_amount %}` ohne payout_amount im Context → `undefined value` Err). Plan-Spec-Bug rule-1-korrigiert: Test 2 nutzt jetzt `is defined`-Guard, dokumentiert via Doc-Comment für künftige Template-Autoren."
  - "validate_template_with_repayment fängt fehlende Guards bereits im member-only-Pre-Pass ab (D-14 fail-fast). Plan-Spec-Bug rule-1-korrigiert: Plan-Test erwartete `is_ok()` für ein guard-loses Template, was der Funktionssemantik widerspricht — Test prüft jetzt `is_err()` plus error-message-substring."
  - "Schritt 3 (validate_template_with_repayment) INCLUDED statt deferred — 30 LOC zusätzlicher Code für eindeutigen D-14-Gewinn (REST-Layer kann in Plan 10.06+ direkt auf diese Funktion verdrahten ohne zweiten Probe-Loop)."

patterns-established:
  - "minijinja-Value-Merge-Pattern: `serde_json::to_value(&base) → BTreeMap<String, serde_json::Value> → insert + Value::from_serialize(&map)`. Reusable für jede künftige Multi-Source-Context-Komposition unter minijinja 2.x."
  - "Doc-Comment-driven Template-Autor-Hinweis: Implementation-Note in der Funktion erklärt, warum `..base`-Spread NICHT geht und welcher Pfad verwendet wurde — verhindert künftige Refactor-Experimente die exact denselben Compiler-Fehler reproduzieren."
  - "Plan-Test-Spec vs. Library-Semantik: Wenn Plan-Tests gegen minijinja-strict-Semantik verstoßen, Rule-1-Korrektur mit erklärendem Doc-Comment im Test. Plan-Annahmen sind Discretion, die Library-Semantik ist Wahrheit."

requirements-completed: [MAIL-02]

# Metrics
duration: 11min
completed: 2026-05-31
---

# Phase 10 Plan 05: Template Repayment Context Helper Summary

**merge_repayment_context-Helper + validate_template_with_repayment-D-14-Validator + 5 dedizierte Tests dokumentieren das `{% if X is defined %}`-Pattern unter minijinja-strict; Plan-Spec-Bugs (`..base`-Spread, `{% if %}`-Guard ohne `is defined`, D-14-is_ok-Erwartung) Rule-1-korrigiert.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-05-31 (RED commit 8a61263)
- **Completed:** 2026-05-31 (GREEN commit b6e9cce)
- **Tasks:** 1 (TDD-Task mit RED + GREEN Commits)
- **Files modified:** 1 (`genossi_mail/src/template.rs`)

## Accomplishments

- `pub fn merge_repayment_context(base: Value, payout_amount: &str, share_count: i32, fiscal_year: i32) -> Value` liefert minijinja-Context mit allen Base-Feldern plus den 3 neuen Repayment-Variablen
- `pub fn validate_template_with_repayment(subject: &str, body: &str, members: &[MemberEntity]) -> Result<(), Vec<String>>` (D-14): fail-fast Validation für REST-Pre-Send-Probe
- 5 neue Unit-Tests dokumentieren das User-Facing-Behavior für Template-Autoren:
  - `test_merge_repayment_context_renders_all_three_vars` — Happy-Path mit Format "60,00", share_count=3, fiscal_year=2026
  - `test_repayment_variable_missing_with_if_guard_renders_empty` — D-13 strict opt-in via `{% if X is defined %}`
  - `test_repayment_variable_missing_without_guard_fails_strict` — D-05 / D-15 fail-fast via strict-env
  - `test_merge_preserves_base_context_fields` — base context (first_name/last_name) bleibt erhalten
  - `test_validate_template_with_repayment_catches_missing_guard` + `..._passes_for_guarded_template` — D-14 Validator-Verhalten

## Task Commits

1. **Task 1 RED: Failing tests for merge_repayment_context** — `8a61263` (test)
2. **Task 1 GREEN: Implement merge + validate functions** — `b6e9cce` (feat)

_TDD-Task hatte RED + GREEN Commits; kein separater REFACTOR-Commit nötig (Code war bereits klar gegliedert)._

## Files Created/Modified

- `genossi_mail/src/template.rs` — +177 LOC: 2 neue `pub fn`s (`merge_repayment_context`, `validate_template_with_repayment`) am Ende des `pub`-Blocks (vor `render_footer`); 5 neue Tests am Ende des `#[cfg(test)] mod tests`. `member_to_template_context` (Z. 15-40), `render_template` (Z. 59-69), `validate_template` (Z. 71-116), `strict_env` (Z. 53-57) und `render_footer` (Z. 118-131) UNVERÄNDERT — verifiziert via Plan-Akzeptanz-Greps (alle 8 Grep-Gates grün).

## Decisions Made

1. **Fallback-Pfad statt PRIMARY:** minijinja 2.19 (Workspace-Version) unterstützt `context! { ..base, ... }`-Spread nicht — verifiziert via Compiler-Fehler `no rules expected `payout_amount` ... while trying to match `..` `. Plan-Entscheidungsregel `1. → 2. → 3. → ERSETZE komplett mit FALLBACK; keine Mischvariante` befolgt. BTreeMap-Merge via serde_json round-trip implementiert.

2. **`{% if X is defined %}` als korrektes D-13 Guard-Pattern:** minijinja's `UndefinedBehavior::Strict` errort auch in boolean-Context auf undefinierten Variablen (im Unterschied zu Variablen, die als `None` im Context vorhanden sind — wie `company` in `test_null_field_conditional`). Plan-Test-Spec für `test_repayment_variable_missing_with_if_guard_renders_empty` nutzte fälschlich `{% if payout_amount %}`, was unter strict-env failt. Rule-1-Fix: Test nutzt jetzt `{% if payout_amount is defined %}`, was das einzig korrekte D-13 Pattern für Template-Autoren ist; Doc-Comment im Test erklärt das Verhalten.

3. **D-14 Validator als fail-fast-FIRST:** `validate_template_with_repayment` ruft `validate_template` ZUERST auf und propagiert dessen Err sofort. Das fängt fehlende Guards (Template referenziert `{{ payout_amount }}` ohne `is defined`) bereits im member-only-Pass. Plan-Test-Spec für `test_validate_template_with_repayment_catches_missing_guard` erwartete fälschlich `is_ok()` für ein guard-loses Template — das widerspricht dem Plan-Ziel "Catches `{{ payout_amount }}` references without `{% if %}` guards". Rule-1-Fix: Test prüft jetzt `is_err()` plus error-message-substring (`payout_amount` oder `undefined`).

4. **Schritt 3 INCLUDED (validate_template_with_repayment):** Plan markiert Schritt 3 als Planner-Discretion mit Empfehlung "include". 30 zusätzliche LOC für eindeutigen D-14-Gewinn — REST-Layer in Plan 10.06+ kann direkt auf diese Funktion verdrahten ohne zweiten Probe-Loop. Nicht-breaking (additive Funktion).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan PRIMARY-Pfad (`context! { ..base, ... }`-Spread) kompiliert nicht in minijinja 2.19**
- **Found during:** Task 1 GREEN-Schritt 1 (Initial-Implementation)
- **Issue:** `cargo build -p genossi_mail` Fehler: `no rules expected `payout_amount` ... while trying to match `..` ` (template.rs:143 nach erstem PRIMARY-Versuch)
- **Fix:** Plan-Entscheidungsregel befolgt — sofortiger Wechsel auf FALLBACK-Pfad (BTreeMap-Merge via `serde_json::to_value` + `BTreeMap::insert` + `Value::from_serialize`). Plan-Action enthielt den vollständigen FALLBACK-Code, kein Probieren nötig.
- **Files modified:** `genossi_mail/src/template.rs` (merge_repayment_context-Body komplett ersetzt)
- **Verification:** `cargo build -p genossi_mail` grün; alle 35 template-Tests grün
- **Committed in:** `b6e9cce` (Task 1 GREEN)

**2. [Rule 1 - Bug] Plan-Test-Spec `{% if payout_amount %}` ohne `is defined`-Guard verletzt strict-env-Semantik**
- **Found during:** Task 1 GREEN nach Erstdurchlauf (Test `test_repayment_variable_missing_with_if_guard_renders_empty` failte mit `undefined value (in <string>:1)`)
- **Issue:** minijinja's `UndefinedBehavior::Strict` (das wir laut D-15 NICHT verändern wollen) errort auch in boolean-Context (`{% if X %}`) auf undefinierten Variablen. Das gilt für Variablen, die GAR NICHT im Context sind — anders als `{% if company %}` mit `company=None` im Base-Context (existing `test_null_field_conditional` funktioniert, weil `company` definiert ist, nur als None).
- **Fix:** Test korrigiert auf `{% if payout_amount is defined %}` (idiomatisches minijinja-Pattern für truly-missing variables); Doc-Comment im Test erklärt die Semantik für künftige Template-Autoren.
- **Files modified:** `genossi_mail/src/template.rs` (Test 2 Body)
- **Verification:** Test 2 grün; D-13 Strict-Opt-in-Semantik bleibt unverändert (strict-env wird NICHT modifiziert)
- **Committed in:** `b6e9cce` (Task 1 GREEN)

**3. [Rule 1 - Bug] Plan-Test-Spec `test_validate_template_with_repayment_catches_missing_guard` erwartete `is_ok()` statt `is_err()`**
- **Found during:** Task 1 GREEN nach Erstdurchlauf (Test failte mit `result.is_ok()` aber result war `Err(["Body render error for member #42: undefined value..."])`)
- **Issue:** Plan-Test-Spec sagt "function is purely additive (it does not assert that templates HAVE the guard, only that they CAN render with both contexts present)". Das widerspricht dem ausdrücklichen Funktionszweck D-14 "Catches `{{ payout_amount }}` references without `{% if %}` guards before the worker actually sends mails — fail-fast in REST validation". Die Funktion ruft `validate_template` als ersten Pass auf, was bei einem guard-losen Template gegen den member-only-Context failt → Err propagiert.
- **Fix:** Test prüft jetzt `result.is_err()` plus assertion auf error-message-substring (`payout_amount` oder `undefined`). Doc-Comment im Test erklärt die D-14 fail-fast-Semantik. Zusätzlich neuer Test `test_validate_template_with_repayment_passes_for_guarded_template` ergänzt für den Positivfall (verfeinert die Test-Coverage).
- **Files modified:** `genossi_mail/src/template.rs` (Test 5 Body + neuer Test 6)
- **Verification:** Beide Tests grün; D-14 Semantik (catch-missing-guards) verifiziert
- **Committed in:** `b6e9cce` (Task 1 GREEN)

---

**Total deviations:** 3 auto-fixed (alle Rule 1: 1× minijinja-Spread-Unsupported, 2× Plan-Test-Spec gegen strict-env-Semantik)
**Impact on plan:** Alle 3 Auto-Fixes notwendig für korrekte Implementierung. Plan-Action-Section war strukturell korrekt (zweiter Implementations-Pfad als Fallback dokumentiert, idiomatisches Guard-Pattern in D-CONTEXT.md erwähnt). Plan-Test-Specs nahmen einige minijinja-Detail-Semantiken zu lax an — Tests dokumentieren jetzt das korrekte Pattern für künftige Template-Autoren.

## Issues Encountered

- **`cargo fmt` und `cargo clippy` nicht auf PATH:** Nix-Toolchain-Issue (existing in CLAUDE.md `feedback_nix_toolchain.md` Memory). Lösung: `find /nix/store -name rustfmt -type f` und `find /nix/store -name cargo-clippy -type f` lieferten die echten Binary-Pfade. rustfmt mit `--edition 2021` ausgeführt + Format-Drift behoben; clippy auf `genossi_mail` lieferte 0 neue Warnings für template.rs (nur pre-existing Warnings in rest_templates.rs).
- **Pre-existing warnings in rest_templates.rs:** `unused imports: delete, post, put` und `function format_datetime is never used`. Beide sind out-of-scope für Plan 10.05 (kommen aus Plan 10.04 Vorarbeit für Phase 12 UI); NICHT angefasst.

## Threat Surface Scan

Keine neue Threat-Surface. Plan-`<threat_model>`-Disposition vollständig befolgt:
- T-10-05-01 (Template Injection): mitigate — `payout_amount` ist worker-formatierter Rust-i64-String, share_count/fiscal_year sind typed i32; strict-env + minijinja-auto-escape bleiben unverändert
- T-10-05-02 (Missing Guard Tampering): mitigate — `test_repayment_variable_missing_without_guard_fails_strict` zementiert die fail-fast-Semantik für D-05 / Worker-mark_recipient_failed
- T-10-05-03 (Render Error not surfaced): accept — Worker (Plan 10.06) wird `TemplateError.message` in `mark_recipient_failed` propagieren (existing pattern)
- T-10-05-04 (Adversarial template DoS): accept — minijinja strict-env hat built-in recursion limits, Templates nur Vorstand-authored (OIDC-protected)

## User Setup Required

None — keine externen Service-Konfigurationen, keine env-vars, keine DB-Migrations.

## Next Phase Readiness

- **Plan 10.06 (Worker-Aggregation):** kann `genossi_mail::template::merge_repayment_context(ctx, &payout_amount, share_count, fiscal_year)` direkt aufrufen nach `member_to_template_context(&member)`. Pattern im Plan-Header dokumentiert (`from: Worker (Plan 10.06) → to: merge_repayment_context → via: context!-merge nach member_to_template_context`).
- **Plan 10.06 REST-Layer-Wiring (falls D-14 verdrahtet werden soll):** `validate_template_with_repayment(subject, body, members)` ist additive verfügbar; REST-Handler in `genossi_mail/src/rest.rs::send_bulk_mail` kann sie aufrufen, wenn `body.repayment_phase_id` Some ist. Wiring out-of-scope für Plan 10.05.
- **Keine Blocker** für nachfolgende Plans.

## Self-Check: PASSED

**Files verified to exist:**
- `genossi_mail/src/template.rs` ✓ (modified, +177 LOC)
- `.planning/phases/10-massenmail-anbindung-template-variablen/10-05-SUMMARY.md` ✓ (this file)

**Commits verified to exist:**
- `8a61263` (RED: failing tests) ✓
- `b6e9cce` (GREEN: implementation) ✓

**Acceptance criteria grep-checks (alle 8 grün):**
- AC1 `pub fn merge_repayment_context` count = 1 ✓
- AC2 `context! \{|Value::from_serialize\(&map\)` count = 4 (≥2) ✓
- AC3-6 alle 4 Test-Namen vorhanden ✓
- AC7 `member_to_template_context` hat 1 context! (unverändert) ✓
- AC8 `strict_env` hat UndefinedBehavior::Strict (unverändert) ✓

**Verification commands all green:**
- `cargo test -p genossi_mail --lib template::tests` → 35 passed, 0 failed
- `cargo build --workspace` → success (nur pre-existing warnings)
- `cargo test --workspace` → keine Regressionen, keine Failures
- `rustfmt --edition 2021 --check genossi_mail/src/template.rs` → FMT OK
- `cargo clippy -p genossi_mail --all-targets` → 0 neue Warnings für template.rs

---
*Phase: 10-massenmail-anbindung-template-variablen*
*Plan: 05*
*Completed: 2026-05-31*
