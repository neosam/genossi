---
phase: quick-260614-ckn
plan: 01
subsystem: ui
tags: [dioxus, frontend, component-first, routing, mail]

requires:
  - phase: quick-260614-9zf
    provides: MailRecipientRenderedContent component (rendered subject/body row)
  - phase: quick-260603-evf
    provides: MailRecipientStatusBadge, NoRepaymentLetterAction, show_toast/ToastContainer
provides:
  - Reusable MailJobsList component (self-contained job history list)
  - MailJobsPage at /mail/jobs (route + Kommunikation nav entry)
  - MailPage send page no longer renders the job list, links to /mail/jobs instead
affects: [mail, communication-ui, future-mail-features]

tech-stack:
  added: []
  patterns:
    - "Self-contained list component owning its own state (signals + initial use_effect load)"
    - "pub(crate) status-mapping helpers shared between list component and detail page (DRY)"

key-files:
  created:
    - genossi-frontend/src/component/mail_jobs_list.rs
    - genossi-frontend/src/page/mail_jobs_page.rs
  modified:
    - genossi-frontend/src/component/mod.rs
    - genossi-frontend/src/page/mod.rs
    - genossi-frontend/src/router.rs
    - genossi-frontend/src/component/top_bar.rs
    - genossi-frontend/src/page/mail_page.rs

key-decisions:
  - "Job list extracted to a self-contained MailJobsList component owning its own state (no Props), since the send page no longer needs any job-state"
  - "job_status_key/job_status_color made pub(crate) in the component and imported by MailJobDetail (DRY) instead of duplicating helpers"
  - "Reused existing unused Key::MailHistory (Gesendete E-Mails / Sent Emails) for nav label, page heading and link button — no new i18n key"

patterns-established:
  - "Pages compose components only (MailJobsPage = layout + MailJobsList), Component-First per genossi-frontend/CLAUDE.md"

requirements-completed: [CKN-01, CKN-02, CKN-03]

duration: 18min
completed: 2026-06-14
---

# Phase quick-260614-ckn Plan 01: Job-Liste vom Mail-Versand auf eigene Seite Summary

**Mail-Job-Liste als wiederverwendbare `MailJobsList`-Komponente extrahiert, auf eine eigene Seite `/mail/jobs` ausgelagert (Route + Nav-Eintrag); die Versand-Seite `/mail` verlinkt nur noch dorthin.**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-06-14T06:57:00Z (approx.)
- **Completed:** 2026-06-14T07:14:45Z
- **Tasks:** 3
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments
- Neue self-contained `MailJobsList`-Komponente (`src/component/mail_jobs_list.rs`) mit eigenem State (jobs/loading/error/expand/detail/toasts), Initial-Load via `use_effect`, inklusive Expand-/Retry-/Recipients-Tabelle, `NoRepaymentLetterAction`-Recovery und `MailRecipientRenderedContent`.
- Neue dünne `MailJobsPage` (`/mail/jobs`), admin-gated analog zur Versand-Seite, komponiert nur Layout + `MailJobsList` (Component-First).
- Route `Route::MailJobsPage` vor der `/mail/jobs/:id`-Variante registriert; Nav-Eintrag (Kommunikation-Gruppe) mit Label `Key::MailHistory` zeigt auf die neue Seite.
- Job-Listen-RSX vollständig aus `MailPage` entfernt (−250 Zeilen) und durch einen Link ersetzt; verwaiste Signale/Closures/Imports bereinigt; `reload_jobs()`-Aufruf nach Senden entfernt, Erfolgsmeldung (`Key::MailJobCreated`) bleibt erhalten.

## Task Commits

1. **Task 1: MailJobsList-Komponente extrahieren** - `6ee2dbd` (feat)
2. **Task 2: MailJobsPage anlegen, Route + Nav-Eintrag, i18n** - `09f32b1` (feat)
3. **Task 3: Job-Liste aus MailPage entfernen, durch Link ersetzen** - `6721aae` (refactor)

