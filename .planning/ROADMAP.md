# Roadmap: Genossi

Mitgliederverwaltungs-Software für Genossenschaften. Aktiver Stand: fünf ausgelieferte Milestones — v1.0..v1.4 — plus v1.5 (Editor-Vervollständigung, Bild-Support & Vorschau, Phases 26-28, getestet & deployed) und aktiver Milestone **v1.6 Antragsteller-Kommunikation** (Phases 29-32). Offene Kandidaten siehe Backlog (999.x).

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phases 1-6 (Phase 5 SKIPPED — echte GV bereits durchgeführt) (shipped 2026-05-29)
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phases 7-13 (shipped 2026-06-02)
- ✅ **v1.2 Mitgliedschaft-Anpassungen** — Phases 14-18 (shipped 2026-06-07)
- ✅ **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** — Phases 19-21 (shipped 2026-06-28)
- ✅ **v1.4 Mail-Formatierung & Antrags-Dokumente** — Phases 22-25 (shipped 2026-07-03)
- 🚧 **v1.5 Editor-Vervollständigung, Bild-Support & Vorschau** — Phases 26-28 (getestet & deployed 2026-07-17)
- 🚧 **v1.6 Antragsteller-Kommunikation** — Phases 29-32 (planning 2026-08-12)

## Phases

<details>
<summary>✅ v1.0 GV-Anwesenheits-Erfassung (Phases 1-6) — SHIPPED 2026-05-29</summary>

- [x] Phase 1: Assembly-Aggregat + Audit-Hardening — completed
- [x] Phase 2: Helfer-Token + Session + AuthContext::Helper — completed
- [x] Phase 3: Attendance-Aggregat + Cascade-Invalidation — completed
- [x] Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback — completed
- [—] Phase 5: Pre-GV-Generalprobe — SKIPPED (echte GV bereits durchgeführt; Hotfixes lieferten echte Erkenntnisse zurück)
- [x] Phase 6: Teilnehmerlisten-Export für Generalversammlungen (PDF/CSV/XLSX) — completed

Archive: `.planning/milestones/v1.0-ROADMAP.md` · `v1.0-REQUIREMENTS.md` · `v1.0-MILESTONE-AUDIT.md`

</details>

<details>
<summary>✅ v1.1 Anteile-Rückzahlungsphase (Phases 7-13, 56 plans) — SHIPPED 2026-06-02</summary>

- [x] Phase 7: RepaymentPhase Backend Foundation (Aggregat + Lifecycle + 5 Audit-Prozesse) — completed
- [x] Phase 8: RepaymentEntry + Auto-Befüllung (10 plans) — completed
- [x] Phase 9: Atomare Auszahlungs-Buchung (12-Schritt-Cascade) — completed
- [x] Phase 10: Massenmail + Template-Variablen (`{{ payout_amount }}`, `{{ share_count }}`, `{{ fiscal_year }}`) — completed
- [x] Phase 11: Export (PDF Auszahlungsliste mit 6-Spalten-Tabelle) — completed
- [x] Phase 12: Frontend Component-First (15 plans, 3-Tab-Detail-Page, Shared-Components) — completed
- [x] Phase 13: RepaymentLetter-Bulk-Anschreiben für Nicht-Email-Mitglieder — completed

Archive: `.planning/milestones/v1.1-ROADMAP.md` · `v1.1-REQUIREMENTS.md` · `v1.1-MILESTONE-AUDIT.md`

</details>

<details>
<summary>✅ v1.2 Mitgliedschaft-Anpassungen während des Geschäftsjahres (Phases 14-18, 24 plans) — SHIPPED 2026-06-07</summary>

- [x] Phase 14: DAO/Domain Foundation (4 plans) — `compute_effective_date` Pure-Function + `RepaymentEntryDao::find_by_member_and_phase` + `/transfer-recipients`-Endpoint mit `MemberSlimTO` — completed 2026-06-04
- [x] Phase 15: Service+REST: Kündigung + Aufstockung (4 plans) — `MembershipAdjustService`-Trait + `cancel_membership` + `increase_shares` + `recalc_dates`-Free-Function-Refactor + 11 E2E-Tests — completed 2026-06-04
- [x] Phase 16: Service+REST: Teil-Rückgabe + Auto-Anlegen-Phase (5 plans, inkl. Gap-Closure 16-05) — `partial_repayment` mit 14-Schritt-Pipeline + Closed-Phase-Status-Guard + Auto-Fill-Skip-Pattern — completed 2026-06-05
- [x] Phase 17: Service+REST: Übertrag (4 plans) — `transfer_shares` 15-Schritt-Single-Tx-Cascade + Voll-Übertrag-Austritts-Cascade + 8 E2E + 2 Race-Patterns — completed 2026-06-06
- [x] Phase 18: Frontend Component-First (7 plans in 3 Wellen) — `MembershipAdjustModal` (1078 LOC) + 4 Sub-Views + `FiscalYearDateInput` + ToastVariant Success + Vorstand-UAT-Sign-Off — completed 2026-06-07

Archive: `.planning/milestones/v1.2-ROADMAP.md` · `v1.2-REQUIREMENTS.md` · `v1.2-MILESTONE-AUDIT.md`

</details>

<details>
<summary>✅ v1.3 Posteingang-Benachrichtigung & Reply-Komfort (Phases 19-21, 11 plans) — SHIPPED 2026-06-28</summary>

**Goal:** Vorstände verpassen keine eingehenden Mails mehr und können bequemer auf sie antworten.

