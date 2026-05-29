# Milestones

## v1.0 GV-Anwesenheits-Erfassung (Shipped: 2026-05-29)

**Phases completed:** 5 phases, 34 plans, 68 tasks

**Key accomplishments:**

- SQLite migrations and DAO traits/impls for the Assembly aggregate plus the

composite-PK assembly_member_snapshot join table — including English-only
AssemblyStatus enum, Auditable impl with 6 audit fields, optimistic-locking
update path, and 17 unit tests.

- Five public TOs (AssemblyStatusTO, AssemblyTO, AssemblyDetailTO, CreateAssemblyRequest, UpdateAssemblyRequest) with ToSchema for OpenAPI, ISO8601 datetime serde on every Optional<PrimitiveDateTime>, and bidirectional Status-enum conversion between DAO and wire format.
- AssemblyService trait + DTOs in `genossi_service::assembly` and full

`AssemblyServiceImpl` in `genossi_service_impl::assembly` covering the
Preparation→Open→Closed lifecycle. `open_assembly` is atomic (single Tx
for status flip, audit entry, and snapshot population). 12 unit tests
total (6 in service-trait crate, 6 mockall-based in service-impl).

- Six Axum handlers in `genossi_rest::assembly` (list/create/get/update/open/close) plus full DI wiring of `AssemblyServiceImpl` into `genossi_bin::RestStateImpl`. Validation helpers, ApiDoc, router registration, and three type-bound updates (`create_app`, `start_server`, `start_test_server`) — workspace builds and 215 e2e tests stay green.
- Three new e2e tests in `genossi_bin/tests/e2e_tests.rs` covering the full Assembly lifecycle (Preparation → Open → Closed) with audit hash chain verification (ASSY-07) and two negative tests for illegal state transitions (Pitfall 3). 218/218 e2e tests green; full workspace test suite green; release build clean. Phase 01 goal end-to-end test-belegt.
- SQLite helper_token table + Auditable DAO trait + race-safe atomic_redeem on UPDATE...RETURNING — proven by 11 unit tests including double-redeem regression
- Typsichere `AuthContext::Helper { session_id, assembly_id }`-Variante als Phase-3-Vorbereitung — ohne cfg-Gate verfügbar in mock_auth und oidc, mit zwei Konstruktions-/Distinktheits-Tests und Smoke-Test gegen versehentliche Feature-Gate-Regression.
- Six REST TOs (HelperTokenStatusTO, HelperTokenTO, HelperTokenCreateResponseTO, CreateHelperTokenRequest, RedeemRequest, RedeemResponse) plus a 4-method HelperTokenService trait with #[automock] mock — proven by 13 unit tests including a defensive token_hash leak guard and a Debug-output guard.
- HelperTokenServiceImpl with gen_service_impl! over 8 deps, 4 service methods (create+list+revoke+redeem), Crockford+SHA256+QR+atomic-redeem orchestration, and ServiceError-discriminator-string convention proven by 11 unit tests including all four D-24 mapping branches.
- Helper-Sessions werden im SessionService an `claims.kind=="helper"` erkannt; D-18 invalidiert die Session sofort, wenn die gebundene Assembly nicht mehr `Open` ist — Pitfall 2 Early-Return verhindert DB-Roundtrip auf dem User-Session-Hot-Path. Mock-Variante erkennt Cookie-Format `helper:<uuid>:<tok>` und cascadiert via optionalem AssemblyStatusProbe.
- Vier Axum-Handler (3 Vorstand admin + 1 Public mit Set-Cookie und Pro-IP-Rate-Limit), zwei neue RestError-Varianten (403/410) für die D-24-Differenzierung, vollständiges DI-Wiring in genossi_bin mit DbAssemblyStatusProbe für HLPR-05-Cascade in mock_auth-Builds — proven by 4 grünen Validation-Tests im genossi_rest und 189 grünen workspace-tests in mock_auth + oidc.
- 10 E2E-Tests in `genossi_bin/tests/e2e_tests.rs` decken HLPR-01/02/04/05/06/07 ab; aufgedeckt + behoben wurden zwei Plan-05-Service-Bugs (redeem pool-deadlock, revoke version-mismatch) und der Mock-Session FK-Constraint-Mismatch — alle 228 e2e_tests.rs-Tests grün, alle 528 workspace-lib-tests grün in beiden Feature-Builds.
- Lightweight Attendance-Join-Tabelle mit atomarem SQLite-UPSERT, idempotentem Soft-Delete-Toggle, DSGVO-Whitelist-View und Snapshot-Membership-Check — alles ohne Audit-Log und ohne Optimistic-Locking.
- Eine neue Trait-Method `list_session_ids_for_assembly` auf HelperTokenDao + SQLite-Impl + 3 grüne Tests; Cascade-Anker (D-12) für Plan 05's `AssemblyServiceImpl::close_assembly`-Erweiterung.
- Trait-Erweiterung `ClaimContext::as_helper(&self) -> Option<Uuid>` mit Default-Impl (failure-closed → None) und einem AuthenticatedContext-Override, der Phase-2-HelperClaims-JSON defensiv parst — die typsichere Brücke für Plan 05's Helper-Permission-Branch.
- Service-Interface-Layer für die GV-Anwesenheits-Erfassung — 4-Methoden-Trait (`list_members`, `mark_present`, `mark_absent`, `stats`), `AttendanceStats`-Domain-Type, `AttendanceMemberTO`-Whitelist mit 7 Feldern, `AttendanceStatsTO` für den Live-Counter — alles ohne ServiceImpl-Wiring, bereit für Plan 05+06 als Konsumenten.
- Service-logic core of Phase 3: a 4-method `AttendanceServiceImpl` plus the central `check_assembly_access` permission funnel (Helper / Vorstand / Full discrimination), AND the cascade-extension to `AssemblyServiceImpl::close_assembly` that invalidates every helper-session bound to the closing GV. All 19 new tests green; all 188 pre-existing service-impl tests stay green.
- Phase 3 final integration: 4 attendance REST handlers, DI-wiring of AttendanceServiceImpl into the binary's RestStateImpl, OpenAPI doc registration, and 6 end-to-end tests against a real-running HTTP server with in-memory SQLite. All 9 Phase-3 requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) plus the SC#8 cascade-DB invariant verified at the integration level.
- GET /api/helper/session und POST /api/helper/logout als append-only Routen im existierenden helper_redeem_router — Frontend kann jetzt Auto-Redirect und Logout ohne /api/attendance-Probe machen.
- Foundation-Layer für Phase-4-Frontend gebaut: web-sys Camera-Features, vendored ZXing-JS-Polyfill, JS-Bridge für nativen BarcodeDetector mit Feature-Detection, Print-CSS für QR-Karten, Tailwind-Safelist und cargo-testbares Crockford-Validation-Modul mit 9 grünen Unit-Tests.
- 14 Phase-4-TOs, 16 async API-Funktionen, 410-Mapping und 67 i18n-Keys (de+en) — Voraussetzung für alle Wave-2-Components und Pages.
- Vier shared Components (AttendanceSearch, AttendanceList, LiveCounter, ConnectionBanner) inklusive Pure-Function-Helpers für unit-testable UI-Logik — der Component-First-Anker für ATTN-06-Reuse zwischen Helfer- und Vorstand-Anwesenheits-Pages.
- Vier Helper-Login-/Token-Components: ManualCodeInput (HLPR-03 iOS-Fallback), QrScanner (BarcodeDetector + ZXing-Polyfill mit Camera-Lifecycle-Cleanup), QrCard (printable Token-Card), HelperShell (no-Vorstand-Chrome Layout mit Locale::De-Forcing).
- Phase:
- mod.rs nimmt 15 Wave-2.1-Components in Empfang — Pages 07-09 koennen jetzt sauber importieren.
- Zwei Vorstand-Pages voll ausimplementiert: `/assemblies` (Liste + Create-Modal) und `/assemblies/{id}` (3-Tab-Detail mit Stammdaten, Tokens, Anwesenheit). Anwesenheits-Tab nutzt EXAKT dieselben 4 Components, die Plan 04-09 für /helper/attendance verwenden wird — ATTN-06 Reuse-Anker etabliert.
- Beide Helfer-Pages voll ausimplementiert: `/helper` (Login mit QR-Scan + Manual-Code parallel + Auto-Redirect) und `/helper/attendance` (4 shared Components in HelperShell-Layout). ATTN-06 Component-Reuse bewiesen: helper_attendance.rs nutzt identische Component-Invocations wie assembly_details.rs Anwesenheits-Tab — einziger Unterschied ist der HelperShell- vs. RequirePrivilege-Wrapper.
- Workspace-Dependency-Promotion fuer rust_xlsxwriter/csv plus neues Typst-Template `teilnehmerliste.typ` mit konditionalem X-von-Y-Kopf und 6-spaltiger Repeat-Header-Tabelle.
- AttendanceExportService Trait + Impl mit Admin+Closed-Funnel, drei Format-Writern (CSV BOM/Semikolon, XLSX rust_xlsxwriter, PDF via Typst-Template), 6-Spalten-DSGVO-Whitelist, kein Audit-Log (D-17), strukturiertes tracing::info! (D-18) — 16/20 Phase-6-Decisions in Code uebertragen.
- HTTP-Endpoint `GET /api/assembly/{aid}/attendance-export/{format}` ist live aufrufbar; AttendanceExportServiceImpl ist in RestStateImpl gewired; 9 E2E-Tests decken PDF/CSV/XLSX-Erfolgspfade, 409 fuer Open/Preparation, 400 fuer unbekanntes Format, include=present-Filter, Filename-Schema und D-12-Post-Close-Edit-Reflexion ab. Plus Rule-2-Auto-Fix: teilnehmerliste.typ in DEFAULT_TEMPLATES — ohne das funktioniert PDF-Export nicht out-of-the-box.
- Closed-only Export-Tab in assembly_details.rs lets Vorstand download attendance lists as PDF/CSV/XLSX via a blob-URL pipeline — Task 1 (i18n) and Task 2 (API + ExportTab) are committed; Task 3 (browser verification checkpoint) is pending human approval.

---
