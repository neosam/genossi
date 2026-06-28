# Roadmap: Genossi

Mitgliederverwaltungs-Software für Genossenschaften. Aktiver Stand: vier ausgelieferte Milestones — v1.0 (GV-Anwesenheits-Erfassung), v1.1 (Anteile-Rückzahlungsphase), v1.2 (Mitgliedschaft-Anpassungen während des Geschäftsjahres), v1.3 (Posteingang-Benachrichtigung & Reply-Komfort). Kein aktiver Milestone — nächster via `/gsd-new-milestone`. Offene Kandidaten siehe Backlog (999.x).

## Milestones

- ✅ **v1.0 GV-Anwesenheits-Erfassung** — Phases 1-6 (Phase 5 SKIPPED — echte GV bereits durchgeführt) (shipped 2026-05-29)
- ✅ **v1.1 Anteile-Rückzahlungsphase** — Phases 7-13 (shipped 2026-06-02)
- ✅ **v1.2 Mitgliedschaft-Anpassungen** — Phases 14-18 (shipped 2026-06-07)
- ✅ **v1.3 Posteingang-Benachrichtigung & Reply-Komfort** — Phases 19-21 (shipped 2026-06-28)

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

_Last updated: 2026-06-28 — v1.3 abgeschlossen (Phasen 19-21, 11 Plans): Inbox-Anhänge + täglicher Digest-Worker + Reply-Modal. Milestone-Audit `passed` (11/11 Reqs, Integration 8/8 sauber, Live-Smoke-Test); Phase-21-Code-Review fand+fixte 1 Critical. Archiviert nach `milestones/v1.3-*`. Kein aktiver Milestone — nächster via `/gsd-new-milestone`. Backlog 999.x offen._
