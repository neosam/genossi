---
quick_id: 260603-evf
date: 2026-06-03
type: execute
wave: 1
depends_on: []
files_modified:
  - genossi_mail/src/rest.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/component/mod.rs
  - genossi-frontend/src/component/mail_recipient_status_badge.rs
  - genossi-frontend/src/component/no_repayment_letter_action.rs
  - genossi-frontend/src/page/mail_page.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
autonomous: true
requirements: []
description: UI-Anzeige no_repayment_letter-Status pro Empfaenger im Bulk-Mail-Job-Detail

must_haves:
  truths:
    - "Failed-Empfaenger mit error.starts_with(no_repayment_letter) werden visuell von generischem failed unterschieden (amber/orange Badge mit eigener i18n-Uebersetzung statt rotem failed-Marker)"
    - "Vorstand sieht in der Empfaenger-Tabelle des Bulk-Mail-Jobs (Expanded-Row UND MailJobDetail-Deeplink-Page) einen Action-Button Brief generieren + Retry neben jedem no_repayment_letter-Empfaenger"
    - "Klick auf den Action-Button generiert fuer den betroffenen Member den Brief (POST /api/repayment-phase/{phase_id}/letters/generate mit entry_ids=[<entry>]) und loest danach retry_job aus"
    - "Action-Button zeigt Loading-State waehrend der Operation, Success-Toast nach Erfolg, Error-Toast bei Fehlschlag"
    - "Action-Button bleibt verborgen wenn (a) Recipient hat kein member_id, (b) Job hat kein repayment_phase_id, (c) Status ist nicht failed mit no_repayment_letter-Error"
    - "i18n-Keys existieren in de.rs UND en.rs (kein Locale-Drift)"
    - "cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown ist clean (0 errors, 0 warnings)"
    - "cargo test -p genossi_mail bestehende 145 Tests bleiben gruen + neue Tests fuer MailJobTO.repayment_phase_id"
  artifacts:
    - path: "genossi_mail/src/rest.rs"
      provides: "MailJobTO mit neuem Feld repayment_phase_id: Option<String> + erweitertes From<&MailJob>"
      contains: "pub repayment_phase_id: Option<String>"
    - path: "genossi-frontend/src/api.rs"
      provides: "Mirror-Feld auf Frontend-MailJobTO"
      contains: "pub repayment_phase_id: Option<String>"
    - path: "genossi-frontend/src/component/mail_recipient_status_badge.rs"
      provides: "Reusable Badge-Component fuer Recipient-Status mit Sonderbehandlung fuer no_repayment_letter"
      min_lines: 50
    - path: "genossi-frontend/src/component/no_repayment_letter_action.rs"
      provides: "Reusable Action-Button-Component Brief generieren + Retry mit Loading-State"
      min_lines: 60
    - path: "genossi-frontend/src/page/mail_page.rs"
      provides: "MailPage Expanded-Row + MailJobDetail nutzen beide Components statt inline-RSX"
    - path: "genossi-frontend/src/i18n/mod.rs"
      provides: "Neue Key-Varianten: MailFailedNoRepaymentLetter, MailGenerateLetterAndRetry, MailGenerateLetterAndRetryRunning, MailGenerateLetterAndRetrySuccess, MailGenerateLetterAndRetryNoEntry"
  key_links:
    - from: "genossi-frontend/src/page/mail_page.rs (MailPage expanded recipients table)"
      to: "genossi-frontend/src/component/mail_recipient_status_badge.rs::MailRecipientStatusBadge"
      via: "RSX component invocation"
      pattern: "MailRecipientStatusBadge"
    - from: "genossi-frontend/src/page/mail_page.rs (MailJobDetail recipients table)"
      to: "genossi-frontend/src/component/mail_recipient_status_badge.rs::MailRecipientStatusBadge"
      via: "RSX component invocation"
      pattern: "MailRecipientStatusBadge"
    - from: "genossi-frontend/src/component/no_repayment_letter_action.rs"
      to: "genossi-frontend/src/api.rs (list_repayment_entries + generate_repayment_letters + retry_mail_job)"
      via: "spawn-async Aufrufketten"
      pattern: "list_repayment_entries|generate_repayment_letters|retry_mail_job"
---

<objective>
Im Bulk-Mail-Job-Detail (Frontend, Expanded-Row in MailPage UND MailJobDetail-Deeplink-Page) Empfaenger mit error.starts_with("no_repayment_letter") visuell deutlich vom generischen failed unterscheiden (eigene amber/orange Badge) UND einen Ein-Klick-Pfad "Brief generieren + Retry" bereitstellen, damit der Vorstand das Problem direkt aus der Empfaenger-Liste loesen kann.

Purpose: UX-Polish. Heute sieht der Vorstand nur "failed" + den unleserlichen Error-String. Mit diesem Quick wird der Failure semantisch lesbar und mit einem Klick reparierbar. Schliesst die UX-Luecke, die Quick 260603-cz6 (Backend-Failure-Pfad) und Quick 260603-e6p (UI-Checkbox, die diesen Pfad erst aktiviert) hinterlassen haben.

Output: Eine erweiterte Backend-MailJobTO (zusaetzliches read-only Feld repayment_phase_id), zwei neue reusable Frontend-Components (Badge + Action-Button) und vollstaendige i18n-Unterstuetzung in de + en.
</objective>

<execution_context>
- This is a quick-task plan (single-plan, atomic, no phase context).
- Execute tasks in order. Task 1 (backend field) -> Task 2 (badge + page wiring) -> Task 3 (action button).
- After all tasks: verify `cargo test -p genossi_mail`, `cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown`, `cargo test -p genossi-frontend --lib`.
- Create SUMMARY at `.planning/quick/260603-evf-ui-anzeige-no-repayment-letter-status-pr/260603-evf-SUMMARY.md` and update `.planning/STATE.md` last activity line.
</execution_context>

