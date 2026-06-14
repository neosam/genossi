---
phase: quick-260614-ckn
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - genossi-frontend/src/component/mail_jobs_list.rs
  - genossi-frontend/src/component/mod.rs
  - genossi-frontend/src/page/mail_jobs_page.rs
  - genossi-frontend/src/page/mod.rs
  - genossi-frontend/src/page/mail_page.rs
  - genossi-frontend/src/router.rs
  - genossi-frontend/src/component/top_bar.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
autonomous: true
requirements: [CKN-01, CKN-02, CKN-03]

must_haves:
  truths:
    - "Die Mail-Versand-Seite (/mail) zeigt KEINE Job-Liste mehr, sondern einen Link/Button zur Job-Seite"
    - "Eine neue Seite /mail/jobs zeigt die vollständige Mail-Job-Liste (mit Expand, Retry, Recipients, NoRepaymentLetterAction)"
    - "Die Hauptnavigation (Kommunikation-Gruppe) enthält einen Eintrag zur neuen Job-Seite"
    - "Die Job-Listen-UI lebt als wiederverwendbare Komponente in src/component/, nicht inline in einer Page"
    - "cargo check -p genossi-frontend kompiliert fehlerfrei"
  artifacts:
    - path: "genossi-frontend/src/component/mail_jobs_list.rs"
      provides: "Wiederverwendbare MailJobsList-Komponente (Job-Liste inkl. Expand/Retry/Recipients)"
      min_lines: 180
    - path: "genossi-frontend/src/page/mail_jobs_page.rs"
      provides: "MailJobsPage-Seite, die MailJobsList rendert (mit TopBar + RequirePrivilege)"
      min_lines: 15
  key_links:
    - from: "genossi-frontend/src/page/mail_jobs_page.rs"
      to: "genossi-frontend/src/component/mail_jobs_list.rs"
      via: "MailJobsList { } im RSX"
      pattern: "MailJobsList"
    - from: "genossi-frontend/src/router.rs"
      to: "MailJobsPage"
      via: "Route-Variante /mail/jobs"
      pattern: "MailJobsPage"
    - from: "genossi-frontend/src/component/top_bar.rs"
      to: "Route::MailJobsPage"
      via: "NavItem in kommunikation_items"
      pattern: "MailJobsPage"
    - from: "genossi-frontend/src/page/mail_page.rs"
      to: "Route::MailJobsPage"
      via: "Link/Navigator-Button statt Job-Liste"
      pattern: "MailJobsPage"
---

<objective>
Die Mail-Job-Liste wird von der Mail-Versand-Seite (`/mail`) auf eine eigene Seite (`/mail/jobs`) ausgelagert. Auf der Versand-Seite ersetzt ein Link/Button die bisher inline gerenderte Job-Liste (Entscheidung des Users: "Komplett entfernen + Link"). Die Job-Listen-UI wird gemäß Component-First-Prinzip als wiederverwendbare Komponente `MailJobsList` nach `src/component/` extrahiert.

Purpose: Der Vorstand interessiert sich beim Versenden in der Regel nicht für die Job-Historie; das Auslagern entrümpelt die Versand-Seite und schafft eine dedizierte Übersicht.
Output: Neue Komponente `MailJobsList`, neue Seite `MailJobsPage` (Route + Nav-Eintrag), bereinigte `MailPage` mit Link.
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@./genossi-frontend/CLAUDE.md

# Frontend Component-First-Prinzip ist verpflichtend (siehe genossi-frontend/CLAUDE.md):
# Job-Listen-UI MUSS in eine Komponente unter src/component/ wandern — NICHT inline in einer Page duplizieren.

<interfaces>
<!-- Kontext aus der Codebase-Erkundung. Executor braucht keine eigene Exploration. -->

Der auszulagernde Job-Listen-Block ist in genossi-frontend/src/page/mail_page.rs:
- Zeilen 584-784: Das komplette `// Mail jobs history`-RSX (h2 Überschrift, Loading/Empty-State,
  for-Loop über jobs, Job-Header mit Progress-Bar, Expand-Logik, Retry-Button, Recipients-Tabelle
  mit MailRecipientStatusBadge / NoRepaymentLetterAction / MailRecipientRenderedContent).
- Zeilen 26-42: Helper-Funktionen `job_status_key(status: &str) -> Key` und
  `job_status_color(status: &str) -> &'static str` — werden NUR von der Job-Liste genutzt
  und wandern mit in die Komponente.
- Zeile 788: `ToastContainer { messages: toast_messages }` gehört zur Job-Liste (NoRepaymentLetterAction-Recovery).

