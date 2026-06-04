# Phase 15: Service+REST: Kündigung + Aufstockung - Context

**Gathered:** 2026-06-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 15 etabliert die **ersten Write-Operationen** für v1.2 via neuem `MembershipAdjustService`. Implementiert die zwei einfachsten (Single-Action-) Operationen — Kündigung und Aufstockung — und legt damit die Konventions-Foundation für Phase 16-17 fest: ADMIN_PRIVILEGE-Permission-Funnel, Server-Layer-Datum-Validierung, `audited_*!`-Macro-Compliance (AUDT-01), und Service-Layer-Konsumption der Phase-14-Pure-Function.

**In scope:**
- **Neues Trait `MembershipAdjustService`** in `genossi_service/src/membership_adjust.rs` (neue Datei) mit zwei Methoden: `cancel_membership` und `increase_shares`. Inkrementell wachsend — Phase 16 fügt `partial_repayment` hinzu, Phase 17 fügt `transfer_shares` hinzu (D-15-13).
- **Impl `MembershipAdjustServiceImpl<Deps>`** in `genossi_service_impl/src/membership_adjust.rs` (existing file, erweitern). Dependencies via `gen_service_impl!`: `member_dao`, `member_action_dao`, `audit_log_dao`, `permission_service`, `uuid_service`, `transaction_dao`. CANC-01/03/04/05 + UPGD-01..04 + PERM-01/02 + AUDT-01.
- **`MembershipAdjustService::cancel_membership(member_id, willensbekundung_date, context, tx) -> Result<(MemberAction, Member), ServiceError>`** — erzeugt `MemberAction::Austritt` mit `effective_date = compute_effective_date(willensbekundung_date).effective_date`, `shares_change = 0`, `date = willensbekundung_date`. `recalc_dates` setzt `Member.exit_date` automatisch. Process-String `"member-adjust.cancel"`.
- **`MembershipAdjustService::increase_shares(member_id, shares, willensbekundung_date, context, tx) -> Result<(MemberAction, Member), ServiceError>`** — erzeugt `MemberAction::Aufstockung` mit `shares_change = +shares`, `transfer_member_id = None`, `date = willensbekundung_date`, `effective_date = None` (sofort wirksam, kein H1/H2) **und** erhöht `Member.current_shares` um `shares` atomar in einer Tx. Process-String `"member-adjust.upgrade"`. Blockt gekündigte Member (UPGD-04).
- **`recalc_dates` Refactor zu Free-Function** in `genossi_service_impl/src/member_action.rs` (D-15-04). Neue Signatur: `pub(crate) async fn recalc_dates<Md: MemberDao, Mad: MemberActionDao>(member_dao: &Md, member_action_dao: &Mad, member_id: Uuid, tx) -> Result<(), ServiceError>`. Aufrufer: bestehende `MemberActionServiceImpl::recalc_dates` (delegiert jetzt an die Free-Function) + neue `MembershipAdjustServiceImpl::cancel_membership`.
- **Pure-Function `validate_willensbekundung_date(date, today) -> Result<(), Vec<ValidationFailureItem>>`** in `genossi_service_impl/src/membership_adjust.rs` (D-15-07). Bounds: `date.year() == today.year() || date.year() == today.year() + 1`. Edge-Case-Tests: Vorjahr-Datum, Übernächstes-Jahr-Datum, today=31.12., Schaltjahr.
- **REST-Endpoints** in `genossi_rest/src/membership_adjust.rs` (neue Datei) oder `genossi_rest/src/member.rs` (erweitern — Entscheidung Planner): `POST /api/members/{id}/cancel` und `POST /api/members/{id}/increase-shares`. Routes registriert in `member::generate_route` (`genossi_rest/src/member.rs:28`) — Sub-Routes MÜSSEN **vor** `/{id}`-catch-all deklariert werden (Phase-14-Lesson D-14-08).
- **Request-DTOs** in `genossi_rest_types/src/lib.rs`: `CancelMembershipRequestTO { willensbekundung_date: Date }` und `IncreaseSharesRequestTO { willensbekundung_date: Date, shares: i64 }`. ISO8601-Date-Serde (existing).
- **Response-DTO**: `{ action: MemberActionTO, member: MemberTO }` als anonymes Response-Struct oder neuer `MembershipAdjustResponseTO`-Typ (Planner-Discretion).
- **DI-Wiring** in `genossi_bin/src/lib.rs::RestStateImpl::new()`: neuer `membership_adjust_service: Arc<MembershipAdjustServiceImpl<...>>`. Trait-Methode auf dem RestState (analog `member_action_service`-Pattern in v1.1).
- **9 E2E-Tests** (5 für Kündigung + 4 für Aufstockung, siehe Roadmap-Success-Criteria) + **2 Edge-Case-Tests für Datum-Bounds** (Vorjahr, Übernächstes Jahr).
- **Unit-Tests** für `validate_willensbekundung_date` (mindestens 6 Edge-Cases analog `compute_effective_date`).
- **Unit-Tests** für `cancel_membership` und `increase_shares` mit MockDaos (Happy-Path, Permission-Denied, Already-Cancelled, Validation-Error).

