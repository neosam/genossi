---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Anteile-Rückzahlungsphase
status: executing
stopped_at: Phase 8 context gathered
last_updated: "2026-05-30T21:17:38.972Z"
last_activity: 2026-05-30 -- Phase 08 planning complete
progress:
  total_phases: 6
  completed_phases: 1
  total_plans: 11
  completed_plans: 5
  percent: 45
---

# State: Genossi — v1.1 Anteile-Rückzahlungsphase

**Initialized:** 2026-05-02
**Last Updated:** 2026-05-29 (v1.1 milestone planning)

## Project Reference

**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit.

**Current Focus:** Phase 07 — repaymentphase-backend-foundation

**Granularity:** coarse (6 Phasen, Backend-First → Service-Logik → Integrationen → Frontend)

## Current Position

Phase: 8
Plan: Not started
Status: Ready to execute
Last activity: 2026-05-30 -- Phase 08 planning complete

## Closure Snapshot (v1.0, 2026-05-29)

- Audit: `tech_debt` (kein Blocker; 4 deferred items in MILESTONES.md vermerkt)
- Tests: ~927 Workspace + 239 E2E, alle grün
- Produktiver E2E-Beweis: echte GV im Mai 2026 mit Genossi durchgeführt
- Hotfixes aus Live-Betrieb: `8e92cfd`, `e245013`, `ed754fc`, `3cdfbb6`, `c6f41fd`, `bb1be0b`
- Archive: `.planning/milestones/v1.0-{ROADMAP,REQUIREMENTS,MILESTONE-AUDIT}.md` + `v1.0-phases/`

## Phase 04 Closure Notes (2026-05-06)

- **Verification (04-VERIFICATION.md):** 14 PASS / 0 FAIL / 1 PENDING after fix `bfffbe2`. Only PENDING is `dx build --release` (blocked by missing `wasm-bindgen-cli@0.2.104` in local Nix profile — pure tooling, must be resolved at the start of Phase 5).
- **UAT (04-UAT-CHECKLIST.md):** 173 manual checkboxes spanning Vorstand flows, HLPR-03 manual code login, SYNC-01 polling/race, ATTN-06 visual diff, Datenschutz, GV lifecycle. To be exercised on real hardware during Phase 5 generalprobe.
- **Tests:** Backend 819 / Frontend 108 / 927 total pass / 0 fail.
- **ATTN-06 Component-Reuse-Anchor:** verified via literal grep diff between `assembly_details.rs::AttendanceTab` and `helper_attendance.rs` — same component invocations, same on_toggle smart-parent wiring.
- **Tooling debt for Phase 5:** install `wasm-bindgen-cli@0.2.104`, then re-run `dx build --release` and verify Tailwind purge result on the actual release artifact.

```
[ ] Phase 1: Assembly-Aggregat + Audit-Hardening                     0/0 plans
[ ] Phase 2: Helfer-Token + Session + AuthContext::Helper            0/0 plans
[ ] Phase 3: Attendance-Aggregat + Cascade-Invalidation              0/0 plans
[ ] Phase 4: Frontend (Component-First)                              0/0 plans
[ ] Phase 5: Pre-GV-Generalprobe und Operations-Plan                 0/0 plans

Overall: 0% complete
```

## Performance Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| v1 Requirements | 22 | Aus 4 Kategorien (ASSY/HLPR/ATTN/SYNC) |
| Coverage | 22/22 (100%) | Keine Orphans |
| Phases | 5 | coarse Granularity |
| Build Order | Backend-First → Frontend → Operations | Genossi-Konvention |
| Audit Scope | ASSY-* + HLPR-* lifecycle | ATTN-* explizit OFF (User-Decision) |
| Phase 03 P01 | 12min | 2 tasks | 5 files |
| Phase 03 P02 | ~10 min | 1 tasks | 3 files |
| Phase 03 P03 | ~7 min | 1 tasks | 1 files |
| Phase 03 P04 | 8min | 2 tasks | 3 files |
| Phase 03 P05 | ~13 min | 2 tasks | 4 files |
| Phase 03 P06 | ~15 min | 3 tasks | 5 files |
| Phase 07 P01 | 5min | 2 tasks | 3 files |
| Phase 07 P02 | 3min | 1 tasks | 2 files |
| Phase 07 P03 | 9min | 2 tasks | 4 files |
| Phase 07 P04 | 8min | 3 tasks | 5 files |
| Phase 07 P05 | 5min | 1 task | 1 file |

## Accumulated Context

### Roadmap Evolution

- Phase 6 added: Teilnehmerlisten-Export für Generalversammlungen (CSV/PDF, an Protokoll anhängbar)
- Phase 5 SKIPPED 2026-05-17: Echte GV bereits erfolgreich durchgeführt — Pre-Generalprobe damit obsolet. Erkenntnisse aus dem realen Einsatz flossen als Hotfixes ins Frontend zurück (live-counter `8e92cfd`, gv-pages button type `e245013`, sort by member_number `ed754fc`, token-codes magic-link `3cdfbb6`). Tooling-Debt (`wasm-bindgen-cli@0.2.104`, Tailwind-Purge am Release-Artefakt) wird bei Bedarf separat ausserhalb GSD abgearbeitet.

