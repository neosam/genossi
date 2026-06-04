# Phase 14: DAO/Domain Foundation - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 14 ist die **read-only Foundation** für v1.2-Mitgliedschaft-Anpassungen. Sie liefert die Pure-Function für die H1/H2-Stichtagsregel sowie die DAO-/Service-/REST-Queries, auf die Phase 15-17 ihre Write-Operationen aufsetzen. Keine `MemberAction`-Erzeugung, kein Member-Update, keine RepaymentEntry-Inserts.

**In scope:**
- **Pure-Function `compute_effective_date(willensbekundung: Date) -> EffectiveDate`** in `genossi_service_impl/src/membership_adjust.rs` (neue Datei, Foundation für Phase 15-17). Struct `EffectiveDate { fiscal_year: i32, effective_date: Date }`. Sichtbarkeit `pub(crate)`. Ausführlicher `///`-Doc-Kommentar mit verankerter H1/H2-Verbands-Konvention. CANC-02.
- **6+ Unit-Tests** für `compute_effective_date`, mindestens die folgenden Edge-Cases: 30.06. (H1), 01.07. (H2), 31.12. (H2 → folgendes Jahr), 01.01. (H1), 29.02.2024 Schaltjahr (H1), 15.03. mittiges Datum (H1).
- **DAO-Methode `RepaymentEntryDao::find_by_member_and_phase(member_id, phase_id, tx) -> Arc<[RepaymentEntryEntity]>`** in `genossi_dao/src/repayment_entry.rs` als Trait-Methode + SQL-Override in `genossi_dao_impl_sqlite/src/repayment_entry.rs`. Filter: `member_id = ? AND phase_id = ? AND deleted IS NULL`. Foundation für PITFALLS-Kat-1-Sum-Check und Auto-Fill-Skip-Pattern in Phase 16.
- **DAO-Tests:** mindestens 2 SQLite-Tests (leere Liste + mehrere Entries für (member, phase)).
- **Service-Methode `MemberService::list_transfer_recipients(exclude_member_id, context, tx) -> Arc<[Member]>`** (Extension von existierendem `MemberService` Trait in `genossi_service/src/member.rs`). Filter: `exit_date IS NULL AND id != exclude_member_id AND deleted IS NULL`. Permission-Gate `ADMIN_PRIVILEGE`. TRSF-06 Foundation.
- **3 Service-Unit-Tests** mit MockMemberDao (Happy-Path: 3 aktive Members → 2 zurück; alle gekündigt → leere Liste; nur exclude_self → leere Liste).
- **REST-Endpoint `GET /api/members/transfer-recipients?exclude_self={uuid}`** in `genossi_rest/src/member.rs`, gemountet via `member::generate_route` an `/api/members`. Permission `ADMIN_PRIVILEGE`. Response: `Vec<MemberSlimTO>` als JSON.
- **`MemberSlimTO`** als neuer Typ in `genossi_rest_types/src/lib.rs` mit Feldern: `id, member_number, first_name, last_name, title (Option), salutation (Option<Salutation>)`. Konvertierung `impl From<&Member> for MemberSlimTO`.
- **1 E2E-Test** in `genossi_bin/tests/`: Setup mit 3 Members (1 mit `exit_date IS NOT NULL` → ausgefiltert; 1 = exclude_self → ausgefiltert; 1 aktiv → enthalten). Admin-Auth.
- **Default-Impl-Test im DAO-Trait-Modul** mit MockTransaction für die Trait-Definition (falls eine Default-Impl bereitgestellt wird; siehe D-14-08).

**Out of scope (deferred / explizit nicht):**
- Keine Write-Operationen: keine `MemberAction`, kein `RepaymentEntry`-Insert, kein `Member.current_shares`-Update, kein `recalc_dates`-Trigger. Alle Writes leben in Phase 15-17.
- Kein Search-Query / keine Pagination am Endpoint — volle gefilterte Liste reicht bei <200 Genossenschafts-Mitgliedern.
- Keine Permission-Validation auf REST-Layer für das `exclude_self`-UUID-Format jenseits des UUID-Parsings — Service-Layer-Filter ist ausreichend.
- Keine Datums-Validierung (z.B. "Willensbekundungsdatum muss im offenen GJ liegen") — die liegt in Phase 15 (PERM-02).
- Keine Tests für die Pure-Function aus Phase 15/16/17 heraus — die testen ihre Service-Methoden separat und treffen `compute_effective_date` indirekt.