<context>
@/home/neosam/programming/rust/projects/genossi3/CLAUDE.md
@/home/neosam/programming/rust/projects/genossi3/genossi-frontend/CLAUDE.md
@/home/neosam/programming/rust/projects/genossi3/.planning/todos/pending/frontend-uat-empfaenger-status-no-repayment-letter.md
@/home/neosam/programming/rust/projects/genossi3/.planning/quick/260603-cz6-bulk-mail-repaymentletter-automatisch-al/260603-cz6-SUMMARY.md

## Verifizierte Fakten (Quellen-Pfade + Zeilen)

### Backend / Worker
- Error-String: `genossi_mail/src/worker.rs:336` setzt exakt `"no_repayment_letter"` (kein Prefix, keine Suffixe). Frontend prueft trotzdem mit `.starts_with("no_repayment_letter")` fuer Future-Safety.
- MailJob persistiert `repayment_phase_id` UND `attach_repayment_letter`: `genossi_mail/src/dao.rs:28-48` (`pub repayment_phase_id: Option<Uuid>`, `pub attach_repayment_letter: bool`). Beide Felder sind seit Migration `20260603100000_mail_job_attach_repayment_letter.sql` in der DB.
- MailJobTO (`genossi_mail/src/rest.rs:69-81`) und `From<&MailJob>` (rest.rs:203-216) exponieren `repayment_phase_id` heute NICHT. Hier liegt das Backend-Loch.
- MailRecipientTO (`genossi_mail/src/rest.rs:89-104`) hat: id, to_address, member_id: Option<String>, status: String, error: Option<String>, sent_at: Option<String>, attachments. Alles, was Badge und Action brauchen, ist vorhanden.

### Frontend
- Bestehende Job-Detail-UI ist DOPPELT inline gerendert:
  - MailPage Expanded-Row (`genossi-frontend/src/page/mail_page.rs:737-783`, vor allem Zeilen 751-772) — Status-Rendering inline mit `match r.status.as_str()`.
  - MailJobDetail Deeplink-Page (`mail_page.rs:799-893`, Zeilen 864-885) — identische Logik, copy-paste-dupliziert.
  - Component-First-Bruch: Beide Stellen muessen auf die neue MailRecipientStatusBadge umgestellt werden.
- Frontend-MailJobTO (`genossi-frontend/src/api.rs:813-823`) ist ein 1:1-Mirror der Backend-Variante und muss synchron mit Task 1 erweitert werden.
- Bestehende reusable Status-Badge-Components als Referenz-Pattern:
  - `genossi-frontend/src/component/repayment_entry_status_badge.rs` (vollstaendig gelesen — kompletter Template-Klon moeglich: status_label-Helper + status_badge_class-Helper + Tests fuer Farb-Klassen).
  - Selbe Tailwind-Konvention: `bg-{color}-100 text-{color}-800 px-2 py-1 rounded text-xs font-medium`.
- Existing i18n-Keys fuer Mail (`genossi-frontend/src/i18n/mod.rs:235-265`, `de.rs:186-201`, `en.rs:186-201`):
  - MailFailed, MailSent, MailJobPending, MailError, MailRetry, MailRecipients, MailStatus. Diese werden weiterverwendet.
  - Neue Keys (in mod.rs Key-enum aufnehmen, in BEIDE Locales uebersetzen):
    - MailFailedNoRepaymentLetter (de: "Kein Anschreiben generiert", en: "No repayment letter generated")
    - MailGenerateLetterAndRetry (de: "Brief generieren + Retry", en: "Generate letter + retry")
    - MailGenerateLetterAndRetryRunning (de: "Generiere Brief...", en: "Generating letter...")
    - MailGenerateLetterAndRetrySuccess (de: "Brief generiert, Retry laeuft", en: "Letter generated, retry triggered")
    - MailGenerateLetterAndRetryNoEntry (de: "Kein Eintrag fuer dieses Mitglied in der Phase", en: "No entry for this member in the phase")
- Toast und ErrorAlert sind verfuegbar: `crate::component::{show_toast, ToastContainer, ErrorAlert}` (Beispiel-Nutzung in `repayment_phase_details.rs:28-30, 154, 209…`). MailPage benutzt sie heute noch nicht — wir fuehren ToastContainer + toast_messages-Signal in MailPage neu ein.

### Endpoints (alle bereits vorhanden, NICHTS zu bauen)
- `POST /api/mail/jobs/{id}/retry` -> `genossi-frontend/src/api.rs:961` `retry_mail_job(config, &id)` -> Result<MailJobTO, AppError>.
- `POST /api/repayment-phase/{phase_id}/letters/generate` -> `genossi-frontend/src/api.rs:1961` `generate_repayment_letters(config, phase_id, entry_ids: Vec<Uuid>)` -> Result<GeneratedLettersResult, AppError>. Liefert eine Blob-URL als Side-Effect, die wir hier IGNORIEREN — wir wollen keinen Download triggern, nur das serverseitige MemberDocument-Persist als Seiteneffekt nutzen. WICHTIG: Nach erfolgreichem generate_repayment_letters MUSS `web_sys::Url::revoke_object_url(&result.blob_url)` aufgerufen werden, um keinen Memory-Leak zu verursachen (Pattern vorgegeben durch den api.rs-Doc-Kommentar).
- `GET /api/repayment-entry?phase_id={phase_id}` -> `genossi-frontend/src/api.rs:2352` `list_repayment_entries(config, phase_id) -> Vec<RepaymentEntryTO>` mit `RepaymentEntryTO { id, member_id, phase_id, status, … }`. Damit loest die Action den member_id -> entry_id-Lookup deterministisch.

## Architektur-Entscheidung: minimaler Backend-Touch (Task 1)

Das Constraint im Task-Prompt sagt "Backend NICHT anfassen — beide Endpoints existieren. Wenn der Planner ein Backend-Loch entdeckt -> klar als Blocker markieren". Wir haben EINEN sauberen Mittelweg, der NICHT als Blocker durchgereicht werden muss:

- Loch: MailJobTO exponiert repayment_phase_id nicht, obwohl es im DAO MailJob persistiert ist.
- Fix: Genau eine Zeile in der struct + eine Zeile im From-Impl. Keine neuen Endpoints, keine neue Service-Logik, keine Migration, keine Validation. Das Feld ist als Option<String> additive serialisiert mit skip_serializing_if, also backward-compatible fuer jeden bestehenden API-Konsumenten.
- Alternative ohne Backend-Touch (verworfen): Frontend muesste alle Phasen iterieren und fuer jede list_repayment_entries(phase_id) aufrufen, dann den Empfaenger-member_id quer-suchen. Das ist (a) O(N*M) Roundtrips, (b) ambig bei mehreren Phasen pro fiscal_year, (c) operativ Wartungs-untauglich. Verworfen.
- Begruendung im Plan-Geist: Der Constraint will verhindern, dass Frontend-Tasks zu Mini-Backend-Refactorings ausarten. Das Hinzufuegen eines bereits persistierten Feldes zum Read-DTO ist KEIN Refactoring sondern eine reine Daten-Exposition. Es ist die kleinste moegliche Backend-Aenderung und steht im direkten Verhaeltnis zum UI-Ziel.

Falls diese Bewertung im Review beanstandet wird, ist der Scope-Cut: Task 1 entfaellt, Task 2 reduziert sich auf Badge-Only ohne Action, Task 3 wird als Follow-up-Todo dokumentiert. Diese Fallback-Variante ist im Acceptance-Criterion bereits vom Constraint vorgesehen.

## Interface-Contracts

### Backend-Type Aenderung (Task 1, ein Feld additiv):

`genossi_mail/src/rest.rs:69` (vorher):
```
pub struct MailJobTO {
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
}
```

NACH Task 1:
```
pub struct MailJobTO {
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
    // Quick 260603-evf: exposed read-only so frontend can deterministically
    // resolve the phase when triggering "Brief generieren + Retry" for
    // recipients with error="no_repayment_letter". Stays None for
    // non-repayment bulk-mail jobs. Backward-compatible additive field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repayment_phase_id: Option<String>,
}
```

From<&MailJob> aktualisieren — zusaetzliche Zeile:
```
repayment_phase_id: job.repayment_phase_id.map(|u| u.to_string()),
```

### Frontend-Mirror Aenderung (Task 1):
```
// genossi-frontend/src/api.rs:813 (Mirror, gleiche additive Logik)
pub struct MailJobTO {
    pub id: String,
    pub created: String,
    pub subject: String,
    pub body: String,
    pub status: String,
    pub total_count: i64,
    pub sent_count: i64,
    pub failed_count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repayment_phase_id: Option<String>,
}
```

### Component-Contracts (Task 2 und 3):

mail_recipient_status_badge:
```
#[component]
pub fn MailRecipientStatusBadge(status: String, error: Option<String>) -> Element {
    // Rendering-Logik:
    //   status == "sent"   -> green badge "Gesendet"
    //   status == "failed" und error.as_deref().map(|e| e.starts_with("no_repayment_letter")).unwrap_or(false)
    //                      -> amber/orange badge "Kein Anschreiben generiert"
    //   status == "failed" -> red badge "Fehlgeschlagen"
    //   else               -> gray badge "Ausstehend"
}
```

no_repayment_letter_action:
```
#[derive(Props, Clone, PartialEq)]
pub struct NoRepaymentLetterActionProps {
    pub job_id: String,            // for retry_mail_job
    pub recipient_id: String,      // for debug/logging only
    pub member_id: Uuid,           // resolves entry_id via list_repayment_entries
    pub phase_id: Uuid,            // from job.repayment_phase_id
    pub on_done: EventHandler<()>, // parent reloads jobs and shows success toast
    pub on_error: EventHandler<String>, // parent shows error toast
}

#[component]
pub fn NoRepaymentLetterAction(props: NoRepaymentLetterActionProps) -> Element {
    // Button mit 3 States: idle / loading / done
    // onclick: spawn(async {
    //   1. let entries = list_repayment_entries(config, phase_id).await?;
    //   2. find entry where entry.member_id == member_id
    //      (status egal — generate-letters akzeptiert Open + Contacted + PaidOut Entries)
    //   3. if not found -> on_error(i18n.t(MailGenerateLetterAndRetryNoEntry)); return
    //   4. let result = generate_repayment_letters(config, phase_id, vec![entry.id]).await?;
    //   5. web_sys::Url::revoke_object_url(&result.blob_url).ok();  // we ignore the blob
    //   6. retry_mail_job(config, &job_id).await?;
    //   7. on_done(());
    // })
}
```