**Out of scope (deferred / explizit nicht):**
- **Keine `MemberAction::Verkauf` bei Kündigung** (CANC-05) — v1.1-PaidOut-Cascade übernimmt das beim späteren Ausbezahlt-Toggle.
- **Kein `RepaymentEntry`-Insert** bei Kündigung — Auto-Befüllung in `open_repayment_phase` (existing v1.1-Logik) picked den Member über `exit_date in fiscal_year` auf.
- **Keine direkte `Member.exit_date`-Mutation** — alleinig via `recalc_dates`-Hook (CANC-04, Konsistenz mit v1.1-Pattern).
- **Kein Voll-Übertrag / Teil-Übertrag** (Phase 17).
- **Keine Teil-Rückgabe** (Phase 16).
- **Keine UI** — Frontend ist Phase 18.
- **Keine CANC-06-Vorschau** (Frontend-Concern, Phase 18).
- **Keine Audit-Trail-Erweiterung** über AUDT-01 hinaus (AUDT-02 ist Phase-17-Konzern für gemeinsame Process-Strings beim Übertrag-Pair).

</domain>

<decisions>
## Implementation Decisions

### Service-Komposition

- **D-15-01:** **Direkter DAO-Call + `audited_create!`-Macro** in `MembershipAdjustService`-Methoden statt Delegation an `MemberActionService::create()`. **Why:** `MemberActionService::create()` checkt `MANAGE_MEMBERS_PRIVILEGE` (`member_action.rs:304`), aber PERM-01 verlangt `ADMIN_PRIVILEGE` für alle v1.2-Ops. Außerdem ermöglicht der direkte Call einen eigenen Audit-Process-String (`member-adjust.cancel` / `member-adjust.upgrade`) — Foundation für AUDT-02 in Phase 17 (gemeinsamer Process-String beim Übertrag-Pair). **How to apply:** Service-Methode beginnt mit `permission_service.check_permission(ADMIN_PRIVILEGE, context)`, dann `audited_create!(self, self.member_action_dao, &new_action_entity, "member-adjust.cancel", &user_id, tx)`.

- **D-15-02:** **Audit-Process-Strings: `"member-adjust.cancel"` und `"member-adjust.upgrade"`** als const-strings im `membership_adjust.rs`-Modul. **Why:** Konsistent mit `MEMBER_ACTION_SERVICE_PROCESS = "member-action"`-Pattern in `member_action.rs`. Klare v1.2-Domain-Namespace im Audit-Log; Filter via `process = "member-adjust.*"` möglich. Foundation für AUDT-02 (Übertrag-Pair shared "member-adjust.transfer"). **How to apply:** `const CANCEL_PROCESS: &str = "member-adjust.cancel";` und `const UPGRADE_PROCESS: &str = "member-adjust.upgrade";` am Top von `membership_adjust.rs`.

- **D-15-03:** **`Member.current_shares`-Update via generischem `MemberDao::update()` + `audited_update!`-Macro** (NICHT via targeted `update_current_shares`-DAO-Methode). **Why:** AUDT-01 verlangt `audited_*!`-Macro-Compliance; das Macro ruft hardkodiert `$dao.update($new_entity, $tx)` auf (`audit_macros.rs:42`). Eine targeted `update_current_shares` wäre **nicht** macro-auditierbar und würde den Grep-Gate verletzen ("0 direkte DAO-create/update-Calls außerhalb der Macros"). Der Macro-Diff loggt automatisch nur das geänderte Feld (`current_shares`) — kein Datenmüll. **How to apply:** Im Service: `let mut updated_member = member.clone(); updated_member.current_shares += shares; updated_member.version = uuid_service.new_v4().await; audited_update!(self, self.member_dao, member.id, &updated_member.into(), UPGRADE_PROCESS, &user_id, tx)`.

- **D-15-04:** **`recalc_dates` Refactor zu Free-Function** in `genossi_service_impl/src/member_action.rs`. Neue Signatur: `pub(crate) async fn recalc_dates<...>(member_dao, member_action_dao, member_id, tx) -> Result<(), ServiceError>`. **Why:** Beide Services (MemberActionServiceImpl + MembershipAdjustServiceImpl) brauchen die Hook nach Action-Create für CANC-04. Free-Function macht Dependencies explizit, vermeidet doppelte Maintenance und behält Pure-Helper-Konvention (analog `compute_dates` Z.155). **How to apply:** Bestehende `MemberActionServiceImpl::recalc_dates` (Z. 180) wird zu Wrapper, der die Free-Function aufruft: `recalc_dates(&self.member_dao, &self.member_action_dao, member_id, tx).await`. Trait-Constraints auf Generic-Bounds (`Md: MemberDao<Transaction=...>`).