</domain>

<decisions>
## Implementation Decisions

### Pure-Function Shape + Placement

- **D-14-01:** **Return-Struct `EffectiveDate { fiscal_year: i32, effective_date: Date }`.** Named fields, selbsterklärend am Call-Site. **Why:** Phase 15 (Kündigung) liest nur `effective_date` für `MemberAction::Austritt.effective_date`; Phase 16 (Teil-Rückgabe) liest nur `fiscal_year` für RepaymentPhase-Lookup. Tuple `(i32, Date)` aus Roadmap-Vorlage wäre kürzer, aber `let result = compute_effective_date(d); use result.fiscal_year` ist sauberer als `let (fy, _) = ...`. **How to apply:** Struct in `membership_adjust.rs` neben der Funktion definieren. `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` für Test-Komfort.

- **D-14-02:** **Modul-Placement `genossi_service_impl/src/membership_adjust.rs` (neue Datei).** Foundation für Phase 15-17, wo `cancel_membership`, `increase_shares`, `partial_repayment`, `transfer_shares` als Service-Methoden ergänzt werden. **Why:** `member_action.rs` (das die Pure-Function-Verwandte `compute_dates` enthält) würde durch v1.2 zu groß werden — ARCHITECTURE.md §1 schlägt "Extension von `MemberActionService`" vor, aber separates Modul daneben ist sauberer für künftiges Wachstum. **How to apply:** `lib.rs` registriert `pub mod membership_adjust;`. In Phase 15-17 wird das Modul mit weiteren Funktionen und einem `MembershipAdjustService`-Trait/Impl gefüllt.

- **D-14-03:** **Visibility `pub(crate)`** für `compute_effective_date` und `EffectiveDate`. **Why:** Konsistent mit `compute_dates` (`member_action.rs:155`, `pub(crate)`). Service-Layer wrapped die Logik intern; REST-Layer ruft die Pure-Function nicht direkt auf (Validation in Phase 15 PERM-02 ist Service-internal). **How to apply:** `pub(crate) fn compute_effective_date(...)` + `pub(crate) struct EffectiveDate`. Tests im selben Modul via `#[cfg(test)] mod tests`.

### H1/H2 Edge-Case-Semantik

- **D-14-04:** **Halbjahres-Grenze `month <= 6`.** 30.06. zählt zu H1 (`fiscal_year = year(d)`, `effective_date = 31.12. year(d)`). 01.07. zählt zu H2. **Why:** Verbands-Konvention "erstes Halbjahr inkludiert Juni komplett"; ARCHITECTURE.md §3 Code-Vorlage nutzt `month <= 6`. **How to apply:** Branch `if willensbekundung.month() as u8 <= 6 { ... } else { ... }`. Doc-Kommentar verankert die Regel explizit (siehe D-14-07).

- **D-14-05:** **31.12.YYYY → H2 → effective_date = 31.12.YYYY+1, fiscal_year = YYYY+1.** Jahresletzter Tag ist `month == 12` → H2-Zweig. **Why:** Konsistent mit der `month <= 6`-Regel ohne Sonderfall. "Wer im Dezember kündigt, geht erst Ende nächstes Jahr" entspricht v1.1-PaidOut-Cascade-Erwartung (späteres Auszahlungs-GJ). **How to apply:** Test-Case `compute_effective_date(2026-12-31) == EffectiveDate { fiscal_year: 2027, effective_date: 2027-12-31 }`.

- **D-14-06:** **Schaltjahr-Test mit 29.02.2024 → H1 → 31.12.2024.** Defensive Coverage trotz fehlender mathematischer Notwendigkeit (31.12. ist nie Schalttag). **Why:** Roadmap-Success-Criteria nennt Schaltjahr-Februar explizit; Test sichert ab, dass `time::Date::from_calendar_date` mit Schalttag korrekt umgeht. **How to apply:** Ein dedizierter Test `test_schaltjahr_29_februar_h1` + Assertion auf `fiscal_year == 2024` und `effective_date.day() == 31, .month() == December`.

