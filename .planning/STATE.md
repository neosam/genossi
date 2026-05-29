---
gsd_state_version: 1.0
milestone: v1.1
milestone_name: Anteile-Rückzahlungsphase
status: planning
last_updated: "2026-05-29T18:04:19.561Z"
last_activity: 2026-05-29
progress:
  total_phases: 0
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# State: Genossi — GV-Anwesenheits-Erfassung

**Initialized:** 2026-05-02
**Last Updated:** 2026-05-02 (roadmap creation)

## Project Reference

**Core Value:** Genossenschaften verwalten ihre Mitglieder ohne Excel — verbandskonform, nachvollziehbar (Audit-Hashchain), mit weniger manueller Arbeit. Dieser Milestone bringt papierlose Anwesenheits-Erfassung auf der Generalversammlung.

**Current Focus:** v1.0 shipped (2026-05-29) — planning next milestone (Kandidat: Auszahlungs-/Anteils-Feature, siehe `.planning/seeds/`)

**Granularity:** coarse (5 active + 1 SKIPPED Phasen für v1.0; nächster Milestone TBD)

## Current Position

Phase: Not started (defining requirements)
Plan: —
Status: Defining requirements
Last activity: 2026-05-29 — Milestone v1.1 started

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

**Last action (2026-05-17, Phase 06 Discuss):** `/gsd-discuss-phase 6` durchgeführt — `06-CONTEXT.md` + `06-DISCUSSION-LOG.md` erstellt und committed (`30a3c2b`). 20 Implementierungsentscheidungen (D-01..D-20) erfasst: drei Export-Formate parallel (PDF via Typst, CSV semikolon/UTF-8-BOM, XLSX via rust_xlsxwriter), `?include=all|present`-Query, Status-Closed-only, Vorstand-only via OIDC, Snapshot-Daten aus `assembly_member_snapshot`, Sortierung `member_number ASC`, Endpoint `GET /api/assembly/{aid}/attendance-export/{format}`, Filename `gv-{YYYY-MM-DD}-teilnehmer.{ext}`, kein Audit-Hashchain-Eintrag. PDF-Layout minimal (Kopf mit GV-Titel + Datum + „X von Y anwesend", dann Tabelle). 6 Deferred Ideas (Sammelexport, E-Mail-Versand, Unterschriftenspalte, Logo, Multi-Sheet, Export-Audit). Nächster Schritt: `/gsd-plan-phase 6`.

**Last action (2026-05-17):** Phase 5 (Pre-GV-Generalprobe) als SKIPPED markiert in ROADMAP.md und STATE.md — echte GV bereits erfolgreich mit Genossi durchgeführt; Hotfixes aus dem realen Einsatz sind bereits committed (live-counter, button types, sort by member_number, token-codes magic-link). Nächster Schritt: `/gsd-discuss-phase 6` für Teilnehmerlisten-Export (CSV/PDF an Protokoll anhängbar).

**Last action (Phase 03-06):** REST handlers + DI-Wiring + 6 E2E tests komplett — Phase 3 vollständig abgeschlossen. 4 attendance REST-Handler in genossi_rest/src/attendance.rs (list_members/mark_present/mark_absent/get_stats) mit lokalem map_attendance_error (PermissionDenied → 403, D-26). RestStateImpl in genossi_bin DI-gewired mit AttendanceServiceImpl + 6 Deps. 6 grüne E2E-Tests gegen real-laufenden HTTP-Server mit in-memory SQLite — alle 9 Phase-3-Requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) + SC#8-Cascade-DB direkt verifiziert. 234/234 E2E-Tests grün (228 vorher + 6 neu); 4 unit-Tests + 6 E2E = 10 neue grüne Tests. 4 Task-Commits (`a553b6a`, `b72b72c`, `e39af6b`, `e90bd33`).

**Next action:** Phase 04 plans created (10 plans across 5 waves). Run /gsd-execute-phase 04. Wave 1 (parallel): Plans 01 (Backend Helper-Endpoints) + 02 (Frontend Foundation) + 03 (api.rs + i18n). Wave 2 (parallel): Plans 04 (shared attendance) + 05 (helper login) + 06 (vorstand layout). Wave 3 (sequential): Plans 07 (routing) → 08 (vorstand pages). Wave 4: Plan 09 (helfer pages). Wave 5: Plan 10 (UAT checkpoint).

**Files written this session (Plan 06):**

- `genossi_rest/src/attendance.rs` (NEW — 4 Handler + AttendanceRestState-Trait + 2 Router-Builder + map_attendance_error + ApiDoc + 4 Unit-Tests)
- `genossi_rest/src/lib.rs` (MOD — `pub mod attendance` + ApiDoc-nest + 2 `.nest()` für `/api/attendance/{aid}` und `/api/assembly/{aid}` stats + AttendanceRestState-Bound auf create_app/start_server)
- `genossi_rest/src/test_server.rs` (MOD — AttendanceRestState-Bound auf start_test_server)
- `genossi_bin/src/lib.rs` (MOD — type alias AttendanceDao + AttendanceServiceDependencies + AttendanceService + RestStateImpl.attendance_service Field + RestStateImpl::new() Construction + impl AttendanceRestState)
- `genossi_bin/tests/e2e_tests.rs` (MOD — 4 neue Imports + create_open_assembly_with_members Helper + 6 E2E-Tests)
- `.planning/phases/03-attendance-aggregat-cascade-invalidation/03-06-SUMMARY.md` (NEW)
- `.planning/STATE.md` (MOD — diese Aktualisierung)
- `.planning/ROADMAP.md` (MOD — Phase 3 als COMPLETE markiert)
- `.planning/REQUIREMENTS.md` (MOD — alle 9 Phase-3-Requirements als END-TO-END-VERIFIED markiert)

---
*State initialized: 2026-05-02*
*Phase 03 Plan 01 completed: 2026-05-04*
*Phase 03 Plan 02 completed: 2026-05-04*
*Phase 03 Plan 03 completed: 2026-05-04*
*Phase 03 Plan 04 completed: 2026-05-04*
*Phase 03 Plan 05 completed: 2026-05-04*
*Phase 03 Plan 06 completed: 2026-05-04*
*Phase 03 COMPLETE: 2026-05-04*
