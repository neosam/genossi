---
quick_id: 260603-n3m
slug: mail-test-endpoints-dummy-repayment-cont
status: complete
one_liner: Detect-based Dummy-Repayment-Merge im None-Arm beider Test-Endpoints — Template-Editor-Preview rendert `{{ payout_amount }}` jetzt mit Sentinel-Werten + `used_dummy_repayment` Banner ohne `repayment_phase_id`.
commits:
  - baba99e
files_modified:
  - genossi_mail/src/template.rs
  - genossi_mail/src/rest.rs
test_counts:
  genossi_mail_baseline: 159
  genossi_mail_after: 168
  delta: 9
  added_in_template_rs: 7
  added_in_rest_rs: 2
audit_path_protection:
  worker_rs_dummy_grep_hits: 0
  repayment_letter_rs_dummy_grep_hits: 0
key_decisions:
  - "Detect-based Merge (Substring-Match auf 4 Repayment-Var-Namen) statt always-merge — haelt `used_dummy_repayment` Banner vertrauenswuerdig, luegt nicht bei Pure-Member-Templates"
  - "Substring-Suche statt AST-Parsing: Jinja-Ausdrucks-Varianten (`{{ X }}`, `{% if X is defined %}`, `{# X #}`) enthalten den Variablen-Namen jeweils als Substring; False-Positives bei Literalen sind harmlos (Dummy-Merge ist additiv)"
  - "Guarded References (`{% if X is defined %}`) werden bewusst auch detektiert — wenn ein else-Zweig dieselbe Variable nutzt, wuerde der Render trotzdem auf undefined laufen"
tags: [mail-template, dummy-repayment, quick]
---

# Quick 260603-n3m: Dummy-Repayment-Merge in Mail-Test-Endpoints (Continuation von kon)

**Commit:** `baba99e`
**Branch:** main
**Predecessor:** 260603-kon (PreviewResponse-Banner + Some(phase_id)/None-Resolve-Fallback)
**Successor:** none

## Was wurde umgesetzt

### Task 1 — `template_uses_repayment_vars` Detection-Helper (`genossi_mail/src/template.rs`)

Neue `pub fn template_uses_repayment_vars(subject: &str, body: &str) -> bool` direkt unter
`dummy_repayment_context()` (Z. 215–244). Implementation: Substring-Suche auf den vier
Repayment-Variablen-Namen `payout_amount`, `share_count`, `share_value`, `fiscal_year`
gegen die Konkatenation aus Subject und Body.

Doc-Comment erklaert WARUM Substring statt AST-Parse und WARUM Detection statt always-merge
(Banner-Vertrauenswuerdigkeit).

7 neue Unit-Tests im `mod tests`-Block am Dateiende:
- `test_template_uses_repayment_vars_pure_member_template` (false)
- `test_template_uses_repayment_vars_detects_payout_amount_in_body`
- `test_template_uses_repayment_vars_detects_share_count`
- `test_template_uses_repayment_vars_detects_share_value`
- `test_template_uses_repayment_vars_detects_fiscal_year`
- `test_template_uses_repayment_vars_detects_in_subject`
- `test_template_uses_repayment_vars_detects_guarded_reference`

### Task 2 — None-Arm-Merge in beiden Test-Handlern (`genossi_mail/src/rest.rs`)

Die `_ => (base_ctx, false),`-Arme in `preview_mail` (~Z. 649) und
`send_test_mail_with_template` (~Z. 797) wurden durch identische
Detection+Merge-Bloecke ersetzt:

```rust
_ => {
    if crate::template::template_uses_repayment_vars(&body.subject, &body.body) {
        let (payout, share_count, share_value, fiscal_year) =
            crate::template::dummy_repayment_context();
        (crate::template::merge_repayment_context(base_ctx, payout, share_count, share_value, fiscal_year), true)
    } else {
        (base_ctx, false)
    }
}
```