- **D-14-07:** **Ausführlicher `///`-Doc-Kommentar auf `compute_effective_date`** verankert die H1/H2-Verbands-Konvention im Code. **Why:** Verbands-rechtliche Regel; spätere Wartende müssen wissen, *warum* `month <= 6` (nicht z.B. `<= 5`) gewählt wurde. Roadmap-Test-Cases sind die Spec, aber der Doc-Kommentar erklärt das Warum. **How to apply:** Kommentar enthält: (1) Verbands-Konvention zitieren ("H1 = Monat 1-6, H2 = 7-12"), (2) Verhalten bei Grenze (30.06. ist H1, 01.07. ist H2, 31.12. ist H2), (3) Hinweis auf 6 Edge-Case-Tests im selben Modul.

### Transfer-Recipients Endpoint Shape

- **D-14-08:** **DAO-Methode `RepaymentEntryDao::find_by_member_and_phase` als Trait-Methode mit SQL-Override in SQLite-Impl.** SQL: `SELECT ... WHERE member_id = ? AND phase_id = ? AND deleted IS NULL`. **Why:** Phase 16 ruft das im kritischen Sum-Check + Auto-Fill-Skip-Pattern auf (PITFALLS Kat 1). SQL-WHERE-Klausel skaliert besser als Default-Impl via `dump_all().filter(...)` bei wachsender Genossenschaft. Trade-off: bricht das `find_by_phase_id`-Default-Impl-Pattern, aber Performance-Pfad rechtfertigt das. **How to apply:** Trait-Definition in `genossi_dao/src/repayment_entry.rs` mit Default-Impl als Fallback (via `dump_all`), damit Mock-Impls nichts zu tun haben. SQLite-Impl überschreibt mit SQL-Query. SQLite-Test in `genossi_dao_impl_sqlite/src/repayment_entry.rs` mit echter In-Memory-DB (leere Liste, mehrere Entries inkl. einer für andere Phase).

- **D-14-09:** **DAO-Return-Type `Arc<[RepaymentEntryEntity]>`.** Konsistent mit `find_by_phase_id` (Z. 138) und gesamtem v1.1-Codebase. **Why:** Cheap-Clone, idiomatisch für Sharing zwischen Tasks; Vec wäre Mehraufwand ohne Nutzen. **How to apply:** Konvertierung `let filtered: Vec<...> = ...; Ok(filtered.into())`.

- **D-14-10:** **Endpoint `GET /api/members/transfer-recipients?exclude_self={uuid}` als Sub-Route von `/api/members`.** Mount via `member::generate_route` (Erweiterung von `genossi_rest/src/member.rs:28`). Query-Param-Name `exclude_self`. **Why:** Roadmap-Vorlage (TRSF-06); konsistent mit anderen `GET /api/members/...`-Endpoints. `exclude_self` ist domain-spezifisch und selbsterklärend (das Mitglied, das den Transfer auslöst).

- **D-14-11:** **Endpoint admin-only via `ADMIN_PRIVILEGE`.** Service-Layer-Permission-Check. **Why:** Konsistent mit REQUIREMENTS PERM-01 (alle v1.2-Operationen admin-only). Endpoint wird nur vom v1.2-Übertrag-Dialog genutzt. Schützt Member-Liste vor Helfer-Auth (die ja restriktiv auf Mitgliedsnummer/Name beschränkt sein soll).

- **D-14-12:** **Response-Type `Vec<MemberSlimTO>` mit reduziertem Schema.** Neuer Typ `MemberSlimTO { id: Uuid, member_number: i32, first_name: Arc<str>, last_name: Arc<str>, title: Option<Arc<str>>, salutation: Option<Salutation> }` in `genossi_rest_types/src/lib.rs`. **Why:** Endpoint ist Display-Listing-Only (Empfänger-Search-Dropdown in Phase 18). Voller `MemberTO` würde Adresse/IBAN/Bankdaten ausliefern — unnötig für Search-Use-Case. Klarer API-Vertrag, kein Datenleck. **How to apply:** `impl From<&Member> for MemberSlimTO` in `genossi_rest_types`. Frontend (Phase 18) konsumiert das im `MemberSearch`-Component, der diese Felder bereits darstellt.

### Service-Layer