State-Signale, die NUR die Job-Liste nutzt (werden in die Komponente verschoben):
  - jobs: Signal<Vec<MailJobTO>>
  - loading: Signal<bool>            (aktuell geteilt mit error; in der Komponente eigenes loading-Signal)
  - error: Signal<Option<api::AppError>>  (Komponente bekommt eigenes error-Signal)
  - expanded_job_id: Signal<Option<String>>
  - job_detail: Signal<Option<MailJobDetailTO>>
  - toast_messages: Signal<Vec<(u64, String)>>
  - toast_counter: Signal<u64>
  - reload_jobs (Closure, ruft api::get_mail_jobs)

WICHTIG: Auf der Mail-Versand-Seite wird `success_msg` nach erfolgreichem Senden mit
Key::MailJobCreated gesetzt und (Zeile 566) `reload_jobs()` aufgerufen. Nach dem Auslagern
gibt es auf der Versand-Seite keine Job-Liste mehr → der `reload_jobs()`-Aufruf in
mail_page.rs (Zeile ~566) MUSS entfernt werden (kein Job-State mehr vorhanden). Die
Erfolgsmeldung (success_msg / Key::MailJobCreated) bleibt erhalten.

API-Funktionen (aus crate::api, bereits vorhanden):
  - api::get_mail_jobs(&config) -> Result<Vec<MailJobTO>, AppError>
  - api::get_mail_job_detail(&config, &id) -> Result<MailJobDetailTO, AppError>
  - api::retry_mail_job(&config, &id) -> Result<_, AppError>
  - Typen: MailJobTO, MailJobDetailTO  (aus crate::api)

Genutzte Komponenten (aus crate::component, re-exportiert in component/mod.rs):
  - MailRecipientStatusBadge, MailRecipientRenderedContent
  - NoRepaymentLetterAction, is_no_repayment_letter_failure
  - show_toast, ToastContainer, ErrorAlert, TopBar

Router (genossi-frontend/src/router.rs):
  - Route-Enum mit #[route("...")]-Varianten. Bestehende verwandte Routen:
      #[route("/mail")] MailPage {}
      #[route("/mail/templates")] MailTemplatesPage {}
      #[route("/mail/jobs/:id")] MailJobDetail { id: String }   <-- existiert bereits (Detail-Deeplink)
  - Re-Exports oben in router.rs: `pub use crate::page::MailPage;` etc.
  - NEU: #[route("/mail/jobs")] MailJobsPage {}  — ACHTUNG: muss VOR der :id-Variante stehen?
    Dioxus matcht exakte Segmente vor Parametern; /mail/jobs und /mail/jobs/:id kollidieren nicht.
    Platziere die neue Variante direkt vor `#[route("/mail/jobs/:id")] MailJobDetail`.

Navigation (genossi-frontend/src/component/top_bar.rs:77-91):
  `kommunikation_items` (nur wenn show_admin). Bestehende Einträge: Mail (MailPage),
  MailTemplates (MailTemplatesPage), "Posteingang" (InboxPage). NEU: Eintrag für MailJobsPage.

i18n (genossi-frontend/src/i18n/):
  - Key-Enum in mod.rs. Vorhandene relevante Keys: Key::MailJobs ("Mail-Aufträge"/"Mail Jobs"),
    Key::MailHistory ("Gesendete E-Mails"/"Sent Emails", AKTUELL UNGENUTZT — ideal für Nav-Label
    und Link-Button-Text), Key::MailNoHistory, Key::MailRecipients, Key::MailJobRunning/Done/
    Failed/Pending, Key::MailRetry, Key::MailTo/Status/Error, Key::Loading, Key::MailJobCreated.
  - Übersetzungen MÜSSEN in BEIDEN Locales (de.rs + en.rs) gepflegt werden (nur En + De existieren).
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: MailJobsList-Komponente extrahieren</name>
  <files>genossi-frontend/src/component/mail_jobs_list.rs, genossi-frontend/src/component/mod.rs</files>
  <action>
Erstelle die wiederverwendbare Komponente `genossi-frontend/src/component/mail_jobs_list.rs` (CKN-03, Component-First).