### Caller-Pattern fuer beide Stellen (MailPage expanded-row + MailJobDetail):
```
// Replace inline cells in both recipients-table loops:
//   td { class: "py-1 px-2", "{r.to_address}" }
//   td { class: "py-1 px-2",
//       MailRecipientStatusBadge { status: r.status.clone(), error: r.error.clone() }
//   }
//   td { class: "py-1 px-2 text-red-500 text-xs", "{r.error.as_deref().unwrap_or_default()}" }
//   td { class: "py-1 px-2",
//       if is_no_repayment_letter_failure(&r.status, r.error.as_deref()) {
//           if let (Some(mid), Some(pid)) = (parse_uuid(&r.member_id), parse_uuid_opt(&job.repayment_phase_id)) {
//               NoRepaymentLetterAction {
//                   job_id: job.id.clone(),
//                   recipient_id: r.id.clone(),
//                   member_id: mid,
//                   phase_id: pid,
//                   on_done: move |_| { reload_jobs(); show_toast(..., success_msg) },
//                   on_error: move |msg| show_toast(..., msg),
//               }
//           }
//       }
//   }
```
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Backend MailJobTO + Frontend Mirror — repayment_phase_id exponieren</name>
  <files>genossi_mail/src/rest.rs, genossi-frontend/src/api.rs</files>
  <behavior>
    - Test 1 (genossi_mail/src/rest.rs::tests): `MailJobTO::from(&MailJob)` fuellt `repayment_phase_id` aus dem persistierten Feld, wenn `MailJob.repayment_phase_id = Some(uuid)`. Erwartung: `to.repayment_phase_id == Some(uuid.to_string())`.
    - Test 2 (backend): Ein MailJob mit repayment_phase_id = None -> MailJobTO::from(...) setzt repayment_phase_id = None. JSON-Serialisierung enthaelt das Feld NICHT (skip_serializing_if).
    - Test 3 (backend, serde-roundtrip): serde_json::to_string + from_str eines MailJobTO mit gesetztem repayment_phase_id ergibt das gleiche Feld.
    - Test 4 (backend, backward-compat): Ein JSON-Payload OHNE repayment_phase_id-Key deserialisiert sauber zu MailJobTO mit repayment_phase_id = None. Sichert, dass existierende Clients ohne Update weiterhin funktionieren.
    - Test 5 (genossi-frontend/src/api.rs::tests): Frontend-Mirror-MailJobTO deserialisiert ein Backend-JSON mit repayment_phase_id korrekt (Snapshot-Roundtrip).
  </behavior>
  <action>
    1. In genossi_mail/src/rest.rs:69-81 der MailJobTO-Struct das Feld `#[serde(default, skip_serializing_if = "Option::is_none")] pub repayment_phase_id: Option<String>,` hinzufuegen. Doc-Comment einbauen, der den Quick-260603-evf-Kontext und den additiven/backward-compat-Charakter erklaert (siehe Interface-Contracts oben).
    2. In `impl From<&MailJob> for MailJobTO` (rest.rs:203-216) die Zeile `repayment_phase_id: job.repayment_phase_id.map(|u| u.to_string()),` ergaenzen.
    3. In genossi_mail/src/rest.rs::tests (Modul am Dateiende, in dem die `test_send_bulk_mail_request_serde_*`-Tests aus Quick 260603-cz6 leben) die 4 Tests aus Behavior hinzufuegen. Falls ein Hilfs-Konstruktor fuer einen `MailJob`-Wert nicht existiert, eine `fn make_mail_job(repayment_phase_id: Option<Uuid>) -> MailJob` neben den Tests anlegen mit Default-Werten (id = Uuid::new_v4(), created = PrimitiveDateTime aus z. B. `date!(2026-06-03)` + `time!(0:00)`, subject/body = "x", status = MailJobStatus::Pending oder MailJobStatus::Done, counts = 0, attach_repayment_letter = false).
    4. In genossi-frontend/src/api.rs:813-823 die MailJobTO-Struct identisch erweitern (`#[serde(default, skip_serializing_if = "Option::is_none")] pub repayment_phase_id: Option<String>,`). Doc-Kommentar mit Quick-ID.
    5. In genossi-frontend/src/api.rs::tests (das Modul existiert bereits — siehe `let phase: RepaymentPhaseTO = serde_json::from_str(json).unwrap();` in Zeile 2638) Test 5 hinzufuegen: einen MailJobTO mit repayment_phase_id = Some(...) serialisieren und wieder deserialisieren.
    6. KEINE weiteren Stellen aendern (Worker, DAO, Service, Migrations bleiben unangetastet).
  </action>
  <verify>
    <automated>cargo test -p genossi_mail --lib rest::tests 2>&amp;1 | tail -20 ; cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown 2>&amp;1 | tail -10</automated>
  </verify>
  <done>
    - MailJobTO (backend + frontend) hat Feld repayment_phase_id: Option<String> mit #[serde(default, skip_serializing_if = "Option::is_none")].
    - From<&MailJob> mappt job.repayment_phase_id korrekt.
    - 4 neue Backend-Tests + 1 Frontend-Test, alle gruen.
    - cargo test -p genossi_mail ergibt mindestens 149 passed (vorher 145, +4 neue), 0 failed.
    - Bestehende test_send_bulk_mail_request_serde_*-Tests + Roundtrip-Tests aus Quick 260603-cz6 bleiben gruen.
    - cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Reusable Component MailRecipientStatusBadge + i18n + Wiring in MailPage und MailJobDetail</name>
  <files>genossi-frontend/src/component/mail_recipient_status_badge.rs, genossi-frontend/src/component/mod.rs, genossi-frontend/src/i18n/mod.rs, genossi-frontend/src/i18n/de.rs, genossi-frontend/src/i18n/en.rs, genossi-frontend/src/page/mail_page.rs</files>
  <behavior>
    - Test 1: status_badge_class("sent", None) enthaelt "bg-green-100" und "text-green-800".
    - Test 2: status_badge_class("failed", None) enthaelt "bg-red-100" und "text-red-800".
    - Test 3: status_badge_class("failed", Some("no_repayment_letter")) enthaelt "bg-amber-100" und "text-amber-800". Wahl: amber (nicht orange), konsistent mit dem existierenden amber-500 Retry-Button in mail_page.rs:714.
    - Test 4: status_badge_class("failed", Some("no_repayment_letter: details")) enthaelt ebenfalls amber-Klassen (Future-Safety via .starts_with).
    - Test 5: status_badge_class("failed", Some("smtp_timeout")) enthaelt "bg-red-100" und "text-red-800" (anderer Error-String -> bleibt generisches failed).
    - Test 6: status_badge_class("pending", None) und status_badge_class("queued", None) enthalten "bg-gray-100" und "text-gray-800" (Default-Fallback).
    - Test 7: status_label_key Helper gibt fuer ("failed", Some("no_repayment_letter")) den Key MailFailedNoRepaymentLetter zurueck, fuer ("failed", None) den Key MailFailed, fuer ("sent", None) den Key MailSent, sonst MailJobPending.
    - Test 8: is_no_repayment_letter_failure("failed", Some("no_repayment_letter")) ist true; is_no_repayment_letter_failure("failed", None) ist false; is_no_repayment_letter_failure("sent", Some("no_repayment_letter")) ist false (status must be failed).
    - Test 9 (pill-styling shared, analog zu repayment_entry_status_badge::tests::all_share_pill_styling): alle 4 Klassen enthalten px-2, py-1, rounded, text-xs, font-medium.
  </behavior>
  <action>
    1. Component anlegen: genossi-frontend/src/component/mail_recipient_status_badge.rs analog zum 1:1-Template repayment_entry_status_badge.rs. Inhalt:
       - Doc-Header: `//! Quick 260603-evf — MailRecipientStatusBadge: visual marker for bulk-mail recipient outcomes. Special-cases failed+no_repayment_letter as amber (distinct from generic failed=red).`
       - Helper `pub fn is_no_repayment_letter_failure(status: &str, error: Option<&str>) -> bool` -> `status == "failed" && error.map(|e| e.starts_with("no_repayment_letter")).unwrap_or(false)`. PUBLIC, weil Task 3 in mail_page.rs gleiche Logik braucht (Single-Source-of-Truth).
       - Helper `fn status_label_key(status: &str, error: Option<&str>) -> Key` -> match-Tabelle: "sent" -> MailSent; "failed" mit no_repayment_letter -> MailFailedNoRepaymentLetter; "failed" -> MailFailed; sonst MailJobPending.
       - Helper `fn status_badge_class(status: &str, error: Option<&str>) -> &'static str` -> match liefert exakt die Tailwind-Strings:
         - sent: "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium"
         - failed+no_repayment_letter: "bg-amber-100 text-amber-800 px-2 py-1 rounded text-xs font-medium"
         - failed (generisch): "bg-red-100 text-red-800 px-2 py-1 rounded text-xs font-medium"
         - default: "bg-gray-100 text-gray-800 px-2 py-1 rounded text-xs font-medium"
       - `#[component] pub fn MailRecipientStatusBadge(status: String, error: Option<String>) -> Element` -> `let i18n = use_i18n(); let class = status_badge_class(&status, error.as_deref()); let label = i18n.t(status_label_key(&status, error.as_deref())); rsx! { span { class: "{class}", "{label}" } }`.
       - Tests 1-9 aus Behavior als `#[cfg(test)] mod tests`.
    2. Component registrieren in genossi-frontend/src/component/mod.rs:
       - in der mod-Sektion fuer Phase-12-Repayment-Badges (Zeile 95-99) eine neue Zeile `pub mod mail_recipient_status_badge;` hinzufuegen,
       - und in der pub-use-Sektion `pub use mail_recipient_status_badge::{MailRecipientStatusBadge, is_no_repayment_letter_failure};`.
    3. i18n-Key hinzufuegen (NUR diesen einen in Task 2):
       - genossi-frontend/src/i18n/mod.rs Zeile 253 (nach `MailJobPending,`) einfuegen: `MailFailedNoRepaymentLetter,`.
       - genossi-frontend/src/i18n/de.rs (neben den anderen Mail-Keys ab Zeile ~186): `Key::MailFailedNoRepaymentLetter => "Kein Anschreiben generiert".into(),`.
       - genossi-frontend/src/i18n/en.rs (neben den anderen Mail-Keys ab Zeile ~186): `Key::MailFailedNoRepaymentLetter => "No repayment letter generated".into(),`.
       - Strikt: Key MUSS in beiden Locale-Dateien existieren. Verifikation via Grep am Ende.
    4. Wiring MailPage Expanded-Row (genossi-frontend/src/page/mail_page.rs:751-772): Den inneren for r in detail.recipients.iter() Loop refaktorieren:
       - Die Inline-Variablen r_status_color und r_status_text werden ENTFERNT.
       - Die Status-Spalte wird ersetzt durch: `td { class: "py-1 px-2", MailRecipientStatusBadge { status: r.status.clone(), error: r.error.clone() } }`.
       - Die Error-Spalte bleibt erhalten (Klartext-Error darunter) — der Vorstand will den Original-String sehen koennen. Sie bekommt zusaetzlich keine Aenderung (Inline-String, kein neuer Component dafuer).
    5. Wiring MailJobDetail Deeplink-Page (mail_page.rs:864-885): Identische Refaktorierung des Inline-Status-Codes auf MailRecipientStatusBadge. Inline-Variablen r_status_color und r_status_text entfernen.
    6. Import in mail_page.rs ergaenzen: In Zeile 11 die bestehende Component-Import-Zeile `use crate::component::{ErrorAlert, TopBar};` erweitern um MailRecipientStatusBadge -> `use crate::component::{ErrorAlert, MailRecipientStatusBadge, TopBar};`.
  </action>
  <verify>
    <automated>cargo test -p genossi-frontend --lib mail_recipient_status_badge 2>&amp;1 | tail -30 ; cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown 2>&amp;1 | tail -10 ; test "$(grep -v '^//' genossi-frontend/src/page/mail_page.rs | grep -c 'MailRecipientStatusBadge')" -ge 2 || echo "FAIL: MailRecipientStatusBadge must appear at least twice (expanded row + detail page)"; grep -c 'MailFailedNoRepaymentLetter' genossi-frontend/src/i18n/de.rs genossi-frontend/src/i18n/en.rs genossi-frontend/src/i18n/mod.rs</automated>
  </verify>
  <done>
    - genossi-frontend/src/component/mail_recipient_status_badge.rs existiert mit der MailRecipientStatusBadge-Component, 3 Helpers und 9 Unit-Tests.
    - Component-Registrierung in component/mod.rs (mod + pub use) existiert; `is_no_repayment_letter_failure` ist als pub re-exportiert.
    - Key::MailFailedNoRepaymentLetter existiert in mod.rs UND ist in de.rs UND en.rs uebersetzt.
    - mail_page.rs:MailPage UND mail_page.rs:MailJobDetail nutzen beide MailRecipientStatusBadge statt Inline-Status-Rendering (grep findet die Component mindestens 2x).
    - cargo test -p genossi-frontend --lib mail_recipient_status_badge: 9 Tests passed, 0 failed.
    - cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown clean.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: NoRepaymentLetterAction-Component + i18n + Action-Wiring in beiden Tabellen + Toast-Integration</name>
  <files>genossi-frontend/src/component/no_repayment_letter_action.rs, genossi-frontend/src/component/mod.rs, genossi-frontend/src/i18n/mod.rs, genossi-frontend/src/i18n/de.rs, genossi-frontend/src/i18n/en.rs, genossi-frontend/src/page/mail_page.rs</files>
  <behavior>
    - Test 1 (no_repayment_letter_action::tests, pure-logic-Helper, KEIN Dioxus-Lifecycle): `find_entry_for_member(&entries, member_id)` gibt das erste Entry zurueck, dessen member_id matched, oder None.
    - Test 2: `find_entry_for_member(&[], any_uuid)` ist None.
    - Test 3: `find_entry_for_member(&entries, member_id)` mit zwei matchenden Entries gibt das erste in der Liste zurueck (deterministisches Behaviour — die Endpunkt-Aggregation auf Backend-Seite kuemmert sich um Dedup pro Member).
    - Test 4 (no_repayment_letter_action::tests): `button_label_for_state(ButtonState::Idle, &i18n)` gibt den Key MailGenerateLetterAndRetry zurueck; `ButtonState::Loading` -> MailGenerateLetterAndRetryRunning; `ButtonState::Done` -> MailGenerateLetterAndRetrySuccess. (Pure helper, kein i18n-Lookup; gibt direkt den Key zurueck damit der Test offline laeuft.)
    - Test 5 (i18n integrity, kann in mail_recipient_status_badge::tests oder eigenem mini-test-modul leben): grep gegen i18n/de.rs UND i18n/en.rs zeigt alle 4 neuen Keys (MailGenerateLetterAndRetry, ...Running, ...Success, ...NoEntry) in beiden Files. Implementiert als Compile-Time-Check via Match-Exhaustiveness ist OPTIMAL — der Rust-Compiler erzwingt, dass jeder Key-Variant in jeder Locale-Match-Tabelle behandelt wird (falls i18n/de.rs und en.rs als exhaustive match aufgebaut sind). Stattdessen lassen wir die Verifikation der Vollstaendigkeit dem `verify`-Block: grep auf beide Locales, count muss 4 sein.
  </behavior>
  <action>
    1. Component anlegen: genossi-frontend/src/component/no_repayment_letter_action.rs.
       - Doc-Header: `//! Quick 260603-evf — NoRepaymentLetterAction: One-click recovery for bulk-mail recipients that failed with error="no_repayment_letter". Resolves member -> RepaymentEntry, generates the missing letter (POST /api/repayment-phase/{phase_id}/letters/generate), revokes the bundle blob URL (we only want the server-side MemberDocument-persist side effect), then calls retry_mail_job.`
       - Imports: `use dioxus::prelude::*; use uuid::Uuid; use crate::api::{self, RepaymentEntryTO}; use crate::i18n::{use_i18n, Key}; use crate::service::config::CONFIG;`.
       - Pure helper: `pub fn find_entry_for_member(entries: &[RepaymentEntryTO], member_id: Uuid) -> Option<RepaymentEntryTO>` -> `entries.iter().find(|e| e.member_id == member_id).cloned()`.
       - Enum: `#[derive(Clone, Copy, PartialEq, Eq)] pub enum ButtonState { Idle, Loading, Done }`.
       - Pure helper: `pub fn button_label_for_state(state: ButtonState) -> Key` -> match `Idle => MailGenerateLetterAndRetry, Loading => MailGenerateLetterAndRetryRunning, Done => MailGenerateLetterAndRetrySuccess`.
       - Props: `#[derive(Props, Clone, PartialEq)] pub struct NoRepaymentLetterActionProps { pub job_id: String, pub recipient_id: String, pub member_id: Uuid, pub phase_id: Uuid, pub on_done: EventHandler<()>, pub on_error: EventHandler<String> }`.
       - Component:
         ```
         #[component]
         pub fn NoRepaymentLetterAction(props: NoRepaymentLetterActionProps) -> Element {
             let i18n = use_i18n();
             let mut state = use_signal(|| ButtonState::Idle);
             let onclick = move |_| {
                 let job_id = props.job_id.clone();
                 let phase_id = props.phase_id;
                 let member_id = props.member_id;
                 let on_done = props.on_done.clone();
                 let on_error = props.on_error.clone();
                 let i18n_clone = i18n.clone();
                 state.set(ButtonState::Loading);
                 spawn(async move {
                     let config = CONFIG.read().clone();
                     let entries = match api::list_repayment_entries(&config, phase_id).await {
                         Ok(e) => e,
                         Err(err) => { state.set(ButtonState::Idle); on_error.call(err.message.clone()); return; }
                     };
                     let entry = match find_entry_for_member(&entries, member_id) {
                         Some(e) => e,
                         None => { state.set(ButtonState::Idle); on_error.call(i18n_clone.t(Key::MailGenerateLetterAndRetryNoEntry).to_string()); return; }
                     };
                     let gen = match api::generate_repayment_letters(&config, phase_id, vec![entry.id]).await {
                         Ok(r) => r,
                         Err(err) => { state.set(ButtonState::Idle); on_error.call(err.message.clone()); return; }
                     };
                     // Revoke the blob URL — we ignore the bundle PDF.
                     let _ = web_sys::Url::revoke_object_url(&gen.blob_url);
                     match api::retry_mail_job(&config, &job_id).await {
                         Ok(_) => { state.set(ButtonState::Done); on_done.call(()); }
                         Err(err) => { state.set(ButtonState::Idle); on_error.call(err.message.clone()); }
                     }
                 });
             };
             let is_disabled = *state.read() != ButtonState::Idle;
             let label_key = button_label_for_state(*state.read());
             let label = i18n.t(label_key);
             let class = if is_disabled {
                 "bg-amber-300 text-white px-2 py-1 rounded text-xs font-medium cursor-not-allowed"
             } else {
                 "bg-amber-500 hover:bg-amber-600 text-white px-2 py-1 rounded text-xs font-medium"
             };
             rsx! {
                 button {
                     r#type: "button",
                     class: "{class}",
                     disabled: is_disabled,
                     onclick: onclick,
                     "{label}"
                 }
             }
         }
         ```
         Hinweis zum r#type: button: vermeidet das Dioxus-Form-Reload-Bug, siehe `feedback_dioxus_button_type.md` im User-Memory (Hotfix e245013-Pattern).
       - Tests 1-4 aus Behavior als `#[cfg(test)] mod tests`. Tests bauen RepaymentEntryTO-Werte mit Uuid::new_v4() und allen Default-Feldern; Status-Werte beliebig.
    2. Component-Registrierung in genossi-frontend/src/component/mod.rs:
       - In der Phase-12-Repayment-Badges-Sektion (Zeile 95-99) `pub mod no_repayment_letter_action;` ergaenzen,
       - und `pub use no_repayment_letter_action::{NoRepaymentLetterAction, NoRepaymentLetterActionProps};`.
    3. i18n-Keys hinzufuegen (4 neue):
       - genossi-frontend/src/i18n/mod.rs (nach MailFailedNoRepaymentLetter, das in Task 2 hinzugefuegt wurde):
         ```
         MailGenerateLetterAndRetry,
         MailGenerateLetterAndRetryRunning,
         MailGenerateLetterAndRetrySuccess,
         MailGenerateLetterAndRetryNoEntry,
         ```
       - genossi-frontend/src/i18n/de.rs:
         ```
         Key::MailGenerateLetterAndRetry => "Brief generieren + Retry".into(),
         Key::MailGenerateLetterAndRetryRunning => "Generiere Brief...".into(),
         Key::MailGenerateLetterAndRetrySuccess => "Brief generiert, Retry laeuft".into(),
         Key::MailGenerateLetterAndRetryNoEntry => "Kein Eintrag fuer dieses Mitglied in der Phase".into(),
         ```
       - genossi-frontend/src/i18n/en.rs:
         ```
         Key::MailGenerateLetterAndRetry => "Generate letter + retry".into(),
         Key::MailGenerateLetterAndRetryRunning => "Generating letter...".into(),
         Key::MailGenerateLetterAndRetrySuccess => "Letter generated, retry triggered".into(),
         Key::MailGenerateLetterAndRetryNoEntry => "No entry for this member in the phase".into(),
         ```
       - Alle 4 Keys MUESSEN in BEIDEN Locales existieren. Verify-Block enforced das per grep.
    4. Wiring in mail_page.rs — Toast-Container in beiden Komponenten verfuegbar machen:
       - `MailPage` (Zeile ~40): zwei neue Signals `let mut toast_messages = use_signal(|| Vec::<(usize, String)>::new()); let mut toast_counter = use_signal(|| 0usize);`. Am Ende des aeusseren `rsx!` (vor dem letzten schliessenden `}`) `ToastContainer { messages: toast_messages.read().clone(), on_dismiss: move |id: usize| { let mut msgs = toast_messages.write(); msgs.retain(|(mid, _)| *mid != id); } }` einfuegen (Pattern aus repayment_phase_details.rs:209).
       - `MailJobDetail` (Zeile ~800): gleiche zwei Signals + ToastContainer.
       - Imports erweitern auf `use crate::component::{ErrorAlert, MailRecipientStatusBadge, NoRepaymentLetterAction, ToastContainer, TopBar, show_toast, is_no_repayment_letter_failure};` (Zeile 11).
    5. Action-Spalte in beiden Tabellen ergaenzen:
       - In der Expanded-Row-Tabelle in MailPage (~Zeile 745, thead-Block): neue `th { class: "py-1 px-2", "" }` als 4. Spalte hinzufuegen.
       - In der MailJobDetail-Tabelle (~Zeile 858, thead-Block): selbe vierte th-Spalte.
       - In beiden tbody-Loops (nach der Error-Spalte) eine vierte td-Spalte einfuegen:
         ```
         td { class: "py-1 px-2",
             if is_no_repayment_letter_failure(&r.status, r.error.as_deref()) {
                 {
                     // member_id aus r.member_id (Option<String>) und phase_id aus job.repayment_phase_id (Option<String>) parsen
                     let mid = r.member_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                     let pid = job_repayment_phase_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
                     // job_repayment_phase_id ist je nach Stelle entweder `job.repayment_phase_id.clone()` (Expanded-Row) oder `d.job.repayment_phase_id.clone()` (MailJobDetail)
                     match (mid, pid) {
                         (Some(mid), Some(pid)) => rsx! {
                             NoRepaymentLetterAction {
                                 job_id: job.id.clone(),
                                 recipient_id: r.id.clone(),
                                 member_id: mid,
                                 phase_id: pid,
                                 on_done: move |_| {
                                     show_toast(&mut toast_messages, &mut toast_counter, i18n.t(Key::MailGenerateLetterAndRetrySuccess).to_string());
                                     // Optional: reload_jobs(); reload_job_detail(); — siehe Implementierungs-Notiz unten
                                 },
                                 on_error: move |msg: String| {
                                     show_toast(&mut toast_messages, &mut toast_counter, msg);
                                 },
                             }
                         },
                         _ => rsx! { "" }
                     }
                 }
             }
         }
         ```
       - Implementierungs-Notiz fuer den Executor: An der Expanded-Row-Stelle existiert bereits ein `reload_jobs`-Closure in der Naehe (siehe mail_page.rs:721 `Ok(_) => reload_jobs(),`). Dieser MUSS im on_done-Callback ebenfalls aufgerufen werden, damit die Tabelle nach Retry frische Counts zeigt. An der MailJobDetail-Stelle wird stattdessen `detail.set(None); spawn(reload_detail-Block kopieren aus use_effect)` oder einfach kein Reload getriggert — der Vorstand klickt dann "Back" und wieder rein. Pragmatic: in MailJobDetail nur den Toast zeigen und die Detail-Daten manuell re-fetchen via `let id_clone = d.job.id.clone(); spawn(async move { let _ = api::get_mail_job_detail(&CONFIG.read().clone(), &id_clone).await.map(|new_d| detail.set(Some(new_d))); });`.
    6. Bulk-Action ist EXPLIZIT OUT OF SCOPE: Wir erweitern mail_page.rs NICHT um einen "Alle no_repayment_letter-Briefe generieren + Retry"-Button. Wird als Follow-up-Todo im SUMMARY dokumentiert.
  </action>
  <verify>
    <automated>cargo test -p genossi-frontend --lib no_repayment_letter_action 2>&amp;1 | tail -30 ; cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown 2>&amp;1 | tail -10 ; for key in MailGenerateLetterAndRetry MailGenerateLetterAndRetryRunning MailGenerateLetterAndRetrySuccess MailGenerateLetterAndRetryNoEntry ; do test "$(grep -c "$key" genossi-frontend/src/i18n/de.rs)" -ge 1 || echo "FAIL: $key missing in de.rs"; test "$(grep -c "$key" genossi-frontend/src/i18n/en.rs)" -ge 1 || echo "FAIL: $key missing in en.rs"; done ; test "$(grep -v '^//' genossi-frontend/src/page/mail_page.rs | grep -c 'NoRepaymentLetterAction')" -ge 2 || echo "FAIL: NoRepaymentLetterAction must appear at least twice (expanded row + detail page)"</automated>
  </verify>
  <done>
    - genossi-frontend/src/component/no_repayment_letter_action.rs existiert mit der NoRepaymentLetterAction-Component, pure helpers find_entry_for_member und button_label_for_state, ButtonState-Enum, und 4 Unit-Tests.
    - Component-Registrierung in component/mod.rs existiert (mod + pub use).
    - Alle 4 neuen i18n-Keys existieren in mod.rs UND sind in de.rs UND en.rs uebersetzt.
    - mail_page.rs:MailPage UND mail_page.rs:MailJobDetail rendern beide den NoRepaymentLetterAction-Button als 4. Tabellen-Spalte fuer no_repayment_letter-Empfaenger (grep findet die Component mindestens 2x).
    - ToastContainer ist in beiden Pages eingebunden; on_done feuert Success-Toast, on_error feuert Error-Toast.
    - cargo test -p genossi-frontend --lib no_repayment_letter_action: 4 Tests passed, 0 failed.
    - cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown clean.
  </done>
