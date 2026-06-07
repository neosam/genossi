---
quick_id: 260607-mw9
slug: kontoinhaber-feld-am-member
status: complete
one_liner: Optionales account_holder-Feld am Member — Vorstand kann auf der Detail-Seite einen abweichenden Kontoinhaber setzen; Auszahlungs-Anschreiben adressiert ihn im Recipient-Block (Fallback Mitgliedsname).
---

# Quick 260607-mw9 — Kontoinhaber-Feld am Member

## Übersicht

Genau ein neues optionales Feld `account_holder: Option<Arc<str>>` durchgängig
durch alle Schichten (DB → DAO → Service → REST → Frontend → Typst-PDF).
Der Wert landet im Auszahlungs-Anschreiben als Recipient-Adressblock; ist er
nicht gesetzt, fällt das Template zurück auf den existierenden Mitgliedsnamen
(`title + first_name + last_name`).

Anrede im Brief-Body bleibt bewusst auf dem Mitgliedsnamen — der Brief
richtet sich textlich ans Mitglied, der Adress-Header an den Kontoinhaber.

## Geänderte Files (gruppiert nach Layer)

### DB / DAO

- `migrations/sqlite/20260607000000_add_account_holder_to_member.sql` — neue Spalte (nullable TEXT)
- `genossi_dao/src/member.rs` — `MemberEntity.account_holder` + `Auditable::audit_fields()` APPENDED AT END
- `genossi_dao_impl_sqlite/src/member.rs` — `MemberDb.account_holder`, SELECT/INSERT/UPDATE-SQL erweitert, `TryFrom<&MemberDb>` ergänzt
- Bind-Order verifiziert: INSERT 25 Spalten / 25 `?` / 25 `.bind()`; UPDATE 24 SET + 2 WHERE / 26 `.bind()`

### Service-Layer

- `genossi_service/src/member.rs` — `Member.account_holder` + beide `From`-Impls; neue Tests `test_member_to_entity_preserves_account_holder` + `test_member_to_entity_none_account_holder_roundtrip`
- `genossi_service_impl/src/member.rs` — Create-Path propagiert `account_holder` vom Eingabe-Item

### REST-Types

- `genossi_rest_types/src/lib.rs` — `MemberTO.account_holder` (skip_serializing_if=None), beide `From`-Impls, neue Tests `test_member_to_serializes_account_holder_when_some` + `test_member_to_omits_account_holder_when_none` + `test_member_to_account_holder_roundtrip`; PII-Slim-Guard erweitert (`!obj.contains_key("account_holder")` in `MemberSlimTO`-PII-Test)
- `genossi-frontend/rest-types/src/lib.rs` — gespiegelter Mirror-Crate fürs Frontend; Test-Helper `make_member` / `sample_member` ergänzt

### PDF-Pipeline (Typst)

- `genossi_service_impl/src/pdf_generation.rs` — `account_holder` in vier JSON-Builder eingefügt:
  - `PdfGenerator::build_inputs(&Member)` (generischer Pfad für nicht-Repayment-Templates)
  - `build_inputs_repayment_letter` (Single-Letter)
  - `build_inputs_repayment_letters_bundle` Bundle-Loop (pro Recipient)
  - `build_inputs_repayment_letters_bundle` First-Recipient-Compat + Empty-Bundle-Compat
- `templates/defaults/auszahlungs_anschreiben.typ` — neuer Helper `account-holder-for(m)` mit `m.at("account_holder", default: none)` (defensive), Recipient-Block nutzt jetzt `#account-holder-for(m)` statt `#name`
- `templates/auszahlungs_anschreiben.typ` — byte-identische Kopie
- `genossi_bin/templates/auszahlungs_anschreiben.typ` — byte-identische Kopie

Verifikation: `diff -q` zwischen den 3 Templates exitet 0/silent.

### Frontend (Dioxus / WASM)

