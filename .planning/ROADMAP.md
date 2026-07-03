# Roadmap: Genossi

Mitgliederverwaltungs-Software für Genossenschaften. Aktiver Stand: vier ausgelieferte Milestones — v1.0 (GV-Anwesenheits-Erfassung), v1.1 (Anteile-Rückzahlungsphase), v1.2 (Mitgliedschaft-Anpassungen während des Geschäftsjahres), v1.3 (Posteingang-Benachrichtigung & Reply-Komfort). **Aktiver Milestone: v1.4 Mail-Formatierung & Antrags-Dokumente (Phases 22-25).** Offene Kandidaten siehe Backlog (999.x).

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phases 1-6 (Phase 5 SKIPPED — echte GV bereits durchgeführt) (shipped 2026-05-29)
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phases 7-13 (shipped 2026-06-02)
- ✅ **v1.2 Mitgliedschaft-Anpassungen** — Phases 14-18 (shipped 2026-06-07)
- ✅ **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** — Phases 19-21 (shipped 2026-06-28)
- 🚧 **v1.4 Mail-Formatierung & Antrags-Dokumente** — Phases 22-25 (active, defined 2026-06-29)

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

### 🚧 v1.4 Mail-Formatierung & Antrags-Dokumente (Phases 22-25) — ACTIVE

**Goal:** Vorstände versenden professionell formatierte HTML-Mails (statt nur Rohtext) und können den originalen Mitgliedsantrag als Datei am Antrag hinterlegen, die beim Aktivieren automatisch ans Mitglied übergeht.

- [ ] **Phase 22: 8bit + Shared Mail-Body Helper** - Ein geteilter Body-Bau-Helfer für alle Sendepfade; opt-in 8bit entfernt `=`-Soft-Breaks (MAIL-01..05)
- [x] **Phase 23: HTML Mail Backend** - `multipart/alternative` (Text+HTML) mit autoescapender HTML-Render-Env + serverseitiger ammonia-Sanitization (HTML-01..05) ✅ 2026-07-02
- [x] **Phase 24: WYSIWYG Frontend Editor** - Wiederverwendbare Dioxus-`contenteditable`-Component ersetzt `body_editor`, mit Paste-Cleanup + Live-Vorschau (EDIT-01..05) ✅ 2026-07-02
- [ ] **Phase 25: Application File Upload + Audited Carryover** - Admin-Upload an `Application`, auditierte Kopie als `MemberDocument` beim `confirm` (APDOC-01..05)

**Dependency-Reihenfolge:** Phase 22 → 23 → 24 ist strikt sequenziell (jede baut auf der vorherigen auf: der geteilte Body-Helfer aus 22 verhindert drei divergierende HTML-Implementierungen; der `body_html`-Wire + ammonia-Gate aus 23 müssen existieren, bevor der Editor in 24 HTML postet). **Phase 25 ist dependency-technisch unabhängig** von der Mail-Strecke und kann parallel zu 22→23→24 laufen — sie teilt keinen Code mit den Mail-Features und isoliert die audit-kritische `confirm()`-Cascade mit eigener UAT.

**Harte Ordering-Constraint:** Das ammonia-Gate (Phase 23) MUSS strikt vor oder mit dem WYSIWYG-Editor (Phase 24) landen — niemals danach. Frontend-Sanitization ist keine Sicherheitsgrenze.

## Phase Details

### Phase 22: 8bit + Shared Mail-Body Helper

**Goal**: Alle ausgehenden Mails laufen über einen einzigen Body-Bau-Helfer mit konsistentem `charset=utf-8`, und der Text-Teil kann (config-gated) als 8bit gesendet werden, sodass Empfänger keine sichtbaren `=`-Soft-Line-Breaks mehr sehen.
**Depends on**: Nothing (erste Phase des Milestones; baut auf bestehendem `genossi_mail` auf, keine Schema-Änderungen)
**Requirements**: MAIL-01, MAIL-02, MAIL-03, MAIL-04, MAIL-05
**Success Criteria** (what must be TRUE):

  1. Test-Mail, Massenmail und Digest laufen alle über denselben geteilten Body-Bau-Helfer in `genossi_mail` und erzeugen konsistent `charset=utf-8` (der bestehende Charset-Bug im Test-Mail-Pfad ist behoben). (MAIL-01)
  2. Mit aktivierter 8bit-Kodierung enthalten empfangene Text-Mails keine sichtbaren `=`-Soft-Line-Breaks mehr (Umlaute und lange Zeilen kommen sauber an). (MAIL-02)
  3. Die Kodierung ist per Konfiguration umschaltbar; der Default bleibt quoted-printable, sodass das Produktivverhalten unverändert ist, bis der Betreiber opt-in aktiviert. (MAIL-03)
  4. Bestehende reine Text-Mails (Massenmail, Test-Mail, Digest) kommen mit Default-Config unverändert korrekt an (keine Regression). (MAIL-05)
  5. Die `8BITMIME`-Unterstützung des Produktiv-Relays wird per EHLO-Capability-Check verifiziert, bevor 8bit in Produktion aktiviert wird — dokumentierter Verifikations-Schritt, aus der Dev-Umgebung nicht durchführbar (Relay nur über Produktiv-Netz erreichbar), also Verify-in-Prod statt automatisierter Test. (MAIL-04)

