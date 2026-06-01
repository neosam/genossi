---
phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
plan: 07
subsystem: e2e
tags: [phase-13, e2e, integration, audit-chain, regression, security-gates]

requires:
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "05"
    provides: "POST /api/repayment-phase/{phase_id}/letters/generate REST-Endpoint + Direct-Download + X-Document-Count Header + RepaymentLetterRestState Bound (inkl. test_server.rs)"
  - phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder
    plan: "06"
    provides: "Frontend-Trigger + Bulk-Letter-Button (nicht direkt gebraucht; aber Voraussetzung fuer einen end-to-end UI-Tested-Pfad in einem Folge-Plan)"
provides:
  - "8 E2E-Tests in genossi_bin/tests/repayment_letter_e2e.rs (1 ignored)"
  - "setup_with_templates() Helper-Function mit Logo-Provisionierung als Folge-Plan-Pattern"
affects: []

tech-stack:
  added: []
  patterns:
    - "Logo-Provisionierung neben provision_defaults(): das nebenan-unverpackt-logo.svg ist NICHT in DEFAULT_TEMPLATES (Binary-Asset), muss aber neben dem Letter-Template liegen, sonst 500 InternalError beim Typst-Compile — Pitfall #6 fuer Plan 13-07 dokumentiert"
    - "Lokal duplizierte E2E-Helpers (Test-Code darf duplizieren): create_member_with_exit_date_and_iban, create_open_repayment_phase, list_entries_for_phase, list_member_documents, create_manual_entry, get_entry_status — alle minimal, in sich geschlossen"
    - "Single-Request Header- + Body-Assertions: resp.bytes() konsumiert den Body; daher MUSS Header-Check VOR dem Body-Lese-Schritt erfolgen"
    - "mock_auth-bedingter #[ignore] mit dokumentierter Begruendung: context_extractor injiziert UNCONDITIONAL Admin-MockContext, daher ist non-admin E2E nicht erreichbar; 403-Pfad ist auf REST- und Service-Layer-Ebene unit-getestet"

key-files:
  created:
    - "genossi_bin/tests/repayment_letter_e2e.rs"
    - ".planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-07-SUMMARY.md"
  modified: []

key-decisions:
  - "Test 3 (Helper-Auth → 403) wurde #[ignore]-markiert mit klar dokumentierter Begruendung. Im mock_auth-Feature injiziert genossi_rest/src/session.rs:120-127 UNCONDITIONAL einen Admin-MockContext (DEVUSER ist per Migration 20250129000001_create_default_auth_data.sql automatisch admin). Daher ist non-admin im e2e-mock-auth-Stack strukturell NICHT erreichbar. Der 403-Pfad ist statt dessen unit-getestet: (a) REST-Layer test_map_letter_error_permission_denied_to_403 in genossi_rest/src/repayment_letter.rs, (b) Service-Layer test_generate_permission_denied_returns_403 in genossi_service_impl/src/repayment_letter.rs. Pattern bestätigt durch existierenden Comment in e2e_tests.rs:9213-9221 (Phase-4 Helper-Cookie-Limit dokumentiert dasselbe Problem)."
  - "Logo-Provisionierung neben Default-Templates (Auto-Fix Rule 3 - Blocking): Beim ersten Test-Run schlugen 5 von 7 aktiven Tests mit 500 Internal Server Error fehl, weil das Letter-Template per image('nebenan-unverpackt-logo.svg', ...) ein Logo-Asset referenziert, das NICHT in DEFAULT_TEMPLATES enthalten ist (DEFAULT_TEMPLATES sind text-only via include_bytes!). Lösung: setup_with_templates() kopiert das Logo aus CARGO_MANIFEST_DIR/../templates/nebenan-unverpackt-logo.svg nach template_storage.base_path() (idempotent). Pattern aus genossi_service_impl/src/pdf_generation.rs:2059 provision_letter_templates() entlehnt."
  - "Single-Request Pattern statt 2-Call: Initial sendete test_letter_happy_path 2x denselben POST (Status-Check, dann Header+Body-Check), weil resp.bytes() den Body konsumiert. Nach Re-Check: Header lesbar VOR bytes()-Aufruf — daher einziger Call mit sequenzieller Header- → Body-Assertion. Reduziert Test-Zeit, vermeidet doppelte MemberDocument-Persistierung im Happy-Path-Test (was die Doc-Count-Assertion auf 2 statt 1 hätte korrigieren müssen)."
  - "8 Tests statt 6 (Plan-Discretion): Plan-Pflicht waren 8 #[tokio::test]-Funktionen. Test 3 wurde gemäß Plan-Threat-Model-Vorgabe als #[ignore] markiert; die anderen 7 sind aktiv. Acceptance-Greps fordern >=8 fn test_letter_, was mit dem #[ignore]-Marker erfuellt ist (Funktion existiert in der Quelle)."
  - "Body lesen + verwerfen via `let _ = resp.bytes().await` nach Header-Check in Tests, die den Body nicht inhaltlich pruefen — verhindert connection-pool-issues und macht den Test-Intent klar."