Verschiebe die GESAMTE Job-Listen-Logik aus `mail_page.rs` in diese Komponente:
1. Die beiden privaten Helper-Funktionen `job_status_key` (mail_page.rs:26-33) und `job_status_color` (mail_page.rs:35-42) — kopiere sie hierher. Mache `job_status_key`/`job_status_color` `pub(crate)` NICHT nötig, sie bleiben modul-privat hier.
2. Eine `#[component] pub fn MailJobsList() -> Element`, die ihren EIGENEN State besitzt (kein Props-Durchreichen nötig, da die Komponente self-contained ist):
   - `let mut jobs = use_signal(|| Vec::<MailJobTO>::new());`
   - `let mut loading = use_signal(|| true);`
   - `let mut error: Signal<Option<api::AppError>> = use_signal(|| None);`
   - `let mut expanded_job_id = use_signal(|| None::<String>);`
   - `let mut job_detail = use_signal(|| None::<MailJobDetailTO>);`
   - `let mut toast_messages = use_signal(|| Vec::<(u64, String)>::new());`
   - `let mut toast_counter = use_signal(|| 0u64);`
   - `let i18n = use_i18n();`
   - `let reload_jobs = move || { spawn(async move { ... api::get_mail_jobs ... }) };` (übernehme die Closure 1:1 aus mail_page.rs:133-148)
   - `use_effect(move || { reload_jobs(); });` (Initial-Load wie mail_page.rs:150-152)
   - Im RSX: zuerst optional `ErrorAlert` bei error, dann der Job-History-Block 1:1 übernommen aus mail_page.rs:584-784 (h2 mit Key::MailJobs, Loading/Empty-State, for-Loop, Progress-Bar, Expand, Retry, Recipients-Tabelle mit MailRecipientStatusBadge / NoRepaymentLetterAction / MailRecipientRenderedContent), abschließend `ToastContainer { messages: toast_messages }` (aus mail_page.rs:788).
   - Imports: `use dioxus::prelude::*; use uuid::Uuid; use crate::api::{self, MailJobDetailTO, MailJobTO}; use crate::component::{is_no_repayment_letter_failure, show_toast, ErrorAlert, MailRecipientRenderedContent, MailRecipientStatusBadge, NoRepaymentLetterAction, ToastContainer}; use crate::i18n::{use_i18n, Key}; use crate::service::config::CONFIG;`

Registriere die Komponente in `genossi-frontend/src/component/mod.rs` analog zu den bestehenden Quick-Sektionen:
```
// ─── Quick 260614-ckn ─── MailJobsList (Job-Liste ausgelagert) ──────
pub mod mail_jobs_list;
pub use mail_jobs_list::MailJobsList;
```

Füge am Ende der Datei `mail_jobs_list.rs` ein `#[cfg(test)] mod tests` hinzu (Projekt-Regel: Tests für Änderungen). Teste die reinen Helper-Funktionen:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn job_status_key_maps_known_states() {
        assert_eq!(job_status_key("running"), Key::MailJobRunning);
        assert_eq!(job_status_key("done"), Key::MailJobDone);
        assert_eq!(job_status_key("failed"), Key::MailJobFailed);
        assert_eq!(job_status_key("anything_else"), Key::MailJobPending);
    }
    #[test]
    fn job_status_color_maps_known_states() {
        assert_eq!(job_status_color("running"), "text-blue-600");
        assert_eq!(job_status_color("done"), "text-green-600");
        assert_eq!(job_status_color("failed"), "text-red-600");
        assert_eq!(job_status_color("pending"), "text-gray-600");
    }
}
```
(Stelle sicher, dass `Key` `PartialEq + Debug` ableitet — falls nicht, prüfe i18n/mod.rs; das Key-Enum derived bereits Clone/Copy/PartialEq für Vergleiche im bestehenden Code, z.B. `expanded_job_id.read().as_ref() == Some(...)` betrifft Strings, nicht Key — verifiziere Key-Derives und ergänze ggf. PartialEq/Debug NUR falls sie fehlen.)
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3 && cargo test -p genossi-frontend mail_jobs_list 2>&1 | tail -20</automated>
  </verify>
  <done>mail_jobs_list.rs existiert, exportiert MailJobsList, enthält job_status_key/job_status_color + Tests; component/mod.rs re-exportiert MailJobsList; `cargo test -p genossi-frontend mail_jobs_list` grün.</done>
</task>

<task type="auto">
  <name>Task 2: MailJobsPage anlegen, Route + Nav-Eintrag, i18n</name>
  <files>genossi-frontend/src/page/mail_jobs_page.rs, genossi-frontend/src/page/mod.rs, genossi-frontend/src/router.rs, genossi-frontend/src/component/top_bar.rs, genossi-frontend/src/i18n/mod.rs, genossi-frontend/src/i18n/de.rs, genossi-frontend/src/i18n/en.rs</files>
  <action>
CKN-01: Neue Seite + Route + Navigations-Eintrag.

1. `genossi-frontend/src/page/mail_jobs_page.rs` erstellen. Folge dem Page-Muster (TopBar + RequirePrivilege wie MailPage). Die Seite ist dünn — sie komponiert nur die Komponente (Pages enthalten kein rohes RSX über das Layout hinaus):
```rust
use dioxus::prelude::*;
use crate::auth::RequirePrivilege;
use crate::component::{MailJobsList, TopBar};
use crate::i18n::{use_i18n, Key};