### Datum-Bounds-Strategie

- **D-15-05:** **Pure-Function `validate_willensbekundung_date(date: Date, today: Date) -> Result<(), Vec<ValidationFailureItem>>`** in `genossi_service_impl/src/membership_adjust.rs`. **Why:** Pure-Function-Konvention konsistent mit `compute_effective_date` (D-14-04..07) — Edge-Cases als deterministische Unit-Tests, Service-Methode ruft sie am Eintritt auf. Testbar ohne Time-Mock. **How to apply:** Function gibt `Vec<ValidationFailureItem>` zurück (leer = valid). Service wraped `if !errors.is_empty() { return Err(ServiceError::ValidationError(errors)) }`.

- **D-15-06:** **Bounds = Kalender-Jahr-basiert** — Datum ist valid wenn `date.year() == today.year() || date.year() == today.year() + 1`. **Why:** Intuitiv für Vorstand ("'25er oder '26er Willensbekundung"). Erlaubt Backdating innerhalb des laufenden Jahres und Forward bis Ende nächstes Jahr (für H2-Wirksamkeit). Match zur PROJECT.md-Constraint "Datepicker default today(), nur offenes GJ + nächstes GJ erlaubt" — fiscal-year und kalender-year sind in der Genossenschaft synchron (GJ = Kalenderjahr). **How to apply:** Inline-Vergleich `date.year() == today.year() || date.year() == today.year() + 1`. Edge-Case: today=31.12.2026 erlaubt [2026-01-01, 2027-12-31]. Edge-Case: today=01.01.2027 erlaubt [2027-01-01, 2028-12-31] — Backdating in 2026 dann nicht mehr möglich (Vorstand muss zeitnah eintragen).

- **D-15-07:** **`today` als Parameter (nicht intern via `OffsetDateTime::now_utc()`)** in der Pure-Function. **Why:** Testbar als deterministische Pure-Function. Service-Caller holt `let today = time::OffsetDateTime::now_utc().date()` (konsistent mit Codebase-Pattern, siehe `audit_macros.rs:97`, `attendance.rs:175`, `application.rs:195`). Timezone-UTC ist die Codebase-Convention; deutsche Vorstands-Zeitzone hat keinen Mismatch für Tagesgenauigkeit relevant (Validation greift erst bei kompletten Jahres-Boundary-Verletzungen). **How to apply:** Signatur `fn validate_willensbekundung_date(date: Date, today: Date) -> Result<(), Vec<ValidationFailureItem>>`. Service-Methode: `let today = time::OffsetDateTime::now_utc().date(); validate_willensbekundung_date(req.willensbekundung_date, today)?;`.

- **D-15-08:** **Error-Shape: `ServiceError::ValidationError(Vec<ValidationFailureItem>)`** mit `field: "willensbekundung_date"`, `message: "must be in current fiscal year (YYYY) or next fiscal year (YYYY+1)"`. **Why:** Konsistent mit `validate_action`-Pattern (`member_action.rs:309`). Mapped via ServiceError → RestError → HTTP 400 (existing `RestError::BadRequest`-Mapping). Frontend (Phase 18) kann das `field`-Pointing für Inline-Form-Fehler nutzen. i18n-Ready. **How to apply:** `vec![ValidationFailureItem { field: Arc::from("willensbekundung_date"), message: Arc::from(format!("must be in fiscal year {} or {}", today.year(), today.year() + 1)) }]`.

### REST-Endpoint-Shape

- **D-15-09:** **Separate Sub-Routes pro Operation**: `POST /api/members/{id}/cancel` und `POST /api/members/{id}/increase-shares`. **Why:** Konsistent mit existing `/api/members/{id}/...`-Pattern (z.B. `/api/members/{id}/actions` in v1.1). Klares OpenAPI-Schema pro Operation. Frontend (Phase 18) macht spezifische Requests. Phase 16 ergänzt `/partial-repayment`, Phase 17 ergänzt `/transfer-shares`. **How to apply:** Routes in `member::generate_route` (`genossi_rest/src/member.rs:28`) — Sub-Routes MÜSSEN **vor** `/{id}`-catch-all deklariert werden (Phase-14-Lesson D-14-08). Handler-Funktionen in neuer Datei `genossi_rest/src/membership_adjust.rs` (Planner-Discretion: oder in `member.rs` inline, falls < 200 LOC).

- **D-15-10:** **Request-DTOs in `genossi_rest_types/src/lib.rs`**: `CancelMembershipRequestTO { willensbekundung_date: Date }` und `IncreaseSharesRequestTO { willensbekundung_date: Date, shares: i64 }`. **Why:** Konsistent mit `MemberSlimTO`-Pattern (D-14-12). Klare OpenAPI-Schemas + ISO8601-Date-Serde. Frontend kann TOs als Builder verwenden. **How to apply:** `#[derive(Debug, Serialize, Deserialize, ToSchema)]` + `#[serde(with = "iso8601_date")]` für Date-Feld (existing Pattern, siehe `genossi_rest_types/src/lib.rs:10`).