- **D-14-13:** **Service-Methode `MemberService::list_transfer_recipients(exclude_member_id: Uuid, context, tx) -> Arc<[Member]>`** als Erweiterung des existierenden `MemberService`-Traits in `genossi_service/src/member.rs`. **Why:** Konsistent mit ARCHITECTURE.md §1 "Extension von Service" statt neuer Service-Crate. `MemberService` ist der natürliche Owner. Mit `Arc<[Member]>` (volles Member, nicht Slim-TO) — Slim-Conversion passiert auf REST-Layer. **How to apply:** Trait-Signatur ergänzen; `MemberServiceImpl::list_transfer_recipients` macht Permission-Check + `member_dao.all(tx).await? .iter().filter(|m| m.exit_date.is_none() && m.id != exclude_member_id).cloned().collect().into()`.

### Test-Strategie

- **D-14-14:** **Vollständige Test-Coverage über alle Layer.**
  - DAO: 1 Default-Impl-Test im Trait-Modul `genossi_dao/src/repayment_entry.rs` mit `MockTransaction`-Setup; 2 SQLite-Tests in `genossi_dao_impl_sqlite/src/repayment_entry.rs` (leere Liste + mehrere Entries inkl. ausgefilterte Phase) mit echter In-Memory-DB.
  - Service: 3 Unit-Tests in `genossi_service_impl/src/member.rs` (oder `membership_adjust.rs`) mit `MockMemberDao` (Happy 3-Members; alle gekündigt; nur self).
  - Pure-Function: 6 Edge-Case-Tests (30.06., 01.07., 31.12., 01.01., 29.02.2024 Schaltjahr, 15.03. mittig) in `membership_adjust.rs::tests`.
  - REST: 1 E2E-Test in `genossi_bin/tests/` mit echter In-Memory-DB, Admin-Auth, 3 Members (gekündigt/self/aktiv → 1 zurück).
  - **Why:** Klare Layer-Trennung im Test-Pyramiden-Sinne; jeder Layer hat seine eigene Verantwortung getestet, ohne Über-Test im E2E. **How to apply:** Test-Datei-Konventionen wie v1.1 Phase 7-13 (siehe `13-CONTEXT.md` für E2E-Setup-Vorbild).

- **D-14-15:** **Test-Count ist Mindest-Anforderung mit Planner-Discretion nach oben.** Roadmap nennt 6/2/3/1 als Floor; wenn Planner zusätzliche Edge-Cases identifiziert (z.B. soft-deleted Member im Service-Filter), darf er ergänzen. **Why:** Roadmap-Counts decken die kritischen Pfade; Edge-Case-Erweiterung ist legitimer Planner-Spielraum. **How to apply:** Planner darf zusätzliche Tests hinzufügen, aber nicht die Roadmap-Pflicht-Tests weglassen.

### Claude's Discretion

- **Default-Impl-Strategie:** Trait-Methode darf optional eine Default-Impl via `dump_all().filter(...)` haben (Pattern aus `find_by_phase_id` Z. 138), damit `MockRepaymentEntryDao` ohne Erweiterung mit Phase-14-Service-Tests interagieren kann. SQLite-Impl überschreibt mit SQL-Query. Planner darf entscheiden, ob der Default-Impl-Path im Trait nötig ist oder ob `#[automock]` die Mock-Generierung sauberer löst.
- **`MemberSlimTO`-Field-Set:** Vorschlag oben (`id, member_number, first_name, last_name, title, salutation`) ist Mindest-Set. Planner darf weitere Felder hinzufügen, wenn Phase-18-UI sie zeigt (z.B. `current_shares` für Empfänger-Search-Anzeige "X Anteile bisher"). Aber: keine sensiblen Felder (IBAN, Adresse, Email).
- **Endpoint-OpenAPI-Schema:** Utoipa-Definition mit Status-Codes 200 (Vec<MemberSlimTO>), 401 (no auth), 403 (no admin), 400 (invalid UUID in exclude_self).
- **`compute_effective_date`-Funktionsname:** Festgelegt auf `compute_effective_date` (Roadmap-Vorlage). Planner darf Inline-Helper (z.B. `is_h1(month) -> bool`) hinzufügen, falls die Branch-Logik komplexer wird (z.B. wenn später Verband Sonderregeln einführt).
- **Doc-Kommentar-Sprache:** Deutsch im `///`-Doc-Kommentar OK (verbands-spezifischer Kontext); Code-Identifier englisch (`fiscal_year`, `effective_date`).
- **Reihenfolge der Plan-Dateien:** Empfehlung — `plan_01_pure_function.md`, `plan_02_dao_find_by_member_and_phase.md`, `plan_03_service_list_transfer_recipients.md`, `plan_04_rest_endpoint_and_e2e.md`. Planner darf zusammenfassen oder weiter aufteilen.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Projekt-Foundation
- `.planning/PROJECT.md` — v1.2-Milestone, Constraints, Architektur-Regeln (Layered DAO/Service/REST, Audit-Pflicht, Component-First Frontend).
- `.planning/REQUIREMENTS.md` §CANC-02, §TRSF-06, §PERM-01 — Phase-14-Requirements und ihre Phase-15-17-Folgen (PERM-02, PERM-03).
- `.planning/ROADMAP.md` §Phase 14 — Phase-Goal, Success-Criteria, REQ-Mapping (CANC-02 + TRSF-06).