#[component]
pub fn MailJobsPage() -> Element {
    let i18n = use_i18n();
    rsx! {
        RequirePrivilege { privilege: "admin",
            TopBar {}
            div { class: "container mx-auto p-6",
                h1 { class: "text-2xl font-bold mb-6", {i18n.t(Key::MailHistory)} }
                MailJobsList {}
            }
        }
    }
}
```
WICHTIG: Verifiziere die exakte RequirePrivilege-Signatur und das Container-/TopBar-Muster gegen MailPage (mail_page.rs) und eine zweite admin-gated Page (z.B. inbox_page.rs), und passe Privileg/Wrapper exakt an das dort verwendete Muster an. Verwende dieselbe Privilege-Bedingung, die `kommunikation_items` in top_bar.rs gated (show_admin → "admin").

2. `genossi-frontend/src/page/mod.rs`: `pub mod mail_jobs_page;` und `pub use mail_jobs_page::MailJobsPage;` ergänzen (alphabetisch bei den mail-Einträgen).

3. `genossi-frontend/src/router.rs`:
   - Re-Export ergänzen: `pub use crate::page::MailJobsPage;`
   - Route-Variante hinzufügen, direkt VOR `#[route("/mail/jobs/:id")] MailJobDetail`:
     ```
     #[route("/mail/jobs")]
     MailJobsPage {},
     ```

4. `genossi-frontend/src/component/top_bar.rs`: In `kommunikation_items` (nach dem Mail-Eintrag, vor MailTemplates oder nach Posteingang — platziere ihn direkt nach dem Mail-NavItem) einen Eintrag hinzufügen:
   ```rust
   kommunikation_items.push(NavItem {
       label: i18n.t(Key::MailHistory).to_string(),
       route: Route::MailJobsPage {},
   });
   ```

5. i18n: Key::MailHistory ("Gesendete E-Mails" / "Sent Emails") existiert bereits in mod.rs/de.rs/en.rs und ist aktuell ungenutzt — wiederverwenden für Nav-Label, Seiten-Überschrift und Link-Button (Task 3). KEIN neuer Key nötig. Falls ein präziserer Begriff gewünscht ist, ist MailHistory semantisch korrekt ("Gesendete E-Mails"). Keine i18n-Änderung erforderlich, ABER prüfe per grep, dass MailHistory in beiden Locales vorhanden ist; falls ein Locale fehlt, ergänzen.
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3 && cargo check -p genossi-frontend 2>&1 | tail -25</automated>
  </verify>
  <done>MailJobsPage existiert und rendert MailJobsList; Route::MailJobsPage unter /mail/jobs registriert; Nav-Eintrag in kommunikation_items zeigt auf MailJobsPage; `cargo check -p genossi-frontend` fehlerfrei.</done>
</task>

<task type="auto">
  <name>Task 3: Job-Liste aus MailPage entfernen, durch Link ersetzen</name>
  <files>genossi-frontend/src/page/mail_page.rs</files>
  <action>
CKN-02: Job-Liste KOMPLETT von der Versand-Seite entfernen und durch Link/Button ersetzen ("Komplett entfernen + Link", User-Entscheidung).

In `genossi-frontend/src/page/mail_page.rs`:
1. Entferne den kompletten "Mail jobs history"-RSX-Block (Zeilen 584-784) und den `ToastContainer { messages: toast_messages }` (Zeile 788), die zur Job-Liste gehörten.
2. Ersetze den entfernten Job-Listen-Block durch einen Link/Button zur neuen Seite. Verwende `dioxus::prelude::Link` mit `to: Route::MailJobsPage {}` (importiere `use crate::router::Route;` falls nicht vorhanden) ODER `use_navigator()` analog zu vorhandenen Navigations-Mustern im Frontend — prüfe per grep, welches Muster (Link vs navigator().push) im Projekt verbreitet ist und folge dem dominanten Muster. Beschriftung: `{i18n.t(Key::MailHistory)}`. Style analog zu bestehenden Buttons (z.B. die border/rounded card-Optik, in der vorher die Liste stand):
   ```rust
   div { class: "bg-white rounded-lg shadow p-6",
       Link {
           to: Route::MailJobsPage {},
           class: "text-blue-600 hover:underline font-medium",
           {i18n.t(Key::MailHistory)}
       }
   }
   ```