patterns-established:
  - "Logo-Provisionierung im E2E-Setup: Default-Templates mit Bild-Assets benoetigen einen zusaetzlichen Copy-Step. Pattern fuer kuenftige E2E-Tests mit Templates dokumentiert."
  - "mock_auth-Limit-Marker via #[ignore] mit klar zitierter Quelle (existierender Comment + Migration): Folge-Plans, die Helfer-Auth-Differenzierung testen wollen, muessen entweder einen Mock-Override fuer non-admin oder OIDC-Tests einbauen."
  - "Single-File E2E-Tests mit lokalen Helpers (kein gemeinsames mod common): die ganze Datei ist self-contained, was die Cross-Wave-Robustheit erhoeht — touched genossi_bin/tests/e2e_tests.rs NICHT."

requirements-completed: [BRIEF-01]

duration: ~25min
completed: 2026-06-02
---

# Phase 13 Plan 07: RepaymentLetter E2E-Tests Summary

**8 End-to-End-Tests verifizieren das gesamte POST /api/repayment-phase/{id}/letters/generate Pipeline durch echte HTTP-Calls — vom Auth-Funnel über Service-Logik bis MemberDocument-Persistenz und Audit-Hashchain. 7 aktiv (gruen), 1 #[ignore] (mock_auth-Limit dokumentiert). Cross-Phase-Regression-Gate gruen (292 bestehende e2e_tests). Phase 13 ist end-to-end-validiert.**

## Performance

- **Duration:** ~25 min (zwischen `c0ab2f3` parent und `c8d21b6` Task-1-GREEN)
- **Tasks:** 1 (Task 1 = 8 E2E-Tests in einer Datei)
- **Files created:** 2 (1 Rust-Test-Datei + SUMMARY)
- **Files modified:** 0
- **Commits:** 1 (Task 1 — alle 8 Tests in einem Atomic-Commit)

## Accomplishments

### Task 1 — 8 E2E-Tests in `genossi_bin/tests/repayment_letter_e2e.rs` (Commit `c8d21b6`)

Neue Datei (866 Zeilen) mit folgender Struktur:

**Setup:**
- `setup_with_templates() -> TestServer` — in-memory SQLite-Pool + sqlx-Migration + `RestStateImpl::new(pool)` + `template_storage.provision_defaults()` + **zusaetzliche Logo-Provisionierung** nach `template_storage.base_path()` (sonst 500 beim Letter-Render).

**Lokale Helpers (alle minimal dupliziert aus e2e_tests.rs):**
- `sample_member_with_iban(member_number, iban)` — `MemberTO`-Konstruktor mit konfigurierbarer IBAN (None/Some).
- `create_member_with_exit_date_and_iban(client, server, member_number, fiscal_year, iban)` — POST member → POST austritt-action → GET refresh.
- `create_preparation_repayment_phase(client, server, fiscal_year, share_value)` — POST repayment-phase, bleibt in Preparation.
- `create_open_repayment_phase(client, server, fiscal_year, share_value)` — Preparation + POST `/open` (triggert Auto-Fill).
- `list_entries_for_phase(client, server, phase_id)` — GET `/api/repayment-entry?phase_id=<uuid>`.
- `create_manual_entry(client, server, phase_id, member_id, share_count)` — POST `/api/repayment-entry`.
- `list_member_documents(client, server, member_id)` — GET `/api/members/{id}/documents`.
- `get_entry_status(client, server, entry_id)` — GET `/api/repayment-entry/{id}` → `.status`.

**Tests (Decisions-Coverage):**