- **D-15-11:** **Response-Body: `{ action: MemberActionTO, member: MemberTO }`** als anonymes Struct oder benannt `MembershipAdjustResponseTO`. **Why:** Frontend (Phase 18) braucht *sofortiges* Update für Re-Render der Member-Detail-Page (exit_date / current_shares neu). Single-Round-Trip statt POST→GET-Refresh. **How to apply:** REST-Handler bauen `Json(serde_json::json!({ "action": MemberActionTO::from(&action), "member": MemberTO::from(&member) }))` oder via benanntem Struct in `genossi_rest_types`. Planner-Discretion.

- **D-15-12:** **HTTP-Status-Codes:** 200 (Success), 400 (Bad Request — Validation, Already-Cancelled, UUID-Parse), 401 (No Auth), 403 (No Admin Privilege), 404 (Member not found). **Why:** Konsistent mit existing `RestError`-Mapping (`genossi_rest/src/error.rs`). **How to apply:** Already-Cancelled wird als `ServiceError::Conflict` → HTTP 409 OR als `ServiceError::ValidationError` → HTTP 400 (Planner-Discretion; Roadmap nennt "409 für Already-Cancelled" → `Conflict` ist die natürliche Wahl).

### MembershipAdjustService-Trait-Shape

- **D-15-13:** **Trait wächst inkrementell**: Phase 15 definiert `MembershipAdjustService` mit nur **2 Methoden** (`cancel_membership`, `increase_shares`). Phase 16 ergänzt `partial_repayment`, Phase 17 ergänzt `transfer_shares`. **Why:** Mock-Burden minimal (nur 2 Methoden zu stubben). Jede Phase macht *atomare* Trait-Änderung. Konsistent mit v1.1-Pattern (MemberActionService wuchs auch über mehrere Phasen). Vermeidet `todo!()`-Panics oder leere `ServiceError::InternalError("not_yet_implemented")`-Returns. **How to apply:** Trait-Definition in Phase 15 enthält *nur* `cancel_membership` + `increase_shares`. Phase 16-Plan dokumentiert die Trait-Erweiterung als bewussten Schritt.

- **D-15-14:** **Trait-Datei: `genossi_service/src/membership_adjust.rs`** (neue Datei). Impl: `genossi_service_impl/src/membership_adjust.rs` (existing — Phase 14 hat die Datei mit der Pure-Function + EffectiveDate-Struct angelegt). **Why:** Layer-Trennung konsistent mit Codebase-Konvention (siehe `genossi_service/src/member_action.rs` + `genossi_service_impl/src/member_action.rs`). Phase 14 hat `genossi_service_impl/src/membership_adjust.rs` bereits angelegt — Phase 15 fügt die Service-Impl an. **How to apply:** Re-Export in `genossi_service/src/lib.rs` nach existing Pattern (kein glob, explizit). Mock-Generation via `#[automock]` analog `MemberActionService`.

- **D-15-15:** **Method-Signaturen mit granularen Parametern**: `cancel_membership(member_id: Uuid, willensbekundung_date: Date, context, tx) -> Result<(MemberAction, Member), ServiceError>` und `increase_shares(member_id: Uuid, shares: i64, willensbekundung_date: Date, context, tx) -> Result<(MemberAction, Member), ServiceError>`. **Why:** Return-Tuple matches Response-Body-Shape (D-15-11). Konsistent mit `MemberActionService::create`-Pattern (`member_action.rs:289`). Kein DTO-Wrapping nötig — domain-Werte direkt. **How to apply:** Trait-Methode als `async fn` mit `Authentication<Self::Context>` für context und `Option<Self::Transaction>` für tx (genossi-Standard, siehe `member_action.rs:289-294`).

- **D-15-16:** **DI-Wiring**: neuer `membership_adjust_service: Arc<MembershipAdjustServiceImpl<...>>` in `RestStateImpl`. Dependencies via `gen_service_impl!`: `member_dao`, `member_action_dao`, `audit_log_dao`, `permission_service`, `uuid_service`, `transaction_dao`. **Why:** Konsistent mit Phase 7-13-Pattern. Single-Responsibility — neuer Service-Slot statt Erweiterung bestehender Services. **How to apply:** Trait-Methode auf RestState (`fn membership_adjust_service() -> &Arc<...>`) + Init in `RestStateImpl::new()` (siehe `genossi_bin/src/lib.rs`).

### Claude's Discretion

