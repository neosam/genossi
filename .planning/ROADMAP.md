# Roadmap: Genossi

Mitgliederverwaltungs-Software für Genossenschaften. Aktiver Stand: drei ausgelieferte Milestones — v1.0 (GV-Anwesenheits-Erfassung), v1.1 (Anteile-Rückzahlungsphase), v1.2 (Mitgliedschaft-Anpassungen während des Geschäftsjahres). Aktuell in Arbeit: **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** (Phase 19 Anhänge als Vorläufer geshippt; Phasen 20–21 definiert).

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phases 1-6 (Phase 5 SKIPPED — echte GV bereits durchgeführt) (shipped 2026-05-29)
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phases 7-13 (shipped 2026-06-02)
- ✅ **v1.2 Mitgliedschaft-Anpassungen** — Phases 14-18 (shipped 2026-06-07)
- 🚧 **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** — Phase 19 (Anhänge) geshippt; Phasen 20 (Inbox-Digest) + 21 (Reply-Modal) in Arbeit

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

### 🚧 v1.3 Posteingang-Benachrichtigung & Reply-Komfort (Phases 19–21)

**Goal:** Vorstände verpassen keine eingehenden Mails mehr und können bequemer auf sie antworten.

- [x] Phase 19: E-Mail-Anhänge anzeigen (Vorläufer, geshippt 2026-06-09)
- [ ] Phase 20: Inbox-Digest — täglicher Posteingangs-Benachrichtigungs-Worker (DIGEST-01..07)
- [ ] Phase 21: Reply-Komfort — Antwort im vollflächigen Modal (REPLY-01..04)

> Nicht in v1.3: v1.2-Tech-Debt (CR-02 Permission-Ordering, Phase-18-UX-Polish, Mail-Subsystem-Triage, 16 deferred v1.1-Quick-Tasks) bleibt Backlog/Kandidat — siehe Backlog (999.x) unten und `milestones/v1.2-MILESTONE-AUDIT.md`.

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
| 20. Inbox-Digest (täglicher Benachrichtigungs-Worker) | v1.3   | 0/3            | Planned     | -          |
| 21. Reply-Komfort (Antwort im Modal)               | v1.3      | 0/0            | Not started | -          |

### Phase 19: E-Mail-Anhänge anzeigen — Backend-Endpoint zum Abrufen von Anhängen aus eingehenden Mails (mail-parser nutzt async-imap bereits) plus Dioxus-Frontend-UI zum Anzeigen/Herunterladen der Attachments in der Mail-Ansicht.

**Goal:** Eingehende E-Mail-Anhänge im Vorstands-Inbox persistent speichern (10-MB-Cap, Filesystem via DocumentStorage), per Vorstand-only-Endpoint ausliefern (Download + optional Inline-Preview), und im Dioxus-Frontend per Component-First-Liste mit Image-Thumbnail / PDF-Vorschau / Download-Action sichtbar machen — inkl. einmaligem Backfill-Worker für Bestandsmails.
**Requirements**: TBD (v1.3 noch nicht definiert; Scope = CONTEXT.md D-01..D-14 + UI-SPEC)
**Depends on:** Phase 18
**Plans:** 7/7 plans complete

Plans:
- [x] 19-01-PLAN.md — DAO + Migration (InboundMailAttachment-Entity, Trait, SQLite-Impl, Migration `20260608000000`)
- [ ] 19-02-PLAN.md — Service + IMAP (parse_raw_mail-Erweiterung, persist_attachment mit 10-MB-Cap + Rollback, fetch_one_by_uid mit UIDVALIDITY-Check, Poll-Worker-Persistenz)
- [ ] 19-03-PLAN.md — REST Endpoints (DetailTO-Extension, /api/inbox/{mail_id}/attachments/{attachment_id} mit ?disposition=inline, content_disposition_inline-Helper, 5 E2E-Tests)
- [x] 19-04-PLAN.md — Backfill Worker (run_attachment_backfill für Bestandsmails, einmaliger tokio::spawn, best-effort silent-skip)
- [x] 19-05-PLAN.md — Frontend Components (InboxAttachmentList + InboxAttachmentListItem, format_size Util, 7 i18n-Keys in De+En)
- [x] 19-06-PLAN.md — Frontend Page Wiring (MVP-Hinweis gelöscht, InboxAttachmentList eingebunden, WASM-Build grün; Vorstand-Sichtprüfung Task 2 pending)