3. Entferne alle nun verwaisten Signale/Closures/Imports, die NUR die Job-Liste betrafen:
   - `jobs`, `expanded_job_id`, `job_detail`, `toast_messages`, `toast_counter` (use_signal-Deklarationen)
   - `reload_jobs`-Closure (mail_page.rs:133-148) und das zugehörige `use_effect(move || { reload_jobs(); });` (150-152)
   - Den `reload_jobs()`-Aufruf nach erfolgreichem Senden (mail_page.rs ~566) — ENTFERNEN (kein Job-State mehr). Die Erfolgsmeldung `success_msg.set(Some(i18n.t(Key::MailJobCreated)...))` BLEIBT.
   - Helper-Funktionen `job_status_key`/`job_status_color` (26-42) löschen (jetzt in der Komponente).
   - Verwaiste Imports bereinigen: `MailJobDetailTO`, `MailJobTO` aus dem api-Import; `is_no_repayment_letter_failure`, `show_toast`, `MailRecipientRenderedContent`, `MailRecipientStatusBadge`, `NoRepaymentLetterAction`, `ToastContainer` aus dem component-Import — ABER nur entfernen, wenn sie nirgends sonst in mail_page.rs (z.B. in MailJobDetail-Funktion ab Zeile 796) genutzt werden. Die `MailJobDetail`-Funktion (796+) bleibt in mail_page.rs unverändert und nutzt einige dieser Imports/Helper weiter — prüfe Nutzung sorgfältig per grep VOR dem Entfernen.

WICHTIG zu den Helpern: `job_status_key`/`job_status_color` werden auch von `MailJobDetail` (mail_page.rs:846-847) genutzt. Lösche sie NICHT aus mail_page.rs, sondern importiere sie aus der Komponente: mache sie in mail_jobs_list.rs `pub(crate)` und ersetze die lokalen Definitionen in mail_page.rs durch `use crate::component::mail_jobs_list::{job_status_key, job_status_color};` — oder belasse Duplikat-Helper in mail_page.rs für MailJobDetail. Bevorzugt: `pub(crate)` in der Komponente + Import (DRY, kein Duplikat). Passe Task-1-Tests entsprechend an (Helper bleiben modul-erreichbar).

Verifiziere nach den Änderungen, dass `cargo clippy -p genossi-frontend` keine "unused"-Warnungen für die bereinigten Imports/Signale wirft.
  </action>
  <verify>
    <automated>cd /home/neosam/programming/rust/projects/genossi3 && cargo check -p genossi-frontend 2>&1 | tail -15 && echo "=== unused check ===" && cargo build -p genossi-frontend 2>&1 | grep -iE "unused|never used" | head</automated>
  </verify>
  <done>mail_page.rs enthält keinen Job-Listen-RSX-Block mehr; stattdessen Link zu Route::MailJobsPage; keine verwaisten Signale/Imports; MailJobDetail-Funktion weiterhin intakt; `cargo build -p genossi-frontend` ohne unused-Warnungen aus diesem Plan.</done>
</task>

</tasks>

<verification>
- `cargo test -p genossi-frontend mail_jobs_list` grün (Helper-Tests).
- `cargo check -p genossi-frontend` und `cargo build -p genossi-frontend` fehlerfrei.
- `cargo clippy -p genossi-frontend` ohne neue unused-Warnungen.
- Grep-Gate: `grep -c MailJobsList genossi-frontend/src/component/mail_jobs_list.rs` ≥ 1; `grep MailJobsPage genossi-frontend/src/router.rs` matcht; `grep -c "Mail jobs history" genossi-frontend/src/page/mail_page.rs` == 0 (Job-Liste entfernt).
</verification>

<success_criteria>
- /mail zeigt keine Job-Liste, sondern einen Link zur Job-Seite (CKN-02).
- /mail/jobs zeigt die vollständige Job-Liste über die MailJobsList-Komponente (CKN-01).
- Nav-Eintrag (Kommunikation-Gruppe) führt zu /mail/jobs (CKN-01).
- Job-Listen-UI lebt als wiederverwendbare Komponente in src/component/ (CKN-03, Component-First).
- Frontend kompiliert und Helper-Tests laufen grün.
</success_criteria>

<output>
After completion, create `.planning/quick/260614-ckn-job-liste-vom-mail-versand-auf-separate-/260614-ckn-SUMMARY.md`
</output>