- **Handler-Datei-Placement**: `genossi_rest/src/membership_adjust.rs` (neue Datei mit `pub fn extend_member_routes(router) -> Router` Pattern) **oder** Erweiterung von `genossi_rest/src/member.rs` mit `cancel_membership` / `increase_shares`-Handler-Funktionen. Planner darf je nach Datei-Größe entscheiden — falls `member.rs` > 600 LOC wird, dann separate Datei.
- **Response-DTO-Naming**: anonymes JSON-Object `{"action": ..., "member": ...}` oder benannter Typ `MembershipAdjustResponseTO` in `genossi_rest_types`. Bevorzugt benannt für OpenAPI-Schema, aber für 2 Endpoints ist anonymous ggf. pragmatisch.
- **Already-Cancelled-Detection-Heuristik**: `member.exit_date IS NOT NULL` (komplettes Bild) oder `actions.find(ActionType::Austritt)` (Action-First). Beide funktionieren, weil `recalc_dates` `exit_date` aus den Actions ableitet. Planner-Discretion. HTTP 409 (`ServiceError::Conflict`) als Response.
- **`Member.current_shares` ist non-negative Invariant**: bei Aufstockung kann nicht negativ werden (UPGD-01 sagt "Anzahl `n`" implizit positiv). Service darf eine Validation hinzufügen `shares > 0` (HTTP 400 bei `shares <= 0`).
- **Trait-Methoden-Sync zwischen genossi_service und genossi_service_impl**: standardmäßig per `#[async_trait]`. Mock via `#[automock]` für Service-Unit-Tests.
- **Reihenfolge der Plan-Dateien**: Empfehlung — `plan_01_trait_and_validate_date.md` (Trait-Definition + Pure-Function), `plan_02_cancel_membership.md` (cancel_membership-Impl + Tests), `plan_03_increase_shares.md` (increase_shares-Impl + Tests), `plan_04_rest_endpoints_and_e2e.md` (REST-Layer + E2E). Planner darf zusammenfassen oder weiter aufteilen.
- **`recalc_dates` Free-Function-Refactor**: Planner darf entscheiden, ob die Refactor in `plan_01` oder als eigener Plan-Schritt (`plan_00_refactor_recalc_dates.md`) liegt. Wichtig: keine behavior-change in `MemberActionService` (rein Compile-Time-Refactor).
- **OpenAPI-Annotationen**: Utoipa `#[utoipa::path(...)]` mit 200, 400, 401, 403, 404, 409 (Conflict bei Already-Cancelled).

### Folded Todos

None — keine offenen Todos für Phase 15 gefunden.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Projekt-Foundation
- `.planning/PROJECT.md` — v1.2-Milestone, Constraints (Layered DAO/Service/REST, Audit-Pflicht, ADMIN_PRIVILEGE für v1.2-Ops, today()-Default für Datepicker).
- `.planning/REQUIREMENTS.md` §CANC-01, §CANC-03, §CANC-04, §CANC-05, §UPGD-01..04, §PERM-01, §PERM-02, §AUDT-01 — Phase-15-Requirements.
- `.planning/ROADMAP.md` §Phase 15 — Phase-Goal, Success-Criteria (5 E2E für Kündigung, 4 E2E für Aufstockung, 2 Edge-Case-Datum-Bounds).
- `.planning/ROADMAP.md` §Constraints (Phase 15+16+17) — `audited_*!`-Macros-Grep-Gate.
- `.planning/ROADMAP.md` §Discuss-Phase-Decisions §Phase 15 — ActionType-Persistenz = TEXT (verifiziert in `migrations/sqlite/20260331000002_create_member_actions_table.sql:4`); Datum-Bounds-Implementierung = Pure-Function + Service-Aufruf (D-15-05..08).

### Domain & Architektur
- `.planning/notes/membership-adjust-design.md` — Master-Design-Doc, vier Operationen, H1/H2-Logik.
- `.planning/research/ARCHITECTURE.md` §1 (Placement-Decision: `genossi_service_impl/src/membership_adjust.rs`), §7 (Permission-Funnel `ADMIN_PRIVILEGE`).
- `.planning/research/PITFALLS.md` §Kat 4 (H1/H2-Edge-Cases — Foundation für Datum-Bounds), §Kat 5/6 (Audit-Macro-Wiring).

### Phase-14-Decisions (carried forward)
- `.planning/phases/14-dao-domain-foundation/14-CONTEXT.md` — vollständig: alle Phase-14-Decisions (`compute_effective_date` Pure-Function-Konvention, `pub(crate)`-Visibility, ADMIN_PRIVILEGE-Permission-Funnel, `Arc<[T]>`-Returns, ISO8601-Date-Serde).
- D-14-02: Modul-Placement `genossi_service_impl/src/membership_adjust.rs` (Phase 14 hat die Datei mit Pure-Function angelegt; Phase 15 erweitert).
- D-14-11: `ADMIN_PRIVILEGE` als v1.2-Permission-Standard.