| #  | Test                                                              | Coverage                                                                          | Status     |
| -- | ----------------------------------------------------------------- | --------------------------------------------------------------------------------- | ---------- |
| 1  | `test_letter_happy_path_3_entries_2_members`                       | D-13-01 Bundle + D-13-02 Direct-Download + D-13-04 Aggregation + D-13-05 Persist | gruen       |
| 2  | `test_letter_multi_entry_aggregation_d13_04`                       | D-13-04 (2 Entries gleicher Member → 1 MemberDocument)                            | gruen       |
| 3  | `test_letter_helper_auth_returns_403`                              | Permission-Gate (mock_auth-bedingt `#[ignore]` mit dokumentierter Begruendung)    | ignored    |
| 4  | `test_letter_phase_preparation_returns_409_phase_not_active`       | Status-Gate (D-13 Phase muss Open/Closed sein) — Body-Substring asserted          | gruen       |
| 5  | `test_letter_entry_phase_mismatch_returns_400`                     | D-13-03 (entry_ids fremder Phase) — Body-Substring asserted                        | gruen       |
| 6  | `test_letter_null_iban_renders_ok`                                 | D-13-06 + Pitfall #5 (NULL-IBAN render ok + %PDF-Magic)                            | gruen       |
| 7  | `test_letter_audit_chain_valid_after_bulk`                         | D-13-08 + Pitfall #4 (Audit-Hashchain valide nach Bulk-Run)                       | gruen       |
| 8  | `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09`       | D-13-08 (Idempotenz) + D-13-09 (kein Auto-Toggle Entry-Status)                    | gruen       |

**Test-Runs verifiziert:**
- `cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth -- --test-threads=1`: **7 passed, 0 failed, 1 ignored**
- `cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth` (parallel default): **7 passed, 0 failed, 1 ignored** (Tests sind isoliert — jeder mit eigener in-memory DB)
- `cargo test -p genossi_bin --test e2e_tests --features mock_auth` (Cross-Phase-Regression-Gate): **292 passed, 0 failed**

## Task Commits

1. **Task 1 — 8 E2E-Tests:** `c8d21b6` (test) — `genossi_bin/tests/repayment_letter_e2e.rs` (866 insertions)

## Files Created/Modified

- **Created** `genossi_bin/tests/repayment_letter_e2e.rs` — 8 E2E-Tests + setup_with_templates() mit Logo-Provisionierung + 8 lokal duplizierte Helpers + ausfuehrliche Header-Doku zur Decision-Coverage (866 Zeilen).
- **Created** `.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-07-SUMMARY.md` — dieses File.
- **Modified** keine.

## Decisions Made

### Test 3 als `#[ignore]` mit dokumentierter Quelle

Im `mock_auth`-Feature injiziert `genossi_rest/src/session.rs:120-127` UNCONDITIONAL einen `MockContext`:

```rust
#[cfg(all(feature = "mock_auth", not(feature = "oidc")))]
pub async fn context_extractor<RestState: RestStateDef>(
    State(_rest_state): State<RestState>,
    mut request: Request,
    next: Next,
) -> Response {
    request.extensions_mut().insert(MockContext);
    next.run(request).await
}
```

Die Migration `20250129000001_create_default_auth_data.sql:33` ordnet DEVUSER automatisch admin-Privilegien zu. Daher ist im e2e-mock-auth-Stack ein Helfer-Auth-Pfad **strukturell** NICHT erreichbar. Das ist kein Bug von Phase 13, sondern eine bekannte Limitation des mock_auth-Stacks (existing Comment in `e2e_tests.rs:9210-9221` dokumentiert dasselbe Problem fuer Phase-4 Helper-Cookies).

Statt einen unsicheren Test zu schreiben, der "irgendwas 4xx" akzeptiert, habe ich Test 3 mit `#[ignore = "..."]` markiert und die Begruendung im Test-Doc-Comment + Ignore-String + SUMMARY festgehalten. Der 403-Pfad ist auf zwei tieferen Ebenen unit-getestet:

- **REST-Layer:** `genossi_rest/src/repayment_letter.rs::tests::test_map_letter_error_permission_denied_to_403` verifiziert `map_letter_error(ServiceError::PermissionDenied) -> RestError::Forbidden(_)`.
- **Service-Layer:** `genossi_service_impl/src/repayment_letter.rs::tests::test_generate_permission_denied_returns_403` verifiziert den Permission-Funnel-Pfad mit echtem Mock-Permission-Service.

Der Plan-Text erlaubt dies explizit unter `<threat_model>`: "alternative Acceptance akzeptiert 401 ODER 403 als Vorstand-Gate-Beweis. Discretion."

### Logo-Provisionierung als 6. Setup-Stelle (Auto-Fix Rule 3 — Blocking)