- [x] Phase 19: E-Mail-Anhänge anzeigen (Vorläufer, geshippt 2026-06-09)
- [x] Phase 20: Inbox-Digest — täglicher Posteingangs-Benachrichtigungs-Worker (DIGEST-01..07) — completed 2026-06-26
- [x] Phase 21: Reply-Komfort — Antwort im vollflächigen Modal (REPLY-01..04) — completed 2026-06-28 (Code-Review fand+fixte 1 Critical; Live-Smoke-Test bestanden)

Archive: `.planning/milestones/v1.3-ROADMAP.md` · `v1.3-REQUIREMENTS.md` · `v1.3-MILESTONE-AUDIT.md`

</details>

<details>
<summary>✅ v1.4 Mail-Formatierung & Antrags-Dokumente (Phases 22-25, 16 plans) — SHIPPED 2026-07-03</summary>

**Goal:** Vorstände versenden professionell formatierte HTML-Mails (statt nur Rohtext) und können den originalen Mitgliedsantrag als Datei am Antrag hinterlegen, die beim Aktivieren automatisch ans Mitglied übergeht.

- [x] Phase 22: 8bit + Shared Mail-Body Helper (3 plans) — Shared `build_message` factory + opt-in 8bit encoding + docs/OPERATIONS.md runbook (MAIL-01..05) — completed 2026-07-02
- [x] Phase 23: HTML Mail Backend (4 plans) — `multipart/alternative` + ammonia sanitization + html_env autoescape + FMT-01 German date format (HTML-01..05, FMT-01) — completed 2026-07-02
- [x] Phase 24: WYSIWYG Frontend Editor (4 plans) — Reusable Dioxus contenteditable component + toolbar/link-dialog + preview HTML render (EDIT-01..05) — completed 2026-07-03 (UAT smoke deferred)
- [x] Phase 25: Application File Upload + Audited Carryover (5 plans) — application_documents table + service + REST endpoints + audited MemberDocument carryover at confirm() + CR-02 fix + Frontend slot component (APDOC-01..05) — completed 2026-07-03 (UAT smoke deferred)

Archive: `.planning/milestones/v1.4-ROADMAP.md` · `v1.4-REQUIREMENTS.md` · `v1.4-MILESTONE-AUDIT.md`

</details>

<details>
<summary>🚧 v1.5 Editor-Vervollständigung, Bild-Support & Vorschau (Phases 26-28) — GETESTET & DEPLOYED 2026-07-17</summary>

**Goal:** Der WYSIWYG-Editor bekommt vollen Formatierungs-Umfang (Listen, Überschriften), Vorstand kann Inline-Bilder direkt im Editor hochladen und in HTML-Mails einbetten, und das gerenderte HTML lässt sich in Desktop-/Mobile-Vorschau prüfen bevor die Mail versendet wird.

- [ ] **Phase 26: Editor-Formatierung vervollständigen** — Listen (ul/ol), Überschriften (H2/H3), Toolbar-Erweiterung + Grep-Gate; v1.4-Phase-24-UAT-Checklist wird im gleichen Zug abgehakt (EDIT-06..10)
- [ ] **Phase 27: Bild-Support Backend + Editor-Upload** — `mail_asset`-Entität (kein Audit) + Upload-REST + Bytes-REST + ammonia `<img data-genossi-asset-id>`-Regel + CID-Renderer + `multipart/related` (IMG-01..09)
- [ ] **Phase 28: Desktop/Mobile-Vorschau** — sandboxed iframe-Preview mit umschaltbaren Breakpoints (~640px / ~360px), rendert ammonia-sanitisierte HTML mit Bildern via `/api/mail/assets/{id}/bytes` (PREV-01..05)

**Build order:** 26 → 27 → 28 strikt sequenziell. Phase 27 (Bild-Support) fasst dieselben `ammonia`-Regeln an, die Phase 26 erweitert — Sequenzierung vermeidet Merge-Konflikte auf `sanitize.rs`. Phase 28 (Preview) braucht Phase 27's `/api/mail/assets/{id}/bytes`-Endpoint, um Bilder in der Vorschau zu rendern.

**Audit scope (v1.5):** Kein Audit-Log für die neue `mail_asset`-Entität (Non-Kern-Entität, analog `application_documents`-Pattern aus v1.4 Phase 25). Bestehende auditierte Entitäten (Member/MemberAction/MemberDocument/Application) bleiben unverändert. Neue Backend-Dependency: keine — `ammonia` (Phase 23) wird nur um Regeln erweitert.

**Backward-Compat:** v1.4-Templates ohne Bilder senden weiterhin OHNE `multipart/related`-Wrapper (IMG-09). Bestehende WYSIWYG-Component wird erweitert, nicht ersetzt. Ammonia bleibt server-side only (kein WASM-Bundle).

Archive: TBD (bei Milestone-Close)

</details>

<details open>
<summary>🚧 v1.6 Antragsteller-Kommunikation (Phases 29-32) — PLANNING 2026-08-12</summary>

**Goal:** Vorstände können Personen mit abgegebener Beitrittserklärung (Applications) direkt per E-Mail kontaktieren — insbesondere Zahlungserinnerungen —, mit wiederverwendbaren Vorlagen und nachvollziehbarer Kommunikations-Historie, auch bevor die Person Mitglied ist.