### Vorbild-Phasen (Pattern-Quelle)
- `.planning/milestones/v1.1-phases/07-repaymentphase-backend-foundation/07-CONTEXT.md` — Layered Service-Foundation-Pattern (Trait + Impl + DI-Wiring).
- `.planning/milestones/v1.1-phases/09-paid-out-cascade-toggle/` — Atomare Multi-DAO-Operation in einer Tx (Vorbild für `increase_shares` mit zwei audited-Calls in einer Tx, falls vorhanden — Planner prüft).
- `.planning/milestones/v1.1-phases/13-repaymentletter-bulk-anschreiben-f-r-nicht-email-mitglieder/13-CONTEXT.md` — Permission-Funnel-Pattern, E2E-Test-Setup-Vorbild.

### Code-Referenzen (Files, die berührt werden)
- `genossi_service/src/membership_adjust.rs` — **neue Datei** (Trait-Definition für MembershipAdjustService).
- `genossi_service/src/lib.rs` — neue Modul-Registrierung `pub mod membership_adjust;`.
- `genossi_service_impl/src/membership_adjust.rs` — bestehende Datei (Phase 14: Pure-Function + EffectiveDate). **Erweitern:** `MembershipAdjustServiceImpl<Deps>`, `validate_willensbekundung_date`-Pure-Function, Service-Method-Impl für cancel + upgrade.
- `genossi_service_impl/src/member_action.rs:155-177` (`compute_dates`) — Vorbild für Pure-Function-Pattern.
- `genossi_service_impl/src/member_action.rs:180-203` (`recalc_dates`) — **Refactor** zu Free-Function `pub(crate) async fn recalc_dates<...>(...)`. Bestehende Methode wird zu Wrapper.
- `genossi_service_impl/src/member_action.rs:289-356` (`create`) — Vorbild für `audited_create!`-Macro-Anwendung + Member-Existence-Check + recalc_dates-Hook.
- `genossi_service_impl/src/member_action.rs:309` (`validate_action`) — Vorbild für `Vec<ValidationFailureItem>`-Pattern.
- `genossi_service_impl/src/audit_macros.rs` — `audited_create!` + `audited_update!`-Macro-Definitionen (kein Anpassungsbedarf).
- `genossi_service_impl/src/macros.rs` — `gen_service_impl!`-Macro (Vorbild für DI-Wiring).
- `genossi_dao/src/member.rs:111` (`update`) — generischer Update-Pfad für Member-Entity (für audited_update! der current_shares-Mutation).
- `genossi_dao/src/member_action.rs:9` (`ActionType`-Enum) — Varianten `Austritt`, `Aufstockung` existieren bereits.
- `genossi_dao/src/member_action.rs:56` (`MemberActionEntity`) — Entity-Struct für `MemberAction::Austritt` / `Aufstockung`-Erzeugung.
- `genossi_service/src/permission.rs:28` (`ADMIN_PRIVILEGE = "admin"`) — Permission-Konstante.
- `genossi_service/ServiceError` — Variants `ValidationError`, `Conflict`, `EntityNotFound`, `PermissionDenied`.
- `genossi_rest/src/member.rs:28-74` (`generate_route` + `get_all_members`-Handler) — Vorbild für REST-Handler-Pattern. **Neue Sub-Routes** `/cancel` und `/increase-shares` MÜSSEN vor `/{id}` registriert werden (D-14-08-Lesson).
- `genossi_rest/src/membership_adjust.rs` — **neue Datei** (optional, Planner-Discretion) für Handler-Funktionen.
- `genossi_rest/src/lib.rs:582` — bestehender Mount `.nest("/api/members", member::generate_route())` ändert sich nicht.
- `genossi_rest_types/src/lib.rs` — neue Request-DTOs (`CancelMembershipRequestTO`, `IncreaseSharesRequestTO`); optional `MembershipAdjustResponseTO`.
- `genossi_rest_types/src/lib.rs:10` — bestehende ISO8601-Date-Serde (`iso8601_date`-Modul) wiederverwenden.
- `genossi_bin/src/lib.rs::RestStateImpl::new()` — neue `membership_adjust_service`-DI-Wiring + Trait-Methode auf RestState.
- `genossi_bin/tests/` — neue E2E-Tests (analog v1.1-Phase-13-Pattern).
- `migrations/sqlite/20260331000002_create_member_actions_table.sql:4` — ActionType TEXT-Persistenz (verifiziert).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`compute_effective_date` Pure-Function** (`genossi_service_impl/src/membership_adjust.rs:21`): liefert `EffectiveDate { fiscal_year, effective_date }` für H1/H2-Stichtag. Phase 15 ruft sie in `cancel_membership` auf — Result `effective_date` füllt `MemberAction::Austritt.effective_date`.
- **`audited_create!` und `audited_update!` Macros** (`genossi_service_impl/src/audit_macros.rs`): erwarten `self` mit `audit_log_dao` + `uuid_service`-Feldern. Macro ruft hardkodiert `$dao.create(...)` bzw. `$dao.update(...)` auf — daher generisches `MemberDao::update` für current_shares-Mutation (nicht targeted).
- **`MemberActionService::create`** (`genossi_service_impl/src/member_action.rs:289-356`): Vorbild für die exakte Sequence — `use_transaction → current_user_id → check_permission → validate → member_existence_check → audited_create! → recalc_dates → recalc_migrated → commit`. Phase 15 spiegelt das Pattern, ersetzt `MANAGE_MEMBERS_PRIVILEGE` durch `ADMIN_PRIVILEGE` und `MEMBER_ACTION_SERVICE_PROCESS` durch `member-adjust.cancel`/`upgrade`.
- **`recalc_dates`** (`genossi_service_impl/src/member_action.rs:180-203`): Hook, der `Member.exit_date` aus den MemberActions neu berechnet (via `compute_dates` Z.155). **Refactor** zu Free-Function in Phase 15 (D-15-04), so dass `MembershipAdjustService::cancel_membership` ihn aufrufen kann.
- **`recalc_migrated`** (`genossi_service_impl/src/member_action.rs:205-229`): zweiter Hook, der `Member.migrated`-Flag neu berechnet. **Auch refactor zu Free-Function** (parallel zu `recalc_dates`) — wird ebenfalls von `cancel_membership` und `increase_shares` benötigt? **Planner-Check:** für Austritt: ja (analog `recalc_dates`); für Aufstockung: zu prüfen (Eintritts-Action-Existenz unverändert).
- **`validate_action` Pattern** (`genossi_service_impl/src/member_action.rs:309`): sammelt `Vec<ValidationFailureItem>`. Vorbild für `validate_willensbekundung_date`.
- **ISO8601-Date-Serde** (`genossi_rest_types/src/lib.rs:10`): wiederverwendbar für `CancelMembershipRequestTO` und `IncreaseSharesRequestTO`.
- **`MemberTO` + `MemberActionTO`** (`genossi_rest_types/src/lib.rs`): bestehende TOs für Response-Body-Komposition.
- **`gen_service_impl!`** (`genossi_service_impl/src/macros.rs`): DI-Wiring-Macro. Vorbild für `MembershipAdjustServiceImpl<Deps>`.
- **`error_handler`** (`genossi_rest/src/error.rs`): REST-Wrapper für `Result<Response, RestError>` → HTTP-Response.