</task>

</tasks>

<verification>
End-of-plan checks (nach Task 3):
1. cargo test -p genossi_mail (mindestens 149 passed nach Task 1).
2. cargo test -p genossi-frontend --lib (alle bestehenden + 13 neue Tests aus Task 2 und 3 passen).
3. cargo check --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown ist clean.
4. cargo clippy --manifest-path genossi-frontend/Cargo.toml --target wasm32-unknown-unknown --all-targets 2>&1 | tail -20: 0 warnings.
5. grep-Inventur:
   - `grep -c 'MailRecipientStatusBadge' genossi-frontend/src/page/mail_page.rs` >= 2
   - `grep -c 'NoRepaymentLetterAction' genossi-frontend/src/page/mail_page.rs` >= 2
   - Alle 5 neuen i18n-Keys (MailFailedNoRepaymentLetter, MailGenerateLetterAndRetry, ...Running, ...Success, ...NoEntry) je 1x in de.rs UND en.rs.
6. Manueller Smoke-Test (vom Executor zu dokumentieren im SUMMARY):
   - `cargo run --bin genossi` starten, gegen Swagger einen Bulk-Mail-Job mit attach_repayment_letter=true und einem Member ohne RepaymentLetter ausloesen, Frontend-Mail-Page laden, Job aufklappen, erwarten: amber Badge "Kein Anschreiben generiert" + Button "Brief generieren + Retry"; Klick triggert die Aktion, Loading-State erscheint, danach Toast "Brief generiert, Retry laeuft".