2 neue Tests im `mod tests`-Block am Dateiende von `rest.rs`:
- `test_dummy_merge_applies_when_no_phase_id_and_template_uses_repayment_var`
  (End-to-End-Beweis: Detection + Merge + Render → Output enthaelt `"99,99"`)
- `test_dummy_merge_does_not_apply_for_pure_member_template`
  (Negativ-Pfad: Pure-Member-Template triggert die Detection nicht)

Nach allen Edits per-File `rustfmt --edition 2021` auf beide Dateien angewendet
(Tool-Pfad: `/nix/store/.../rustfmt-preview-1.93.0/bin/rustfmt`).

### Task 3 — Commit via gsd-sdk

Single feat-Commit `baba99e` mit exakt den zwei `files_modified`-Eintraegen — keine
Pre-Existing-Drift in anderen Dateien wurde mit-gestaged.

## Verhaltens-Matrix

| Request | Template-Vars | Resolve-Result | ctx + flag (NACH) | ctx + flag (VOR) |
|---------|---------------|----------------|-------------------|------------------|
| `Some(phase_id)` | egal | `Some(real)` | real-merge, `used_dummy_repayment=false` | **unchanged** |
| `Some(phase_id)` | egal | `None` (kein offener Entry) | dummy-merge, `used_dummy_repayment=true` (kon) | **unchanged** (kon) |
| `None` / `""` | Repayment-Var referenziert | n/a | dummy-merge, `used_dummy_repayment=true` (**n3m**) | **changed (war: undefined-var error)** |
| `None` / `""` | nur Member-Vars | n/a | base-only, `used_dummy_repayment=false` (skip_serializing_if greift) | **unchanged** |

Note: row 3 ist die einzige Verhaltensaenderung (kon hatte hier noch `(base_ctx, false)`,
was bei `{{ payout_amount }}`-Templates strict-env minijinja mit "undefined variable" failen
liess). Rows 1, 2 und 4 sind 1:1 Wire-kompatibel zu 260603-kon.

## Audit-Pfad-Schutz

Grep-Gates nach Commit:

```
grep -c "dummy_repayment\|template_uses_repayment_vars" genossi_mail/src/worker.rs                  -> 0
grep -c "dummy_repayment\|template_uses_repayment_vars" genossi_service_impl/src/repayment_letter.rs -> 0
```

Sentinel-Werte (`"99,99"`, `99`, `"99,99"`, `2099`) bleiben Single-Source-of-Truth in
`dummy_repayment_context()` (`template.rs:212`). Test `test_dummy_repayment_context_sentinel_values_locked`
(aus 260603-kon) verteidigt sie weiterhin gegen versehentliche Aenderungen.

Bestaetigung der Untouched-Files in dieser Session:
- `git diff --name-only genossi_service_impl/src/repayment_letter.rs` → leere Ausgabe
- `genossi_mail/src/worker.rs` ist zwar im Working-Tree als modifiziert markiert,
  aber die Modifikationen stammen aus einer vorherigen Session (siehe Notes-for-Orchestrator);
  in diesem Commit wurde `worker.rs` NICHT gestaged — Audit-Path-Grep liefert 0.

## Verification Output

`cargo test -p genossi_mail` (letzte 30 Zeilen):

