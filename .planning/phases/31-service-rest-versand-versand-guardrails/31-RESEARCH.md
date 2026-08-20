# Phase 31: Service + REST Versand (Versand + Guardrails) — Research

**Researched:** 2026-08-20
**Domain:** Rust Backend — Service- + REST-Schicht, Wiederverwendung von `genossi_mail` (Job-Queue-Mailversand), DSGVO-/Admin-Guardrails
**Confidence:** HIGH (alle Claims direkt gegen den Live-Code verifiziert)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** `ApplicationService::send_mail(...) -> Result<(), ServiceError>` — kein stilles `()` wie `send_confirmation_mail`. Versand bleibt job-queue-basiert (`create_job` → Worker sendet via SMTP). Synchron als Fehler zurückgegeben: Permission (403), Not-Found (404), Status ≠ `Offen` (409), fehlende Adresse (400), enqueue-Fehler (500). SMTP-/Render-Delivery passiert per-recipient im Worker → `outbound_status = "failed"`, sichtbar in der Kommunikations-Historie (nicht in der POST-Response).
- **D-02 (1b):** KEIN expliziter SMTP-Config-Präsenz-Pre-Flight beim Versand — der SMTP-Server wird bei der Konfiguration ohnehin getestet. Delivery-Scheitern äußert sich im Recipient-`outbound_status`, nicht als synchroner Fehler. Kontrast zu `send_confirmation_mail`: `send_mail` schluckt Fehler nicht still (echtes `Result` + sichtbarer Recipient-Status), nicht dass SMTP synchron getestet wird.
- **D-03 (2a=A):** `send_mail` nimmt rohen Content — `subject` + `body` (+ optional `body_html`) mit Platzhaltern — genau wie die anderen Mail-Sends (keine Client-Auflösung). Stempelt `RecipientInput { address: application.email, member_id: None, application_id: Some(app.id) }` und ruft `create_job`. `template_id` kann wie im Member-Pfad mitgeführt werden.
- **D-04 (KERN-Seam):** Der Worker-Renderer `resolve_rendered_content` (`genossi_mail/src/render.rs`, konsumiert in `worker.rs:391`) bekommt einen Application-Zweig: wenn `recipient.application_id.is_some()`, wird der Kontext über `application_to_template_context` (Phase 30) gebaut — Config (`share_value_cents`, `bank_iban`, `bank_name`, `bank_bic`, `genossenschaft_name`) einmal aufgelöst, `open_amount` via `format_eur_de`. Zentraler Backend-Seam — sowohl Send (Worker) als auch Preview konsumieren ihn.
- **D-05 (2b):** Keine strict-Render-/`validate_application_template`-Prüfung beim Versand — `validate_application_template` (Phase 30) gatet bereits bei Template-Create/Update. `body_html` wird wie überall an der Store-Boundary ammonia-sanitisiert (bestehender Gate).
- **D-06:** Der Application-Preview-Render-Endpoint wird in Phase 31 (Backend) gebaut. Rendert den Entwurf (subject+body) gegen denselben Application-Zweig aus D-04, gibt die aufgelöste Vorschau zurück (Preview == Worker-Output garantiert, ein Renderer-Seam). Spiegelt das bestehende Member-`/api/mail/preview`-Muster. Phase 32 verdrahtet nur die UI (APMAIL-04).
- **D-07 (3a=A):** „zuletzt gesendet" = MAX über die outbound-Historie, basierend auf dem Enqueue-/`created`-Zeitpunkt (nicht `sent_at`), damit der Anti-Doppelversand-Guard sofort beim Absenden greift — unabhängig vom Worker-Erfolg.
- **D-08 (3b=B):** Der Wert wird serverseitig aggregiert und als dediziertes Feld/Service-Methode geliefert (z. B. `last_sent_at: Option<...>`, angehängt an die Application-Detail-Response oder eigener kleiner Endpoint) — keine Client-Aggregation.
- **D-09 (3c):** `GET /api/applications/{id}/communications` liefert volle Einträge inkl. Betreff (wie die Member-Timeline). Bestehender `CommunicationEntry` / `CommunicationEntryTO` (`subject`, `date`, `outbound_status`, `to_address`) wird 1:1 wiederverwendet — nur ein paralleler Handler (`get_application_communications`, DAO existiert seit Phase 29) + Admin-Gate nötig. Kein Body-Snapshot (APHIST-FUT-01, verschoben).
- **D-10 (4a=A):** `send_mail`, Preview- und Communications-Endpoints verlangen `MANAGE_MEMBERS_PRIVILEGE` — exakt wie `confirm`/`reject`. Nicht-Admin → 403.
- **D-11 (4c):** Reihenfolge wie bei `confirm` (CR-02-Muster): Permission-Check zuerst, dann Not-Found (404), dann Status-Guard `Offen`-only (409). Kein user-attributierbarer Seiteneffekt vor dem Permission-Check.
- **D-12 (4b=A):** Fehlende `application.email` → sauberer Backend-Fehler (400 / `ValidationError`) als Defense-in-Depth; Phase 32 deaktiviert zusätzlich den Button. Nie ein stiller Fehlversuch.
- **D-13:** Kein Massenversand-Pfad, kein Freitext-Empfänger (immer `application.email`), kein Open-/Click-Tracking; Timeline outbound-only. Per Service-/E2E-Test verifiziert. Der `Offen`-only-Guard (D-11) ist zugleich die DSGVO-transaktionale Rechtsgrundlage-Grenze.

### Claude's Discretion
- Exakte Pfad-/Request-/Response-Shapes (Preview-Endpoint, send-Request-Body), solange das Member-Muster gespiegelt wird.
- Ob `last_sent_at` auf der bestehenden `get_application`-Response mitfährt oder als eigener kleiner Endpoint kommt — solange serverseitig aggregiert (D-08).
- OpenAPI-Doku-Detail, i18n-Keys, exakte ServiceError-Variante für die no-email-Validierung.