### Phase 20: Inbox-Digest — täglicher Posteingangs-Benachrichtigungs-Worker

**Goal:** Ein Scheduler-Worker verschickt einmal pro Kalendertag zur konfigurierten Uhrzeit eine Zusammenfassungs-Mail aller offenen (nicht-archivierten) Posteingangs-Mails an eine oder mehrere konfigurierbare Empfänger-Adressen — mit Titel, Absender, Eingangszeitpunkt je Mail und einem Deep-Link auf `/inbox` (via `APP_URL`). Versand nur bei nicht-leerem Posteingang; Empfänger und Uhrzeit werden über das bestehende Runtime-Config-System (Config-Seite, wie SMTP-Settings) gepflegt.
**Requirements:** DIGEST-01, DIGEST-02, DIGEST-03, DIGEST-04, DIGEST-05, DIGEST-06, DIGEST-07
**Depends on:** Phase 19
**Plans:** 3 plans in 2 Wellen

Success criteria:
1. Vorstand trägt auf der Config-Seite Empfänger-Adressen und Versand-Uhrzeit ein; beide bleiben nach Reload erhalten.
2. Zur konfigurierten Uhrzeit erhält jeder konfigurierte Empfänger genau eine Digest-Mail pro Tag, sofern offene Mails vorliegen.
3. Bei leerem Posteingang oder ohne konfigurierte Empfänger geht keine Mail raus (kein Fehler).
4. Die Digest-Mail listet jede offene Mail mit Titel, Absender und Eingangszeitpunkt und enthält einen funktionierenden Link auf `/inbox`.

Plans:
- [ ] 20-01-PLAN.md — DB-Foundation (Migration `digest_state` + DigestStateDao-Trait + DigestStateDaoSqlite-Upsert + Tests) [Wave 1]
- [ ] 20-02-PLAN.md — Digest-Worker (poll-loop nach timestamp_worker.rs + reine Helfer is_due/parse/build_* + Tests + DI-Wiring lib.rs/main.rs) [Wave 2, depends_on 20-01]
- [ ] 20-03-PLAN.md — Frontend Config-Abschnitt "Posteingangs-Benachrichtigung" (Empfänger + Uhrzeit + Inline-Validierung + Save) [Wave 1]

### Phase 21: Reply-Komfort — Antwort im vollflächigen Modal

**Goal:** Das Antworten auf eine eingegangene Mail öffnet künftig in einem vollflächigen Modal (bestehende `modal.rs`-Component) mit großem Textfeld statt im schmalen Inline-Feld. Abbrechen ohne Senden ist möglich; das Absenden nutzt die unveränderte bestehende Sende-Logik und zeigt Erfolg-/Fehler-Feedback wie bisher.
**Requirements:** REPLY-01, REPLY-02, REPLY-03, REPLY-04
**Depends on:** Phase 19
**Plans:** 0 plans (run /gsd-plan-phase 21 to break down)

Success criteria:
1. Vorstand klickt „Antworten" und ein vollflächiges Modal öffnet sich mit einem großen Eingabefeld.
2. Das Antwort-Textfeld bietet sichtbar deutlich mehr Schreibfläche als das bisherige Inline-Feld.
3. Vorstand kann das Modal abbrechen/schließen ohne zu senden und landet wieder in der Mail-Ansicht.
4. Senden aus dem Modal verschickt die Antwort wie bisher und zeigt Erfolg-/Fehler-Feedback.

Plans:
- [ ] TBD (run /gsd-plan-phase 21 to break down)

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

_Last updated: 2026-06-26 — Phase 20 (Inbox-Digest) geplant: 3 Plans in 2 Wellen (DB-Foundation + Frontend parallel in Wave 1, Worker + DI-Wiring in Wave 2), alle DIGEST-01..07 gemappt. Vorher: 2026-06-26 — Milestone v1.3 definiert: Phasen 20 (Inbox-Digest, DIGEST-01..07) + 21 (Reply-Modal, REPLY-01..04). Frühere In-App-Hilfe-Phase-20 ins Backlog 999.5 verschoben. 2026-06-14 — Backlog-Sektion + 7 Todos aus Code-Audit ergänzt._
