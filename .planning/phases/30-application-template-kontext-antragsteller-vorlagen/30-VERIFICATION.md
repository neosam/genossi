---
phase: 30-application-template-kontext-antragsteller-vorlagen
verified: 2026-08-20T15:34:51Z
status: passed
score: 4/4 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 30: application-template-kontext-antragsteller-vorlagen Verification Report

**Phase Goal:** Vorlagen können gegen einen eigenen Application-Kontext gerendert werden, und der Vorstand hat eine mitgelieferte deutsche „Zahlungserinnerung" mit korrekt berechnetem, korrekt formatiertem offenem Betrag.
**Verified:** 2026-08-20T15:34:51Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Eine Vorlage rendert mit Application-Platzhaltern (Anrede, Vorname, Nachname, Titel, Anzahl Anteile, offener Betrag) über eine eigene `application_to_template_context`-Funktion — eigener „Antragsteller"-Vorlagentyp, getrennt vom Member-Pool | ✓ VERIFIED | `genossi_mail/src/template.rs:79-101` implements `pub fn application_to_template_context(app, share_value_cents, bank_iban, bank_name, bank_bic, genossenschaft_name) -> Value` exposing exactly `first_name, last_name, salutation, title, shares, open_amount, bank_iban, bank_name, bank_bic, genossenschaft_name`. Grep confirms zero occurrences of `member_number`/`current_shares`/`join_date` in this function. Pool separation confirmed via `mail_templates.template_type` column (migration `20260820000000...sql`), immutable after create, threaded through DAO/TO/service (`genossi_mail/src/dao.rs:251`, `dao_sqlite.rs`, `rest_templates.rs`, `mail_template_service.rs`). Unit tests `test_application_to_template_context_renders_all_keys`, `test_application_context_omits_member_only_keys` pass (part of 305/305 `genossi_mail` green, independently re-run). |
| 2 | Der „offene Betrag" wird zur Laufzeit als `Anteile × share_value_cents` berechnet (Quelle: dieselbe Config wie `send_confirmation_mail`), in korrektem deutschem Euro-Format angezeigt und niemals auf der Application gespeichert | ✓ VERIFIED | `genossi_service/src/euro.rs::format_eur_de` is the single canonical formatter (9/9 unit tests independently re-run and green: `0→"0,00 €"`, `5→"0,05 €"`, `123456→"1.234,56 €"`, `100000000→"1.000.000,00 €"`, `-123456→"-1.234,56 €"`, `-5→"-0,05 €"`, `i64::MIN` no panic). `application_to_template_context` computes `open_amount = format_eur_de(share_value_cents * app.shares as i64)` (`template.rs:88`) — same formatter `send_confirmation_mail` was retrofitted onto (`genossi_service_impl/src/application.rs:99-100`: `let amount_str = genossi_service::euro::format_eur_de(total_cents);`). No new field was added to `ApplicationEntity`/audited entity — amount is computed at call time from `share_value_cents` config param + `app.shares`, never persisted. |
| 3 | Eine deutsche Standard-Vorlage „Zahlungserinnerung" ist als Seed vorhanden und rendert den Haupt-Use-Case ohne manuelle Konfiguration | ✓ VERIFIED | Migration `migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql` inserts UUID `00000000-0000-0000-0000-000000000003`, name `'Zahlungserinnerung'`, `template_type='application'`, formal Sie-form German body using only applicant-context keys with guarded `{% if bank_bic %}`. e2e test `mail_template_predefined_present_after_migration` (independently re-run, passing) confirms presence after real migrations. e2e test `mail_template_zahlungserinnerung_seed_is_render_safe` (independently re-run, passing) fetches the ACTUAL seeded body via `GET /api/mail/templates` and asserts `validate_application_template(&tpl.subject, &tpl.body).is_ok()` — proving zero content drift between seed SQL and the context builder. |
| 4 | Die Validierung einer Antragsteller-Vorlage schlägt bei unbekannten oder Member-only-Platzhaltern kontrolliert fehl (kein `strict`-Render-Crash beim Versand); die bestehenden Member-Template-Tests bleiben grün (die `validate_template`-Signatur ändert sich nicht) | ✓ VERIFIED | `validate_template(subject: &str, body: &str, members: &[MemberEntity]) -> Result<(), Vec<String>>` signature is byte-identical to pre-phase (`template.rs:196-200`), body untouched. New generic core `validate_rendered` (private) + `validate_application_template` (public) added additively. `MailTemplateServiceImpl::create`/`::update` (`mail_template_service.rs:97-101`, `159-163`) call `validate_application_template` only when `template_type == "application"`, returning `MailTemplateError::BadRequest` (mapped to HTTP 400 in `rest_templates.rs:118`) on failure — member path stays validation-free. Unit tests `create_application_rejects_member_only_placeholder`, `create_application_accepts_valid_applicant_body`, `create_member_with_member_number_stays_valid`, `update_application_rejects_member_only_placeholder` all present and passing. Full `genossi_mail` suite (305/305, independently re-run) and full `genossi_bin` e2e suite (319/319, independently re-run) both green — no regression. |