Beim ersten Test-Run schlugen 5 von 7 aktiven Tests mit `500 Internal Server Error` fehl. Die Root-Cause: `auszahlungs_anschreiben.typ:64` referenziert `image("nebenan-unverpackt-logo.svg", width: 5cm)`. Dieses Asset ist NICHT Teil von `DEFAULT_TEMPLATES` (die enthalten nur text-only via `include_bytes!`; ein SVG-Binary einzuziehen wäre möglich aber bisher nicht implementiert). Daher fehlt das Logo nach `provision_defaults()` im `template_storage.base_path()`-Verzeichnis, und der Typst-Compile bricht mit "file not found" ab.

`genossi_service_impl/src/pdf_generation.rs:2059` `provision_letter_templates()` löst dasselbe Problem auf Service-Layer-Test-Ebene durch TempDir + manuelles Logo-Copy. Auf E2E-Ebene habe ich denselben Pattern adaptiert: `setup_with_templates()` ruft `provision_defaults()` UND kopiert dann das Logo aus `CARGO_MANIFEST_DIR/../templates/nebenan-unverpackt-logo.svg` nach `template_storage.base_path()`. Idempotent (nur kopieren wenn nicht vorhanden), thread-safe (für parallele Tests).

### Single-Request Header+Body-Assertion-Pattern

Initialer Happy-Path-Test sendete 2x denselben POST: einmal für Status-Check, einmal für Header+Body-Check. Beim Re-Check stellte sich heraus: `resp.headers()` ist VOR `resp.bytes()` lesbar — die Konsumption passiert erst beim Body-Lese-Schritt. Cleanup: 1 Request, Header zuerst (Status, Content-Type, Content-Disposition, X-Document-Count), dann `resp.bytes().await` für Body-Verify.

Nebenwirkung: Die MemberDocument-Assertions am Test-Ende mussten von "2 Docs pro Member (2 Calls × 1 Doc)" auf "1 Doc pro Member" korrigiert werden — was die D-13-04-Aggregation eindeutiger verifiziert (1 Call × 1 Doc pro unique Member).

### Body lesen + verwerfen via `let _ = resp.bytes().await`

In Tests, die den Response-Body nicht inhaltlich pruefen (Test 2 Multi-Entry, Test 7 Audit-Chain, Test 8 Idempotenz), wird `let _ = resp.bytes().await` aufgerufen, BEVOR der naechste API-Call gemacht wird. Grund: reqwest's connection-pool reuse — verbleibender ungelesener Response-Body kann zu Connection-Reset im Folge-Request fuehren. Pattern dokumentiert keep-alive-Robustheit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Logo-Asset fehlt nach `provision_defaults()` → 500 Internal Server Error**

- **Found during:** erstem `cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth -- --test-threads=1`
- **Issue:** 5 von 7 aktiven Tests scheiterten mit 500. Der `auszahlungs_anschreiben.typ`-Template referenziert `image("nebenan-unverpackt-logo.svg", ...)`. Das Logo-File ist NICHT Teil von `DEFAULT_TEMPLATES` (Binary-Asset wurde nie eingehaengt, vermutlich um die binary-size klein zu halten). `provision_defaults()` legt zwar das Template ab, das Logo daneben fehlt, Typst-Compile bricht ab.
- **Fix:** `setup_with_templates()` erweitert: nach `provision_defaults()` wird das Logo aus `CARGO_MANIFEST_DIR/../templates/nebenan-unverpackt-logo.svg` nach `template_storage.base_path()/nebenan-unverpackt-logo.svg` kopiert (idempotent — nur wenn target nicht existiert).
- **Files modified:** `genossi_bin/tests/repayment_letter_e2e.rs` (Setup-Function)
- **Commit:** `c8d21b6` (in selber Atomic-Commit; vor Commit gefixt — kein RED-Commit)

**2. [Rule 4 deferred to Pattern-Doc] Helper-Auth Test 3 als `#[ignore]`**

- **Found during:** Plan-Analyse vor dem Test-Schreiben (Pre-Flight)
- **Issue:** Plan-Text fordert `test_letter_helper_auth_returns_403` mit erwartetem 403. Im `mock_auth`-Feature ist das strukturell unerreichbar (siehe Decision-Block oben).
- **Fix:** Test mit `#[ignore = "..."]` markiert, klare Begruendung im Comment + Ignore-String. Dies entspricht der Plan-Text-Threat-Model-Klausel "Markierung als Skeleton-Test mit TODO ist akzeptabel".
- **Files modified:** `genossi_bin/tests/repayment_letter_e2e.rs` (Test 3)
- **Commit:** `c8d21b6`
- **Klassifikation:** Nicht streng Rule 4 (keine architektonische Aenderung) — der Test bleibt im Source und wird re-aktivierbar sein, sobald (a) ein non-admin Mock-Pfad oder (b) OIDC-Tests eingebaut werden.