### Domain & Architektur
- `.planning/notes/membership-adjust-design.md` — Master-Design-Doc, vier Operationen, H1/H2-Logik, UI-Skeleton.
- `.planning/research/ARCHITECTURE.md` §1 (Placement-Decision), §3 (H1/H2-Pure-Function-Skeleton), §7 (Permission-Funnel `ADMIN_PRIVILEGE`).
- `.planning/research/PITFALLS.md` §Kat 1 (Doppelbuchungs-Prävention — Foundation für Phase 16), §Kat 4 (H1/H2-Edge-Cases), §Kat 7 (Empfänger-Search Self-Transfer + Soft-Delete).
- `.planning/research/SUMMARY.md` — Research-Synthese.

### Vorbild-Phasen (Pattern-Quelle)
- `.planning/milestones/v1.1-phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — Layered Service-Foundation-Pattern (Trait + Impl + DI-Wiring), Status-Konvention "Englisch im Code, Frontend i18n".
- `.planning/milestones/v1.1-phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-CONTEXT.md` — Permission-Funnel-Pattern (`check_permission("admin", ...)`), Direct-Download-Response-Pattern (für REST-Endpoint-Konventionen), E2E-Test-Setup.
- `.planning/v1.1-INTEGRATION.md` — v1.1-Audit-Hashchain bleibt grün (E2E-Pattern).

### Code-Referenzen (Files, die berührt werden)
- `genossi_service_impl/src/membership_adjust.rs` — **neue Datei** (Pure-Function + Struct).
- `genossi_service_impl/src/lib.rs` — neue Modul-Registrierung `pub mod membership_adjust;`.
- `genossi_service_impl/src/member_action.rs:155-177` — Vorbild `compute_dates` (Pure-Function-Pattern, `pub(crate)`).
- `genossi_dao/src/repayment_entry.rs:91-150` — bestehender `RepaymentEntryDao`-Trait; Z. 138 `find_by_phase_id` ist das Default-Impl-Vorbild. Neue Methode `find_by_member_and_phase` mit optionaler Default-Impl, SQL-Override in SQLite.
- `genossi_dao_impl_sqlite/src/repayment_entry.rs:71-413` — SQLite-Impl mit `dump_all` + `find_by_phase_id`-SQL-Override. Neue SQL-Methode `find_by_member_and_phase` ergänzen; Tests im selben File (Vorbild Z. 349 `test_dump_all_returns_sorted_entries`, Z. 388 `test_find_by_phase_id_filters_correctly`).
- `genossi_service/src/member.rs:106-143` — bestehender `MemberService`-Trait; neue Methode `list_transfer_recipients` ergänzen.
- `genossi_service_impl/src/member.rs:33-90` — bestehender `MemberServiceImpl`; neue Methode dort (oder ggf. in `membership_adjust.rs` mit Re-Export). Vorbild `get_all` (Z. 35).
- `genossi_rest/src/member.rs:28-74` — bestehender `member`-Router + `get_all_members`-Handler; neue Route `/transfer-recipients` mit Handler `get_transfer_recipients` ergänzen.
- `genossi_rest/src/lib.rs:582` — bestehender Mount `.nest("/api/members", member::generate_route())` ändert sich nicht; neue Sub-Route ist intern in `member::generate_route`.
- `genossi_rest_types/src/lib.rs` — neuer `MemberSlimTO`-Typ + `impl From<&Member> for MemberSlimTO`.
- `genossi_service/src/permission.rs:28` — `ADMIN_PRIVILEGE = "admin"`-Konstante (re-export aus `genossi_service_impl`).
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — keine neue Service-DI-Wiring (Erweiterungen bestehender Services), aber Trait-Methoden-Sync zwischen Trait und Impl prüfen.
- `genossi_bin/tests/` — neue E2E-Test-Datei oder Ergänzung in bestehender `member_*_tests.rs`. Setup-Pattern aus `test_server.rs` wiederverwenden.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`compute_dates`-Pure-Function-Vorbild** (`genossi_service_impl/src/member_action.rs:155-177`): freie Funktion `pub(crate)`, Tuple-Return; nimmt `&MemberEntity + &[MemberActionEntity]`, gibt `(Date, Option<Date>)`. Phase 14 folgt demselben Stil mit Struct-Return.
- **`find_by_phase_id`-Default-Impl-Vorbild** (`genossi_dao/src/repayment_entry.rs:138-150`): `dump_all().filter(|e| e.phase_id == ... && e.deleted.is_none()).collect().into()`. Phase 14 spiegelt das Pattern, ergänzt um Member-Filter; SQL-Override in SQLite skaliert.
- **`MemberDao::find_by_member_number`-Pattern** (`genossi_dao/src/member.rs:155`): existierende Single-Filter-Query, Vorbild für Member-Filter-Logik.
- **`MemberService::get_all`** (`genossi_service/src/member.rs:110`): existierende `Arc<[Member]>`-Listing-Methode mit Authentication-Context und optionaler Transaction. Phase-14-Methode `list_transfer_recipients` folgt derselben Signatur.
- **`get_all_members`-REST-Handler** (`genossi_rest/src/member.rs:53-74`): `error_handler`-Wrapper, JSON-Response, `Vec<MemberTO>`-Mapping. Phase-14-Handler `get_transfer_recipients` ist strukturell identisch, mit `MemberSlimTO`-Mapping + Query-Param-Parsing.
- **`ADMIN_PRIVILEGE`-Konstante** (`genossi_service/src/permission.rs:28`): `pub const ADMIN_PRIVILEGE: &str = "admin"`. Phase 14 nutzt diese direkt für den Service-Layer-Permission-Check.

### Established Patterns
- **Layered DAO/Service/REST mit Trait-Boundaries**: DAO-Trait in `genossi_dao`, SQLite-Impl in `genossi_dao_impl_sqlite`, Service-Trait in `genossi_service`, Impl in `genossi_service_impl`, REST in `genossi_rest`. Phase 14 erweitert bestehende Traits, baut **keinen** neuen Service.
- **Default-Impl auf DAO-Traits via `dump_all`**: `find_by_id`, `all`, `find_by_phase_id`-Pattern. Phase 14 stellt Default-Impl für `find_by_member_and_phase` bereit + SQL-Override in SQLite.
- **Soft-Delete-Filter `deleted IS NULL`**: Genossi-übergreifender Default-Filter in allen DAO-Queries.
- **`Arc<[T]>`-Return-Type für Listen**: Etabliert in Service- und DAO-Layern (cheap-clone Sharing).
- **Permission-Funnel `check_permission(ADMIN_PRIVILEGE, context)` am Service-Methoden-Eintritt**: Etabliert in `RepaymentPhaseServiceImpl` (`repayment_phase.rs:107`) und allen v1.1-Admin-Endpoints. Phase 14 folgt diesem Pattern.
- **Pure-Function-Convention `pub(crate)` mit `#[cfg(test)] mod tests`**: `compute_dates` und andere Helfer-Funktionen sind durchgehend `pub(crate)`.