- `genossi-frontend/src/i18n/mod.rs` — neuer `Key::AccountHolder`
- `genossi-frontend/src/i18n/de.rs` — `"Kontoinhaber"`
- `genossi-frontend/src/i18n/en.rs` — `"Account Holder"`
- `genossi-frontend/src/page/member_details.rs` — `MemberTO`-Default ergänzt + neues Input-Feld direkt unter Bankverbindung (Pattern wie `bank_account`, Empty-String → None)
- `genossi-frontend/src/component/membership_adjust_modal.rs` — `to_member_to(slim)`-Adapter ergänzt `account_holder: None`

### Tests (Adapter — verteilte Konstruktoren)

Folgende Files konstruieren `MemberEntity { ... }` direkt (Service-Layer + Test-Helpers); jeder Konstruktor wurde um `account_holder: None,` ergänzt:

- `genossi_dao/src/member.rs` (Test-Helper `make_entity_with_exit`)
- `genossi_service/src/member.rs` (Test-Helper `make_member`)
- `genossi_service_impl/src/member.rs` (Test `sample_member_entity`)
- `genossi_service_impl/src/validation.rs`
- `genossi_service_impl/src/member_import.rs` (Produktiv-Pfad: Excel-Import setzt None, Verbandsformat hat kein Pendant)
- `genossi_service_impl/src/application.rs` (Produktiv-Pfad: Application→Member Konversion setzt None)
- `genossi_service_impl/src/assembly.rs` (Test-Helper `make_member`)
- `genossi_service_impl/src/repayment_export.rs`
- `genossi_service_impl/src/repayment_letter.rs`
- `genossi_service_impl/src/repayment_entry.rs`
- `genossi_service_impl/src/repayment_phase.rs`
- `genossi_service_impl/src/pdf_generation.rs` (zwei Helper: `sample_member_with_iban`, `test_member`)
- `genossi_service_impl/src/membership_adjust.rs`
- `genossi_service_impl/src/member_action.rs`
- `genossi_mail/src/template.rs`
- `genossi_mail/src/rest.rs`
- `genossi_rest/src/dev.rs` (Test-Daten-Generator)

E2E-Tests (`MemberTO`-Konstruktoren ergänzt):

- `genossi_bin/tests/e2e_tests.rs` (`sample_member`) + ATTN-01 PII-Forbidden-Liste erweitert (Rule 2 Defense-in-Depth)
- `genossi_bin/tests/repayment_letter_e2e.rs` (`sample_member_with_iban`)
- `genossi_bin/tests/transfer_recipients_e2e.rs` (`sample_member`) + PII-Slim-Body-Check (`!body.contains("\"account_holder\"")`)
- `genossi_bin/tests/membership_adjust_e2e.rs` (`sample_member`)

### Neue Tests

| Test | Pfad | Zweck |
| --- | --- | --- |
| `test_auditable_fields_count` (geändert 21 → 22 + contains-Check) | `genossi_dao/src/member.rs` | Audit-Feldzahl & Existenz |
| `test_auditable_account_holder_appended_at_end` (NEU) | `genossi_dao/src/member.rs` | Hashchain-Stabilität (FROZEN-Order) |
| `test_member_to_entity_preserves_account_holder` (NEU) | `genossi_service/src/member.rs` | Round-trip Some |
| `test_member_to_entity_none_account_holder_roundtrip` (NEU) | `genossi_service/src/member.rs` | Round-trip None |
| `test_member_to_serializes_account_holder_when_some` (NEU) | `genossi_rest_types/src/lib.rs` | JSON enthält Wert |
| `test_member_to_omits_account_holder_when_none` (NEU) | `genossi_rest_types/src/lib.rs` | `skip_serializing_if` |
| `test_member_to_account_holder_roundtrip` (NEU) | `genossi_rest_types/src/lib.rs` | TO ↔ Member |
| `test_member_slim_to_serializes_no_pii_fields` (erweitert) | `genossi_rest_types/src/lib.rs` | PII-Guard mit `account_holder` |
| `test_build_inputs_includes_account_holder_when_some` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Generic build_inputs |
| `test_build_inputs_account_holder_null_when_none` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Generic build_inputs Null |
| `test_build_inputs_repayment_letter_includes_account_holder_when_some` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Single-Letter Builder |
| `test_build_inputs_repayment_letter_account_holder_null_when_none` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Single-Letter Null |
| `test_build_inputs_bundle_includes_account_holder_per_recipient` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Bundle pro Recipient |
| `test_build_inputs_bundle_empty_compat_has_account_holder_null` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Empty-Bundle Compat |
| `test_render_repayment_letter_with_account_holder_renders_ok` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Echter Typst-Render Smoke |
| `test_render_repayment_letter_account_holder_none_falls_back_to_member_name` (NEU) | `genossi_service_impl/src/pdf_generation.rs` | Fallback-Smoke |

