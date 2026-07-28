# Roadmap: Genossi

Mitgliederverwaltungs-Software für Genossenschaften. Aktiver Stand: fünf ausgelieferte Milestones — v1.0..v1.4 — plus aktiver Milestone **v1.5 Editor-Vervollständigung, Bild-Support & Vorschau** (Phases 26-28). Offene Kandidaten siehe Backlog (999.x).

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phases 1-6 (Phase 5 SKIPPED — echte GV bereits durchgeführt) (shipped 2026-05-29)
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phases 7-13 (shipped 2026-06-02)
- ✅ **v1.2 Mitgliedschaft-Anpassungen** — Phases 14-18 (shipped 2026-06-07)
- ✅ **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** — Phases 19-21 (shipped 2026-06-28)
- ✅ **v1.4 Mail-Formatierung & Antrags-Dokumente** — Phases 22-25 (shipped 2026-07-03)
- 🚧 **v1.5 Editor-Vervollständigung, Bild-Support & Vorschau** — Phases 26-28 (planning 2026-07-17)

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

<details open>
<summary>🚧 v1.5 Editor-Vervollständigung, Bild-Support & Vorschau (Phases 26-28) — PLANNING 2026-07-17</summary>

**Goal:** Der WYSIWYG-Editor bekommt vollen Formatierungs-Umfang (Listen, Überschriften), Vorstand kann Inline-Bilder direkt im Editor hochladen und in HTML-Mails einbetten, und das gerenderte HTML lässt sich in Desktop-/Mobile-Vorschau prüfen bevor die Mail versendet wird.

- [ ] **Phase 26: Editor-Formatierung vervollständigen** — Listen (ul/ol), Überschriften (H2/H3), Toolbar-Erweiterung + Grep-Gate; v1.4-Phase-24-UAT-Checklist wird im gleichen Zug abgehakt (EDIT-06..10)
- [ ] **Phase 27: Bild-Support Backend + Editor-Upload** — `mail_asset`-Entität (kein Audit) + Upload-REST + Bytes-REST + ammonia `<img data-genossi-asset-id>`-Regel + CID-Renderer + `multipart/related` (IMG-01..09)
- [ ] **Phase 28: Desktop/Mobile-Vorschau** — sandboxed iframe-Preview mit umschaltbaren Breakpoints (~640px / ~360px), rendert ammonia-sanitisierte HTML mit Bildern via `/api/mail/assets/{id}/bytes` (PREV-01..05)

**Build order:** 26 → 27 → 28 strikt sequenziell. Phase 27 (Bild-Support) fasst dieselben `ammonia`-Regeln an, die Phase 26 erweitert — Sequenzierung vermeidet Merge-Konflikte auf `sanitize.rs`. Phase 28 (Preview) braucht Phase 27's `/api/mail/assets/{id}/bytes`-Endpoint, um Bilder in der Vorschau zu rendern.

**Audit scope (v1.5):** Kein Audit-Log für die neue `mail_asset`-Entität (Non-Kern-Entität, analog `application_documents`-Pattern aus v1.4 Phase 25). Bestehende auditierte Entitäten (Member/MemberAction/MemberDocument/Application) bleiben unverändert. Neue Backend-Dependency: keine — `ammonia` (Phase 23) wird nur um Regeln erweitert.

**Backward-Compat:** v1.4-Templates ohne Bilder senden weiterhin OHNE `multipart/related`-Wrapper (IMG-09). Bestehende WYSIWYG-Component wird erweitert, nicht ersetzt. Ammonia bleibt server-side only (kein WASM-Bundle).

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

_Last updated: 2026-07-17 — v1.5 Editor-Vervollständigung, Bild-Support & Vorschau gestartet (Phases 26-28, fortlaufende Nummerierung nach v1.4 Phase 25). 19 REQs (EDIT-06..10, IMG-01..09, PREV-01..05) auf 3 Phasen gemappt, 100% Coverage. Build-Order 26→27→28 strikt sequenziell (ammonia-Regeln-Konflikt zwischen 26/27; Preview braucht 27's Assets-Bytes-Endpoint). v1.0-v1.4 Historie + Backlog 999.x unverändert erhalten._