### Integration Points
- **REST-Mount**: Neue Sub-Route `/transfer-recipients` lebt **innerhalb** `member::generate_route` (`genossi_rest/src/member.rs:28`). `lib.rs:582` ändert sich nicht. Sub-Route-Anker: `.route("/transfer-recipients", get(get_transfer_recipients::<RestState>))`.
- **OpenAPI**: Utoipa-Annotationen analog `get_all_members` (Z. 42-51). Schema-Definition für `MemberSlimTO` in `genossi_rest_types`.
- **Service-Layer-Wiring**: `MemberServiceImpl` existiert bereits, neue Methode wird einfach ergänzt; keine neue DI-Wiring nötig in `genossi_bin/src/lib.rs::RestStateImpl::new()`.
- **Test-Server**: `genossi_rest/src/test_server.rs` bietet `start_test_server`-Utility; In-Memory-DB-Setup für E2E identisch zu v1.1-Phase-7-bis-13-Pattern.
- **Audit-Layer**: **Keine Audit-Macros** in Phase 14 nötig, weil keine Write-Operationen. Phase 15 wird die Audit-Pflicht (AUDT-01) etablieren.

</code_context>

<specifics>
## Specific Ideas

- **`EffectiveDate`-Struct-Derive:** `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`. `Copy` ist OK weil `i32 + time::Date` beide Copy sind. Vereinfacht Test-Assertions (`assert_eq!`).
- **6 Edge-Case-Test-Namen-Konvention:**
  - `test_compute_effective_date_30_juni_is_h1`
  - `test_compute_effective_date_01_juli_is_h2`
  - `test_compute_effective_date_31_dezember_is_h2_next_year`
  - `test_compute_effective_date_01_januar_is_h1`
  - `test_compute_effective_date_schaltjahr_29_februar_is_h1`
  - `test_compute_effective_date_mittiges_datum_15_maerz_is_h1`
