---
phase: 30-application-template-kontext-antragsteller-vorlagen
plan: 03
subsystem: api
tags: [mail-templates, minijinja, strict-render, validation, applicant-context, dsgvo, euro-format]

# Dependency graph
requires:
  - phase: 30-01
    provides: "genossi_service::euro::format_eur_de (deutscher Euro-Formatter für open_amount)"
  - phase: 30-02
    provides: "mail_templates.template_type ('member'|'application'), Service-create(template_type), Zahlungserinnerung-Seed (UUID …0003)"
provides:
  - "application_to_template_context(app, share_value_cents, bank_iban, bank_name, bank_bic, genossenschaft_name) -> minijinja::Value — reiner/synchroner Antragsteller-Kontext-Builder (nur Antragsteller-Keys + Genossenschafts-Zahlungsdaten, kein Member-Key)"
  - "dummy_application_context() — Sentinel-Probe mit identischer Schlüsselmenge für Author-Zeit-Validierung"
  - "validate_rendered(subject, body, &[Value]) — generischer Strict-Render-Kern"
  - "validate_application_template(subject, body) — Antragsteller-Template-Validierung, Err bei member-only/Syntaxfehler"
  - "Create/Update-Validierungs-Injektion in MailTemplateServiceImpl für template_type == 'application' (BadRequest/400)"
affects: [phase-31-service-rest-versand, mail-template-crud]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pool-getrennter, reiner Render-Kontext-Builder: exponiert nur die erlaubte Feldmenge, member-only-Keys sind nie definiert → Strict-Render fällt fehl statt zu leaken (DSGVO-Trennung by omission)"
    - "Generischer validate_rendered-Kern + typ-spezifische Delegation (validate_application_template); member-spezifische Fehlertexte bleiben im bestehenden validate_template"
    - "Additive, typ-konditionale Validierungs-Injektion an create/update (nur 'application'), Member-Pfad bleibt unverändert validierungsfrei"

key-files:
  created: []
  modified:
    - genossi_mail/src/template.rs
    - genossi_mail/src/mail_template_service.rs
    - genossi_bin/tests/e2e_tests.rs

key-decisions:
  - "validate_template bleibt unverändert (Signatur + member_number-Fehlertexte); validate_rendered ist ein NEUER generischer Kern, der nur den Antragsteller-Pfad bedient — kein Umbau des grün getesteten Member-Pfads (D-09, geringstes Risiko)"
  - "Validierung ist eine NEUE additive Injektion an create/update (kein bestehendes create/update validierte zuvor — Member-Templates werden weiterhin erst beim Send validiert); Guard keyt strikt auf template_type == 'application' (D-10)"
  - "dummy_application_context nutzt Sentinel-Werte (DUMMY-VORNAME, shares 99, open_amount '9.999,99 €', bank_bic Some('DUMMYBIC')) und definiert exakt die 10 Antragsteller-Keys — nicht mehr, nicht weniger (D-08)"
  - "e2e-Beweis liest den TATSÄCHLICH geseedeten …0003-Body über GET /api/mail/templates und validiert ihn — kein Content-Drift zwischen Seed-SQL und Builder (APTPL-03/D-14)"

patterns-established:
  - "Author-Zeit-Validierung gegen einen Sentinel-Dummy-Kontext fängt Strict-Render-Bomben (member-only-Platzhalter) ab, bevor ein Template persistiert — der Send-Pfad crasht nie unter UndefinedBehavior::Strict"

requirements-completed: [APTPL-01, APTPL-04, APTPL-03]