</verification>

<success_criteria>
- Failed-Empfaenger mit `error.starts_with("no_repayment_letter")` werden mit amber Badge und i18n-Label "Kein Anschreiben generiert" / "No repayment letter generated" angezeigt (NICHT mehr generic rot/failed).
- Action-Button "Brief generieren + Retry" steht in beiden Tabellen (Expanded-Row der MailPage UND Deeplink MailJobDetail-Page) zur Verfuegung, immer wenn der Recipient ein no_repayment_letter-Failure ist UND member_id UND job.repayment_phase_id vorhanden sind.
- Klick auf den Button laeuft den 3-Schritt-Flow (list_repayment_entries -> generate_repayment_letters -> retry_mail_job), zeigt Loading-State, bei Erfolg Success-Toast, bei Fehler Error-Toast. Blob-URL aus generate_repayment_letters wird sauber per revoke_object_url freigegeben.
- i18n: 5 neue Keys (1 Badge + 4 Action) existieren in mod.rs und sind in de.rs UND en.rs vollstaendig uebersetzt. Kein Locale-Drift.
- Component-First eingehalten: 2 neue reusable Components in src/component/. Beide werden von BEIDEN Job-Detail-Stellen genutzt (kein copy-paste, einheitliche Logik).
- cargo check + cargo clippy: clean (0 errors, 0 warnings).
- cargo test -p genossi_mail und cargo test -p genossi-frontend --lib: alle gruen, neue Tests (13 insgesamt) passen.
- Backward-compat: Bestehende E2E-Tests (cargo test --test e2e_tests) bleiben gruen, da der zusaetzliche repayment_phase_id-Field-Schluessel additiv und mit skip_serializing_if versehen ist.
</success_criteria>