Resultat: 47 pdf_generation-Tests + 44 repayment_letter-Tests + 23 genossi_dao member-Tests + 7 genossi_rest_types member_slim_to-Tests + 18 frontend-rest-types-Tests — alle grün. Workspace-`cargo test --lib`: 414 Tests in genossi_service_impl, 0 Fehler.

## Commits (jj change-IDs + git short hashes + Messages)

| jj change | git short | Message |
| --- | --- | --- |
| `xkktxvmq` | `8378de9f` | feat(member): add optional account_holder column (DAO + migration) [260607-mw9] |
| `pvqtspmm` | `7dfad179` | feat(member): account_holder in service/REST/frontend layers + i18n [260607-mw9] |
| `putoqrwo` | `fde0518f` | feat(repayment): use account_holder in Auszahlungs-Anschreiben recipient block [260607-mw9] |

## audit_fields() Vorher → Nachher (21 → 22)

Vorher (Stand main, `genossi_dao/src/member.rs:202-245`):
1. member_number 2. first_name 3. last_name 4. salutation 5. title 6. email 7. company 8. comment 9. street 10. house_number 11. postal_code 12. city 13. join_date 14. shares_at_joining 15. current_shares 16. current_balance 17. action_count 18. migrated 19. exit_date 20. bank_account 21. status.

Nachher:
1-21 unverändert, **22. account_holder** APPENDED AT END.

Test `test_auditable_account_holder_appended_at_end` verifiziert per `fields.last() == "account_holder"` dass kein anderes Feld hinzukommt UND dass die Position das letzte Slot ist — sonst bricht der SHA256-Hashchain auf existierenden audit_log-Rows (Phase-7-Lektion, FROZEN-Order).

## Deviations vom Plan

**Auto-Fixes (Rule 1/2/3 — Scope Boundary respektiert):**

1. **[Rule 2 - Missing critical functionality] `account_holder` als Spread-Bug-Vorbeugung in `MembershipAdjustModal::to_member_to(slim)`.**
   - Found during: Task 2, Frontend-Build-Fehler.
   - Issue: Beim Verändern von `MemberTO` schlägt der Frontend-Code in `membership_adjust_modal.rs` fehl, weil der Slim-To-Full-Adapter alle Felder explizit listet.
   - Fix: `account_holder: None,` ergänzt (PII-Guard: Slim hat das Feld auch nicht).
   - File: `genossi-frontend/src/component/membership_adjust_modal.rs`.

2. **[Rule 2 - Missing critical functionality] PII-Slim-Forbidden-Liste in 2 E2E-Tests erweitert.**
   - Issue: `bank_account` ist in der Forbidden-Liste, aber `account_holder` (ähnliche PII-Stufe) war neu. Defense-in-Depth.
   - Fix: `account_holder` zur `forbidden`-Liste in `e2e_tests.rs::test_attendance_members_response_has_no_pii_fields` und in der Body-Substring-Liste in `transfer_recipients_e2e.rs` ergänzt.
   - Files: `genossi_bin/tests/e2e_tests.rs`, `genossi_bin/tests/transfer_recipients_e2e.rs`.

3. **[Rule 2 - Missing critical functionality] `account_holder` im PII-Guard-Test für `MemberSlimTO`.**
   - Issue: Unit-Test `test_member_slim_to_serializes_no_pii_fields` listete `email`, `bank_account`, `iban`, `street`, `current_shares`, `current_balance`, `postal_code`, `city` — `account_holder` fehlte.
   - Fix: Assertion ergänzt.
   - Files: `genossi_rest_types/src/lib.rs`.

