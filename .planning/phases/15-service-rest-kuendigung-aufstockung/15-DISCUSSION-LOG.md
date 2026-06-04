# Phase 15: Service+REST: Kündigung + Aufstockung - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-04
**Phase:** 15-service-rest-kuendigung-aufstockung
**Areas discussed:** Service-Komposition, Datum-Bounds-Strategie, REST-Endpoint-Shape, MembershipAdjustService-Trait-Shape

---

## Gray-Area-Auswahl

| Option | Description | Selected |
|--------|-------------|----------|
| Service-Komposition | MemberActionService::create() wiederverwenden vs. direkter DAO-Call mit eigenem Permission-Check | ✓ |
| Datum-Bounds-Strategie | Pure-Function vs. Service-internal-Inline-Check vs. zusätzliche REST-Validierung | ✓ |
| REST-Endpoint-Shape | Separate Sub-Routes vs. unifizierter Endpoint vs. Top-Level-Route | ✓ |
| MembershipAdjustService-Trait-Shape | Trait mit allen 4 Methoden inkl. todo!() vs. inkrementell wachsend | ✓ |

**User's choice:** Alle 4 ausgewählt (multiSelect).

---

## Service-Komposition

### Q1: Wie soll MembershipAdjustService::cancel_membership / increase_shares die MemberAction erzeugen?

| Option | Description | Selected |
|--------|-------------|----------|
| Direkter DAO-Call + audited_create! | Eigener Permission-Check (ADMIN_PRIVILEGE), eigener Process-String "member-adjust.cancel"/"upgrade", recalc_dates zu pub(crate)-Helper umbauen | ✓ |
| MemberActionService::create() wiederverwenden | recalc_dates kommt gratis, aber Permission ist MANAGE_MEMBERS_PRIVILEGE statt ADMIN_PRIVILEGE; Process-String wäre "member-action" | |
| Hybrid: ADMIN_PRIVILEGE-Pre-Check + delegierter Call | Doppel-Check, recalc_dates kommt gratis, Process-String bleibt "member-action" | |

**User's choice:** Direkter DAO-Call + audited_create!
**Notes:** Klare ADMIN-only-Permission ist PERM-01-Anforderung; eigener Audit-Namespace setzt Foundation für AUDT-02 in Phase 17.

### Q2: Wie soll recalc_dates für MembershipAdjustService nutzbar gemacht werden?

| Option | Description | Selected |
|--------|-------------|----------|
| Free-Function extrahieren | recalc_dates wird `pub(crate) async fn recalc_dates(member_dao, member_action_dao, member_id, tx)` in member_action.rs. Beide Services rufen sie auf. Explizite Dependencies. | ✓ |
| Auf MemberActionService-Trait expose | recalc_dates wird Teil des Traits (pub fn). MembershipAdjustService injiziert MemberActionService und ruft action_service.recalc_dates(...) auf. | |
| Inline duplizieren im neuen Service | DRY-Verletzung, aber maximale Entkopplung. | |

**User's choice:** Free-Function extrahieren
**Notes:** Konsistent mit Pure-Helper-Konvention (analog compute_dates).

### Q3: Wie soll Member.current_shares bei increase_shares aktualisiert werden?

| Option | Description | Selected |
|--------|-------------|----------|
| Neue targeted DAO-Methode update_current_shares | Konsistent mit update_migrated / update_dates-Pattern. Effizientes SQL. | (überrideen durch Q4) |
| Generisches MemberDao::update() mit geladener Member-Entity | Member laden, mutieren, generischer Update | (final gewählt in Q4) |
| Eigene audit-aware Methode in membership_adjust | Inline-SQL — bricht DAO-Abstraktion. | |

**User's choice (initial):** Targeted DAO-Methode update_current_shares — dann in Q4 überrideen.

### Q4: AUDT-01-Compliance für current_shares-Update?

| Option | Description | Selected |
|--------|-------------|----------|
| Generischer MemberDao::update() via audited_update! | AUDT-01-rein. Macro diffed automatisch -> logged nur current_shares-Änderung. SQL hits full member-update, aber das ist <50 Bytes Overhead. | ✓ |
| Targeted update_current_shares + manuelle Audit-Log-Calls | Bricht das audited_*!-Macros-Versprechen aus AUDT-01 (grep-Gate würde failen). | |
| audited_*-Macro erweitern, dass es targeted-Methoden unterstützt | Mehr Komplexität in den Macros. | |

**User's choice:** Generischer MemberDao::update() via audited_update!
**Notes:** AUDT-01-Compliance hat Vorrang vor SQL-Mikro-Effizienz; Macro diffed automatisch.