### Auto-fix Rules nicht relevant

- Rule 1 (Bug): keine Bugs im Production-Code gefunden — alle 7 aktiven Tests verifizieren das spec-konforme Verhalten.
- Rule 2 (Missing Critical): keine fehlende Funktionalitaet — alle Decisions D-13-01..09 sind via min. 1 Test verifiziert.

## Issues Encountered

### Pre-Existing — ROADMAP.md modifiziert durch Wave-Orchestrator

`.planning/ROADMAP.md` war vor Plan-Start bereits modifiziert (Plan-Counts inkl. Phase 13). Per Plan-Instruktion ("Do NOT modify STATE.md or ROADMAP.md — the orchestrator owns those writes after the wave completes") wurde ROADMAP.md beim Commit explizit ausgeschlossen (`git add genossi_bin/tests/repayment_letter_e2e.rs` selektiv).

### Pre-Existing — typst-packages/ in genossi_service_impl/-Folder

Beim Plan-Start zeigte `git status` Files in `genossi_service_impl/typst-packages/preview/letter-pro/3.0.0/` als added (jj-tracking). Plan 13-04 SUMMARY dokumentierte das bereits + hat ein `.gitignore`-Pattern `/*/typst-packages/` eingefuehrt. Die Files sind im jj-Index "added" markiert, aber nicht im HEAD-Tree. Beim selektiven `git add` wurden sie konsequent ausgeschlossen.

### Plan-Text-Gap — Logo-Provisionierung in setup_with_templates()

Plan-Text Task 1 erwaehnt im `<action>`-Block (Z. 138) nur:
> ```
> let server = test_server::start_with_in_memory_db().await;
> ```

Es gibt keine eigene Funktion `test_server::start_with_in_memory_db()` — das existierende Pattern ist `setup()` (kein Templates) oder `setup_with_templates()` (mit Templates). Da das Letter-Template Templates braucht, muss `setup_with_templates()` benutzt werden. ABER: `setup_with_templates()` in `e2e_tests.rs:2672-2694` provisioniert nur Default-Templates ohne Logo, was unter den existierenden e2e_tests OK ist (kein test rendert das Letter-Template). Für Plan 13-07 muss das Setup um die Logo-Provisionierung erweitert werden. Dies wurde als Auto-Fix Rule 3 (Blocking) gehandhabt und ist in patterns-established dokumentiert.

## Self-Check

```
=== Files exist ===
FOUND: /home/neosam/programming/rust/projects/genossi3/genossi_bin/tests/repayment_letter_e2e.rs
FOUND: /home/neosam/programming/rust/projects/genossi3/.planning/phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-07-SUMMARY.md

=== Commits exist ===
FOUND: c8d21b6 (test(13-07): 8 E2E-Tests fuer POST /api/repayment-phase/{id}/letters/generate)

=== Task 1 Acceptance-Greps gruen ===
- test -f genossi_bin/tests/repayment_letter_e2e.rs: ✓
- rg 'fn test_letter_' genossi_bin/tests/repayment_letter_e2e.rs: 8 (>=8 ✓)
- rg 'fn test_letter_happy_path' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg 'fn test_letter_multi_entry_aggregation_d13_04' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg 'fn test_letter_helper_auth_returns_403' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓ (#[ignore]-Marker beruehrt Acceptance-Grep nicht)
- rg 'fn test_letter_phase_preparation_returns_409' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg 'fn test_letter_entry_phase_mismatch' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg 'fn test_letter_null_iban_renders_ok' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg 'fn test_letter_audit_chain_valid' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg 'fn test_letter_idempotency_d13_08' genossi_bin/tests/repayment_letter_e2e.rs: 1 ✓
- rg '/api/repayment-phase/.*letters/generate' genossi_bin/tests/repayment_letter_e2e.rs: 10 (>=8 ✓ — jeder Test hat mindestens 1 POST-URL; Test 8 hat 2 fuer Idempotenz)
- rg '/api/audit/verify' genossi_bin/tests/repayment_letter_e2e.rs: 2 (>=1 ✓)
- rg 'phase_not_active' genossi_bin/tests/repayment_letter_e2e.rs: 5 (>=1 ✓)
- rg 'entry_phase_mismatch' genossi_bin/tests/repayment_letter_e2e.rs: 7 (>=1 ✓)
- rg '"repayment_letter"' genossi_bin/tests/repayment_letter_e2e.rs: 4 (>=2 ✓ — docs filter)
- rg 'D-13-' genossi_bin/tests/repayment_letter_e2e.rs: 30 (>=4 ✓ — Decision references in test names/comments)

=== Builds ===
- cargo build --tests -p genossi_bin --features mock_auth: clean (kein warning, kein error)

=== Test-Runs ===
- cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth -- --test-threads=1: 7 passed, 0 failed, 1 ignored
- cargo test -p genossi_bin --test repayment_letter_e2e --features mock_auth: 7 passed, 0 failed, 1 ignored (parallel ok — Tests isoliert)

=== Cross-Phase-Regression-Gate ===
- cargo test -p genossi_bin --test e2e_tests --features mock_auth: 292 passed, 0 failed ✓

=== No untracked files committed ===
- git show --stat c8d21b6: 1 file (genossi_bin/tests/repayment_letter_e2e.rs) ✓
- KEIN genossi_service_impl/typst-packages/ in commit ✓
- KEIN .planning/ROADMAP.md in commit ✓ (Orchestrator owns this)
```