**Plans**: 3 plans

- [ ] 22-01-PLAN.md — Wave 1: `MailEncoding` enum + `SmtpConfig.encoding` + `smtp_encoding` KV parsing (MAIL-03)
- [ ] 22-02-PLAN.md — Wave 2: Extract `build_message` into `genossi_mail::send`, rewire worker + test-mail paths, MIME-byte tests (MAIL-01, MAIL-02, MAIL-05)
- [ ] 22-03-PLAN.md — Wave 1 (parallel): `docs/OPERATIONS.md` runbook for the 8BITMIME EHLO check (MAIL-04)

### Phase 23: HTML Mail Backend

**Goal**: Eine Mail kann mit Text- UND HTML-Teil als `multipart/alternative` versendet werden, wobei mitglieds-/nutzergelieferte Werte sicher escaped und vom Vorstand verfasstes HTML serverseitig saniert werden.
**Depends on**: Phase 22 (nutzt den geteilten `mail_body`-Helfer für die korrekte MIME-Verschachtelung; ohne ihn drohen drei divergierende HTML-Implementierungen)
**Requirements**: HTML-01, HTML-02, HTML-03, HTML-04, HTML-05, FMT-01
**Success Criteria** (what must be TRUE):

  1. Eine mit Text- und HTML-Body gesendete Mail kommt als `multipart/alternative` an (Text zuerst, dann HTML); mit Anhang ist die Struktur korrekt als `mixed{ alternative{plain, html}, attachments }` verschachtelt. (HTML-01)
  2. Der Plain-Text-Teil stammt aus dem bestehenden, vom Autor verfassten `body` — keine Ableitung aus dem HTML, keine zusätzliche Crate. (HTML-02)
  3. Legacy-Templates/-Jobs ohne HTML (`body_html` NULL nach der forward-only `ADD COLUMN … NULL`-Migration) versenden weiterhin reine Text-Mails (backward-kompatibel). (HTML-03)
  4. Template-Variablen werden in Text- UND HTML-Body interpoliert; ein Mitglied namens `<script> & Co` erscheint im HTML-Body HTML-escaped (`&lt;script&gt; &amp;`), während die vom Autor verfasste Markup-Struktur erhalten bleibt — die HTML-Render-Variante nutzt eine separate autoescapende minijinja-Env, `strict_env()` bleibt für Text und Subject unverändert. (HTML-04)
  5. Vom Vorstand verfasstes HTML wird an allen Eintritts-Punkten (`create_job`, Template-Create/Update, Test-Mail-Pfad) serverseitig mit `ammonia` saniert (Whitelist fett/kursiv/Links/Listen/Absätze; `javascript:`/`data:`-Links und Event-Handler werden gestrippt), bevor es gespeichert/versendet wird. (HTML-05)
  6. Datums-Template-Variablen (`join_date`, `exit_date`, ggf. weitere) werden im deutschen Format `DD.MM.YYYY` (z. B. `02.07.2026`) gerendert statt im technischen `.to_string()`-Default — konsistent in Text- und HTML-Mails (`genossi_mail/src/template.rs:17-18`). (FMT-01)

**Plans**: 4/4 plans executed

- [x] 23-01-PLAN.md — Wave 1: 3 forward-only migrations + DAO structs (body_html, rendered_html_body) + dao_sqlite roundtrip tests (HTML-03) ✅ 2026-07-02
- [x] 23-02-PLAN.md — Wave 2: ammonia dep + `sanitize.rs` helper + `html_env()` + `render_html_template()` + `format_de()` + `RenderedContent` struct in render.rs (HTML-04, HTML-05, FMT-01) ✅ 2026-07-02
- [x] 23-03-PLAN.md — Wave 3: `build_message` 4-branch decision tree (singlepart/mixed/alternative/mixed-wrapping-alternative) + 5 MIME-byte tests (HTML-01, HTML-02) ✅ 2026-07-02
- [x] 23-04-PLAN.md — Wave 4: Wire sanitize at 4 D-03 entry points (create_job, template create/update, send_test_mail_with_body) + worker persists rendered_html_body + REST DTOs (body_html) + e2e HTTP tests (HTML-01, HTML-03, HTML-05) ✅ 2026-07-02