### Key Decisions (carry-over aus PROJECT.md)

| Decision | Rationale | Phase |
|----------|-----------|-------|
| GV als eigene Entität (`Assembly`) statt globalem Zustand | Mehrere GVs pro Jahr, Historie für Protokoll, klare Lifecycle-Grenzen | Phase 1 |
| Anwesenheit als Join-Tabelle, nicht Member-Flag | Saubere Mehrfach-GV-Historie, kein Datenverlust | Phase 3 |
| Plan 03-01: `search: Option<String>` statt `Option<&str>` in DAO-Trait | `async_trait` + `automock` verlangen named lifetime auf borrowed Option-Parametern; owned String ist projekt-konsistent (DAO allokiert intern eh `format!("%{}%", ...)` neu) | Phase 3, Plan 01 |
| Plan 03-02: hand-rolled `TestHelperTokenDao`-Mock liegt in `genossi_service_impl/src/helper_token.rs::tests` (NICHT in `assembly.rs::tests` wie das Plan-Dokument behauptete) | DAO-Trait-Erweiterungen müssen den existierenden hand-rolled Mock synchron pflegen, sonst E0046 in `cargo test -p genossi_service_impl`. Plan 05 wird einen **neuen** Mock in `assembly.rs::tests` anlegen, der ebenfalls die neue Method listen muss. | Phase 3, Plan 02 |
| Plan 03-03: `ClaimContext::as_helper`-Signatur ist `Option<Uuid>`, NICHT `Option<(Arc<str>, Uuid)>` — `session_id` ist NICHT Teil des Phase-2-Helper-Claims-JSON (verifiziert in `genossi_service_impl/src/session.rs:17-30`); sie liegt in `AuthenticatedContext.user_id` (Format `helper:<token_id>`) bzw. wird via `HelperTokenDao::list_session_ids_for_assembly` im Cascade-Pfad gelesen. Plan 05 destrukturiert `Some(helper_aid)` als reine `Uuid`. | Phase 3, Plan 03 |
| Plan 03-03: `permission::MockContext` (`genossi_service/src/permission.rs:150`) ist Unit-Struct ohne `Default`-Impl — distinkt von `auth_types::MockContext`. Plan 05+ Test-Fixtures müssen Unit-Construct-Syntax `MockContext` verwenden, kein `::default()`. | Phase 3, Plan 03 |
| Plan 03-04: AttendanceService-Trait-Test 3 nutzt `#[test]` (sync) statt `#[tokio::test]` (async) — `genossi_service` hat keine tokio dev-dependency. Test verifiziert nur die `#[automock]`-Builder-API (`expect_*` für alle 4 Methods); der reale `await`-Pfad wird in Plan 06's REST-Tests gegen `AttendanceServiceImpl` exerciert. Symmetrisches Pattern für zukünftige genossi_service-Trait-Tests. | Phase 3, Plan 04 |
| Plan 03-04: AttendanceMemberTO-PII-Guard hat 3 Verteidigungslinien: (1) strikte 7-Feld-Whitelist auf Struct-Ebene, (2) Doc-Comment-Verbot für `From<&MemberTO>`, (3) Konversion exklusiv aus `AttendanceMemberRow` (DAO-7-Spalten-SELECT-Whitelist). Plan 06's E2E-Test verifiziert das gleiche Pattern auf HTTP-Response-JSON. | Phase 3, Plan 04 |
| Plan 03-05: `check_assembly_access` Permission-Funnel ist EXKLUSIV im AttendanceServiceImpl (NICHT im PermissionService) und wird von ALLEN 4 Endpoint-Methods als erster DAO-touchender Schritt aufgerufen. Helper-Branch terminiert mit `return Ok(assembly)` nach Status-Check — fällt NIEMALS in den admin-Branch durch (failure-closed). Admin-Branch hat KEINEN Status-Check (D-20, ASSY-06). | Phase 3, Plan 05 |
| Plan 03-05: `close_assembly` Cascade-Reihenfolge: `audited_update!` → `list_session_ids_for_assembly` (in tx) → `tx.commit()` → pool-loop `delete_session` mit `tracing::warn!` bei Fehlern. Conflict-2-Resolution: pool-basierte `delete_session` deadlockt gegen offenen BEGIN, daher MUSS commit VOR der Loop. Continue-on-Error mit Defense-in-Depth via Phase-2 D-18 verify_user_session-Status-Check. | Phase 3, Plan 05 |
| Plan 03-05: `genossi_bin` `helper_token_dao` wird BEVOR `AssemblyServiceImpl`-Construction angelegt und dann mit `HelperTokenServiceImpl` per `Arc::clone` geteilt — exakt EIN HelperTokenDaoImpl pro Prozess. Pattern-Anker für künftige Service-DAO-Sharing-Setups. | Phase 3, Plan 05 |
| Plan 03-05: Hand-rolled `TestHelperTokenDao` + `TestPermissionDao` Mocks duplizieren bewusst den existierenden Mock in `helper_token.rs::tests` (Pitfall 4). Beide Test-Module müssen synchron mitgepflegt werden, weil mockall-`automock` den `Transaction`-Type hartkodiert; cross-Module-Sharing erfordert `pub(crate)`-Refactor (out-of-scope). | Phase 3, Plan 05 |
| Plan 03-06: Differential `map_attendance_error` (PermissionDenied → 403 Forbidden) lebt LOKAL in `genossi_rest/src/attendance.rs`, NICHT als globaler `From<ServiceError>`-Override. Begründung: globale Änderung würde Phase-1+2-Endpoints brechen. Pattern reusable für künftige Endpoint-Familien mit eigener Status-Code-Policy. | Phase 3, Plan 06 |
| Plan 03-06: Stats-Endpoint registriert als separater `Router::nest("/api/assembly/{assembly_id}", attendance::generate_stats_route())` neben `assembly::generate_route()` unter `/api/assembly`. Axum erlaubt mehrere `.nest`-Aufrufe mit unterschiedlich-spezifischen Pfad-Prefixes. Pattern für cross-namespace-Endpoints, deren Implementation in einem anderen Service als der Pfad-Namespace lebt. | Phase 3, Plan 06 |
| Plan 03-06: `assembly_member_snapshot_dao` jetzt Arc-shared via `.clone()` zwischen AssemblyServiceImpl und AttendanceServiceImpl — exakt EIN DAO pro Prozess (Mirror des helper_token_dao-Sharing-Patterns von Plan 05). | Phase 3, Plan 06 |
| Plan 03-06: Hash-chain-Burst-Test reduziert von 100 auf 40 Toggles — global `api_rate_layer` cap (60 burst, 1/sec refill) hätte 100 Toggles + 4 surrounding REST calls als 429 Too Many Requests gedrosselt. ATTN-05-Invariante (count_before == count_after) ist unabhängig von der Burst-Größe; 40 reicht für volle Verifikation. | Phase 3, Plan 06 |
| Plan 07-01: i64-Cent-Konvention für `share_value` etabliert — neue Pattern-Vorlage für Phase 8 `RepaymentEntry.amount`, Phase 9 MemberAction-Cascade und Phase 11 Export-Multiplikation; SQLite INTEGER ist 8-Byte → Rust i64; Validierung `> 0` ohne Obergrenze (D-12) | Phase 7, Plan 01 |
| Plan 07-01: KEIN UNIQUE-Constraint auf `fiscal_year` (D-08) — mehrere RepaymentPhases pro Geschäftsjahr in beliebigen Statuskombinationen explizit erlaubt (Q1+Q4-Phasen, Korrektur-Phasen, parallele Vorbereitung+Closed). Frontend (Phase 12) sortiert per `fiscal_year DESC, created DESC` zur Auffindbarkeit. | Phase 7, Plan 01 |
| Plan 07-01: Audit-fields-Reihenfolge `fiscal_year/share_value/status/opened_at/closed_at` ist frozen (T-07-01-01 Hash-Chain-Konsistenz) — Plan 03 Service-Tests müssen diese Reihenfolge per Unit-Test einfrieren, weil spätere Reihenfolge-Änderung historische Audit-Einträge brechen würde. | Phase 7, Plan 01 |
| Plan 07-01: `opened_at`/`closed_at` als optionale Spalten persistiert (D-13) — NICHT nur im Audit-Log, analog `assembly.opened_at`/`closed_at`. Begründung: Phase 11 Filename-Schema und Audit-Lesbarkeit. Audit-Log erfasst diese Felder zusätzlich automatisch via Diff. | Phase 7, Plan 01 |
| Plan 07-02: `parse_datetime` via `use crate::assembly::parse_datetime` statt Duplikation — `pub(crate)`-Helper aus Plan 1 ist bereits Cross-Modul-shared (Pattern aus `assembly_member_snapshot`). Spart 17 LOC bei reiner String-Parser-Coupling ohne Domain-Coupling. | Phase 7, Plan 02 |
| Plan 07-02: `RepaymentPhaseDaoImpl::new` akzeptiert `Arc<SqlitePool>` (nicht `SqlitePool` wie im Plan-Text) — folgt etabliertem `AssemblyDaoImpl`-Pattern; Plan 03 (Service-Wiring in `genossi_bin`) erwartet diese Signatur. | Phase 7, Plan 02 |
| Plan 07-02: Pre-Exists-Check (`SELECT COUNT(*) ... WHERE id = ? AND deleted IS NULL`) **vor** UPDATE trennt `NotFound` von `ConflictError` sauber — ohne Pre-Check würde `rows_affected == 0` in beiden Fällen (missing id + version-mismatch) `ConflictError` feuern, was zwei distinkte Error-Semantiken vermischt; T-07-02-03 Mitigation. | Phase 7, Plan 02 |
| Plan 07-03: Inline-Field-Validator (`validate_phase_fields`) statt Erweiterung von `validation.rs` — `validation.rs` ist anderer Concern (Mitglieder-Konsistenzberichte); für 2-Feld-Entities (`fiscal_year`/`share_value`) wäre eigener Service-Stub Overkill. Pattern-Anker aus PATTERNS.md §5. | Phase 7, Plan 03 |
| Plan 07-03: Edit-Matrix-Check (D-04) wird BEVOR Version-Check ausgeführt — atomare D-07-Ablehnung liefert semantisch klarere Fehlermeldung (`"Cannot change fiscal_year: phase is Open (D-04/D-07)"`) als generisches `"Version mismatch"`. Test 6 verifiziert mit Mock-update().times(0): no-write on D-07 violation. | Phase 7, Plan 03 |
| Plan 07-03: Audit-Disziplin-Grep-Gate als Pre-Merge-Check für T-07-03-01 Repudiation-Defense — Filter `grep -v '^//' | grep -v '^*'` (Doc-Comments + Doc-Comment-Continuations) verhindert Self-Invalidation des Gates durch Anti-Pattern-Erwähnung in Kommentaren. 0 direkte DAO-create/update außerhalb `audited_*!`-Macros verifiziert; jede Schreibroute durch Audit-Hashchain garantiert. | Phase 7, Plan 03 |
| Plan 07-03: Phase-8-TODOs als Code-Kommentare in `open_`/`close_repayment_phase` — PHAS-02 Auto-Befüllung der RepaymentEntries in `open_repayment_phase`; PHAS-03 Pending-Entry-Validation in `close_repayment_phase`. Anker für Phase 8, damit Erweiterungspunkte sofort gefunden werden — verhindert Wiederholung der Plan-7-Codebase-Suche bei Phase-8-Start. | Phase 7, Plan 03 |
| Plan 07-04: REST-Pfad Singular `/api/repayment-phase` (D-14) — konsistent mit `/api/member`, `/api/assembly`, `/api/application`; OpenAPI-Tag bleibt Plural `RepaymentPhases` (Convention für Tag-Names). Frontend (Phase 12) muss diesen Pfad nutzen. | Phase 7, Plan 04 |
| Plan 07-04: Keine lokalen `map_*_error`-Override im REST-Layer — globales `From<ServiceError> for RestError` in `genossi_rest/src/lib.rs:97-113` reicht für Phase 7 (ValidationError → 400, EntityNotFound → 404, Conflict → 409, PermissionDenied → 401). Phase 3 Plan 06 hatte einen lokalen Override für 403-Mapping in Attendance; Phase 7 ist Vorstand-only ohne Helper-Differenzierung — kein 403-Bedarf. | Phase 7, Plan 04 |
| Plan 07-04: Minimale REST-Validatoren (`validate_create_*`/`validate_update_*` als `Ok(())`-Stubs) — strukturelle Pflicht durch serde-Deserialisierung, Range-Checks (D-11/D-12) im Service-Layer; Pattern-Anker für strukturelle Pflichtfeld-Erweiterungen ohne Code-Refactor. Distinkt von `genossi_rest/src/assembly.rs:28-104`, die aufwändigere Validation hat (256-Zeichen-Limits auf String-Feldern, `name`-empty-Check). Phase 7 hat nur zwei numerische Felder; serde reicht. | Phase 7, Plan 04 |
| Plan 07-04: 5-Deps-DI für RepaymentPhaseServiceImpl (statt 10+ Deps wie Assembly): RepaymentPhaseDao + AuditLogDao + PermissionService + UuidService + TransactionDao. Phase 7 ist signifikant simpler als Assembly (kein Snapshot, kein Helper-Token-Cascade, kein Member-Permission-Dao). Etabliert das "simpler-than-Assembly"-Pattern für Phase 8 RepaymentEntry. | Phase 7, Plan 04 |
| Plan 07-04: `audit_log_dao.clone()` geteilt mit allen audited Services (Member, Assembly, Application, RepaymentPhase) — gleiche Hash-Chain pro Prozess (T-07-04-05 Repudiation-Mitigation). Plan 05 E2E `/api/audit/verify` wird die Chain-Konsistenz über alle 5 RepaymentPhase-Lifecycle-Events (create/update/open/close/delete) und die existierende Member/Assembly/Application-Spur verifizieren. | Phase 7, Plan 04 |
| Plan 07-05: Optimistic-Locking via Stale-Retry-Pattern statt direkter Version-Bump-Assertion — codebase-weite Service-Konvention (Assembly, RepaymentPhase, Member) gibt nach `audited_update!` die LOKALE entity mit alter Version zurück; DAO bumpt die DB-Version atomar (siehe `genossi_dao_impl_sqlite/src/repayment_phase.rs:150`), aber Bump wird NICHT propagiert. Stale-retry-PUT mit alter Version → 409 `"Version mismatch"` verifiziert die DB-Konsistenz end-to-end. Architekt-Korrektur (DAO-Update als `Result<Uuid, _>`) wäre Rule-4-Change; als tech-debt-Item für eine spätere Phase markiert. | Phase 7, Plan 05 |
| Plan 07-05: Audit-Endpoint-Pfad `/api/audit/repayment_phase/{id}` mit Underscore (nicht Bindestrich) — `entity_type` ist die `Auditable`-Trait-Constant `"repayment_phase"` aus Plan 01, nicht der REST-Pfad-Namespace. Pattern-konsistent mit `/api/audit/assembly/{id}` aus Phase 1. | Phase 7, Plan 05 |
| Plan 07-05: 7 E2E-Tests = 1 Lifecycle-Happy-Path + 6 Negative-Paths verifizieren alle 5 ROADMAP-SC und Decisions D-04..D-12 end-to-end. Substring-Body-Assertions (`body.contains("fiscal_year")` / `"share_value"` / `"Version mismatch"`) als minimal-invasive Diagnostik statt brittle exakter Fehlertexte. Pattern-Vorlage für Phase-8-RepaymentEntry- und Phase-9-PayoutCascade-E2E-Tests. | Phase 7, Plan 05 |
| One-Time-Use-QR pro Helfer | Verhindert Token-Weitergabe an Unbefugte | Phase 2 |
| Helfer-Memo-Name = Freitext, kein Identitäts-Anker | Reine UX-Hilfe für Vorstand beim Drucken | Phase 2 |
| GV-Status final nach Schluss; Vorstand-Korrekturen ohne Re-Open | Vermeidet Status-Pingpong, hält Audit-Story einfach | Phase 1 |
| Manual-Code-Fallback (8–12 alphanumerisch) zusätzlich zu QR | iOS-Safari-Kamera-Quirks bekannt, Worst-Case auf echter GV vermeiden | Phase 4 |
| Atomarer Redeem via `UPDATE ... WHERE used_at IS NULL RETURNING ...` | Verhindert Race-Condition zwischen parallelen Scans | Phase 2 |
| Member-Universe-Snapshot beim GV-Öffnen | Stabiles Y im Counter, Late-Joins/Austritte verfälschen Protokoll nicht | Phase 1 |
| Sync nur bei Refresh, kein Live-Push | Doppel-Abhaken durch Idempotenz abgefangen, kein SSE/WebSocket nötig | Phase 3+4 |
| Anwesenheit ohne Audit-Hashchain | Verband fordert nur Anzahl, nicht den Anhakel-Vorgang | Phase 3 |
| Helfer-View auch für Vorstand zugänglich (ohne QR) | Vorstand will im Notfall mithelfen, kein UI-Duplikat | Phase 3+4 |