4. **[Plan Sub-Step nachgezogen] `account_holder` auch in `pdf_generation.rs::build_inputs_repayment_letter` UND `build_inputs_repayment_letters_bundle` (alle 4 Code-Pfade) eingefügt.**
   - Reason: Der Plan adressierte nur `PdfGenerator::build_inputs` (generic). Aber das Auszahlungs-Anschreiben (Plan-Ziel!) läuft durch die `_repayment_letter` und `_bundle` Pfade — ohne Patch dort wäre `m.account_holder` im Template `none`.
   - Fix: 4 JSON-Builder-Pfade ergänzt (Single + Bundle-Loop + First-Recipient-Compat + Empty-Bundle-Compat) inkl. Unit-Tests.

5. **[Rule 3 - Blocking] `Member { ... }`-Konstruktor in `genossi_rest/src/dev.rs` ergänzt.**
   - Plan listete diesen nicht explizit, aber der Service-Layer-`Member`-Struct hat ein neues Pflicht-Feld → Compile-Fehler ohne Patch.
   - File: `genossi_rest/src/dev.rs`.

**Out-of-Scope (Scope Boundary — pre-existing, NICHT angefasst):**

- `genossi_service_impl/src/membership_adjust.rs` und ähnliche Files haben pre-existing rustfmt-Diffs (z.B. `tx_dao.expect_use_transaction().returning(...)`-Zeilen), die NICHTS mit `account_holder` zu tun haben — Scope-Boundary respektiert, NICHT auto-formattiert.

Keine Architectural Changes (Rule 4) notwendig — alles Pattern-Match zu existing `bank_account` und Audit-Pattern.

## Manueller Verifikations-Checkpoint (Task 4)

Task 4 ist ein `checkpoint:human-verify`-Gate und wird vom Executor NICHT automatisch ausgeführt. Der Vorstand muss folgende 6 Schritte manuell prüfen (vollständige Anleitung in `260607-mw9-PLAN.md` Z. 332-367):

1. `cargo run --bin genossi` + `dx serve` starten
2. `sqlite3 genossi.db ".schema member" | grep account_holder` → 1 Treffer (Migration angewendet)
3. Member-Detail-Seite: Feld "Kontoinhaber" setzen / löschen / reload — persistiert
4. `GET /api/audit/member/{id}` zeigt `account_holder` mit old/new; `GET /api/audit/verify` → `is_valid: true`
5. Member A mit `account_holder = "Erika Mustermann"` → PDF zeigt Erika im Recipient-Block oben links, Anrede bleibt aufs Mitglied; Member B ohne → Recipient-Block zeigt Mitgliedsnamen (Fallback)
6. `GET /api/members/slim` enthält KEIN `account_holder` (PII-Whitelist intakt)

Bei "approved" Anweisung kann das Quick als komplett markiert werden. Bei "issues: ..." entsteht ggf. ein Bug-Quick als Folge.

## Verification Summary

- `cargo build --workspace`: grün
- `cargo test --workspace --lib`: alle 414+ Tests grün (jeder einzelne `test result: ok. … 0 failed`)
- `cargo test -p genossi_service_impl pdf_generation`: 47 passed, 0 failed
- `cargo test -p genossi_service_impl repayment_letter`: 44 passed, 0 failed
- `cargo test -p genossi_rest_types member_slim_to_tests`: 7 passed, 0 failed
- `cd genossi-frontend && cargo check`: grün (32 pre-existing warnings, keine neuen Fehler)
- `diff -q` zwischen den 3 Template-Pfaden: silent (byte-identisch)
- Bind-Order-Symmetrie verifiziert: INSERT 25/25, UPDATE 24+2/26
- `audit_fields()` Vec.last() == `"account_holder"` (FROZEN-Order)
- Rustfmt --check auf allen patched Files (außer pre-existing-Diffs in membership_adjust.rs): clean

## Self-Check: PASSED

Alle 14 erwarteten Files vorhanden (Migration + DAO + Service + REST + 3 Templates + Frontend i18n×3 + Frontend page + SUMMARY).
Alle 3 Commits in git log gefunden (`8378de9`, `7dfad17`, `fde0518`).