**UI hint**: no

### Phase 24: WYSIWYG Frontend Editor

**Goal**: Ein Vorstand ohne HTML-Kenntnisse verfasst formatierte Mails (fett/kursiv/Links/Listen) in einem wiederverwendbaren WYSIWYG-Editor, der sauberes, sanitisierbares HTML erzeugt und eine Live-Vorschau bietet.
**Depends on**: Phase 23 (benötigt den `body_html`-API-Wire zum Posten; das ammonia-Gate muss existieren, bevor HTML aus dem Editor akzeptiert wird — harte Ordering-Constraint)
**Requirements**: EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05
**Success Criteria** (what must be TRUE):

  1. Ein Vorstand formatiert Mail-Text (fett, kursiv, Links, Aufzählungs- und nummerierte Listen) in einem WYSIWYG-Editor, der den bestehenden `body_editor`-Textarea ersetzt und als wiederverwendbare Dioxus-Component gebaut ist (keine neuen Frontend-Dependencies; `contenteditable` + `execCommand` über vorhandenes web-sys). (EDIT-01)
  2. Der Editor erzeugt semantische `<b>/<i>`-Tags (`styleWithCSS=false` erzwungen), die die ammonia-Sanitization überleben — nicht inline-`style`-Spans, die gestrippt würden. (EDIT-02)
  3. Der HTML-Inhalt des Editors wird beim Absenden zuverlässig aus dem contenteditable-DOM ausgelesen und mit dem Dioxus-State synchronisiert (kein Datenverlust beim Submit). (EDIT-03)
  4. Eingefügter Inhalt (Paste, z. B. aus Word/Browser) wird beim Einfügen bereinigt, sodass kein verschmutztes Markup in den Mail-Body gelangt. (EDIT-04)
  5. Eine Live-Vorschau zeigt dem Vorstand das gerenderte HTML vor dem Versand. (EDIT-05)

**Plans**: 4/4 plans complete

- [x] 24-01-PLAN.md — Wave 1: Backend seam (PreviewRequest/Response body_html + preview_mail HTML render + ReplyRequest body_html + InboxService::reply sanitize-on-store) + frontend api mirror + Cargo web-sys features (ClipboardEvent, DataTransfer) + 19 new i18n keys in de.rs+en.rs (EDIT-01, EDIT-04, EDIT-05) ✅ 2026-07-03
- [x] 24-02-PLAN.md — Wave 2: New WysiwygEditor + WysiwygToolbar + WysiwygLinkDialog Dioxus components + exec_command_* helpers in js.rs (styleWithCSS=false at mount, 13 ammonia-safe buttons, in-app Modal for link dialog, plain-text paste handler) (EDIT-01, EDIT-02, EDIT-03, EDIT-04) ✅ 2026-07-03
- [x] 24-03-PLAN.md — Wave 3: Migrate all 3 MailBodyEditor call sites (mail_page.rs, reply_form.rs, mail_templates.rs) to WysiwygEditor + body_html signal wiring end-to-end + extend TemplatePreview to render backend body_html via dangerous_inner_html + delete body_editor.rs (EDIT-01, EDIT-03, EDIT-05) ✅ 2026-07-03
- [x] 24-04-PLAN.md — Wave 4: 2 new e2e HTTP tests (preview_body_html_round_trips_to_response + inbox_reply_body_html_sanitized_and_persisted in genossi_bin/tests/e2e_tests.rs) + 24-UAT-CHECKLIST.md (12 checkbox steps, 3 HARD FAIL GATES) + auto-approved human-verify smoke test (browser-interactive walkthrough deferred to Vorstand smoke session) (EDIT-01..05) ✅ 2026-07-02

**UI hint**: yes

### Phase 25: Application File Upload + Audited Carryover