### Established Patterns
- **`audited_*!`-Macro-Compliance**: AUDT-01 Grep-Gate erwartet 0 direkte DAO-create/update-Calls außerhalb der Macros. Phase 15 etabliert das Pattern für v1.2 (es war schon in v1.1 für Member/Application etabliert).
- **Permission-Funnel am Service-Methoden-Eintritt**: `permission_service.check_permission(ADMIN_PRIVILEGE, context).await?` (siehe `repayment_phase.rs:107`, `member_action.rs:304`).
- **Transaction-Lifecycle**: `let tx = self.transaction_dao.use_transaction(tx).await?` am Start, `self.transaction_dao.commit(tx).await?` am Ende. Alle DAO-Calls dazwischen mit `tx.clone()`.
- **Soft-Delete-Filter `deleted IS NULL`**: Genossi-übergreifend.
- **Pure-Function-Konvention `pub(crate)` + `#[cfg(test)] mod tests`**: für Helpers im Service-Impl-Crate.
- **`Vec<ValidationFailureItem>`**: für Field-Level-Validation-Errors.
- **`ServiceError → RestError → HTTP-Status`-Mapping**: ValidationError → 400, Conflict → 409, EntityNotFound → 404, PermissionDenied → 403, Unauthorized → 401.

### Integration Points
- **REST-Mount**: Neue Sub-Routes `/cancel` und `/increase-shares` leben **innerhalb** `member::generate_route` (`genossi_rest/src/member.rs:28`). `lib.rs:582` ändert sich nicht. Reihenfolge — **vor** `/{id}`-Routes deklarieren (D-14-08-Lesson).
- **Service-Layer-Wiring**: Neuer `membership_adjust_service`-Slot in `RestStateImpl`. Trait-Methode auf RestState (`fn membership_adjust_service(&self) -> &Arc<...>`).
- **OpenAPI**: Utoipa-Annotationen analog v1.1-Phase-13 (siehe `13-CONTEXT.md` für Vorbild). Schemas für `CancelMembershipRequestTO`, `IncreaseSharesRequestTO`, `MembershipAdjustResponseTO` (oder anonymes JSON).
- **Audit-Layer**: `audit_log_dao` muss in `MembershipAdjustServiceDeps` und `RestStateImpl::new()` verdrahtet werden (existing in v1.1).
- **Test-Server**: `genossi_rest/src/test_server.rs::start_test_server` für E2E-Tests; In-Memory-DB-Setup wie v1.1-Phase-7-13.