- **Endpoint-Path im Sub-Routing:** Wichtig — Reihenfolge in `member::generate_route` so wählen, dass `/transfer-recipients` **vor** `/{id}` kommt, sonst matched Axum den UUID-Path-Parser auf das Wort `"transfer-recipients"` und liefert 400. Vorbild: `/import` (Z. 35) und `/not-reached-by/{job_id}` (Z. 37) liegen ebenfalls vor `/{id}`-Routes — Phase 14 fügt `/transfer-recipients` analog hinzu.
- **`MemberSlimTO`-Reihenfolge der Felder:** so wählen wie die Anzeige im Frontend-Search-Dropdown später läuft (Mitgliedsnummer, Anrede, Titel, Vorname, Nachname). Spart Frontend-Mapping.
- **Service-Method-Return-Type:** `MemberService::list_transfer_recipients` gibt `Arc<[Member]>` zurück (voller Member); REST-Layer mappt zu `MemberSlimTO`. Klare Layer-Trennung: Service kennt Domain (Member), REST formt TO.
- **`exclude_self`-Query-Param-Validierung:** REST-Handler parsed UUID; bei Parse-Error 400 `BadRequest("invalid_exclude_self_uuid")`. Service nimmt `Uuid` direkt.

</specifics>

<deferred>
## Deferred Ideas

- **Search-Query-Parameter `?q=foo`** für Server-seitige Substring-Suche auf Name/Mitgliedsnummer — bei wachsender Genossenschaft (>500 Members) sinnvoll. Aktuell unnötig wegen <200-Members-Realität. Phase 14+x oder spätere Skalierungs-Phase.
- **Pagination für `/api/members/transfer-recipients`** — heute volle Liste, bei großen Genossenschaften limitieren. v2+.
- **`current_shares`-Anzeige im `MemberSlimTO`** für Empfänger-Search-Dropdown ("X Anteile bisher") — Phase 18 darf entscheiden, ob das nötig ist; Slim-TO-Erweiterung wäre dann minimal.
- **`compute_effective_date` als `pub`-Re-Export für REST-Layer-Validierung** — heute `pub(crate)`. Phase 15 PERM-02 (Datums-Validierung am REST-Layer) prüft nach, ob die Pure-Function von außen erreichbar sein muss. Falls ja, dann Re-Export aus `genossi_service_impl/src/lib.rs`. Aktuell defaulted auf Service-internal.
- **DAO-Override-SQL-Migration für `find_by_phase_id`** — heute Default-Impl. Wenn Phase-14-SQL-Override-Pattern sich bewährt, ggf. Folge-Quick zur Migration für Konsistenz. Aktuell pragmatisch unangetastet (PITFALLS-Kat-1 ist nur an `find_by_member_and_phase` gekoppelt).
- **`MembershipAdjustService`-Trait + Impl** — in Phase 14 nur Datei `membership_adjust.rs` mit der Pure-Function. Der Service-Trait + Impl wird in Phase 15-17 schrittweise gefüllt. Phase 14 selbst hat keinen neuen Service.

</deferred>

---

*Phase: 14-dao-domain-foundation*
*Context gathered: 2026-06-04*