**Goal**: Ein Admin kann den originalen Mitgliedsantrag als Datei an eine `Application` hinterlegen; beim Aktivieren wird die Datei automatisch als auditiertes `MemberDocument` ans Mitglied kopiert. (Unabhängig — parallelisierbar zu 22→23→24.)
**Depends on**: Nothing (dependency-technisch unabhängig von der Mail-Strecke; kann parallel zu Phase 22→23→24 laufen)
**Requirements**: APDOC-01, APDOC-02, APDOC-03, APDOC-04, APDOC-05
**Success Criteria** (what must be TRUE):

  1. Ein Admin lädt eine Datei (z. B. eingescannter Original-Antrag als PDF) an eine `Application` hoch; sie wird über `DocumentStorage` im Filesystem gespeichert (nicht in der DB) und spiegelt das bestehende `member_document`-Upload-Muster (Multipart, `DefaultBodyLimit`, MIME-Allowlist, UUID-Pfad gegen Path-Traversal). (APDOC-01)
  2. Der Upload-Endpunkt ist admin-only (der Antrags-Submit-Pfad bleibt `PUBLIC`); dabei wird die carry-forward CR-02 Permission-Check-Ordering an dieser Stelle korrekt umgesetzt (`check_permission()` vor `current_user_id()`). (APDOC-02)
  3. Beim `confirm` einer `Application` wird ein hinterlegtes Antrags-Dokument **übernommen** (Ownership-Übergabe — Move-Semantik: die `application_documents`-Zeile wird soft-deleted und die Datei physisch an den Member-Pfad verschoben) und als auditiertes `MemberDocument` am Mitglied angelegt — innerhalb derselben atomaren Aktivierungs-Transaktion, via `audited_create!` unter `APPLICATION_SERVICE_PROCESS` mit `DocumentType::Other` + beschreibender Bezeichnung („Original-Antrag (übernommen bei Bestätigung am DD.MM.YYYY)“). (APDOC-03)
  4. Die Aktivierung ist robust gegen Edge-Cases: Antrag ohne Dokument übernimmt nichts (kein Fehler), Re-Aktivierung wird durch den bestehenden `Offen`-Status-Guard verhindert (keine Doppel-Übernahme), fehlende Datei auf dem Filesystem → Transaktion rollt zurück. (APDOC-04)
  5. Das Antrags-Dokument ist im Frontend an der Application sichtbar und herunterladbar (admin-only). (APDOC-05)

**Plans**: 4/5 plans executed

- [x] 25-01-PLAN.md — Wave 1 (parallel doku-fix): APDOC-03 wording sync in REQUIREMENTS.md + ROADMAP.md + remove contradicting Out-of-Scope bullet (APDOC-03)
- [x] 25-02-PLAN.md — Wave 1 (parallel): SQLx migration `application_documents` (single-slot partial unique index) + `ApplicationDocumentDao` trait/entity + SQLite impl (APDOC-01)
- [x] 25-03-PLAN.md — Wave 2: `ApplicationDocumentService` trait + impl with CR-02 permission-check ordering + replace-in-place + 6 unit tests (APDOC-01, APDOC-02)
- [x] 25-04-PLAN.md — Wave 3: `ApplicationDocumentTO` + 3 REST endpoints (POST/GET/DELETE `/api/applications/{id}/document`) + `confirm()` CR-02 fix + audited Move-transfer to `MemberDocument` + genossi_bin DI wiring (APDOC-01..04)
- [ ] 25-05-PLAN.md — Wave 4: Frontend `ApplicationDocumentSlot` component + api.rs helpers + i18n keys (De+En) + backend `?meta=1` metadata branch + 3 e2e HTTP tests + 25-UAT-CHECKLIST.md (APDOC-02, APDOC-04, APDOC-05)

**UI hint**: yes

> **Audit-Hinweis (Phase 25):** Die neue `application_documents`-Tabelle ist **nicht** auditiert (gleiche Ausnahme wie die GV-Entitäten). Das beim `confirm` kopierte `MemberDocument` **ist** auditiert (`MemberDocument` ist eine auditierte Entität) und MUSS über `audited_create!` in derselben Aktivierungs-Transaktion mit dem `APPLICATION_SERVICE_PROCESS`-String erzeugt werden. Mail-/Editor-Arbeit (Phasen 22-24) benötigt keinerlei Audit.

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
| 22. 8bit + Shared Mail-Body Helper                 | v1.4      | 3/3            | Ready to verify | 2026-07-02 |
| 23. HTML Mail Backend                              | v1.4      | 3/4 | In Progress|  |
| 24. WYSIWYG Frontend Editor                        | v1.4      | 4/4 | Complete   | 2026-07-02 |
| 25. Application File Upload + Audited Carryover     | v1.4      | 4/5 | In Progress|  |

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

_Last updated: 2026-06-29 — v1.4 Mail-Formatierung & Antrags-Dokumente gestartet (Phases 22-25, fortlaufende Nummerierung nach v1.3 Phase 21). 20 REQs (MAIL/HTML/EDIT/APDOC je 5) auf 4 Phasen gemappt, 100% Coverage. Build-Order 22→23→24 sequenziell, Phase 25 parallelisierbar. v1.0-v1.3 Historie + Backlog 999.x unverändert erhalten._