## Files Created/Modified
- `genossi-frontend/src/component/mail_jobs_list.rs` (created) - Wiederverwendbare `MailJobsList`-Komponente + pub(crate) `job_status_key`/`job_status_color` + Helper-Tests
- `genossi-frontend/src/page/mail_jobs_page.rs` (created) - `MailJobsPage`-Seite, rendert `MailJobsList` (TopBar + RequirePrivilege admin)
- `genossi-frontend/src/component/mod.rs` - `pub mod`/`pub use` für `MailJobsList`
- `genossi-frontend/src/page/mod.rs` - `pub mod`/`pub use` für `MailJobsPage`
- `genossi-frontend/src/router.rs` - Re-Export + `#[route("/mail/jobs")] MailJobsPage {}`
- `genossi-frontend/src/component/top_bar.rs` - Nav-Eintrag in `kommunikation_items`
- `genossi-frontend/src/page/mail_page.rs` - Job-Liste entfernt, Link eingefügt, State/Closures/Imports bereinigt, Helper aus Komponente importiert

## Decisions Made
- `MailJobsList` ist self-contained (kein Props-Durchreichen), weil die Versand-Seite keinen Job-State mehr benötigt.
- `job_status_key`/`job_status_color` als `pub(crate)` in der Komponente, von `MailJobDetail` importiert (DRY statt Duplikat in `mail_page.rs`).
- Bestehender, bisher ungenutzter `Key::MailHistory` für Nav-Label, Überschrift und Link wiederverwendet — kein neuer i18n-Key, beide Locales (de/en) bereits vorhanden.

## Deviations from Plan

None - plan executed exactly as written.

(Die einzige Abweichung von den im Plan vorgegebenen *verify*-Befehlen: Das Frontend ist im Root-Workspace via `exclude` ausgeklammert, daher schlägt `cargo … -p genossi-frontend` mit "package ID specification did not match any packages" fehl. Stattdessen wurde äquivalent `cargo … --manifest-path genossi-frontend/Cargo.toml …` ausgeführt — gleiches Crate, gleiches Ergebnis. Keine Code-Abweichung.)

## Issues Encountered
- `cargo -p genossi-frontend` funktioniert nicht (Workspace-`exclude`). Gelöst durch `--manifest-path genossi-frontend/Cargo.toml`.

## Verification (real output)

- `cargo test --manifest-path genossi-frontend/Cargo.toml mail_jobs_list` → `test result: ok. 2 passed; 0 failed` (beide Helper-Tests grün).
- `cargo check --manifest-path genossi-frontend/Cargo.toml` → `Finished` (nur vorbestehende dead-code-Warnungen auf ungenutzte i18n-Keys, nicht aus diesem Plan).
- `cargo build --manifest-path genossi-frontend/Cargo.toml` → `Finished`; `grep -iE "unused|never used"` zeigt nur vorbestehende Re-Export-/api-Warnungen, **keine** zu `mail_page`/`mail_jobs`/`MailJobTO`/`reload_jobs`/`job_status` (verifiziert per gefilterten Grep → kein Treffer).
- `cargo clippy --manifest-path genossi-frontend/Cargo.toml` → keine **neuen** unused-/dead-code-Warnungen aus diesem Plan; verbleibende Hinweise (`mut repayment_phase_id` no longer needed, redundant closure) sind vorbestehende, repo-weit übliche Stil-Muster und nicht durch diese Änderung eingeführt.
- Grep-Gates: `grep -c MailJobsList …/mail_jobs_list.rs` = 2 (≥1); `grep -c MailJobsPage …/router.rs` = 2 (matcht); `grep -c "Mail jobs history" …/mail_page.rs` = 0 (Liste entfernt).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `/mail/jobs` ist als dedizierte Job-Übersicht verfügbar; Versand-Seite ist entrümpelt.
- `MailJobsList` steht für weitere Wiederverwendung bereit (z. B. Einbettung in Dashboards).

## Self-Check: PASSED

- FOUND: genossi-frontend/src/component/mail_jobs_list.rs
- FOUND: genossi-frontend/src/page/mail_jobs_page.rs
- FOUND commit: 6ee2dbd (Task 1)
- FOUND commit: 09f32b1 (Task 2)
- FOUND commit: 6721aae (Task 3)

---
*Phase: quick-260614-ckn*
*Completed: 2026-06-14*