- [x] **Phase 29: DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller)** — Migration nullable `application_id BLOB` + Index auf `mail_recipients`; `MailRecipient`/`RecipientInput.application_id`; `create_job` threaded es durch; `CommunicationDao::get_application_communications` (outbound-only); Carry-over-Mechanik bei `confirm()` → Erinnerung erscheint in Mitglieds-Timeline (APHIST-01, APHIST-03) (completed 2026-08-12)
- [ ] **Phase 30: Application-Template-Kontext (Antragsteller-Vorlagen)** — eigener „Antragsteller"-Vorlagentyp via `application_to_template_context`; extrahierter generischer `validate_rendered`-Kern (Member-Tests bleiben grün); `format_eur_de`-Helper; `share_value_cents` aus derselben Config wie `send_confirmation_mail`; geseedete Standard-Vorlage „Zahlungserinnerung" (APTPL-01..04)
- [ ] **Phase 31: Service + REST Versand (Versand + Guardrails)** — `ApplicationService::send_mail` → `Result<_, ServiceError>` (nicht das stille `()`-Pattern); Status-Guard `Offen`-only (409); `POST /api/applications/{id}/mail` + `GET /api/applications/{id}/communications`, admin-only; „zuletzt gesendet"-Daten; Service-/E2E-Tests (APMAIL-01..02, APCMP-01..02, APHIST-02)
- [ ] **Phase 32: Frontend Compose-Dialog** — `api.rs`-Funktionen (dediziert, nicht member-umgeleitet); neuer Application-Mail-Compose-Dialog mit Wiederverwendung `mail_compose/*` + `communication_timeline.rs`; „E-Mail senden"-Button + Last-Sent-Anzeige auf `application_detail.rs`; Live-Preview + Confirm-before-send; deaktiviert-ohne-Adresse & deaktiviert-während-pending (APMAIL-03..04, APUI-01..03)

**Build order:** 29 (Schema/Linkage) und 30 (Template-Kontext) können parallel laufen, müssen aber beide vor 31 landen. 31 (Service + REST) hängt an 29+30. 32 (Frontend) hängt an 31. Harte Dependency-Kette: das `application_id`-Feld muss existieren und persistiert werden, bevor der Service es stempeln kann; die Endpoints müssen existieren, bevor der Dialog etwas aufrufen kann.

**Load-bearing Entscheidungen (aus Research + Produkt-Entscheiden 2026-08-12):**

- **D1 — Eigener „Antragsteller"-Vorlagentyp** (getrennt vom Member-Pool) — vermeidet die strict-render-Bombe durch Member-only-Platzhalter. → Phase 30
- **D2 — Historie-Carry-over bei `confirm()`** — als Antragsteller gesendete Erinnerungen erscheinen nach der Bestätigung in der Mitglieds-Timeline. Konkreter Mechanismus (Back-fill `member_id` / Union-at-read / Link-Spalte) wird in Phase-29-Planung festgelegt; e2e-verifiziert: Erinnerung → confirm → sichtbar in Member-Timeline. → Phase 29
- **D3 — `share_value_cents`-Quelle** — dieselbe Config-Quelle wie `send_confirmation_mail` (Konsistenz). → Phase 30