**Score:** 4/4 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `genossi_service/src/euro.rs` | `format_eur_de(cents: i64) -> String`, canonical German euro formatter | ✓ VERIFIED | Exists, public, 9 unit tests, `pub mod euro;` wired in `genossi_service/src/lib.rs:10` |
| `migrations/sqlite/20260820000000_mail_templates_add_template_type.sql` | Additive `template_type` column | ✓ VERIFIED | `ALTER TABLE mail_templates ADD COLUMN template_type TEXT NOT NULL DEFAULT 'member';` |
| `migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql` | Zahlungserinnerung seed | ✓ VERIFIED | UUID …0003, `template_type='application'`, applicant-only keys, guarded bic |
| `application_to_template_context` (genossi_mail/src/template.rs) | Applicant context builder | ✓ VERIFIED | Pure/synchronous, exact 10-key set, no `.await`/`config.get` |
| `dummy_application_context` (genossi_mail/src/template.rs) | Sentinel probe for author-time validation | ✓ VERIFIED | Same key set as real builder, verified by `test_dummy_application_context_defines_same_keys` |
| `validate_rendered` generic core (genossi_mail/src/template.rs) | Context-agnostic strict-render core | ✓ VERIFIED | Private fn, extracted, feeds `validate_application_template` |
| `validate_application_template` (genossi_mail/src/template.rs) | Applicant template validator | ✓ VERIFIED | `pub fn(&str, &str) -> Result<(), Vec<String>>`, delegates to `validate_rendered` with one dummy context |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `send_confirmation_mail` (genossi_service_impl/src/application.rs) | `genossi_service::euro::format_eur_de` | `total_cents -> format_eur_de(total_cents)` | ✓ WIRED | Line 100, naive inline division-based formatting removed |
| `application_to_template_context` | `genossi_service::euro::format_eur_de` | `open_amount` computation | ✓ WIRED | `template.rs:88` |
| `MailTemplateServiceImpl::create`/`update` | `validate_application_template` | type-conditional guard on `template_type == "application"` | ✓ WIRED | `mail_template_service.rs:97-101` (create), `159-163` (update) |
| `MailTemplateError::BadRequest` | HTTP 400 response | `rest_templates.rs::error_handler` | ✓ WIRED | Line 118 |
| `mail_templates` SQL (INSERT + 4 SELECTs) | `MailTemplateDb.template_type` / `MailTemplate.template_type` | column-list threading | ✓ WIRED | 8-site threading confirmed via grep; UPDATE deliberately excludes the column (immutable-after-create) |

### Behavioral Spot-Checks / Test Execution (independently re-run by verifier, not trusted from SUMMARY)

| Command | Result | Status |
|---------|--------|--------|
| `nix develop --command cargo test -p genossi_service --features utoipa euro` | 9 passed, 0 failed | ✓ PASS |
| `nix develop --command cargo test -p genossi_mail` | 305 passed, 0 failed | ✓ PASS |
| `nix develop --command cargo test -p genossi_bin --test e2e_tests mail_template` | 6 passed, 0 failed (incl. `mail_template_zahlungserinnerung_seed_is_render_safe`, `mail_template_predefined_present_after_migration`, `mail_template_type_application_roundtrips_on_wire`) | ✓ PASS |
| `nix develop --command cargo test -p genossi_bin --test e2e_tests` (full suite, run once) | 319 passed, 0 failed | ✓ PASS |
| `nix develop --command cargo clippy -p genossi_service -p genossi_mail -p genossi_service_impl --features utoipa --all-targets` | 1 minor style warning in `euro.rs` (`manual implementation of .is_multiple_of()`), no errors, no warnings in other phase-30 files | ℹ️ INFO |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| APTPL-01 | 30-02, 30-03 | Application-Template-Kontext, eigener Vorlagentyp | ✓ SATISFIED | `application_to_template_context` + `template_type` pool separation, both verified above |
| APTPL-02 | 30-01 | Laufzeit-Berechnung offener Betrag, korrekte deutsche Formatierung, nie gespeichert | ✓ SATISFIED | `format_eur_de` + `open_amount` computation, verified above |
| APTPL-03 | 30-02, 30-03 | Zahlungserinnerung-Seed | ✓ SATISFIED | Seed migration + e2e render-safety proof, verified above |
| APTPL-04 | 30-03 | Validierung schlägt kontrolliert fehl, `validate_template`-Signatur unverändert | ✓ SATISFIED | `validate_application_template` + create/update injection + unchanged signature, verified above |

**Note:** `.planning/REQUIREMENTS.md` line 21-24 checkboxes correctly show `[x]` for all four APTPL requirements. However, the separate "Traceability" table at the bottom of the same file (lines 81, 83, 84) still shows APTPL-01, APTPL-03, APTPL-04 as `Pending` (only APTPL-02 shows `Complete`) — this is a stale documentation artifact, not a code gap. Recommend updating the traceability table status column to `Complete` for all four APTPL rows in a follow-up docs commit; it does not block phase-goal achievement since the code evidence for all four is verified above.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` debt markers found in any phase-30 modified file | — | None |
| `genossi_service/src/euro.rs` | 52 | Clippy style suggestion (`manual implementation of .is_multiple_of()`) | ℹ️ Info | Cosmetic only, does not affect correctness; pre-existing style, not a functional gap |

### Data-Flow Trace (Level 4)

`open_amount` traced end-to-end: `share_value_cents` (config, same source as `send_confirmation_mail`) × `app.shares` (real `Application` entity field, not hardcoded) → `format_eur_de` → `open_amount` context key → rendered into the seeded Zahlungserinnerung body via the e2e render-safety test. No static/empty fallback found; no hardcoded amount anywhere in `application_to_template_context`.

### Human Verification Required

None. All four success criteria are code-verifiable (pure functions, unit tests, e2e tests) and were independently re-executed by the verifier (not trusted from SUMMARY.md claims).

### Gaps Summary

No gaps found. All four Success Criteria from ROADMAP.md and all four requirement IDs (APTPL-01..04) are backed by code that was read directly and by test runs executed independently by this verifier (not copied from SUMMARY.md). The only finding is a stale status cell in the REQUIREMENTS.md traceability table (informational, not a code gap — see Requirements Coverage note above).

---

_Verified: 2026-08-20T15:34:51Z_
_Verifier: Claude (gsd-verifier)_