---

## Datum-Bounds-Strategie

### Q1: Wo lebt die Datum-Bounds-Validierung architektonisch?

| Option | Description | Selected |
|--------|-------------|----------|
| Pure-Function in membership_adjust.rs + Service-Aufruf | Testbar als Edge-Case-Battery. Service ruft sie am Methoden-Eintritt auf. | ✓ |
| Service-internal-Inline-Check | Inline-if-Statement; Edge-Case-Tests laufen indirekt über Service-Tests. | |
| Validator-Struct (ValidationFailureItem Pattern) | validate_willensbekundung-Funktion sammelt Fehler in Vec<ValidationFailureItem>. | |

**User's choice:** Pure-Function in membership_adjust.rs + Service-Aufruf
**Notes:** Konsistent mit Pure-Function-Konvention aus Phase 14.

### Q2: Was sind genau die Bounds?

| Option | Description | Selected |
|--------|-------------|----------|
| Kalender-Jahr-basiert: [today.year-01-01, today.year+1-12-31] | Einfach, intuitiv. Vorstand kann Backdating innerhalb laufenden Jahres machen, Forward bis Ende nächstes Jahr. | ✓ |
| Fiscal-Year-basiert via compute_effective_date | Konsistent mit H1/H2-Logik, aber kontraintuitiv. | |
| RepaymentPhaseDao-Lookup nach status=Open | Realitätsbasiert, aber Tx-Coupling und Race-Conditions. | |

**User's choice:** Kalender-Jahr-basiert

### Q3: Wie kommt das today-Datum in die Pure-Function?

| Option | Description | Selected |
|--------|-------------|----------|
| today als Parameter (testbar) | Service-Caller gibt OffsetDateTime::now_utc().date() an. Deterministisch testbar. | ✓ |
| today wird intern via OffsetDateTime::now_utc() geholt | Weniger Boilerplate, aber nicht-deterministische Tests. | |
| Service hält UuidService-ähnliche Time-Abstraction | Überkompliziert für v1.2. | |

**User's choice:** today als Parameter

### Q4: Welche Error-Shape für Bound-Violations?

| Option | Description | Selected |
|--------|-------------|----------|
| ServiceError::ValidationError mit ValidationFailureItem | Konsistent mit validate_action-Pattern. HTTP 400. i18n-Ready. | ✓ |
| ServiceError::Conflict("date out of bounds") | Einfacher String-Error. HTTP 409. Bricht Validation-Pattern. | |
| ServiceError::BadRequest("date must be...") | ServiceError hat keine BadRequest-Variante. | |

**User's choice:** ServiceError::ValidationError

---

## REST-Endpoint-Shape

### Q1: Welche REST-Endpoint-Struktur?

| Option | Description | Selected |
|--------|-------------|----------|
| Separate Sub-Routes | POST /api/members/{id}/cancel + /increase-shares; Phase 16-17 ergänzen weitere. | ✓ |
| Unifizierter Endpoint mit Body-Discriminator | POST /api/members/{id}/membership-adjust mit Body-Variante. | |
| Top-Level-Route mit Op-Path | POST /api/membership-adjust/{op}. Decouples von /api/members. | |

**User's choice:** Separate Sub-Routes
**Notes:** Konsistent mit existing /api/members/{id}-Routes; klares OpenAPI-Schema pro Operation.

### Q2: Wie ist das Request-Body-DTO geformt?

| Option | Description | Selected |
|--------|-------------|----------|
| Slim DTO pro Operation in genossi_rest_types | CancelMembershipRequestTO + IncreaseSharesRequestTO. Klare OpenAPI-Schemas. | ✓ |
| Einheitliches MembershipAdjustRequestTO mit Optional shares | Weniger Typen, aber unsaubere Felder. | |
| Inline-Body via #[derive(Deserialize)] im REST-Handler | Spart Re-Export, aber Frontend muss Felder selbst kennen. | |

**User's choice:** Slim DTO pro Operation in genossi_rest_types

### Q3: Was kommt im Response-Body zurück?

| Option | Description | Selected |
|--------|-------------|----------|
| Erzeugte MemberActionTO + aktualisiertes MemberTO | Frontend bekommt sofortiges Update für Re-Render. Single-Round-Trip. | ✓ |
| Nur erzeugte MemberActionTO | Frontend muss separat GET /api/members/{id} re-fetchen. | |
| Status 204 No Content | Sparsam, aber unergonomisch. | |
| Nur 201 Created mit Location-Header | Klassisch REST, aber Genossi nutzt diesen Pattern nicht. | |