**Audit scope (v1.6):** Kein Audit-Log für die neue `application_id`-Linkage oder Mail-/Communication-Entities (reine Non-Kern-Mail-Daten). Bestehende auditierte Entitäten (Member/MemberAction/MemberDocument/Application) bleiben unverändert — insbesondere KEIN neues Zahlungsstatus-Feld auf dem auditierten `ApplicationEntity` (offener Betrag wird berechnet, nie gespeichert). Neue Backend-Dependency: keine („add nothing" — pure Wiederverwendung des bestehenden Mail-/Template-/Communication-Subsystems).

**Guardrails (DSGVO/Content-Scoping):** Versand nur bei Status `Offen` (409 sonst) — deckt die transaktionale Rechtsgrundlage. Kein Massenversand, kein Freitext-Empfänger (immer `application.email`), keine Newsletter/Marketing-Inhalte, kein Open-/Click-Tracking. Timeline ist outbound-only (Antragsteller haben keine Inbound-Zuordnung).

Archive: TBD (bei Milestone-Close)

</details>

## Phase Details

### Phase 26: Editor-Formatierung vervollständigen

**Goal:** Vorstand kann im WYSIWYG-Editor Listen und Überschriften wie in einer normalen Text-Verarbeitung setzen — die Formatierung überlebt Save/Reload und ammonia-Sanitization ohne Verlust.
**Depends on:** Phase 25 (v1.4 WYSIWYG-Component + ammonia-Sanitize-Pipeline)
**Requirements:** EDIT-06, EDIT-07, EDIT-08, EDIT-09, EDIT-10
**Success Criteria** (what must be TRUE):

  1. Vorstand kann im Editor ungeordnete UND geordnete Listen (`<ul>`/`<ol>`) via Toolbar setzen; nach Save→Reload sind die Listen-Elemente unverändert im Body.
  2. Vorstand kann Überschriften H2 und H3 via Toolbar setzen; nach Save→Reload sind die Header-Elemente unverändert im Body und werden in der Empfänger-Mail sichtbar gerendert.
  3. Ammonia-Sanitize verliert weder Listen- noch Überschriften-Struktur; ein Grep-Gate analog EDIT-01/02 verifiziert `styleWithCSS=false`-Konsistenz für die neuen Toolbar-Buttons.
  4. v1.4 Phase-24-UAT-Checklist (3 HARD FAIL GATES: styleWithCSS=false-Bold, Paste-Plain, In-App-Modal statt window.prompt) wird im gleichen Vorstand-Smoke-Test mit-abgehakt und der Live-Preview-Render sowie die multipart/alternative-Delivery bestätigt.
  5. Bestehende v1.4-Templates ohne Listen/Überschriften rendern byte-identisch weiter (Backward-Compat auf sanitize.rs).

**Plans:** 3/3 plans executed

- [x] 26-01-PLAN.md — Backend Round-Trip Tests: 3 ammonia-Unit-Tests (UL/OL/H1-H3) + 1 E2E-Template-Round-Trip (EDIT-06, EDIT-07, EDIT-08)
- [x] 26-02-PLAN.md — Frontend Grep-Gate: 2 include_str!-Source-Invariant-Tests für styleWithCSS + onpaste (EDIT-09)
- [x] 26-03-PLAN.md — UAT-Checklist Nachhol + Erweiterung: Copy Phase-24-Checkliste + 4 neue Steps für UL/OL/H2/H3 (EDIT-10)

**UI hint:** yes

### Phase 27: Bild-Support Backend + Editor-Upload

**Goal:** Vorstand kann Inline-Bilder direkt im WYSIWYG-Editor hochladen und in HTML-Mails einbetten; die Empfänger sehen die Bilder in der Mail (inklusive Test-Mail an den Vorstand selbst).
**Depends on:** Phase 26
**Requirements:** IMG-01, IMG-02, IMG-03, IMG-04, IMG-05, IMG-06, IMG-07, IMG-08, IMG-09
**Success Criteria** (what must be TRUE):

  1. Vorstand kann im Editor ein PNG/JPEG/GIF-Bild (bis 5 MB) per Drag&Drop ODER Toolbar-Button hochladen; der Editor zeigt das Bild sofort per `/api/mail/assets/{id}/bytes`-URL an.
  2. Beim Mail-Versand wird das Bild als CID-Referenz (`cid:asset-X@genossi`) in die HTML-Mail geschrieben und als `multipart/related`-Inline-Part angehängt; Gesamt-Mail-Struktur ist `multipart/mixed → multipart/related → multipart/alternative`.
  3. Test-Mail an den Vorstand rendert das Bild im echten Mail-Client (Thunderbird, Outlook, Nextcloud-Webmail) korrekt — kein „broken image"-Icon.
  4. Externe HTTP-`src`, `data:`-URIs und SVG werden serverseitig via `ammonia`-Regel gestrippt; nur `<img data-genossi-asset-id="…">` ist erlaubt, `src` wird nur beim Rendern injiziert.
  5. Gesamt-Mailgröße wird gegen 25 MB Limit geprüft (klarer Fehler VOR SMTP), und bestehende v1.4-Templates ohne Bilder senden weiterhin OHNE `multipart/related`-Wrapper (Backward-Compat).

**Plans:** 4/4 plans executed

Plans:
**Wave 1**

- [x] 27-01-PLAN.md — `mail_asset`-Entität (DAO/SQLite-BLOB/Service/REST/TO/Migration/DI) + Admin-Gate + Magic-Byte-MIME-Sniff + Upload/Bytes-REST (IMG-01, IMG-02, IMG-04)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 27-02-PLAN.md — ammonia `<img>`-Härtung: nur `data-genossi-asset-id`, strippt src/data:/SVG; Phase-26-Listen/Überschriften bleiben grün (IMG-05)
- [x] 27-03-PLAN.md — CID-Renderer (`rewrite_img_cids`) + `multipart/related` + 25-MB-base64-Check + Test-Mail + Backward-Compat, alle `send.rs`-Änderungen in einem Plan (IMG-06, IMG-07, IMG-08, IMG-09)
- [x] 27-04-PLAN.md — Frontend Editor-Upload: Toolbar-Bild-Button + Drag&Drop + FormData-Upload + insertHTML (IMG-03)

**UI hint:** yes

### Phase 28: Desktop/Mobile-Vorschau

**Goal:** Vorstand kann vor dem Versand die tatsächlich sanitisierte HTML-Mail in Desktop- und Mobile-Breite anschauen — Diskrepanzen zwischen dem Editor-DOM und der Empfänger-Sicht werden sofort sichtbar.
**Depends on:** Phase 27 (Assets-Bytes-Endpoint wird für Bilder in der Preview benötigt)
**Requirements:** PREV-01, PREV-02, PREV-03, PREV-04, PREV-05
**Success Criteria** (what must be TRUE):

  1. Vorstand kann im Editor zwischen den drei Modi „Bearbeiten", „Desktop-Vorschau" (~640px) und „Mobile-Vorschau" (~360px) umschalten; die Umschaltung ist visuell klar (z. B. Device-Rahmen), sodass ein versehentliches Tippen im Preview-Modus offensichtlich nichts editiert.
  2. Die Vorschau rendert den ammonia-sanitisierten HTML-Body (nicht das rohe `contenteditable`-DOM); dadurch werden Diskrepanzen — z. B. verlorene Attribute — sofort sichtbar, bevor die Mail versendet wird.
  3. Bilder in der Vorschau werden korrekt angezeigt: `data-genossi-asset-id="X"` wird zu `/api/mail/assets/{id}/bytes` aufgelöst (nur für authentifizierte Vorstands-Sessions).
  4. Preview läuft in einem sandboxed `<iframe>` mit fester Breite; kein CSS bleedet zwischen Editor und Vorschau in beide Richtungen (verifizierbar durch bewusst gesetzte Konflikt-Klassen im Editor-Umfeld).
  5. Alle Preview-Modi funktionieren mit bestehenden v1.4-Templates ohne Bilder (Backward-Compat) UND mit den neuen v1.5-Templates mit Listen/Überschriften/Bildern.

**Plans:** 4/5 plans executed

Plans:
**Wave 1**

- [x] 28-01-PLAN.md — Backend: ammonia-Sanitize vor dem Jinja-Rendering im `preview_mail`-Handler (D-01/D-02) plus vier e2e-Tests
- [x] 28-02-PLAN.md — Frontend-Primitive: `mail_preview_frame.rs` mit `PreviewMode`, `inject_asset_src`, `preview_srcdoc`, `MailPreviewFrame`, Sandbox-Grep-Gate und sieben i18n-Keys

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 28-03-PLAN.md — Editor-Integration: Drei-Modi-Umschalter, Toolbar-Ausblendung, Off-Screen-Hide statt Rendering-Unterdrückung, Preview-Fetch beim Moduswechsel

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 28-04-PLAN.md — Call-Site-Verkabelung: `preview_member_id` auf Page-Ebene gehoben, drei Call-Sites verkabelt (D-03 inkl. Ausstiegsklausel für `reply_form.rs`)

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 28-05-PLAN.md — UAT-Checkliste und Vorstands-Smoke-Abnahme der nicht automatisierbaren Punkte

**Waves:** 1 (28-01, 28-02 parallel) → 2 (28-03) → 3 (28-04) → 4 (28-05)
**UI hint:** yes

### Phase 29: DAO/Schema-Foundation (Kommunikations-Historie pro Antragsteller)

**Goal:** Alle an einen Antragsteller gesendeten Mails werden über eine eigene `application_id`-Linkage erfasst und bleiben auch nach der Bestätigung zum Mitglied in dessen Timeline sichtbar — ohne den `member_id`-Namespace zu vergiften.
**Depends on:** Nothing (erste Phase von v1.6; baut auf dem bestehenden Mail-/Communication-Subsystem `genossi_mail` auf)
**Requirements:** APHIST-01, APHIST-03
**Success Criteria** (what must be TRUE):

  1. Eine gesendete Mail mit gesetztem `application_id` (und `member_id: None`) wird persistiert und über `GET /api/applications/{id}/communications` (outbound-only) als Historie-Eintrag zurückgeliefert.
  2. Der `member_id`-Namespace bleibt sauber — eine Application-UUID landet niemals in `RecipientInput.member_id`; das ist per Test/Grep-Gate gegen Pitfall 5 abgesichert.
  3. Nach `confirm()` einer Application (→ neues Mitglied) erscheint die zuvor als Antragsteller gesendete Erinnerung in der Mitglieds-Timeline des neuen Mitglieds (e2e: Erinnerung → confirm → sichtbar), gemäß dem in der Planung festgelegten Carry-over-Mechanismus (D2).
  4. Die Migration ist forward-only und additiv (nullable `application_id BLOB` + Index auf `mail_recipients`); bestehende `mail_recipients`-Zeilen ohne `application_id` bleiben byte-identisch (NULL-Legacy-Roundtrip), und jede `mail_recipients`-SQL-Spaltenliste ist auf die neue Spalte geprüft.

**Plans:** 2/2 plans complete

Plans:
**Wave 1**

- [x] 29-01-PLAN.md — `application_id`-Linkage durch `genossi_mail` fädeln: additive Migration (nullable `application_id BLOB` + Index) + `MailRecipient`/`RecipientInput`-Feld + alle 6 `mail_recipients`-SQL-Spaltenlisten (inkl. Test-DDL) + `create_job`-Threading + Roundtrip-/NULL-Legacy-/Namespace-Tests (APHIST-01)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 29-02-PLAN.md — Timeline-Read + D2-Carry-over: `get_application_communications` (outbound-only) + `link_application_to_member` (UPDATE) + `MailService::link_application_recipients_to_member` + post-commit best-effort Carry-over-Hook in `confirm()` (D2 Option A) + e2e Erinnerung→confirm→sichtbar (APHIST-01, APHIST-03)

**Waves:** 1 (29-01) → 2 (29-02, hängt an 29-01 wegen application_id-Spalte + Datei-Overlap auf `genossi_mail`)

### Phase 30: Application-Template-Kontext (Antragsteller-Vorlagen)

**Goal:** Vorlagen können gegen einen eigenen Application-Kontext gerendert werden, und der Vorstand hat eine mitgelieferte deutsche „Zahlungserinnerung" mit korrekt berechnetem, korrekt formatiertem offenem Betrag.
**Depends on:** Nothing (entkoppelt von Phase 29; kann parallel laufen, muss aber vor Phase 31 landen)
**Requirements:** APTPL-01, APTPL-02, APTPL-03, APTPL-04
**Success Criteria** (what must be TRUE):

  1. Eine Vorlage rendert mit Application-Platzhaltern (Anrede, Vorname, Nachname, Titel, Anzahl Anteile, offener Betrag) über eine eigene `application_to_template_context`-Funktion — als eigener „Antragsteller"-Vorlagentyp, getrennt vom Member-Pool (D1).
  2. Der „offene Betrag" wird zur Laufzeit als `Anteile × share_value_cents` berechnet (Quelle: dieselbe Config wie `send_confirmation_mail`, D3), in korrektem deutschem Euro-Format angezeigt (Tausenderpunkt, Dezimalkomma, Null-/Negativ-Fall korrekt) und niemals auf der Application gespeichert.
  3. Eine deutsche Standard-Vorlage „Zahlungserinnerung" ist als Seed vorhanden und rendert den Haupt-Use-Case ohne manuelle Konfiguration.
  4. Die Validierung einer Antragsteller-Vorlage schlägt bei unbekannten oder Member-only-Platzhaltern kontrolliert fehl (kein `strict`-Render-Crash beim Versand); die ~40 bestehenden Member-Template-Tests bleiben grün (die `validate_template`-Signatur ändert sich nicht).

**Plans:** 3 plans

Plans:
**Wave 1** *(parallel — kein Datei-Overlap)*

- [ ] 30-01-PLAN.md — `format_eur_de` in `genossi_service` (Null/Negativ/Tausenderpunkt) + `send_confirmation_mail`-Retrofit (APTPL-02, D-11/D-12/D-13)
- [ ] 30-02-PLAN.md — `template_type`-Spalte durch alle 8 `mail_templates`-SQL/Struct/TO/Service-Sites fädeln + „Zahlungserinnerung"-Seed (UUID …0003) (APTPL-01, APTPL-03, D-01/D-02/D-03/D-14)

**Wave 2** *(blocked on Wave 1)*

- [ ] 30-03-PLAN.md — `application_to_template_context` + `dummy_application_context` + `validate_rendered`-Kern + `validate_application_template` + Create/Update-Injection + Seed-Render-Beweis (APTPL-01, APTPL-04, APTPL-03, D-04..D-10)

**Waves:** 1 (30-01, 30-02 parallel) → 2 (30-03, hängt an 30-01 + 30-02 wegen `format_eur_de`, `template_type`-Feld und Datei-Overlap auf `mail_template_service.rs`/`e2e_tests.rs`)

### Phase 31: Service + REST Versand (Versand + Guardrails)

**Goal:** Der Vorstand kann einer Application mit Status `Offen` eine einzelne E-Mail senden — mit echtem Erfolg/Fehler-Feedback, Admin-Gate und Status-/DSGVO-Guard — und der Anti-Doppelversand-Guard („zuletzt gesendet") hat seine Daten.
**Depends on:** Phase 29 (application_id-Linkage) + Phase 30 (Template-Kontext/Formatierung)
**Requirements:** APMAIL-01, APMAIL-02, APCMP-01, APCMP-02, APHIST-02
**Success Criteria** (what must be TRUE):

  1. `POST /api/applications/{id}/mail` versendet an `application.email` (`RecipientInput` mit `member_id: None` + gesetztem `application_id`), nur für Vorstand (`admin`-Rolle); Nicht-Admins erhalten 403.
  2. Der Versand gibt echten Erfolg/Fehler zurück (`ApplicationService::send_mail -> Result<_, ServiceError>`) — bei fehlendem SMTP / fehlender Config sieht der Vorstand einen Fehler, nie ein stilles 200-OK-ohne-Versand (nicht das `()`-schluckende `send_confirmation_mail`-Pattern).
  3. Versand ist nur bei Status `Offen` möglich; abgelehnte oder bereits bestätigte (jetzt Mitglied) Antragsteller liefern HTTP 409 (analog `confirm`/`reject`) — zugleich die DSGVO-Rechtsgrundlage-Grenze.
  4. Der Service liefert pro Application die „zuletzt gesendet am …"-Information (aus der outbound-Historie), damit der Anti-Doppelversand-/Spam-Guard auf der Detailseite angezeigt werden kann.
  5. Es gibt keinen Massenversand- und keinen Freitext-Empfänger-Pfad; Empfänger ist immer die Application selbst (Content-Scoping), kein Open-/Click-Tracking — verifiziert per Service-/E2E-Tests.

**Plans:** TBD

### Phase 32: Frontend Compose-Dialog

**Goal:** Der Vorstand kann auf der Application-Detailseite eine Erinnerung komponieren, in Live-Vorschau prüfen, bewusst bestätigen und absenden — mit sichtbarer Kommunikations-Historie und sauberem No-Email-Handling.
**Depends on:** Phase 31
**Requirements:** APMAIL-03, APMAIL-04, APUI-01, APUI-02, APUI-03
**Success Criteria** (what must be TRUE):

  1. Ein „E-Mail senden"-Button auf der Application-Detailseite öffnet einen Compose-Dialog (Vorbild-Pattern: `member_details.rs`); bei fehlender `application.email` ist der Button deaktiviert/annotiert, nie ein stiller Fehlversuch.
  2. Der Dialog nutzt die bestehenden `component/mail_compose/`-Bausteine (Betreff-Input, WYSIWYG-Editor, Template-Selector, Preview) — kein geforktes UI (Component-First); die API-Aufrufe sind dedizierte `api.rs`-Funktionen, nicht umgeleitete Member-Funktionen.
  3. Vor dem Absenden sieht der Vorstand eine Live-Vorschau mit aufgelösten Platzhaltern und bestätigt den Versand bewusst (confirm-before-send).
  4. Die Kommunikations-Historie wird über die unveränderte, prop-getriebene `communication_timeline.rs`-Komponente auf der Application-Detailseite/im Dialog angezeigt, inklusive prominenter „zuletzt gesendet am …"-Anzeige.
  5. Der Senden-Button ist während eines laufenden Requests deaktiviert (kein Doppelversand), und die Dioxus-`form onsubmit`-Reload-Falle wird via `div`+`onclick`+`r#type:"button"` vermieden.

**Plans:** TBD
**UI hint:** yes

## Progress

| Phase                                              | Milestone | Plans Complete | Status      | Completed  |
| -------------------------------------------------- | --------- | -------------- | ----------- | ---------- |
| 1. Assembly-Aggregat                               | v1.0      | -              | Complete    | 2026-05    |
| 2. Helfer-Token + Session                          | v1.0      | -              | Complete    | 2026-05    |
| 3. Attendance-Aggregat                             | v1.0      | -              | Complete    | 2026-05    |
| 4. Frontend Component-First                        | v1.0      | -              | Complete    | 2026-05    |
| 5. Pre-GV-Generalprobe                             | v1.0      | -              | SKIPPED     | -          |
| 6. Teilnehmerlisten-Export                         | v1.0      | -              | Complete    | 2026-05-29 |
| 7. RepaymentPhase Foundation                       | v1.1      | -              | Complete    | 2026-04+   |
| 8. RepaymentEntry + Auto-Befüllung                 | v1.1      | 10/10          | Complete    | 2026-05    |
| 9. Atomare Auszahlungs-Buchung                     | v1.1      | -              | Complete    | 2026-05    |
| 10. Massenmail + Template-Variablen                | v1.1      | -              | Complete    | 2026-05    |
| 11. Export (PDF)                                   | v1.1      | -              | Complete    | 2026-05    |
| 12. Frontend Component-First (v1.1)                | v1.1      | 15/15          | Complete    | 2026-05    |
| 13. RepaymentLetter-Bulk-Anschreiben               | v1.1      | -              | Complete    | 2026-06-02 |
| 14. DAO/Domain Foundation                          | v1.2      | 4/4            | Complete    | 2026-06-04 |
| 15. Service+REST: Kündigung + Aufstockung          | v1.2      | 4/4            | Complete    | 2026-06-04 |
| 16. Service+REST: Teil-Rückgabe + Auto-Anlegen     | v1.2      | 5/5            | Complete    | 2026-06-05 |
| 17. Service+REST: Übertrag (Atomare 2-Action)      | v1.2      | 4/4            | Complete    | 2026-06-06 |
| 18. Frontend Component-First (v1.2)                | v1.2      | 7/7            | Complete    | 2026-06-07 |
| 19. E-Mail-Anhänge anzeigen                        | v1.3      | 7/7            | Complete    | 2026-06-09 |
| 20. Inbox-Digest (täglicher Benachrichtigungs-Worker) | v1.3   | 3/3 | Complete    | 2026-06-27 |
| 21. Reply-Komfort (Antwort im Modal)               | v1.3      | 1/1 | Complete   | 2026-06-27 |
| 22. 8bit + Shared Mail-Body Helper                 | v1.4      | 3/3            | Complete    | 2026-07-02 |
| 23. HTML Mail Backend                              | v1.4      | 4/4            | Complete    | 2026-07-02 |
| 24. WYSIWYG Frontend Editor                        | v1.4      | 4/4            | Complete    | 2026-07-03 |
| 25. Application File Upload + Audited Carryover    | v1.4      | 5/5            | Complete    | 2026-07-03 |
| 26. Editor-Formatierung vervollständigen           | v1.5      | 3/3 | In Progress|  |
| 27. Bild-Support Backend + Editor-Upload           | v1.5      | 4/4 | In Progress|  |
| 28. Desktop/Mobile-Vorschau                        | v1.5      | 4/5 | In Progress|  |
| 29. DAO/Schema-Foundation (Antragsteller-Historie) | v1.6      | 2/2 | Complete    | 2026-08-12 |
| 30. Application-Template-Kontext                   | v1.6      | 0/3 | Not started |  |
| 31. Service + REST Versand                         | v1.6      | 0/0 | Not started |  |
| 32. Frontend Compose-Dialog                        | v1.6      | 0/0 | Not started |  |

---

## Backlog

> Tech-Debt aus Code-Audit 2026-06-14. Strukturelle Brocken, die mehr als einen Quick-Fix
> wert sind (Designentscheidung oder mehrere Dateien). Per `/gsd-review-backlog` in den
> aktiven Milestone promotbar. Mechanische Einzelfixes liegen als Todos in `.planning/todos/pending/`.

### Phase 999.1: mock_auth-Deploy-Footgun absichern (BACKLOG)

**Priorität:** hoch (Security/Build) · **Quelle:** Code-Audit 2026-06-14
**Goal:** Verhindern, dass versehentlich ein Backend ohne Authentifizierung produktiv läuft.
**Befund:**

- `default = ["mock_auth"]` (`genossi_bin/Cargo.toml:7`, `genossi_rest/Cargo.toml:36`) → `cargo run` / `nix run` (default-Package, `flake.nix:26-29`) startet ein API, das jede Permission-Prüfung durchwinkt (`session.rs:119-137`) — voller PII-Zugriff ohne Login.
- NixOS-Modul (`module.nix:155-192`) entkoppelt das Build-Feature vom Runtime-Flag `oidc.enable` → stiller Auth-Bypass bei Fehlkonfiguration möglich.

**Ansatz (Diskussion vor Umsetzung):** Default-Feature auf sicheren Wert setzen ODER Startup-Panic/Compile-Fehler bei `mock_auth` in Release-Builds (`#[cfg(not(debug_assertions))]`); Feature-Wahl im Nix-Modul wieder an `oidc.enable` koppeln oder Assertion ergänzen.
**Routing:** `/gsd-discuss-phase` (Designentscheidung), dann `/gsd-plan-phase`.

### Phase 999.2: MailRecipientsTable-Komponente extrahieren (BACKLOG)

**Priorität:** hoch (Component-First) · **Quelle:** Code-Audit 2026-06-14
**Goal:** Letzte verbliebene Inline-RSX-Duplikation aus Phase quick-260614-ckn beseitigen.
**Befund:** Die Empfänger-Tabelle ist Zeile für Zeile dupliziert zwischen `genossi-frontend/src/component/mail_jobs_list.rs:185-265` und `genossi-frontend/src/page/mail_page.rs:622-711` (`MailJobDetail`). Einziger Unterschied: Padding-Klassen + Reload-Mechanismus. Bei der Job-Listen-Extraktion wurde die Zwillings-Tabelle auf der Detailseite nicht mit-extrahiert.
**Ansatz:** `MailRecipientsTable`-Komponente (Props: `recipients`, `job`/`repayment_phase_id`, `padding`-Variante, `on_recovered`-Callback) in `src/component/` anlegen, in beiden Stellen verwenden. Status-Helper (`job_status_color`/`job_status_key`) sind bereits geteilt.
**Routing:** `/gsd-quick` (klar umrissen) oder `/gsd-plan-phase` als kleine Folge-Phase zu quick-260614-ckn.

### Phase 999.3: Service-Layer für audit_log- und backup-REST-Handler (BACKLOG)

**Priorität:** mittel (Layering) · **Quelle:** Code-Audit 2026-06-14
**Goal:** REST-Handler, die DAO + eigene Transaktion direkt ansprechen, hinter einen Service legen.
**Befund:**

- `genossi_rest/src/audit_log.rs:119-137` (analog `:186-191`, `:232-237`): Handler holt eigene Transaktion und ruft `audit_log_dao().count()/.query()` direkt — kein `AuditLogService`.
- `genossi_rest/src/backup.rs:61-133`: Handler ruft `backup_dao()` direkt.

**Ansatz:** `AuditLogService` + `BackupService` einziehen, die Permission-Check, Transaktion und DAO-Zugriff kapseln; `RestStateDef` sollte `audit_log_dao()`/`backup_dao()`/`audit_transaction()` nicht mehr direkt an Handler exponieren.
**Routing:** `/gsd-plan-phase` (mehrere Dateien, neue Service-Traits).

### Phase 999.4: Daten-Lade-Boilerplate im Frontend in Hook bündeln (BACKLOG)

**Priorität:** niedrig (Redundanz) · **Quelle:** Code-Audit 2026-06-14
**Goal:** Wiederholtes loading/error/use_effect+spawn-Tripel in ~16 Pages durch geteilten Helper ersetzen.
**Befund:** Identisches Muster (`use_signal(|| true)` loading + `Signal<Option<AppError>>` error + `use_effect`→`spawn`-Fetch + `ErrorAlert`-Block) copy-paste in `applications_page.rs`, `audit_log.rs`, `mail_templates.rs`, `inbox_page.rs`, `assemblies.rs`, `config_page.rs` u.a.
**Ansatz:** Einen `use_loader<T>()`-Hook (oder Wrapper-Komponente) bereitstellen, der Loading-, Error- und Fetch-State kapselt. Funktional heute korrekt — reiner DRY-Gewinn.
**Routing:** `/gsd-quick --discuss` (Hook-API muss durchdacht werden).

### Phase 999.5: In-App-Hilfe für Vorstände — durchsuchbare Feature-Referenz (BACKLOG)

**Priorität:** niedrig (Nice-to-have / UX) · **Quelle:** zurückgestellt 2026-06-26 (war aktive Phase 20)
**Goal:** Durchsuchbare Feature-Referenz im Dioxus-Frontend — Übersicht/Navigation plus pro Feature ein erklärender Eintrag, Component-First. Reines Frontend, keine neue Entität, kein Audit.
**Offene Designentscheidungen (vor Umsetzung in discuss-phase klären):**

- Content-Speicherung: zentrales i18n-Key-System (`de.rs`/`en.rs`) vs. eigene Rust-Datenstruktur (`Vec<HelpEntry>`) vs. Markdown-Assets via manganis vs. Backend
- Einbindung & Navigation: eigene Route `/help` + Sidebar-Eintrag (welche Nav-Gruppe?) vs. globaler `?`-Button; Übersicht→Detail vs. Single-Page-Akkordeon
- Suche & Kategorisierung: Client-Substring-Filter (Vorbild `attendance_search`/`member_search`) über Titel (+Body); Gruppierung nach Nav-Bereich
- Eintrags-Tiefe: Kurzbeschreibung vs. Schritt-Anleitung; „Feature öffnen"-Deep-Link; Sprache (De-only vs. De+En); Feature-Scope (alle vs. kuratiert)

**Routing:** `/gsd-discuss-phase` (Designentscheidungen offen), dann `/gsd-plan-phase`.

---

_Last updated: 2026-08-12 — v1.6 Antragsteller-Kommunikation gestartet (Phases 29-32, fortlaufende Nummerierung nach v1.5 Phase 28). 16 REQs (APMAIL-01..04, APTPL-01..04, APHIST-01..03, APCMP-01..02, APUI-01..03) auf 4 Phasen gemappt, 100% Coverage, keine Orphans/Duplikate. Build-Order: 29 (Schema) ∥ 30 (Template-Kontext) → 31 (Service+REST) → 32 (Frontend); harte Dependency-Kette Schema→Service→Frontend. „Add nothing"-Stack (pure Wiederverwendung `genossi_mail`), kein neues Audit-Feld auf `ApplicationEntity`, Versand-Guardrails DSGVO-transaktional (`Offen`-only, kein Bulk/Tracking). v1.0-v1.5 Historie + Backlog 999.x unverändert erhalten._