</code_context>

<specifics>
## Specific Ideas

- **Audit-Process-String-Naming**: `member-adjust.cancel`, `member-adjust.upgrade`. Phase 16 wird `member-adjust.partial-repayment`; Phase 17 `member-adjust.transfer` (shared für Pair, AUDT-02). Konsistente Dot-Hierarchy für Filter `WHERE process LIKE 'member-adjust.%'`.
- **Already-Cancelled-Test-Setup**: vor dem POST /cancel zwei Calls — erst cancel (200), dann cancel (409). HTTP-Status 409 = `ServiceError::Conflict`.
- **Edge-Case-Tests für Datum-Bounds**:
  - `test_validate_willensbekundung_vorjahr_invalid` (date=2025-12-31, today=2026-03-15 → invalid)
  - `test_validate_willensbekundung_aktuelles_jahr_valid` (date=2026-06-15, today=2026-03-15 → valid)
  - `test_validate_willensbekundung_naechstes_jahr_valid` (date=2027-06-15, today=2026-03-15 → valid)
  - `test_validate_willensbekundung_uebernaechstes_jahr_invalid` (date=2028-01-01, today=2026-03-15 → invalid)
  - `test_validate_willensbekundung_today_31_dezember_naechstes_jahr_valid` (date=2027-12-31, today=2026-12-31 → valid)
  - `test_validate_willensbekundung_schaltjahr_29_februar_valid` (date=2024-02-29, today=2024-01-15 → valid)
- **Trait-Method-Doc-Comments**: Deutsch im `///`-Doc-Kommentar OK (Verbands-Kontext); Code-Identifier englisch (`willensbekundung_date`, `effective_date`, `current_shares`).
- **`Member.current_shares`-Mutation in Aufstockung**: Member laden mit `find_by_id` (FAIL wenn None → EntityNotFound), `updated.current_shares += shares`, `updated.version = uuid_service.new_v4()` (Optimistic-Locking-Bump), dann `audited_update!`. Macro liest alte Version aus der DB und vergleicht — Conflict bei Version-Mismatch.
- **`MemberAction::Aufstockung.effective_date = None`** (UPGD-02: sofort wirksam, kein H1/H2). Nicht alle MemberActions haben `effective_date` gesetzt — `Option<Date>`-Feld ist genau dafür da.
- **Test-Naming-Konvention** (analog Phase 14):
  - `test_cancel_membership_happy_path_h1`
  - `test_cancel_membership_happy_path_h2`
  - `test_cancel_membership_permission_denied`
  - `test_cancel_membership_already_cancelled`
  - `test_cancel_membership_audit_chain_verify`
  - `test_increase_shares_happy_path`
  - `test_increase_shares_cancelled_member_blocked`
  - `test_increase_shares_permission_denied`
  - `test_increase_shares_audit_chain_verify`

</specifics>

<deferred>
## Deferred Ideas

- **`MembershipAdjustService::partial_repayment`** — Phase 16. Trait wächst inkrementell.
- **`MembershipAdjustService::transfer_shares`** + AUDT-02 (shared Process-String für Übertrag-Pair) — Phase 17.
- **`compute_effective_date` als `pub`-Re-Export** für REST-Layer-Vorschau (CANC-06) — Phase 18 darf evaluieren, ob Frontend die Pure-Function via WASM nutzt. Aktuell `pub(crate)`.
- **CANC-06 Vorschau-Confirm-Dialog**: zeigt willensbekundung_date, berechneten Stichtag, prognostizierte Ziel-Auszahlungsphase, Wirkungs-Timeline — Phase 18.
- **`recalc_migrated` Free-Function-Refactor**: falls Phase 15 nur `recalc_dates` braucht, kann `recalc_migrated` als private Methode auf MemberActionServiceImpl bleiben. Planner prüft.
- **Targeted `update_current_shares` DAO-Methode** für SQL-Effizienz — heute über generisches `MemberDao::update` aus AUDT-01-Compliance-Gründen. Bei Performance-Problemen (>200 Members oder Bulk-Operations) kann Phase v2 dahin refaktorieren — dann müssten die `audited_*!`-Macros um targeted-Methoden-Support erweitert werden.
- **Audit-Macro-Erweiterung um targeted-Methoden** (`audited_update_with!`) — heute nicht nötig; Bedarf könnte in Phase 16 entstehen, falls dort viele Partial-Updates auftreten.
- **`shares <= 0` Validation für `increase_shares`**: Service darf einen `shares > 0`-Check hinzufügen (HTTP 400 bei `shares <= 0`). Aktuell als Planner-Discretion notiert; Roadmap-UPGD-Reqs erwähnen es nicht explizit.

</deferred>

---

*Phase: 15-service-rest-kuendigung-aufstockung*
*Context gathered: 2026-06-04*