### Open TODOs

- (obsolet) ~~Vor Phase 2 entscheiden: `tower-sessions` 0.14 → 0.15 Upgrade jetzt oder später?~~ — Phase 2 abgeschlossen, hat sich erledigt
- (obsolet) ~~In Phase 5 dokumentieren: Server-Hosting-Entscheidung für GV-Tag~~ — Phase 5 skipped; Production läuft via `deploy-binaries.sh` auf `shifty.nebenan-unverpackt.de`, hat sich in der realen GV bewährt

### Blockers

Keine.

## Deferred Items

Items acknowledged and deferred at milestone close on 2026-05-29:

| Category | Item | Status | Note |
|----------|------|--------|------|
| uat_gap | 02-HUMAN-UAT.md | partial (2 pending scenarios) | Phase 02 — HLPR-05 + AuthContext::Helper-Pipeline-Wiring; in Phase 3 (Cascade-Invalidation SC#8) end-to-end implementiert; durch echte GV produktiv verifiziert |
| uat_gap | 04-UAT-CHECKLIST.md | unknown (0 pending) | Phase 04 — Status nicht final gesetzt; UAT wurde operativ durch echte GV abgeschlossen (Hotfixes 8e92cfd, e245013, ed754fc, 3cdfbb6) |
| verification_gap | 02-VERIFICATION.md | human_needed | Phase 02 BLOCKER-04 — validate_code_format Unicode-Lookalike-Bug; im Milestone-Audit als bekannte Spec-Divergenz dokumentiert (kein Security-Bug); Decision pending |

Details siehe `.planning/v1.0-MILESTONE-AUDIT.md` und `.planning/MILESTONES.md`.

### Skills / Conventions to Apply

- **Audit-Macros**: `audited_create!`, `audited_update!`, `audited_delete!` für Member/MemberAction/MemberDocument/Application-Operationen, die in den GV-Phasen mitlaufen, sowie für Assembly-Lifecycle und HelperPreToken-Erzeugung. NICHT für Anwesenheits-Markierungen.
- **Component-First**: `genossi-frontend/src/component/` (siehe `genossi-frontend/CLAUDE.md`); identische UI auf zwei Pages → eigene Component-Datei
- **Layered Architecture**: DAO trait → SQLite impl → Service trait → Service impl → REST handler — Reihenfolge je Phase
- **Optimistic Locking**: `version: Uuid` für Member/Application — bei Assembly identisch, beim Redeem-Token EXPLIZIT NICHT (dort `used_at IS NULL` als Constraint)
- **Soft-Delete**: `deleted: Option<PrimitiveDateTime>` — Standard für alle neuen Aggregate
- **ISO8601-Datetime**: `genossi_rest_types::iso8601_datetime` serde-Modul für alle TO-Datetime-Felder

## Session Continuity

**Last action (2026-05-29, Phase 07 Plan 05):** Plan `07-05-PLAN.md` ausgeführt — 7 E2E-Tests + Helper-Function `create_preparation_repayment_phase` in `genossi_bin/tests/e2e_tests.rs` (+458 LOC). Lifecycle-Test (`test_repayment_phase_lifecycle_audit_chain_intact`) verifiziert ROADMAP SC#4 (Audit-Hashchain `valid=true` mit `broken_links=[]` und `total_entries >= 4` nach create→open→update→close) und SC#5 (expliziter Feld-Diff-Check: `field_name="share_value"`, `old_value=Some("12000")`, `new_value=Some("13000")` unter Process `"repayment-phase.update"`); plus alle 4 distinkten Audit-Prozesse verifiziert. 6 Negative-Path-Tests prüfen D-04/D-07 (fiscal_year-Change in Open → 409), D-05/D-06 (close from Preparation → 409), D-06 (reopen from Closed → 409), D-09 (DELETE in Open → 409), D-11 (fiscal_year=1999 → 400 mit "fiscal_year"-Substring), D-12 (share_value=0 → 400 mit "share_value"-Substring). 255/255 E2E-Tests grün (Baseline 248 + 7 = 255). Eine Deviation: Direkte Version-Bump-Assertion `version != version_v1` musste durch Stale-Retry-Pattern ersetzt werden — die codebase-weite Service-Konvention (Assembly, RepaymentPhase, Member) gibt nach `audited_update!` die LOKALE entity mit alter Version zurück; DAO bumpt die DB-Version atomar, propagiert sie aber nicht. Test verifiziert die DB-Konsistenz end-to-end via 2. PUT mit alter Version → 409 "Version mismatch". Architekt-Korrektur wäre Rule-4-Change; tech-debt für Folge-Phase markiert. Phase 7 ist abgeschlossen und verifikations-vollständig — alle 5 Plans done, alle 5 ROADMAP-SC verifiziert. Commit `6aa4ff2` (test, +458 LOC). Nächster Schritt: Phase-7-Verification (`/gsd-verify-phase 07`) und/oder Start Phase 8 (RepaymentEntries + Auto-Befüllung).

**Last action (2026-05-29, Phase 07 Plan 04):** Plan `07-04-PLAN.md` ausgeführt — REST-Layer + DI-Wiring für RepaymentPhase. 4 neue TOs in `genossi_rest_types/src/lib.rs` (RepaymentPhaseTO/StatusTO/CreateRequest/UpdateRequest mit ISO8601-Datetime-Serde + utoipa-ToSchema, +210 LOC), neue Datei `genossi_rest/src/repayment_phase.rs` (414 LOC) mit RepaymentPhaseRestState-Trait + 7 Handlern (list/create/get/update/delete/open/close) + generate_route + ApiDoc + 3 Smoke-Tests, Modul-Decl + OpenAPI-Nest + Router-Mount + Trait-Bounds in `genossi_rest/src/lib.rs`, Trait-Bound-Erweiterung in `test_server.rs`, vollständige DI-Wiring in `genossi_bin/src/lib.rs` (type-Alias + 5-Deps-Struct + Service-Konstruktion + Self-Field + RestState-Impl, +53 LOC). 7 OpenAPI-Pfade unter Tag `RepaymentPhases` registriert. `cargo build` + `cargo build --tests -p genossi_bin` grün. Tests: 28/28 in genossi_rest_types (4 davon neu), 59/59 in genossi_rest (3 davon neu). 3 Task-Commits (`00a1134` feat TOs, `a0c212f` feat REST-Layer, `5d05c44` feat DI-Wiring). PHAS-01 + PHAS-04 sind REST-vollständig; PHAS-05 (Audit-Macros greifen) ist im REST-Layer-Pass-Through erfüllt. Nächster Schritt: Plan 07-05 (E2E-Tests).

**Last action (2026-05-29, Phase 07 Plan 03):** Plan `07-03-PLAN.md` ausgeführt — `RepaymentPhaseService`-Trait in `genossi_service/src/repayment_phase.rs` (7 Methoden create/update/open/close/delete/get/get_all + DTOs + #[automock]) und `RepaymentPhaseServiceImpl` in `genossi_service_impl/src/repayment_phase.rs` (5 Prozesskonstanten, `validate_phase_fields`-Helper, Edit-Matrix D-04 mit atomarer fiscal_year-Lock in Open D-07, Lifecycle-Guards D-05/D-06, Soft-Delete-Guard D-09, Field-Validation D-11/D-12, Optimistic-Locking, alle Schreiboperationen via `audited_*!`-Macros) angelegt. Audit-Disziplin-Grep-Gate (T-07-03-01 Mitigation) verifiziert: 0 direkte DAO-create/update außerhalb Macros. 4 Service-Trait-Tests + 13 Service-Impl-Tests grün (17 neue Tests gesamt). Workspace-Build clean (nur pre-existing Warnings). Commits `771c8d5` (feat 266 LOC) + `963f17b` (feat 1108 LOC). Nächster Schritt: Plan 07-04 (REST-Handler).

**Last action (2026-05-29, Phase 07 Plan 02):** Plan `07-02-PLAN.md` ausgeführt — `RepaymentPhaseDaoImpl` (SQLite-Impl des Plan-01-Traits) angelegt mit `RepaymentPhaseDb`-Row, `TryFrom` mit guarded i32-Cast (T-07-02-05), `dump_all`/`create`/`update` inkl. Pre-Exists-Check + Optimistic-Locking via `rows_affected == 0 → ConflictError("Version mismatch")`. ORDER BY ist `fiscal_year DESC, created DESC` (Phase-7-spezifisch). `parse_datetime` via `use crate::assembly::parse_datetime` reused (kein Duplikat). 4 grüne Tokio-Integrationstests gegen in-memory SQLite. Modul-Decl in `genossi_dao_impl_sqlite/src/lib.rs` alphabetisch eingefügt. Commit `6f6bf0f` (feat: 367 LOC added).

**Stopped At:** Phase 8 context gathered
**Resume File:** .planning/phases/08-repaymententry-auto-bef-llung/08-CONTEXT.md

**Last action (2026-05-17, Phase 06 Discuss):** `/gsd-discuss-phase 6` durchgeführt — `06-CONTEXT.md` + `06-DISCUSSION-LOG.md` erstellt und committed (`30a3c2b`). 20 Implementierungsentscheidungen (D-01..D-20) erfasst: drei Export-Formate parallel (PDF via Typst, CSV semikolon/UTF-8-BOM, XLSX via rust_xlsxwriter), `?include=all|present`-Query, Status-Closed-only, Vorstand-only via OIDC, Snapshot-Daten aus `assembly_member_snapshot`, Sortierung `member_number ASC`, Endpoint `GET /api/assembly/{aid}/attendance-export/{format}`, Filename `gv-{YYYY-MM-DD}-teilnehmer.{ext}`, kein Audit-Hashchain-Eintrag. PDF-Layout minimal (Kopf mit GV-Titel + Datum + „X von Y anwesend", dann Tabelle). 6 Deferred Ideas (Sammelexport, E-Mail-Versand, Unterschriftenspalte, Logo, Multi-Sheet, Export-Audit). Nächster Schritt: `/gsd-plan-phase 6`.

**Last action (2026-05-17):** Phase 5 (Pre-GV-Generalprobe) als SKIPPED markiert in ROADMAP.md und STATE.md — echte GV bereits erfolgreich mit Genossi durchgeführt; Hotfixes aus dem realen Einsatz sind bereits committed (live-counter, button types, sort by member_number, token-codes magic-link). Nächster Schritt: `/gsd-discuss-phase 6` für Teilnehmerlisten-Export (CSV/PDF an Protokoll anhängbar).

**Last action (Phase 03-06):** REST handlers + DI-Wiring + 6 E2E tests komplett — Phase 3 vollständig abgeschlossen. 4 attendance REST-Handler in genossi_rest/src/attendance.rs (list_members/mark_present/mark_absent/get_stats) mit lokalem map_attendance_error (PermissionDenied → 403, D-26). RestStateImpl in genossi_bin DI-gewired mit AttendanceServiceImpl + 6 Deps. 6 grüne E2E-Tests gegen real-laufenden HTTP-Server mit in-memory SQLite — alle 9 Phase-3-Requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) + SC#8-Cascade-DB direkt verifiziert. 234/234 E2E-Tests grün (228 vorher + 6 neu); 4 unit-Tests + 6 E2E = 10 neue grüne Tests. 4 Task-Commits (`a553b6a`, `b72b72c`, `e39af6b`, `e90bd33`).

**Next action:** Phase 7 ist abgeschlossen. Run `/gsd-verify-phase 07` für die Phase-Verifikation, oder direkt `/gsd-discuss-phase 8` für den Phase-8-Start (RepaymentEntry + Auto-Befüllung). Code-Anker für Phase 8 existieren als TODO-Kommentare in `genossi_service_impl/src/repayment_phase.rs::open_repayment_phase` (PHAS-02) und `close_repayment_phase` (PHAS-03).

**Files written this session (Plan 07-05):**

- `genossi_bin/tests/e2e_tests.rs` (MOD — +458 LOC: RepaymentPhaseTO + RepaymentPhaseStatusTO Imports, `create_preparation_repayment_phase`-Helper, 7 neue E2E-Tests am Datei-Ende für Lifecycle + Audit-Chain + Edit-Matrix + Validation)
- `.planning/phases/07-repaymentphase-backend-foundation/07-05-SUMMARY.md` (NEW)
- `.planning/STATE.md` (MOD — diese Aktualisierung; Phase 7 Plan 5 als complete markiert)
- `.planning/ROADMAP.md` (MOD — Phase 7 Status auf Complete; Plan-Progress 5/5)
- `.planning/REQUIREMENTS.md` (MOD — PHAS-02 + PHAS-03 als complete markiert)

**Files written 2 sessions ago (Plan 07-04):**

- `genossi_rest_types/src/lib.rs` (MOD — +210 LOC: RepaymentPhaseStatusTO + bidirektionale From-Impls + RepaymentPhaseTO + From<&RepaymentPhase>-Impl + CreateRepaymentPhaseRequest + UpdateRepaymentPhaseRequest + 4 Unit-Tests in mod repayment_phase_to_tests)
- `genossi_rest/src/repayment_phase.rs` (NEW — 414 LOC: RepaymentPhaseRestState-Trait + 7 Handler mit #[utoipa::path] + 2 Validator-Stubs + generate_route + ApiDoc + 3 Smoke-Tests)
- `genossi_rest/src/lib.rs` (MOD — pub mod repayment_phase + OpenAPI nest + Router .nest + Trait-Bounds auf create_app + start_server)
- `genossi_rest/src/test_server.rs` (MOD — Trait-Bound RepaymentPhaseRestState auf start_test_server für Plan 05)
- `genossi_bin/src/lib.rs` (MOD — +53 LOC: type-Alias RepaymentPhaseDao + RepaymentPhaseService + RepaymentPhaseServiceDependencies + 5-Deps-Impl + Self-Field + Service-Konstruktion + RestState-Impl)
- `.planning/phases/07-repaymentphase-backend-foundation/07-04-SUMMARY.md` (NEW)
- `.planning/STATE.md` (MOD — diese Aktualisierung)
- `.planning/ROADMAP.md` (MOD — Phase 7 Plan-Progress aktualisiert)
- `.planning/REQUIREMENTS.md` (MOD — PHAS-01, PHAS-04 als REST-complete markiert)

**Files written previous session (Plan 07-03):**

- `genossi_service/src/repayment_phase.rs` (NEW — RepaymentPhase Domain-Typ + RepaymentPhaseSubmission/Update DTOs + #[automock] RepaymentPhaseService-Trait mit 7 Methoden + 4 Unit-Tests, 266 LOC)
- `genossi_service/src/lib.rs` (MOD — `pub mod repayment_phase;` alphabetisch zwischen `permission` und `session`)
- `genossi_service_impl/src/repayment_phase.rs` (NEW — RepaymentPhaseServiceImpl + 5 Prozesskonstanten + validate_phase_fields-Helper + 7 Service-Methoden mit Edit-Matrix/Lifecycle-Guards/Audit-Macros + 4 Hand-rolled Mocks + 13 Unit-Tests, 1108 LOC)
- `genossi_service_impl/src/lib.rs` (MOD — `pub mod repayment_phase;` alphabetisch zwischen `permission` und `rfc3161`)
- `.planning/phases/07-repaymentphase-backend-foundation/07-03-SUMMARY.md` (NEW)

---
*State initialized: 2026-05-02*
*Phase 03 Plan 01 completed: 2026-05-04*
*Phase 03 Plan 02 completed: 2026-05-04*
*Phase 03 Plan 03 completed: 2026-05-04*
*Phase 03 Plan 04 completed: 2026-05-04*
*Phase 03 Plan 05 completed: 2026-05-04*
*Phase 03 Plan 06 completed: 2026-05-04*
*Phase 03 COMPLETE: 2026-05-04*
*Phase 07 Plan 01 completed: 2026-05-29*
*Phase 07 Plan 02 completed: 2026-05-29*
*Phase 07 Plan 03 completed: 2026-05-29*
*Phase 07 Plan 04 completed: 2026-05-29*
*Phase 07 Plan 05 completed: 2026-05-29*
*Phase 07 COMPLETE: 2026-05-29*