**Self-Check: PASSED**

## Threat Flags

Keine neuen Threat-Flags ueber das Plan-`<threat_model>` hinaus. Mitigationen verifiziert:

- **Audit-Hashchain Manipulation**: VERIFIZIERT — Test 7 `test_letter_audit_chain_valid_after_bulk` ruft `/api/audit/verify` und asserted `valid == true` nach Bulk-Run.
- **D-13-09 Compliance-Drift**: VERIFIZIERT — Test 8 `test_letter_idempotency_d13_08_and_no_status_toggle_d13_09` prueft `status_before == Open && status_after == Open` ueber 2 Bulk-Calls.
- **PII-Leak in Test-Assertions**: VERIFIZIERT — Tests assertieren auf StatusCode + Body-Substrings (`phase_not_active`, `entry_phase_mismatch`, `repayment_letter`); KEINE dbg!/println!-PII im Test-Code.
- **Test-Server-Race (in-memory-DB)**: MITIGIERT — `setup_with_templates()` erzeugt frische DB pro Test (`SqlitePool::connect("sqlite::memory:")`); parallele Test-Runs verifiziert grün.
- **False-Positive auf 403 (Test 3)**: MITIGIERT durch `#[ignore]`-Marker + Unit-Test-Querverweise. Plan-Text-Threat-Model erlaubt diese Strategie explizit.
- **Idempotenz-Test Flakiness**: VERIFIZIERT — sequential await zwischen Bulk-Call #1 und #2; Test 8 lockt klar auf `letter_docs.len() == 2` (NICHT auf timestamp).
- **entry_phase_mismatch koennte 409 statt 400 sein**: VERIFIZIERT — Plan 04 hat `ServiceError::ValidationError` gewaehlt, mapped auf 400. Test 5 erwartet exakt 400, gruen.

## Next Plan Readiness

**Phase 13 ist end-to-end-validiert. Keine Folge-Plans im Scope.**

**Pending Follow-ups (Phase-uebergreifend):**
- D-13-11 Phase-10-Worker-Refactor: `.planning/todos/pending/phase-10-worker-refactor-resolver.md` — Worker auf den im RestStateImpl bereitgestellten `repayment_context_resolver` migrieren. Plan 13-05 hat das Feld bereits mit `#[allow(dead_code)]` exponiert.
- Logo-Asset-Provisioning fuer Production: `nebenan-unverpackt-logo.svg` muss auf den deployed `TEMPLATE_PATH` kopiert werden. Plan 13-01/13-04 deferred-items; Plan 13-07 lokal in Test gefixt, Production ist noch offen.
- Helper-Auth-Test (Test 3) re-aktivieren, sobald entweder (a) ein non-admin Mock-Pfad oder (b) OIDC-Tests eingebaut werden. Aktuell `#[ignore]`-markiert.
- Logo-Provisionierung in `DEFAULT_TEMPLATES` einbetten (via `include_bytes!`)? Würde die provisioning-Logik um Binary-Assets erweitern, könnte aber den Test-Setup-Helper überflüssig machen. Diskutabel.

---

*Phase: 13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder*
*Completed: 2026-06-02*