### Deferred Ideas (OUT OF SCOPE)
- **APMAIL-04 UI-Verdrahtung** (Live-Preview + confirm-before-send im Dialog) → Phase 32. Der Backend-Preview-Endpoint dafür wird aber schon in Phase 31 gebaut (D-06).
- **APMAIL-03 Button-Disable/Annotation bei fehlender Adresse** → Phase 32 (Backend-Guard 400 ist Phase 31, D-12).
- **APHIST-FUT-01** Betreff-/Body-Snapshot je Timeline-Eintrag → eigene Zukunfts-Phase.
- **APMAIL-FUT-01** Massen-Erinnerung an alle `Offen`-Antragsteller (Bulk-Send) → nach v1.6.
- Kein neues Audit-Feld auf `ApplicationEntity`. Keine neue Backend-Dependency („add nothing").
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| APMAIL-01 | Vorstand kann Application (`Offen`) eine einzelne Mail senden; `RecipientInput{ member_id: None, application_id: Some }`; `POST /api/applications/{id}/mail`, admin-only | `create_job`-Signatur + `RecipientInput`-Shape verifiziert (`service.rs:54,97`); Empfänger = `app.email`; Admin-Gate via `check_permission(MANAGE_MEMBERS_PRIVILEGE)` (Muster `confirm`, `application.rs:300`) |
| APMAIL-02 | Versand gibt echtes `Result<_, ServiceError>` zurück — keine stille 200-Falle | `send_confirmation_mail` (Anti-Vorbild, `application.rs:44`, gibt `()` zurück, `tracing::error!` + `return`) vs. `confirm` (echtes `Result`); neue Methode folgt `confirm` |
| APCMP-01 | Versand nur bei Status `Offen`, sonst 409 (DSGVO-Rechtsgrundlage) | Status-Guard-Muster in `confirm` (`application.rs:316`, `ServiceError::Conflict`) |
| APCMP-02 | Inhalt bezogen auf eigene Erklärung; kein Massenversand, kein Tracking | Empfänger fest = `app.email`; single-recipient `create_job`; keine Tracking-Felder im MIME-Builder (`send.rs`) |
| APHIST-02 | „zuletzt gesendet am …" serverseitig aggregiert (Anti-Doppelversand-Guard) | `get_application_communications` existiert (`dao.rs:307`, `dao_sqlite.rs:1128`); MAX über `date` — ⚠️ `date = COALESCE(sent_at, created)`, siehe Pitfall 1 zu D-07 |
</phase_requirements>

## Summary

Diese Phase ist reine **Wiederverwendung** des `genossi_mail`-Subsystems plus dünne Service-/REST-Verdrahtung auf der Application-Seite. Es wird **keine** neue Dependency, **keine** neue Entität und **kein** neues Audit-Feld eingeführt. Die gesamte technische Substanz existiert bereits (Job-Queue, per-recipient-Worker-Render, `RecipientInput.application_id` seit Phase 29, `application_to_template_context` + `format_eur_de` seit Phase 30, `CommunicationEntry`/`CommunicationEntryTO` + `get_application_communications`-DAO). Die Aufgabe ist, diese Bausteine über eine neue `ApplicationService::send_mail`-Methode, einen Application-Zweig im Renderer und drei admin-gegatete REST-Routen zu verbinden.

Der **zentrale und einzige nicht-triviale Seam** ist D-04: `resolve_rendered_content` (`render.rs`) muss einen Application-Zweig bekommen. Das ist heikler als es klingt, weil die Funktion aktuell **weder** einen Application-Resolver **noch** die `ConfigService` in ihrer Signatur hat — `application_to_template_context` braucht aber beides (eine geladene `Application` + fünf Config-Werte). Die Funktion wird an **zwei** Call-Sites konsumiert (Worker `worker.rs:391` und Backfill `backfill.rs:71`); beide müssen die neuen Parameter durchreichen, und beide werden in `genossi_bin` verdrahtet. Das ist die größte und risikoreichste Einzelaufgabe der Phase.

Gute Nachricht für die Service-Schicht: `ApplicationServiceDeps` erreicht **bereits** sowohl `ConfigService` als auch `MailService` (`application.rs:38-39`) — für `send_mail` muss **keine** neue Dependency hinzugefügt werden. Und `send_confirmation_mail` (`application.rs:44`) liefert die exakte Config-Kette (`share_value_cents` / `bank_iban` / `bank_name` / `bank_bic` / `genossenschaft_name`) als kopierbare Referenz — allerdings nur als Config-Auflösungs-Vorbild, **nicht** als Kontroll-Fluss-Vorbild (es ist das explizit zu vermeidende stille-`()`-Anti-Pattern).

**Primary recommendation:** `send_mail` nach dem `confirm`-Muster bauen (CR-02-Ordering, echtes `Result`); den Application-Zweig als **kleine reine Render-Helferfunktion** extrahieren, die von BEIDEN — Worker-`resolve_rendered_content`-Zweig UND Preview-Endpoint — aufgerufen wird (garantiert D-06 „Preview == Worker-Output"); `resolve_rendered_content` um `ApplicationResolver` + `ConfigService` erweitern und an beiden Call-Sites (Worker + Backfill) plus `genossi_bin` durchreichen; `last_sent_at` und die Communications-Route bewusst über die **Application-Service/REST-Schicht** bauen (nicht über den ungegateten `communication_rest`-Handler), um den Admin-Gate (D-10) sauber zu erzwingen.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Versand-Auslösung + Guards (Permission/Status/Adresse) | Service (`ApplicationServiceImpl::send_mail`) | REST-Handler (dünn) | Business-Logik/Guards gehören in den Service; REST reicht nur `context` + `id` durch (Muster `confirm_application`) |
| Job-Enqueue (raw content → Recipient) | Service → `MailService::create_job` | — | `create_job` existiert; stempelt `application_id` in den Recipient |
| Per-recipient Rendering (Application-Kontext) | Renderer (`resolve_rendered_content` / neue reine Helferfn) | Worker + Preview konsumieren | Ein Renderer-Seam (D-04/D-06); Delivery-Fehler → `outbound_status` |
| Config-Auflösung (5 Keys) | Service ODER Renderer-Seam | — | `application_to_template_context` ist pur (D-07); Config muss vor dem Aufruf aufgelöst werden — im Renderer-Zweig, wo Worker+Preview beide durchlaufen |
| SMTP-Delivery + Status-Persistenz | Worker (`worker.rs`) | — | Existiert; `outbound_status = "sent"/"failed"` |
| „zuletzt gesendet" Aggregation | Service (neue Methode über `get_application_communications`) | DAO (bestehend) | D-08 serverseitig; kein Client-Aggregat |
| Communications-Timeline (Read) | Service/REST (Application-Seite, admin-gegatet) | DAO `get_application_communications` (bestehend) | D-10 Admin-Gate; der bestehende `communication_rest`-Handler ist NICHT gegatet (Pitfall 3) |
| HTML-Sanitisierung | `create_job` Store-Boundary (bestehend, ammonia) | — | D-05; Worker re-sanitisiert NICHT |

## Standard Stack

### Core (alles bereits im Workspace — „add nothing")
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `genossi_mail` (intern) | workspace | Job-Queue, Worker-Render, `create_job`, `CommunicationDao` | Das gesamte Mail-Subsystem; Phase 31 verdrahtet nur | `[VERIFIED: genossi_mail/src/service.rs, render.rs, worker.rs]` |
| `minijinja` | 2.x | strict-env Template-Rendering (`render_template`/`render_html_template`) | Bereits Render-Engine für Member- + Application-Kontext | `[VERIFIED: genossi_mail/src/template.rs:135-179]` |
| `ammonia` | (Phase 23) | HTML-Sanitisierung an Store-Boundary | `create_job` sanitisiert `body_html` bereits (`service.rs:420`) | `[VERIFIED: genossi_mail/src/service.rs:417-423]` |
| `axum` | 0.8.3 | REST-Handler + Router | Bestehende Handler-Muster in `application.rs` | `[VERIFIED: genossi_rest/src/application.rs]` |
| `utoipa` | 5.0 | OpenAPI-Registrierung der neuen Routen | Bestehende `#[utoipa::path]`-Annotationen + `ApiDoc` | `[VERIFIED: genossi_rest/src/application.rs:493]` |
| `mockall` | 0.13 | Unit-Test-Mocks (`MockMailService`, `MockConfigService`, `MockPermissionService`) | Bestehende Test-Infrastruktur in `application.rs`-Tests | `[VERIFIED: genossi_service_impl/src/application.rs:731,908-928]` |
| `time` | 0.3 | `PrimitiveDateTime` für `date`/`last_sent_at` | Bestehender Typ auf `CommunicationEntry.date` | `[VERIFIED: genossi_mail/src/dao.rs:279]` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Application-Zweig in `resolve_rendered_content` erweitern | Separater 2. Render-Pfad nur für Preview | Verletzt D-06 („ein Renderer-Seam") + DRY; Preview würde vom Worker-Output driften. NICHT tun. |
| `last_sent_at` via MAX über `get_application_communications`-Einträge im Service | Dedizierte DAO-Query `MAX(created)` | Siehe Pitfall 1 — `date` ist `COALESCE(sent_at, created)`, nicht `created`. Beide Optionen sind valide; Entscheidung nötig. |
| Communications-Route in Application-REST/Service (gegatet) | `communication_rest.rs`-Handler 1:1 spiegeln | Der bestehende Handler ist NICHT permission-gegatet (Pitfall 3) — 1:1-Spiegel würde D-10 verletzen. |

**Installation:** Keine. Alle Crates sind bereits Workspace-Members / Dependencies. `[VERIFIED: keine `Cargo.toml`-Änderung nötig]`

## Package Legitimacy Audit

**Nicht anwendbar** — diese Phase installiert **keine** externen Pakete („add nothing", CONTEXT.md/STATE.md „Neue Backend-Dependency: keine"). Alle verwendeten Crates sind bereits im Workspace verankert und in v1.4/v1.5/v1.6-Phasen legitimiert. Keine `cargo add`, keine neue `[dependencies]`-Zeile.

## Architecture Patterns

### System Architecture Diagram

```
POST /api/applications/{id}/mail  (admin-only)
        │  Extension<Context> + Json{subject, body, body_html?, template_id?}
        ▼
confirm_application-analoger REST-Handler (genossi_rest/src/application.rs)
        │  extract_auth_context(context)? → application_service().send_mail(...)
        ▼
ApplicationServiceImpl::send_mail  (genossi_service_impl/src/application.rs)
   1. check_permission(MANAGE_MEMBERS_PRIVILEGE)      → PermissionDenied → 403   ┐ D-11
   2. find_by_id(id) → ok_or(EntityNotFound)          → 404                       │ CR-02
   3. status != Offen → ServiceError::Conflict        → 409  (DSGVO-Grenze)       │ Ordering
   4. app.email is None → ValidationError             → 400  (D-12)              ┘
   5. create_job(subject, body, body_html?, [RecipientInput{                     ┐ D-03
        address: app.email, member_id: None, application_id: Some(app.id) }])    ┘ enqueue
        │  (500 bei enqueue-Fehler)  → Ok(())
        ▼
mail_recipients-Zeile (application_id gesetzt, status="pending")
        │
        ▼  Hintergrund-Worker (start_mail_worker, worker.rs:391)
resolve_rendered_content(recipient, job, ...NEU: app_resolver, config_service)
        │  recipient.application_id.is_some()  ──► NEUER ZWEIG (D-04)
        │        load Application + resolve 5 config keys
        │        ctx = application_to_template_context(app, share_value_cents, iban, name, bic, geno)
        │        render subject/body/body_html  (reine Helferfn — SHARED mit Preview)
        ▼
send_mail_for_recipient → SMTP  →  outbound_status = "sent" | "failed"
        │
        └──► sichtbar in GET /api/applications/{id}/communications
                                    │
POST /api/applications/{id}/mail/preview (admin-only, D-06)
        │  rendert Entwurf über DIESELBE reine Helferfn → PreviewResponse
        ▼  (synchron, kein Send)

GET /api/applications/{id}/communications (admin-only, D-09)
        └─► service (gegatet) → get_application_communications(app_id) → Vec<CommunicationEntryTO>

last_sent_at (D-07/D-08)  ─► service: MAX über get_application_communications(app_id).date
        └─► als Feld auf get_application-Response ODER eigener kleiner Endpoint (Discretion)
```

### Component Responsibilities
| Component | Datei:Zeile (verifiziert) | Rolle in Phase 31 |
|-----------|---------------------------|-------------------|
| `ApplicationService`-Trait | `genossi_service/src/application.rs:107` | `send_mail` (+ ggf. `get_communications`, `last_sent_at`-Methode) hier ergänzen |
| `ApplicationServiceImpl` | `genossi_service_impl/src/application.rs:43,163` | Implementierung; Deps erreichen `ConfigService`+`MailService` bereits |
| `send_confirmation_mail` | `genossi_service_impl/src/application.rs:44` | Config-Ketten-Vorbild (share_value_cents/bank_*/geno_name); Anti-Vorbild für Kontrollfluss |
| `confirm` | `genossi_service_impl/src/application.rs:290` | CR-02-Ordering-Vorbild 1:1 für `send_mail` |
| `resolve_rendered_content` | `genossi_mail/src/render.rs:69` | KERN-Seam: Application-Zweig einfügen (D-04) |
| Worker-Konsument | `genossi_mail/src/worker.rs:391` | Call-Site 1 — neue Params durchreichen |
| Backfill-Konsument | `genossi_mail/src/backfill.rs:71` | Call-Site 2 — neue Params durchreichen |
| `application_to_template_context` | `genossi_mail/src/template.rs:79` | Kontext-Builder (pur); braucht `&Application` + 5 Config-Werte |
| `MailService::create_job` | `genossi_mail/src/service.rs:97` | Enqueue; `RecipientInput.application_id` stempeln |
| `RecipientInput` | `genossi_mail/src/service.rs:54` | `{ address, member_id, application_id }` |
| `get_application_communications` | `genossi_mail/src/dao.rs:307`, `dao_sqlite.rs:1128` | outbound-only Query; existiert |
| `CommunicationEntry`/`CommunicationEntryTO` | `dao.rs:277`, `communication_rest.rs:29` | 1:1 wiederverwenden |
| `preview_mail` (Member) | `genossi_mail/src/rest.rs:661` | Muster für Application-Preview |
| REST-Handler `confirm_application` | `genossi_rest/src/application.rs:361` | Handler-Vorbild (Extension<Context>, error_handler, 409-Doku) |
| `generate_route` | `genossi_rest/src/application.rs:479` | Neue Routen `/{id}/mail`, `/{id}/mail/preview`, `/{id}/communications` anhängen |
| `ApplicationApiDoc` | `genossi_rest/src/application.rs:493` | Neue Handler in `paths(...)` registrieren |

### Pattern 1: CR-02 Permission-First-Ordering (D-11) — Vorbild `confirm`
**What:** Guard-Reihenfolge im Service. **When:** In `send_mail` exakt spiegeln.
```rust
// Source: genossi_service_impl/src/application.rs:290-321 (confirm)
let tx = self.transaction_dao.use_transaction(None).await?;
// 1. Permission ZUERST — vor jedem user-attributierbaren Seiteneffekt
self.permission_service
    .check_permission(MANAGE_MEMBERS_PRIVILEGE, context.clone())
    .await?;                                        // → PermissionDenied → 403
// 2. Not-Found
let entity = self.application_dao
    .find_by_id(id, tx.clone()).await?
    .ok_or(ServiceError::EntityNotFound(id))?;      // → 404
// 3. Status-Guard (zugleich DSGVO-Grenze, APCMP-01)
if entity.status != ApplicationStatus::Offen {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Application status is '{}', expected 'Offen'", entity.status.as_str()))));  // → 409
}
// 4. (Phase 31, D-12) fehlende Adresse → ValidationError → 400
// 5. create_job(...) → enqueue → Ok(())
```

### Pattern 2: `create_job`-Aufruf mit Application-Recipient (D-03)
**What:** raw content + Application-gestempelter Recipient. **When:** Im `send_mail`-Enqueue-Schritt.
```rust
// Source: abgeleitet aus service.rs:97 (Signatur) + application.rs:130-158 (send_confirmation_mail-Aufruf)
let recipient = genossi_mail::service::RecipientInput {
    address: app.email.as_ref()... .to_string(),   // Empfänger fest = Application (D-13)
    member_id: None,                                // Namespace-Trennung (Pitfall 2)
    application_id: Some(app.id),                   // Phase 29 Linkage
};
self.mail_service.create_job(
    &subject, &body,
    body_html,                                      // Option<String> — in create_job ammonia-sanitisiert
    vec![recipient],                                // single-recipient (kein Massenversand, D-13)
    vec![], vec![],                                 // keine Attachments/Static-Docs
    template_id,                                    // optional, wie Member-Pfad
    None,                                           // repayment_phase_id — N/A für Application
    false,                                          // attach_repayment_letter
).await.map_err(|e| /* → ServiceError, 500 */)?;
```

### Pattern 3: Application-Kontext-Bau (D-04) — reine Helferfn, SHARED Worker+Preview
**What:** Config auflösen → `application_to_template_context`. **When:** Im neuen Renderer-Zweig UND im Preview-Endpoint.
```rust
// Source: application_to_template_context — genossi_mail/src/template.rs:79-101
// Config-Auflösung — Vorbild send_confirmation_mail, application.rs:55-97
let share_value_cents: i64 = config.get("share_value_cents").await?.value.parse()?;
let bank_iban = config.get("bank_iban").await?.value.to_string();
let bank_name = config.get("bank_name").await?.value.to_string();
let bank_bic  = config.get("bank_bic").await.ok().map(|e| e.value.to_string());  // optional
let geno_name = config.get("genossenschaft_name").await?.value.to_string();
let ctx = application_to_template_context(
    &app, share_value_cents, &bank_iban, &bank_name, bank_bic.as_deref(), &geno_name);
// open_amount = format_eur_de(share_value_cents * app.shares) wird INTERN gebildet.
let subject = render_template(&job.subject, &ctx)?;   // strict-env
let body    = render_template(&job.body, &ctx)?;
let body_html = job.body_html.as_deref().map(|h| render_html_template(h, &ctx)).transpose()?;
```

### Anti-Patterns to Avoid
- **Stilles `()` (send_confirmation_mail-Muster):** `send_confirmation_mail` (`application.rs:44`) loggt Fehler via `tracing::error!` und `return`t ohne Rückgabe. `send_mail` MUSS `Result<(), ServiceError>` liefern und jeden synchronen Fehlerpfad propagieren (D-01/D-02, APMAIL-02). Das ist der explizite Grund für die Phase.
- **Application-UUID in `member_id`:** Niemals `member_id: Some(app.id)`. Immer `member_id: None` + `application_id: Some(app.id)` (Pitfall 2, Phase-29-Grep-Gate).
- **Zweiter Render-Pfad für Preview:** Verletzt D-06. Preview MUSS über dieselbe reine Helferfn wie der Worker rendern.
- **Communications-Handler ohne Admin-Gate:** Der bestehende `get_member_communications` (`communication_rest.rs:120`) hat KEINEN Permission-Check. Ein 1:1-Kopieren für Application würde D-10 verletzen (Pitfall 3).
- **Worker re-sanitisiert HTML:** Nein — `create_job` sanitisiert an der Store-Boundary (D-05). Doppelte Sanitisierung ist falsch.

### Recommended Task-Struktur (grob, `granularity: coarse`)
```
1. Renderer-Seam: ApplicationResolver-Trait (mirror MemberResolver) + Application-Zweig in
   resolve_rendered_content + ConfigService-Param; beide Call-Sites (worker.rs, backfill.rs)
   + genossi_bin-Wiring anpassen. [KERN, höchstes Risiko]
2. ApplicationService::send_mail (Trait + Impl) nach confirm-Muster (CR-02, D-11/D-12) + create_job.
3. last_sent_at Service-Methode über get_application_communications (Entscheidung Pitfall 1).
4. REST: POST /{id}/mail, POST /{id}/mail/preview, GET /{id}/communications (admin-gegatet) +
   OpenAPI-Registrierung in generate_route + ApplicationApiDoc.
5. Service-Unit-Tests + E2E-Tests (Guards, 403/404/409/400, Namespace-Trennung, kein Tracking).
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Mail-Versand/SMTP/Retry | Eigener SMTP-Pfad im Service | `MailService::create_job` + Worker | Job-Queue, Retry, Status-Persistenz existieren komplett |
| Euro-Formatierung | `format!("{},{:02}")` | `genossi_service::euro::format_eur_de` | Tausenderpunkt, Komma, Negativ-/Null-Fall bereits getestet (`euro.rs:65-106`) |
| Application-Template-Kontext | Manuelles `context!{}` | `application_to_template_context` (`template.rs:79`) | DSGVO-Feldtrennung + salutation-Mapping + open_amount bereits korrekt |
| HTML-Sanitisierung | ammonia im Service aufrufen | `create_job` macht es an der Store-Boundary | D-05; Doppel-Sanitisierung vermeiden |
| Communications-TO-Serde | Neues TO | `CommunicationEntryTO` (`communication_rest.rs:29`) 1:1 | D-09: gleiche Felder (subject/date/outbound_status/to_address) |
| outbound-Historie-Query | Neue SQL | `get_application_communications` (`dao_sqlite.rs:1128`) | Existiert seit Phase 29, soft-delete-korrekt |

**Key insight:** Die Phase hat fast keinen „neuen Algorithmus". Jeder Baustein existiert; die Kunst ist die **korrekte Verdrahtung** + die **Guard-Reihenfolge** + der **eine Renderer-Seam** für Send und Preview.

## Common Pitfalls

### Pitfall 1: `last_sent_at`-Semantik — `date` ist `COALESCE(sent_at, created)`, nicht `created` (D-07)
**What goes wrong:** D-07 verlangt „basierend auf Enqueue-/`created`-Zeitpunkt (nicht `sent_at`)". Die wiederverwendbare DAO-Query liefert aber `date = COALESCE(r.sent_at, r.created)` (`dao_sqlite.rs:1128` SELECT). Wenn man `last_sent_at = MAX(entry.date)` über `get_application_communications` bildet, ist der Wert für erfolgreich gesendete Recipients `sent_at` (leicht später), nicht `created`.
**Why it happens:** Der Communications-DAO ist für die Timeline-Anzeige gedacht (zeigt „wann gesendet"), nicht für den Anti-Doppelversand-Guard.
**How to avoid:** Funktional erfüllt `COALESCE(sent_at, created)` die eigentliche D-07-Anforderung („greift sofort beim Absenden, unabhängig vom Worker-Erfolg"): pending → `created` (sofort da), failed → `created` (`sent_at` NULL), sent → `sent_at`. Der Guard zählt in allen Fällen. **Entscheidung für den Planner:** (A) `last_sent_at = MAX(entry.date)` über den bestehenden DAO — kein neuer Code, akzeptiert `sent_at` für gesendete Zeilen; ODER (B) dedizierte DAO-Methode `SELECT MAX(created) WHERE application_id = ? AND deleted IS NULL` für strikte `created`-Semantik. Empfehlung: (A) reicht für den Guard-Zweck; (B) nur wenn strikte D-07-Wörtlichkeit gefordert ist. **Diese Entscheidung sollte in der Planung explizit getroffen werden.**
**Warning signs:** Test „nach Enqueue (vor Worker) ist last_sent_at gesetzt" muss grün sein — beide Optionen erfüllen das.

### Pitfall 2: Application-UUID darf nie in `member_id` landen (Phase-29-Namespace-Gate)
**What goes wrong:** `RecipientInput{ member_id: Some(app.id), ... }` würde die Application-UUID im Member-Namespace ablegen — bricht die Carry-over-Logik (`link_application_recipients_to_member`) und die Timeline-Trennung.
**Why it happens:** Copy-Paste vom Member-Send-Pfad.
**How to avoid:** Immer `member_id: None` + `application_id: Some(app.id)` (Muster `send_confirmation_mail`-Kommentar, `application.rs:132-136`; `RecipientInput`-Doku, `service.rs:57-60`). Phase 29 hat dafür ein Grep-Gate/Test — dieses gilt weiter; ein E2E-/Service-Test sollte assertieren, dass die erzeugte Recipient-Zeile `member_id IS NULL` hat.
**Warning signs:** Antragsteller-Kommunikation taucht fälschlich in einer Member-Timeline auf.

### Pitfall 3: Der bestehende Communications-Handler ist NICHT admin-gegatet (D-10)
**What goes wrong:** `get_member_communications` (`communication_rest.rs:120`) parst nur die ID und ruft den DAO — **kein** `check_permission`. Der `CommunicationRestState`-Trait exponiert nur `communication_dao()`, hat also gar keinen Zugriff auf `PermissionService`. Ein 1:1-Spiegel für Application wäre ungegatet und verletzt D-10.
**Why it happens:** Die Member-Timeline verlässt sich vermutlich auf Router-/Middleware-Gating; für die neue Route ist das nicht garantiert.
**How to avoid:** Die Application-Communications-Route **in der Application-Service/REST-Schicht** bauen (z. B. neue `ApplicationService`-Methode `get_communications(id, context)` mit `check_permission(MANAGE_MEMBERS_PRIVILEGE)` + Handler in `genossi_rest/src/application.rs`), **nicht** als weiteren `communication_rest`-Handler. So wird der Admin-Gate explizit im Service erzwungen — konsistent mit `confirm`/`send_mail`. (Falls der Planner doch den `communication_rest`-Weg wählt, muss der Trait um Permission-Zugriff erweitert werden — mehr Aufwand.)
**Warning signs:** E2E-Test „Nicht-Admin → 403 auf GET /communications" schlägt fehl.

### Pitfall 4: `resolve_rendered_content`-Signatur wird an ZWEI Call-Sites konsumiert
**What goes wrong:** Man erweitert die Signatur (neue `ApplicationResolver`-/`ConfigService`-Params) und vergisst den zweiten Call-Site → Kompilierfehler oder (schlimmer) inkonsistentes Verhalten.
**Why it happens:** `resolve_rendered_content` wird sowohl vom Worker (`worker.rs:391`) als auch vom Startup-Backfill (`backfill.rs:71`) aufgerufen; der Backfill hat aktuell **keine** `ConfigService` in seiner Signatur.
**How to avoid:** Beide Call-Sites + die `genossi_bin`-Wiring (`start_mail_worker`-Aufruf `lib.rs:1640` und der Backfill-Aufruf) gemeinsam anpassen. `ApplicationResolver` als neuen `#[automock]`-Trait in `genossi_mail` definieren (mirror `MemberResolver`, `template.rs:12`), in `genossi_bin` über den `ApplicationDao` implementieren (`Application::from(&entity)` existiert). Für den Backfill ist der Application-Zweig praktisch nie aktiv (neue Sends persistieren rendered content sofort), aber die Signatur muss trotzdem kompilieren.
**Warning signs:** `cargo build -p genossi_mail` bricht an `backfill.rs`; oder Preview/Send-Rendering divergiert.

### Pitfall 5: `body_html` doppelt sanitisieren (D-05)
**What goes wrong:** Zusätzlicher ammonia-Aufruf im Service/Renderer, obwohl `create_job` bereits sanitisiert (`service.rs:417-423`).
**How to avoid:** `send_mail` reicht `body_html` roh an `create_job` — die Store-Boundary sanitisiert. Der Renderer arbeitet auf bereits-sanitisiertem `job.body_html`. Nicht erneut sanitisieren.
**Warning signs:** Erwartete `<b>`/`<ul>`-Struktur wird doppelt escaped/gestrippt.

## Code Examples

### Config-Kette (Vorbild send_confirmation_mail) — die 5 Application-Kontext-Keys
```rust
// Source: genossi_service_impl/src/application.rs:55-97 (send_confirmation_mail)
let share_value_cents = config.get("share_value_cents").await?.value.parse::<i64>()?;
let bank_iban  = config.get("bank_iban").await?.value.to_string();
let bank_name  = config.get("bank_name").await?.value.to_string();
let bank_bic   = config.get("bank_bic").await.ok().map(|e| e.value.to_string()); // optional
let geno_name  = config.get("genossenschaft_name").await?.value.to_string();
// NB: send_confirmation_mail behandelt Fehler mit tracing::error!+return (Anti-Pattern);
// send_mail MUSS diese in ServiceError umwandeln (echtes Result).
```

### ApplicationResolver-Trait (mirror MemberResolver, für D-04)
```rust
// Muster: genossi_mail/src/template.rs:10-14 (MemberResolver)
#[automock]
#[async_trait]
pub trait ApplicationResolver: Send + Sync + 'static {
    async fn find_application_by_id(&self, id: Uuid)
        -> Result<Option<genossi_service::application::Application>, MailServiceError>;
}
// genossi_bin implementiert über ApplicationDao (Application::from(&entity) existiert).
```

### Application-Struct-Felder (für Kontext + Guards)
```rust
// Source: genossi_service/src/application.rs:12-24
pub struct Application {
    pub id: Uuid,
    pub first_name: Arc<str>, pub last_name: Arc<str>,
    pub salutation: Option<Salutation>, pub title: Option<Arc<str>>,
    pub email: Option<Arc<str>>,        // D-12: None → 400
    pub shares: i32,                     // open_amount = share_value_cents * shares
    pub status: ApplicationStatus,       // APCMP-01: nur Offen → 409 sonst
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `member_id`-Overload für Antragsteller-Mails | dediziertes `application_id` auf `RecipientInput`/`MailRecipient` | Phase 29 | Namespace-Trennung; Phase 31 stempelt `application_id` |
| Member-Kontext mit gelöschten Feldern | eigener `application_to_template_context` | Phase 30 | Phase 31 nutzt ihn im Renderer-Zweig |
| stiller `()`-Confirmation-Mail-Send | echtes `Result`-basiertes `send_mail` | Phase 31 (diese Phase) | Vorstand sieht Fehler statt stiller 200 |

**Deprecated/outdated:** Kein zweiter Render-Pfad für Preview (D-06) — ein Renderer-Seam ist der aktuelle Stand seit dem `resolve_rendered_content`-Extract (Quick 260614-b1t, `render.rs:1-20`).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `last_sent_at` via `MAX(entry.date)` über `get_application_communications` genügt dem D-07-Guard-Zweck trotz `COALESCE(sent_at, created)` | Pitfall 1 | Falls strikte `created`-Semantik gefordert: dedizierte `MAX(created)`-DAO-Query nötig (kleiner Mehraufwand). Planner-Entscheidung. |
| A2 | Ein neuer `ApplicationResolver`-Trait in `genossi_mail` (mirror `MemberResolver`) ist der saubere Weg, damit der Renderer die Application lädt | Pattern 3 / Pitfall 4 | Alternativ könnte der `ApplicationDao` generisch durchgereicht werden; Resolver-Muster ist konsistenter, aber die genaue Form ist Discretion. |
| A3 | Die Application-Communications-Route gehört in die Application-REST/Service-Schicht (nicht `communication_rest.rs`), um D-10-Gate zu erzwingen | Pitfall 3 | Falls stattdessen `communication_rest` erweitert wird, muss dessen State-Trait Permission-Zugriff bekommen — anderer, größerer Umbau. |
| A4 | `ConfigService::get(key)` liefert `entry.value` als String zum Parsen (wie send_confirmation_mail es nutzt) | Config-Kette | Verifiziert an `application.rs:55-97`; Key-Namen exakt `share_value_cents`/`bank_iban`/`bank_name`/`bank_bic`/`genossenschaft_name`. |

**Hinweis:** A1 und A3 sind die zwei Punkte, die vor/während der Planung bewusst entschieden werden sollten.

## Open Questions (RESOLVED)

> Alle drei Fragen wurden bei der Planung verbindlich entschieden (siehe `31-02-PLAN.md` §Objective, OQ1–OQ3). Die Empfehlungen wurden jeweils übernommen.

1. **`last_sent_at`: `date` (COALESCE) oder strikt `created`?**
   - What we know: DAO liefert `COALESCE(sent_at, created)`; funktional erfüllt das den Guard.
   - What's unclear: Ob D-07 strikt `created` verlangt oder ob „ist beim Absenden sofort gesetzt" ausreicht.
   - Recommendation: Option A (MAX über bestehende `date`) — kein neuer DAO-Code; nur bei striktem Bedarf Option B.
   - **RESOLVED → Option A** (31-02, OQ1): `last_sent_at = MAX(entry.date)` über das bestehende `get_application_communications`; kein neuer DAO-Code.

2. **`last_sent_at`-Transport: Feld auf `get_application`-Response oder eigener Endpoint?**
   - What we know: D-08 fordert nur „serverseitig aggregiert"; Transport ist Claude's Discretion.
   - Recommendation: Feld auf der bestehenden `ApplicationTO`/`get_application`-Response (spart einen Roundtrip; Phase 32 zeigt es prominent). Prüfen, ob `get_application` dann eine Extra-Query braucht.
   - **RESOLVED → Feld auf `get_application`** (31-02 OQ2 + 31-03): serverseitig aggregiert über die neue `ApplicationService::last_sent_at`-Methode.

3. **Preview-Endpoint-Ort: Application-REST (`genossi_rest`) oder Mail-REST (`genossi_mail`)?**
   - What we know: Member-`preview_mail` liegt in `genossi_mail/src/rest.rs`; Application braucht aber ApplicationDao+Config.
   - Recommendation: In `genossi_rest/src/application.rs` (nutzt Application-Service + Admin-Gate konsistent), rendert über die geteilte reine Helferfn aus D-04.
   - **RESOLVED → `genossi_rest/src/application.rs`** (31-02 OQ3 + 31-03): admin-gated über den Application-Service, Render über den geteilten `render_application_content`-Kernel.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust-Toolchain (cargo/rustc/rustfmt) | Build/Test | ✓ (nur via `nix develop`) | 2021 edition | — (Memory: Toolchain nur via `nix develop --command`) |
| `genossi_mail`/`genossi_service_impl`/`genossi_rest` Crates | gesamte Phase | ✓ | workspace | — |
| SQLite | E2E-Tests (in-memory) | ✓ | — | — |
| SMTP-Server | Live-Delivery | ✗ (nicht in Tests) | — | Worker markiert `outbound_status="failed"`; Tests prüfen Enqueue + Guards, nicht echten SMTP |

**Missing dependencies with no fallback:** Keine — die Phase ist reines Service/REST-Backend auf bestehendem Stack.
**Wichtig (aus Memory):** Build/Test IMMER über `nix develop --command cargo ...`; `cargo`/`node` fehlen auf dem Base-PATH. Für isolierte Service-Tests aus Phase 30: `nix develop --command cargo test -p genossi_service --features utoipa` (bzw. `-p genossi_service_impl` / `-p genossi_mail`). `cargo fmt` reformatiert ~24 fremde Dateien — gezielt formatieren/prüfen.

## Security Domain

`security_enforcement` ist in `.planning/config.json` nicht explizit `false` → aktiv. Diese Phase ist sicherheits-/datenschutzrelevant (Admin-Gate + DSGVO-Rechtsgrundlage).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control (verifiziert im Repo) |
|---------------|---------|-----------------|
| V1 Access Control | ja | `check_permission(MANAGE_MEMBERS_PRIVILEGE)` in Service, ZUERST (CR-02, D-10/D-11) für send/preview/communications |
| V5 Validation/Sanitization | ja | `body_html` ammonia-sanitisiert an Store-Boundary (`create_job`, `service.rs:417`); strict-env minijinja verhindert Platzhalter-Injection |
| V7 Error Handling | ja | Echtes `Result` (D-01/D-02) — keine stille Fehlerschluckung; Fehler → definierte HTTP-Codes |
| V8 Data Protection (DSGVO) | ja | `Offen`-only-Guard = transaktionale Rechtsgrundlage (APCMP-01); Empfänger fest = `app.email` (kein Freitext, D-13); kein Tracking |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Nicht-Admin sendet/liest Antragsteller-Mails | Elevation of Privilege / Info Disclosure | `check_permission` zuerst (D-10/D-11); Handler übergibt `extract_auth_context(context)?` |
| Mail an abgelehnte/bestätigte Antragsteller (Rechtsgrundlage entfällt) | — / Compliance | Status-Guard `Offen`-only → 409 (APCMP-01) |
| Freitext-Empfänger / Massenversand | Tampering / Spam | Empfänger fest `app.email`, single-recipient `create_job` (D-13); per Test verifiziert |
| HTML-Injection über `body_html` | Tampering (XSS im Client) | ammonia an Store-Boundary + autoescape-`html_env` beim Render |
| Application-UUID leakt in Member-Namespace | Info Disclosure | `member_id: None` + `application_id: Some` (Pitfall 2); Phase-29-Gate |
| Stille 200 verdeckt Fehlversand | Repudiation | echtes `Result` + `outbound_status`-Sichtbarkeit (APMAIL-02) |

## Sources

### Primary (HIGH confidence — direkt gegen Live-Code verifiziert)
- `genossi_mail/src/render.rs:69-217` — `resolve_rendered_content`-Signatur + Zweig-Dispatch (nur Member/passthrough; Application-Zweig fehlt noch)
- `genossi_mail/src/service.rs:54-61,97-108,371-489` — `RecipientInput`, `MailService::create_job`-Signatur, ammonia-Store-Boundary
- `genossi_mail/src/worker.rs:200-232,384-417` — `start_mail_worker`-Signatur + `resolve_rendered_content`-Konsum (Call-Site 1)
- `genossi_mail/src/backfill.rs:25-90` — `run_rendered_backfill` (Call-Site 2, ohne ConfigService)
- `genossi_mail/src/template.rs:12-14,79-101` — `MemberResolver`-Muster, `application_to_template_context`-Signatur
- `genossi_mail/src/dao.rs:277-311` — `CommunicationEntry`-Felder + `get_application_communications`-Trait
- `genossi_mail/src/dao_sqlite.rs:1128` — SQL: `date = COALESCE(r.sent_at, r.created)` (Pitfall 1)
- `genossi_mail/src/communication_rest.rs:29-156` — `CommunicationEntryTO`, `get_member_communications` (KEIN Permission-Gate — Pitfall 3)
- `genossi_mail/src/rest.rs:649-799` — `preview_mail`-Handler (Preview-Muster, D-06)
- `genossi_service_impl/src/application.rs:26-160,290-321` — `ApplicationServiceDeps` (ConfigService+MailService bereits verdrahtet), `send_confirmation_mail` (Anti-Vorbild + Config-Kette), `confirm` (CR-02-Ordering)
- `genossi_service/src/application.rs:12-24,107-155` — `Application`-Struct + `ApplicationService`-Trait
- `genossi_rest/src/application.rs:325-500` — Handler-Muster (`confirm_application`), `generate_route`, `ApplicationApiDoc`
- `genossi_rest/src/lib.rs:634-666` — Route-Mounting (`/api/mail`, `/api/applications`, communications-nest)
- `genossi_bin/src/lib.rs:1614-1640,1956-1958` — `start_mail_worker`-Wiring, `CommunicationRestState`-Impl
- `genossi_service/src/euro.rs:26` — `format_eur_de`

### Secondary
- `.planning/phases/31-.../31-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`

### Tertiary
- Keine (kein Web-Research nötig — reines Internal-Codebase-Verifikations-Research)

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — jede Datei:Zeile gelesen; keine externen Pakete.
- Architecture/Seams: HIGH — alle 10 Seams aus dem Research-Fokus verifiziert; einziger Graubereich ist die `last_sent_at`-Semantik (bewusst als Entscheidung markiert).
- Pitfalls: HIGH — alle fünf direkt aus dem Code abgeleitet (COALESCE-SQL, ungegateter Handler, zwei Call-Sites, Namespace, Doppel-Sanitize).

**Nyquist-Validation:** In `config.json` `nyquist_validation: false` → Abschnitt „Validation Architecture" bewusst ausgelassen. Tests trotzdem verpflichtend (User-Global-CLAUDE.md: „Always make sure you have tests"); siehe Task-5-Skizze.

**Research date:** 2026-08-20
**Valid until:** ~2026-09-20 (stabile interne Codebase; nur bei Umbau von `genossi_mail` neu prüfen)
