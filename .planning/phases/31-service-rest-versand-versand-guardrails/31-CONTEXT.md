# Phase 31: Service + REST Versand (Versand + Guardrails) - Context

**Gathered:** 2026-08-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Der Vorstand kann einer **Application mit Status `Offen`** eine einzelne E-Mail senden —
mit **echtem Erfolg/Fehler-Feedback** (nicht das stille `()`-Pattern), **Admin-Gate**,
**Status-/DSGVO-Guard** (`Offen`-only → 409) — und der Anti-Doppelversand-Guard
(„zuletzt gesendet am …") bekommt seine serverseitig aggregierten Daten.

Konkret (reines Backend, Service + REST):
- `ApplicationService::send_mail(...) -> Result<(), ServiceError>`
- Application-Zweig im **Worker-Renderer** (`resolve_rendered_content`), damit die geseedete
  „Zahlungserinnerung" per-recipient über `application_to_template_context` (Phase 30) rendert
- `POST /api/applications/{id}/mail` (Versand) + `GET /api/applications/{id}/communications`
  (outbound-Historie) + Application-**Preview**-Render-Endpoint, alle admin-only
- serverseitiges `last_sent_at` (aus outbound-Historie) für den Detail-Guard
- Service-/E2E-Tests

**Nicht in dieser Phase:** Frontend-Compose-Dialog, „E-Mail senden"-Button, Last-Sent-Anzeige,
UI-Preview-Verdrahtung (alles Phase 32). Kein Massenversand, kein Freitext-Empfänger, kein
Tracking (Guardrails). Kein neues Audit-Feld auf `ApplicationEntity`. Keine neue
Backend-Dependency („add nothing" — pure Wiederverwendung `genossi_mail`).

</domain>

<decisions>
## Implementation Decisions

### Versand-Semantik & Fehler-Feedback (APMAIL-01/02)
- **D-01:** `ApplicationService::send_mail(...) -> Result<(), ServiceError>` — kein stilles `()`
  wie `send_confirmation_mail`. Der Versand bleibt **job-queue-basiert** (`create_job` → Worker
  sendet via SMTP). **Synchron als Fehler zurückgegeben:** Permission (403), Not-Found (404),
  Status ≠ `Offen` (409), fehlende Adresse (400), enqueue-Fehler (500). **SMTP-/Render-Delivery**
  passiert per-recipient im Worker → `outbound_status = "failed"`, sichtbar in der
  Kommunikations-Historie (nicht in der POST-Response).
- **D-02 (1b):** **KEIN** expliziter SMTP-Config-Präsenz-Pre-Flight beim Versand — der
  SMTP-Server wird bei der Konfiguration ohnehin getestet. Delivery-Scheitern äußert sich im
  Recipient-`outbound_status`, nicht als synchroner Fehler. Der Kontrast zu
  `send_confirmation_mail` ist: `send_mail` **schluckt Fehler nicht still** (echtes `Result` +
  sichtbarer Recipient-Status), nicht dass SMTP synchron getestet wird.

### send_mail-Signatur & Rendering (2a=A, 2b)
- **D-03 (2a=A):** `send_mail` nimmt **rohen** Content — `subject` + `body` (+ optional
  `body_html`) mit Platzhaltern — **genau wie die anderen Mail-Sends** (keine Client-Auflösung).
  Es stempelt `RecipientInput { address: application.email, member_id: None,
  application_id: Some(app.id) }` und ruft `create_job`. `template_id` kann wie im Member-Pfad
  mitgeführt werden.
- **D-04 (KERN-Seam):** Der **Worker-Renderer** `resolve_rendered_content`
  (`genossi_mail/src/render.rs`, konsumiert in `worker.rs:391`) bekommt einen
  **Application-Zweig**: wenn `recipient.application_id.is_some()`, wird der Kontext über
  `application_to_template_context` (Phase 30) gebaut — Config (`share_value_cents`, `bank_iban`,
  `bank_name`, `bank_bic`, `genossenschaft_name`) einmal aufgelöst, `open_amount` via
  `format_eur_de`. Damit rendert die geseedete „Zahlungserinnerung" per-recipient korrekt. Das
  ist der zentrale Backend-Seam der Phase — sowohl Send (Worker) als auch Preview konsumieren ihn.
- **D-05 (2b):** **Keine** strict-Render-/`validate_application_template`-Prüfung beim Versand —
  `validate_application_template` (Phase 30) gatet bereits bei Template-Create/Update, ein
  kaputtes Template kann gar nicht gespeichert werden. `body_html` wird wie überall an der
  Store-Boundary ammonia-sanitisiert (bestehender Gate).

### Preview-Endpoint (2c — in Phase 31, Claude's Discretion zum genauen Shape)
- **D-06:** Der **Application-Preview-Render-Endpoint** wird in **Phase 31** (Backend) gebaut. Er
  rendert den Entwurf (subject+body) gegen denselben Application-Zweig aus D-04 und gibt die
  **aufgelöste Vorschau** zurück (Preview == Worker-Output garantiert, ein Renderer-Seam).
  Begründung: einziger synchroner REST-Konsument von `application_to_template_context`; hält alle
  Application-Mail-Endpoints (send / communications / preview) in einer kohärenten „Service +
  REST"-Scheibe zusammen. Phase 32 verdrahtet nur die UI (APMAIL-04). Der Endpoint spiegelt das
  bestehende Member-`/api/mail/preview`-Muster.

### „Zuletzt gesendet am …" (APHIST-02, 3a=A, 3b=B, 3c)
- **D-07 (3a=A):** „zuletzt gesendet" = **MAX über die outbound-Historie**, basierend auf dem
  **Enqueue-/`created`-Zeitpunkt** (nicht `sent_at`), damit der Anti-Doppelversand-Guard sofort
  beim Absenden greift — unabhängig vom Worker-Erfolg.
- **D-08 (3b=B):** Der Wert wird **serverseitig aggregiert** und als **dediziertes Feld/Service-
  Methode** geliefert (z. B. `last_sent_at: Option<...>`, angehängt an die Application-Detail-
  Response oder eigener kleiner Endpoint) — **keine Client-Aggregation**.
- **D-09 (3c):** `GET /api/applications/{id}/communications` liefert **volle Einträge inkl.
  Betreff** (wie die Member-Timeline). Der bestehende `CommunicationEntry` /
  `CommunicationEntryTO` (`subject`, `date`, `outbound_status`, `to_address`) wird **1:1
  wiederverwendet** — nur ein paralleler Handler (`get_application_communications`, DAO existiert
  seit Phase 29) + Admin-Gate ist nötig. **Kein** Body-Snapshot (APHIST-FUT-01, verschoben).

### Admin-Gate & Guards (4a=A, 4b=A, 4c)
- **D-10 (4a=A):** `send_mail`, Preview- und Communications-Endpoints verlangen
  `MANAGE_MEMBERS_PRIVILEGE` — exakt wie `confirm`/`reject` (Konsistenz; „admin" = diese Rolle).
  Nicht-Admin → 403.
- **D-11 (4c):** Reihenfolge wie bei `confirm` (CR-02-Muster): **Permission-Check zuerst**, dann
  Not-Found (404), dann Status-Guard `Offen`-only (409). Kein user-attributierbarer Seiteneffekt
  vor dem Permission-Check.
- **D-12 (4b=A):** Fehlende `application.email` → **sauberer Backend-Fehler** (400 /
  `ValidationError`) als Defense-in-Depth; Phase 32 deaktiviert zusätzlich den Button. Nie ein
  stiller Fehlversuch.

### Guardrails (APCMP-01/02 — bereits gesetzt, hier bestätigt)
- **D-13:** Kein Massenversand-Pfad, kein Freitext-Empfänger (immer `application.email`), kein
  Open-/Click-Tracking; Timeline outbound-only. Per Service-/E2E-Test verifiziert. Der
  `Offen`-only-Guard (D-11) ist zugleich die DSGVO-transaktionale Rechtsgrundlage-Grenze.

### Claude's Discretion
- Exakte Pfad-/Request-/Response-Shapes (Preview-Endpoint, send-Request-Body), solange das
  Member-Muster gespiegelt wird.
- Ob `last_sent_at` auf der bestehenden `get_application`-Response mitfährt oder als eigener
  kleiner Endpoint kommt — solange serverseitig aggregiert (D-08).
- OpenAPI-Doku-Detail, i18n-Keys, exakte ServiceError-Variante für die no-email-Validierung.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/ROADMAP.md` §"Phase 31" — Goal + 5 Success Criteria + Guardrails + Build-Order
- `.planning/REQUIREMENTS.md` — APMAIL-01/02 (Z. 14-15), APHIST-02 (Z. 29), APCMP-01/02
  (Z. 34-35); Entscheide D1/D2/D3 (Z. 69-71)
- `.planning/phases/30-application-template-kontext-antragsteller-vorlagen/30-CONTEXT.md` —
  Phase-30-Entscheide (D-04..D-14): Application-Kontext, `format_eur_de`, Seed „Zahlungserinnerung",
  `validate_application_template`
- `.planning/phases/29-dao-schema-foundation-kommunikations-historie-pro-antragstel/29-CONTEXT.md`
  — Phase-29-Entscheide: `application_id`-Linkage, D2-Carry-over, `get_application_communications`

### Service-Layer (Versand-Seam)
- `genossi_service/src/application.rs` — `ApplicationService`-Trait (`send_mail` wird hier
  ergänzt), `Application`-Struct (`email`, `shares`, `status`, `salutation`, `title`)
- `genossi_service_impl/src/application.rs` — `send_confirmation_mail` (Z. 44, **Anti-Vorbild**:
  stilles `()` + Config-Kette `share_value_cents`/`bank_*`/`genossenschaft_name`); `confirm`
  (Z. 290, **Vorbild** Permission→NotFound→Status-409-Muster, CR-02-Ordering); `reject` (Z. 583)
- `genossi_service/src/euro.rs` (`format_eur_de`) + `application_to_template_context` (Phase 30)

### Mail-Subsystem (Wiederverwendung — „add nothing")
- `genossi_mail/src/render.rs` — `resolve_rendered_content` (**KERN**: hier kommt der
  Application-Zweig rein, D-04)
- `genossi_mail/src/worker.rs:384-476` — per-recipient Render+Send+Status-Persistenz (Vorbild +
  Konsument von `resolve_rendered_content`)
- `genossi_mail/src/service.rs` — `MailService::create_job` (Z. 97, Signatur),
  `RecipientInput { address, member_id, application_id }`, `link_application_recipients_to_member`
- `genossi_mail/src/dao.rs:277-311` — `CommunicationEntry` (subject/date/outbound_status) +
  `CommunicationDao::get_application_communications` (Phase 29, existiert)
- `genossi_mail/src/communication_rest.rs` — `CommunicationEntryTO` + `get_member_communications`-
  Handler + `CommunicationRestState` (**Vorbild** für den parallelen Application-Handler, D-09)

### REST-Layer
- `genossi_rest/src/application.rs` — bestehende Handler (`confirm_application` Z. 361 /
  `reject_application` Z. 397 mit 409-Doku; `generate_route` Z. 479) — hier kommen die neuen
  Routen (`/{id}/mail`, `/{id}/communications`, Preview) dazu
- Member-`/api/mail/preview`-Handler (Muster für den Application-Preview-Endpoint, D-06) — im
  `genossi_mail`/`genossi_rest` Mail-REST-Bereich lokalisieren

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`resolve_rendered_content`** (`render.rs`): zentraler per-recipient-Renderer; wird um den
  Application-Zweig erweitert (D-04) — Member/Repayment-Zweige bleiben unverändert.
- **`CommunicationEntry`/`CommunicationEntryTO`** + `get_member_communications`-Handler: trägt
  bereits `subject`/`date`/`outbound_status` → Application-Timeline (D-09) ist ein paralleler
  Handler ohne neue Felder.
- **`CommunicationDao::get_application_communications`** (Phase 29): outbound-only Query existiert
  bereits — Service+REST muss sie nur exponieren.
- **`confirm`** (`application.rs:290`): exaktes Permission→NotFound→Status-409-Muster (CR-02) für
  `send_mail` (D-11) 1:1 übernehmbar.
- **`send_confirmation_mail`** (`application.rs:44`): liefert die Config-Kette für den
  Application-Kontext — aber als **Anti-Vorbild** (stilles `()`), das `send_mail` bewusst nicht
  kopiert (D-01).

### Established Patterns
- **Job-Queue + per-recipient Worker-Render** (`worker.rs`): raw Body+Platzhalter im Job, Render
  erst beim Senden → Delivery-Fehler landen im Recipient-`outbound_status` (D-01/D-02).
- **`RecipientInput.application_id` Namespace-Trennung** (Phase 29): Application-UUID nie in
  `member_id` — Grep-Gate/Test aus Phase 29 gilt weiter.
- **CR-02 Permission-First-Ordering**: Permission-Check vor jedem user-attributierbaren
  Seiteneffekt (D-11).
- **`utoipa`-Feature beim isolierten Test:** `nix develop --command cargo test -p genossi_service
  --features utoipa` (aus Phase 30, STATE.md).

### Integration Points
- `resolve_rendered_content` ← neuer Application-Zweig (D-04): einziger Ort, der Send (Worker)
  UND Preview (D-06) mit `application_to_template_context` verbindet.
- `ApplicationServiceDeps` bekommt ggf. `MailService`-/`ConfigService`-Zugriff für `send_mail`
  (Config-Auflösung passiert allerdings im Renderer/Worker, nicht im Service — D-04).
- Neue Routen an `generate_route` in `genossi_rest/src/application.rs` + OpenAPI-Registrierung.
- `last_sent_at` (D-08): neue Service-Methode über `get_application_communications` (MAX `date`).

</code_context>

<specifics>
## Specific Ideas

- „Echter Fehler" heißt hier bewusst: `Result` statt stillem `()` **plus** sichtbarer
  Recipient-`outbound_status` — NICHT synchroner SMTP-Test. Der SMTP-Server ist zur Config-Zeit
  bereits getestet (User-Vorgabe, D-02).
- Die Preview soll garantiert das zeigen, was der Worker sendet — deshalb ein gemeinsamer
  Renderer-Seam (D-06), kein zweiter Render-Pfad.
- „zuletzt gesendet" ist ein Anti-Doppelversand-Guard, kein Delivery-Status → Enqueue-Zeit
  (`created`), nicht `sent_at` (D-07).

</specifics>

<deferred>
## Deferred Ideas

- **APMAIL-04 UI-Verdrahtung** (Live-Preview + confirm-before-send im Dialog) → Phase 32. Der
  Backend-Preview-Endpoint dafür wird aber schon in Phase 31 gebaut (D-06).
- **APMAIL-03 Button-Disable/Annotation bei fehlender Adresse** → Phase 32 (Backend-Guard 400
  ist Phase 31, D-12).
- **APHIST-FUT-01** Betreff-/Body-Snapshot je Timeline-Eintrag (Deep-Link auf exakt gesendeten
  Inhalt) — eigene Zukunfts-Phase.
- **APMAIL-FUT-01** Massen-Erinnerung an alle `Offen`-Antragsteller (Bulk-Send) — bewusst nach
  v1.6.

None weiter — Diskussion blieb im Phasen-Scope.

</deferred>

---

*Phase: 31-service-rest-versand-versand-guardrails*
*Context gathered: 2026-08-20*
