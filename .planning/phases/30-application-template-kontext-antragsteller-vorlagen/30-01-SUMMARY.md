---
phase: 30-application-template-kontext-antragsteller-vorlagen
plan: 01
subsystem: api
tags: [euro-formatting, i18n, mail, application, rust]

requires:
  - phase: 29-dao-schema-foundation
    provides: send_confirmation_mail (existierender Bestätigungs-Mail-Pfad, wird retrofittet)
provides:
  - "genossi_service::euro::format_eur_de(cents: i64) -> String — einziger kanonischer deutscher Euro-Formatter der Domäne"
  - "send_confirmation_mail verwendet format_eur_de statt naiver Inline-Formatierung"
affects: [30-03 application_to_template_context (open_amount), 31 service-rest-versand]

tech-stack:
  added: []
  patterns:
    - "Reines Display-Helfer-Modul im iban.rs-Stil (deutsche Doc-Comments, #[cfg(test)] mod tests, keine Validierung, kein externes Crate)"
    - "Magnitude-Normierung über i128/unsigned_abs, um i64::MIN-Overflow zu vermeiden"

key-files:
  created:
    - genossi_service/src/euro.rs
  modified:
    - genossi_service/src/lib.rs
    - genossi_service_impl/src/application.rs

key-decisions:
  - "D-11/D-13: Vorzeichen wird auf dem Betrag gebildet (Cents nie negativ); Tausenderpunkt manuell (3er-Gruppen von rechts), kein Locale-Crate"
  - "D-12: send_confirmation_mail routet den Betrag-String über format_eur_de — genau ein Euro-Formatter"
  - "ASCII-Leerzeichen (nicht NBSP) vor € — passend zur bestehenden Mail-Wortwahl '… von X €'"

patterns-established:
  - "Domänen-Euro-Formatierung zentral in genossi_service::euro; Konsumenten delegieren statt inline zu formatieren"

requirements-completed: [APTPL-02]

coverage:
  - id: D1
    description: "format_eur_de formatiert Cent-Beträge deutsch (Tausenderpunkt, Dezimalkomma, €), inkl. Null/Negativ/Mehrfachgruppen und i64::MIN ohne Panic"
    requirement: "APTPL-02"
    verification:
      - kind: unit
        ref: "genossi_service/src/euro.rs#format_eur_de_* (9 Tests: 0,5,1234,123456,100000000,123456789012,-123456,-5,i64::MIN)"
        status: pass
    human_judgment: false
  - id: D2
    description: "send_confirmation_mail bildet den Betrag-String über format_eur_de (kein Inline-Division-Format mehr)"
    requirement: "APTPL-02"
    verification:
      - kind: integration
        ref: "nix develop --command cargo test -p genossi_service_impl (437 passed); grep format_eur_de(total_cents) application.rs"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-08-20
status: complete
---

# Phase 30 Plan 01: format_eur_de Domänen-Euro-Formatter Summary

**Ein kanonischer deutscher Euro-Formatter (`genossi_service::euro::format_eur_de`) mit Tausenderpunkt/Dezimalkomma/€, korrekter Null-, Negativ- und i64::MIN-Behandlung; `send_confirmation_mail` auf ihn retrofittet.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-08-20
- **Completed:** 2026-08-20
- **Tasks:** 2
- **Files modified:** 3 (1 erstellt, 2 geändert)

## Accomplishments
- `format_eur_de(cents: i64) -> String` als einziger Euro-Formatter der Domäne: `123456 → "1.234,56 €"`, `0 → "0,00 €"`, `-123456 → "-1.234,56 €"`, `100000000 → "1.000.000,00 €"`.
- Magnitude über `i128`/`unsigned_abs()` normiert — `i64::MIN` kann nicht überlaufen (T-30-01-02 mitigiert).
- `send_confirmation_mail` bildet den offenen Betrag jetzt über `format_eur_de` (D-12) — die naive Inline-Formatierung ohne Tausendertrennzeichen ist entfernt.
- Kein neues Crate: Tausendergruppierung manuell (3er-Blöcke von rechts).