**User's choice:** Erzeugte MemberActionTO + aktualisiertes MemberTO

### Q4: Wo werden die neuen Routes registriert?

| Option | Description | Selected |
|--------|-------------|----------|
| Innerhalb member::generate_route | Sub-Routes vor /{id}-catch-all deklarieren. Reuse existing /api/members-Mount. | ✓ |
| Neue membership_adjust Route-Module | Klarer v1.2-Namespace, aber dann Member-ID via Body statt Path-Param. | |

**User's choice:** Innerhalb member::generate_route
**Notes:** Reihenfolge (Sub-Routes vor /{id}) ist kritisch — Lesson aus Phase 14 D-14-08.

---

## MembershipAdjustService-Trait-Shape

### Q1: Wie soll das Trait modelliert sein?

| Option | Description | Selected |
|--------|-------------|----------|
| Inkrementell wachsend | Phase 15: 2 Methoden; Phase 16+17 ergänzen. Mock-Burden minimal. | ✓ |
| Voll-Trait mit todo!() für Phase 16-17 | Stabile API-Shape, aber Panic-Risiko. | |
| Voll-Trait mit ServiceError::InternalError("not_yet_implemented") | Halbgar. | |

**User's choice:** Inkrementell wachsend
**Notes:** Konsistent mit v1.1-Pattern (MemberActionService wuchs auch über Phasen).

### Q2: Wo lebt das Trait?

| Option | Description | Selected |
|--------|-------------|----------|
| Neue Datei genossi_service/src/membership_adjust.rs | Konsistent mit D-14-02. Klare Layer-Trennung. | ✓ |
| Erweiterung des MemberActionService-Traits | Bricht Domain-Trennung. | |
| Erweiterung des MemberService-Traits | Member-Trait sollte Member-CRUD bleiben. | |

**User's choice:** Neue Datei

### Q3: Welche Method-Signaturen?

| Option | Description | Selected |
|--------|-------------|----------|
| Granulare Parameter | cancel_membership(member_id, willensbekundung_date, context, tx) -> Result<(MemberAction, Member), ServiceError>. Return-Tuple matches Response-Body-Shape. | ✓ |
| Request-Struct als Parameter | Mehr Boilerplate, aber additive Erweiterbarkeit. | |
| Return nur MemberAction | Frontend muss separat fetchen. Bricht den Response-Body-Vertrag. | |

**User's choice:** Granulare Parameter

### Q4: Wie DI-Wiring in RestStateImpl?

| Option | Description | Selected |
|--------|-------------|----------|
| Neuer Service-Slot in RestStateImpl | membership_adjust_service: Arc<MembershipAdjustServiceImpl<...>>. Dependencies via gen_service_impl!. | ✓ |
| Funktion auf bestehendem MemberActionServiceImpl als Methode | Bricht Single-Responsibility. | |

**User's choice:** Neuer Service-Slot

---

## Claude's Discretion

- **Handler-Datei-Placement**: neue `genossi_rest/src/membership_adjust.rs` oder Erweiterung von `member.rs` — Planner-Discretion je nach Datei-Größe.
- **Response-DTO-Naming**: anonymes JSON-Object vs. benannter Typ — Planner-Discretion.
- **Already-Cancelled-Detection-Heuristik**: `member.exit_date IS NOT NULL` vs. `actions.find(ActionType::Austritt)` — beide funktionieren.
- **`shares > 0` Validation für `increase_shares`**: optionaler Service-Layer-Check (Roadmap nicht explizit).
- **`recalc_migrated` Free-Function-Refactor**: Planner prüft, ob nötig (für Aufstockung ggf. nicht).
- **Plan-Datei-Reihenfolge**: Empfehlung `plan_01_trait_and_validate_date` → `plan_02_cancel_membership` → `plan_03_increase_shares` → `plan_04_rest_endpoints_and_e2e`.

## Deferred Ideas

- `MembershipAdjustService::partial_repayment` — Phase 16.
- `MembershipAdjustService::transfer_shares` + AUDT-02 — Phase 17.
- `compute_effective_date` als `pub`-Re-Export (CANC-06 Vorschau) — Phase 18.
- Targeted `update_current_shares` DAO-Methode für SQL-Effizienz — v2-Optimierung falls Performance-Problem.
- Audit-Macro-Erweiterung um targeted-Methoden (`audited_update_with!`) — bei Bedarf in Phase 16+.
- `recalc_migrated` Free-Function-Refactor — falls Phase 16/17 es braucht.