<output>
After Task 3 completion:
1. Create `.planning/quick/260603-evf-ui-anzeige-no-repayment-letter-status-pr/260603-evf-SUMMARY.md` (use the template at .claude/get-shit-done/templates/summary.md).
2. Document explicitly in the SUMMARY:
   - The minimal backend-touch decision (MailJobTO.repayment_phase_id exposure) with the architectural rationale from this plan's `## Architektur-Entscheidung: minimaler Backend-Touch` section.
   - The Bulk-Action scope-cut: append a new follow-up-todo at `.planning/todos/pending/frontend-bulk-no-repayment-letter-action.md` and reference it from the SUMMARY (title: "Bulk-Action: alle no_repayment_letter-Briefe in einem Job auf einmal generieren + retry").
3. Update `.planning/STATE.md` last activity line to:
   `Last activity: 2026-06-03 — Completed quick task 260603-evf: UI-Anzeige no_repayment_letter-Status pro Empfaenger im Bulk-Mail-Job-Detail (Badge + Action-Button + Toast-Feedback, 13 neue Tests)`.
4. NO ROADMAP.md update (per constraints).
5. Commit via gsd-sdk query commit with message `feat(quick-260603-evf): UI-Marker und Aktion fuer no_repayment_letter-Failed-Empfaenger`.
</output>