# Coverage metadata
coverage:
  - id: D1
    description: "application_to_template_context exponiert first_name/last_name/salutation/title/shares/open_amount/bank_iban/bank_name/bank_bic/genossenschaft_name (member-kompatible Namen, open_amount via format_eur_de), keine member-only Keys"
    requirement: "APTPL-01"
    verification:
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_application_to_template_context_renders_all_keys"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_application_to_template_context_open_amount_matches_format_eur_de"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_application_context_omits_member_only_keys"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_application_to_template_context_anrede_herr_frau_neutral"
        status: pass
    human_judgment: false
  - id: D2
    description: "dummy_application_context definiert exakt dieselbe Schlüsselmenge wie der reale Builder"
    requirement: "APTPL-04"
    verification:
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_dummy_application_context_defines_same_keys"
        status: pass
    human_judgment: false
  - id: D3
    description: "validate_application_template: Ok für Antragsteller-Keys/guarded bank_bic, Err für member-only-Key und Syntaxfehler; validate_template-Signatur + Member-Verhalten unverändert (56+ template.rs-Tests grün)"
    requirement: "APTPL-04"
    verification:
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_validate_application_template_accepts_applicant_keys"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_validate_application_template_rejects_member_only_key"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_validate_application_template_rejects_syntax_error"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/template.rs#tests::test_validate_template_signature_and_member_behavior_unchanged"
        status: pass
    human_judgment: false
  - id: D4
    description: "MailTemplateServiceImpl::create/update lehnt kaputte Antragsteller-Templates mit BadRequest (400) ab, Member-Pfad bleibt validierungsfrei"
    requirement: "APTPL-04"
    verification:
      - kind: unit
        ref: "genossi_mail/src/mail_template_service.rs#tests::create_application_rejects_member_only_placeholder"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/mail_template_service.rs#tests::create_application_accepts_valid_applicant_body"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/mail_template_service.rs#tests::create_member_with_member_number_stays_valid"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/mail_template_service.rs#tests::update_application_rejects_member_only_placeholder"
        status: pass
    human_judgment: false
  - id: D5
    description: "Der geseedete Zahlungserinnerung-Body (UUID …0003) validiert strict gegen den Antragsteller-Kontext (kein Send-Crash, kein Content-Drift)"
    requirement: "APTPL-03"
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#mail_template_zahlungserinnerung_seed_is_render_safe"
        status: pass
    human_judgment: false

# Metrics
duration: 20 min
completed: 2026-08-20
status: complete
---

# Phase 30 Plan 03: Antragsteller-Template-Kontext + Author-Zeit-Validierung Summary

**Reiner/synchroner `application_to_template_context`-Builder (nur Antragsteller-Felder + Genossenschafts-Zahlungsdaten, `open_amount` via `format_eur_de`, keine Member-Keys), generischer `validate_rendered`-Kern + `validate_application_template`, und additive Create/Update-Validierung (BadRequest/400) für `template_type == "application"` — plus e2e-Beweis, dass der geseedete Zahlungserinnerung-Body strict-render-sicher ist.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-08-20T09:16:00Z
- **Completed:** 2026-08-20T09:35:51Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments
- `application_to_template_context` gebaut: reiner/synchroner Builder (kein `.await`/`config.get`), exponiert exakt die 10 Antragsteller-Keys mit member-kompatiblen Namen (`salutation` via `Salutation::as_str` → die bestehende `{% if salutation == "Herr" %}`-Anrede greift unverändert); `open_amount = format_eur_de(share_value_cents × shares)`. Member-only-Keys (member_number, current_shares, join_date …) sind bewusst NICHT definiert → sie fallen unter Strict-Render fehl statt Mitgliederdaten zu leaken (DSGVO-Trennung, T-30-03-03).
- `dummy_application_context` als Sentinel-Probe mit identischer Schlüsselmenge; generischer `validate_rendered(subject, body, &[Value])`-Kern extrahiert; `validate_application_template` delegiert genau einen Dummy-Kontext.
- Additive, typ-konditionale Validierungs-Injektion an `MailTemplateServiceImpl::create` (eingehender `template_type`) und `::update` (`existing.template_type`, unveränderlich): kaputte Antragsteller-Templates (member-only/Syntaxfehler) werden mit `MailTemplateError::BadRequest` (400) abgelehnt und persistieren nie; der Member-Pfad bleibt validierungsfrei.
- e2e-Beweis: der über `GET /api/mail/templates` gelesene, TATSÄCHLICH geseedete Zahlungserinnerung-Body (UUID …0003) validiert strict gegen den Antragsteller-Kontext — kein Drift zwischen Seed-SQL und Builder (APTPL-03).

## Task Commits

Jede Aufgabe wurde atomar committet:

1. **Tasks 1 + 2: application_to_template_context + dummy + validate_rendered/validate_application_template** - `62fcbb7` (feat)
2. **Task 3: Create/Update-Validierungs-Injektion + Seed-Render-Beweis** - `f093ea1` (feat)

**Plan-Metadaten:** (dieser docs-Commit)

_Hinweis: Tasks 1 und 2 wurden zusammen committet — beide sind eng gekoppelte, rein additive Ergänzungen derselben Datei (`template.rs`); `validate_application_template` (Task 2) hängt direkt an `dummy_application_context` (Task 1). Ein sauberes Aufsplitten der bereits im Working-Tree liegenden Diffs hätte im colocated jj+git-Setup (Index-Desync-Risiko) unnötiges Risiko bedeutet. Beide Tasks kompilieren und bestehen die Tests im selben Commit._

## Files Created/Modified
- `genossi_mail/src/template.rs` - `application_to_template_context`, `dummy_application_context`, `validate_rendered`, `validate_application_template` + 11 Unit-Tests
- `genossi_mail/src/mail_template_service.rs` - Validierungs-Injektion in `create`/`update` (typ-konditional), 4 Service-Unit-Tests
- `genossi_bin/tests/e2e_tests.rs` - e2e `mail_template_zahlungserinnerung_seed_is_render_safe`

## Decisions Made
- **validate_template bleibt unangetastet:** `validate_rendered` ist ein neuer generischer Kern, der ausschließlich den Antragsteller-Pfad bedient. Der member-spezifische `member #{}`-Fehlertext + die Signatur `(&str, &str, &[MemberEntity])` bleiben exakt erhalten (D-09) — geringstes Risiko, alle 56+ bestehenden template.rs-Tests grün.
- **Additive Injektion, kein bestehendes create/update-Validate erweitert:** Vor diesem Plan validierte kein Service-create/update — Member-Templates werden weiterhin erst zum Send validiert (`rest.rs`). Der neue Guard keyt strikt auf `template_type == "application"` und lässt den Member-Pfad unberührt (D-10).
- **`bank_bic == None` ist defined-as-none** (nicht undefined) → `{% if bank_bic %}` greift sauber (kein Strict-Crash), gespiegelt zur Member-Optional-Feld-Konvention.

## Deviations from Plan

None - plan executed exactly as written.

Beide Validierungs-Grep-Kriterien und alle `<acceptance_criteria>` der drei Tasks erfüllt; die einzige Ausführungsnotiz ist die kombinierte Committung von Task 1 + 2 (siehe Task-Commits-Hinweis oben) — dieselbe Datei, interdependente additive Änderungen, kein Scope-Creep.

## Issues Encountered
None.

## User Setup Required
None - keine externe Service-Konfiguration nötig. `application_to_template_context` nimmt Config-Werte als Parameter entgegen; die Phase-31-Service-Schicht löst sie aus derselben `send_confirmation_mail`-Config auf.

## Next Phase Readiness
- Der Antragsteller-Render-Pfad ist vollständig: Phase 31 (`ApplicationService::send_mail`) kann `application_to_template_context` mit aufgelösten Config-Werten aufrufen und gegen den Antragsteller-Kontext rendern.
- Author-Zeit-Validierung schützt den Send-Pfad: kein Strict-Render-Crash bei member-only/Syntaxfehler-Platzhaltern.
- Phase 30 ist damit vollständig (3/3 Pläne); v1.6 kann mit Phase 31 (Service + REST Versand) fortfahren.

## Self-Check: PASSED
- `genossi_mail/src/template.rs` (modifiziert) — FOUND
- `genossi_mail/src/mail_template_service.rs` (modifiziert) — FOUND
- `genossi_bin/tests/e2e_tests.rs` (modifiziert) — FOUND
- Commits `62fcbb7`, `f093ea1` — vorhanden im git log
- `nix develop --command cargo test -p genossi_mail` — 305 passed, 0 failed
- `nix develop --command cargo test -p genossi_bin --test e2e_tests mail_template` — 6 passed, 0 failed

---
*Phase: 30-application-template-kontext-antragsteller-vorlagen*
*Completed: 2026-08-20*