```
test template::tests::test_validate_template_with_repayment_accepts_unguarded_payout_amount ... ok
test worker::tests::non_reply_mail_has_no_in_reply_to_header ... ok
test worker::tests::multipart_mail_body_has_utf8_charset ... ok
test worker::tests::test_build_member_document_entity_status_failed_with_truncation ... ok
test worker::tests::test_build_member_document_entity_status_sent ... ok
test worker::tests::plain_mail_body_has_utf8_charset ... ok
test worker::tests::reply_mail_includes_in_reply_to_header ... ok
test worker::tests::test_get_send_interval_custom ... ok
test worker::tests::test_get_send_interval_config_error ... ok
test worker::tests::test_get_send_interval_default ... ok
test worker::tests::test_get_send_interval_invalid_value ... ok
test worker_audit::tests::test_compute_entry_hash_matches_service_impl_for_known_input ... ok
test worker_audit::tests::test_compute_entry_hash_produces_64_char_sha256 ... ok
test dao_sqlite::tests::test_static_document_all_active_sorted_by_name ... ok
test dao_sqlite::tests::test_recipient_update_failed ... ok
test dao_sqlite::tests::test_static_document_create_and_find ... ok
test dao_sqlite::tests::test_static_document_find_many_by_ids ... ok
test dao_sqlite::tests::test_recipient_update_persists_message_id ... ok
test dao_sqlite::tests::test_static_document_soft_delete_hides_from_find ... ok
test worker::tests::test_update_job_with_retry_succeeds_on_second_attempt ... ok
test worker::tests::test_update_job_with_retry_fails_after_3_attempts ... ok

test result: ok. 168 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s

   Doc-tests genossi_mail

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Build-Sanity:
- `cargo build -p genossi_mail` → green (3.73s)
- `cargo build --bin genossi` → green (43.77s, Router-Wiring intakt)
- `cargo clippy -p genossi_mail --all-targets` → green, keine neuen Warnings

## Notes for Orchestrator

Pre-existing dirty working-tree (NICHT in diesem Commit eingespielt — bewusst unangetastet
gelassen per Runtime-Context):

```
M genossi-frontend/src/page/{assemblies,assembly_details,config_page,helper_attendance,repayment_phases}.rs
M genossi_bin/src/lib.rs
M genossi_dao/src/{assembly,helper_token,member,repayment_entry,repayment_phase}.rs
M genossi_dao_impl_sqlite/src/{helper_token,repayment_phase}.rs
M genossi_mail/src/{dao_sqlite,inbox,static_document_service,worker}.rs
M genossi_rest/src/{attendance_export,lib,repayment_entry,repayment_export,repayment_phase}.rs
M genossi_rest_types/src/lib.rs
M genossi_service/src/{attendance,iban,repayment_export}.rs
M genossi_service_impl/src/{assembly,attendance,attendance_export,pdf_generation,repayment_context,repayment_entry,repayment_export,template_storage}.rs
```

Davon committed: nichts (Scope-Boundary). Diese Mods existieren weiterhin im Working-Tree
nach dem Commit — Orchestrator entscheidet, wann/ob sie aufgeraeumt werden.

## Self-Check: PASSED

- [x] `template_uses_repayment_vars` ist `pub fn` in `genossi_mail/src/template.rs` (Z. 240–246 nach rustfmt)
- [x] 7 Detection-Tests gruen (`test_template_uses_repayment_vars_*`)
- [x] 2 None-Arm-Tests gruen (`test_dummy_merge_*`)
- [x] Alle 4 `test_preview_response_*`-Tests aus 260603-kon weiter gruen (no regression)
- [x] `grep -c "template_uses_repayment_vars(" genossi_mail/src/rest.rs` Production-Call-Sites = 2 (preview_mail + send_test_mail_with_template)
- [x] `grep -c "dummy_repayment\|template_uses_repayment_vars" genossi_mail/src/worker.rs` = 0
- [x] `grep -c "dummy_repayment\|template_uses_repayment_vars" genossi_service_impl/src/repayment_letter.rs` = 0
- [x] Sentinel-Werte `"99,99"`/`99`/`"99,99"`/`2099` unveraendert in `dummy_repayment_context()`
- [x] `cargo test -p genossi_mail` 168 passed (159 baseline + 9 neu)
- [x] `cargo build --bin genossi` durchlaeuft
- [x] `cargo clippy -p genossi_mail --all-targets` keine neuen Warnings
- [x] Commit `baba99e` enthaelt ausschliesslich `genossi_mail/src/template.rs` und `genossi_mail/src/rest.rs`