## Task Commits

1. **Task 1 (RED): failing tests for format_eur_de** - `d96efa1` (test)
2. **Task 1 (GREEN): implement format_eur_de** - `211f030` (feat)
3. **Task 2: retrofit send_confirmation_mail** - `9ac5c7b` (refactor)

**Plan metadata:** siehe letzter docs-Commit

_TDD-Gate: RED (`test`) → GREEN (`feat`) Sequenz eingehalten._

## Files Created/Modified
- `genossi_service/src/euro.rs` - Neues Display-Helfer-Modul mit `format_eur_de` + `group_thousands` + 9 Unit-Tests (iban.rs-Stil).
- `genossi_service/src/lib.rs` - `pub mod euro;` neben `pub mod iban;` verdrahtet.
- `genossi_service_impl/src/application.rs` - `send_confirmation_mail`: 3 Inline-Zeilen (euros/cents/format!) durch `let amount_str = genossi_service::euro::format_eur_de(total_cents);` ersetzt; `total_cents`-Zeile bleibt.

## Decisions Made
- Vorzeichen auf dem Betrag bilden, nicht auf Euros/Cents getrennt (D-13 / Pitfall 4) — vermeidet `-12,-34`-Bug.
- ASCII-Leerzeichen vor `€` (nicht NBSP), konsistent mit bestehender Mail-Wortwahl.
- Magnitude via `(cents as i128).unsigned_abs()` statt `cents.abs()` — deckt `i64::MIN` panic-frei ab.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Verify-Kommando um `--features utoipa` ergänzt**
- **Found during:** Task 1 (erster Testlauf)
- **Issue:** `nix develop --command cargo test -p genossi_service euro` (Plan-Wortlaut) schlägt beim Kompilieren fehl: `genossi_service/src/auth_types.rs` nutzt `utoipa::ToSchema` unbedingt, während `utoipa` ein optionales Feature ist. Der Fehler ist **pre-existing** (auth_types.rs unverändert) und tritt nur beim Standalone-`-p`-Test mit Default-Features auf.
- **Fix:** Verify-/Test-Kommandos mit `--features utoipa` ausgeführt (`nix develop --command cargo test -p genossi_service --features utoipa euro`). Keine Code-Änderung an auth_types.rs (out of scope).
- **Files modified:** keine (nur Kommando-Anpassung)
- **Verification:** 9/9 euro-Tests grün; `genossi_service_impl` (437) grün; `genossi_bin` baut.
- **Committed in:** n/a (keine Code-Änderung)

---

**Total deviations:** 1 auto-fixed (1 blocking, nur Kommando-Anpassung, kein Scope-Creep)
**Impact on plan:** Keiner — die Plan-Intention (grüne euro-Tests) ist erfüllt; die Feature-Flag-Ergänzung ist ein pre-existing Package-Test-Setup-Detail, nicht Teil dieser Änderung.

## Issues Encountered
- Standalone-Kompilierung von `genossi_service` benötigt das `utoipa`-Feature (siehe Deviation 1). Für nachfolgende Pläne/Verifier relevant: `genossi_service` immer mit `--features utoipa` testen, wenn per `-p` isoliert.

## User Setup Required
None - keine externe Service-Konfiguration erforderlich.

## Next Phase Readiness
- `genossi_service::euro::format_eur_de` steht bereit für Plan 30-03 (`application_to_template_context`, `open_amount`).
- Keine Blocker.

## Self-Check: PASSED

- `genossi_service/src/euro.rs` — FOUND
- `.planning/phases/30-.../30-01-SUMMARY.md` — FOUND
- Commits `d96efa1`, `211f030`, `9ac5c7b` — alle FOUND

---
*Phase: 30-application-template-kontext-antragsteller-vorlagen*
*Completed: 2026-08-20*
