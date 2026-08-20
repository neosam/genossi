---
phase: 32-frontend-compose-dialog
verified: 2026-08-21T00:00:00Z
status: human_needed
score: 5/7 must-haves verified
behavior_unverified: 2
overrides_applied: 0
behavior_unverified_items:
  - truth: "Debounced Live-Vorschau (D-04): schedule_preview/bump_and_preview verwerfen abgelaufene Preview-Läufe über einen Generation-Zähler, bevor preview_application_mail den aufgelösten Body liefert."
    test: "Auf der Compose-Seite (/applications/:id/compose) schnell hintereinander im Betreff/Editor tippen (mehrfach vor Ablauf der 400ms-Debounce), dann beobachten."
    expected: "Nur die Vorschau der zuletzt eingegebenen Version erscheint; keine flackernde/veraltete Zwischenvorschau; während des Wartens bleibt die zuletzt aufgelöste Vorschau sichtbar (kein leerer Zustand)."
    why_human: "schedule_preview/bump_and_preview laufen über gloo_timers::future::TimeoutFuture + spawn — WASM-only async Logik, nicht im Host-Testrunner ausführbar; kein Unit-Test für die Generation-Zähler-Verwerfung vorhanden."
  - truth: "Senden-Button ist während des laufenden Send-Requests deaktiviert (kein Doppelversand, D-05) — 'sending'-Signal togglet um den async api::send_application_mail-Aufruf."
    test: "Auf Compose-Seite senden klicken und (bei langsamer Verbindung/Netzwerk-Drossel) beobachten, ob der Button während des Requests disabled bleibt und Label zu 'Wird gesendet…' wechselt; Doppelklick versuchen."
    expected: "Button ist disabled solange der Request läuft; kein zweiter Sende-Request wird ausgelöst."
    why_human: "sending.set(true)/sending.set(false) um einen async reqwest-Call ist WASM-Laufzeitverhalten; kein Test beweist, dass die UI während des offenen Requests tatsächlich disabled rendert (nur die statische form-onsubmit-Vermeidung ist automatisiert bewiesen, s. no_submit_type_buttons_in_frontend_source)."
human_verification:
  - test: "Compose-Seite öffnen (application_detail → '✉ E-Mail senden' bei vorhandener Adresse) und beobachten, ob Betreff/Body/TemplateSelector sofort mit der Vorlage 'Zahlungserinnerung' vorbefüllt sind und das Dropdown nur Antragsteller-Vorlagen zeigt."
    expected: "Seite öffnet mit Zahlungserinnerung-Vorlage vorausgewählt und befüllt; TemplateSelector-Dropdown enthält keine Mitglieder-Vorlagen."
    why_human: "Mount-Effekt (use_effect/spawn: list_mail_templates → filter → find default) ist WASM-async und nicht host-testbar; keine Unit-Abdeckung für den konkreten Vorbefüll-Ablauf inkl. Fallback-Zweig."
  - test: "Debounced Live-Vorschau beim Tippen im Betreff/Editor beobachten."
    expected: "Vorschau aktualisiert sich mit Verzögerung (~400ms) und zeigt aufgelöste Platzhalter; während des Wartens bleibt die letzte Vorschau sichtbar, kein Flackern."
    why_human: "s. behavior_unverified_items (Generation-Zähler-Debounce, WASM-only)."
  - test: "Senden-Button während eines laufenden Sende-Requests beobachten (disabled + Label 'Wird gesendet…')."
    expected: "Button ist deaktiviert während der Request läuft, verhindert Doppelversand."
    why_human: "s. behavior_unverified_items (async sending-Signal, WASM-only)."
  - test: "Nach erfolgreichem Versand beobachten: Toast + Rücksprung zur Antragsliste."
    expected: "Erfolgs-Toast ('E-Mail-Auftrag erstellt') erscheint, danach Navigation zurück zu Route::ApplicationsPage."
    why_human: "Post-Erfolg-Navigation + Toast-Rendering ist reines WASM-Laufzeitverhalten (show_toast + nav.push), nicht per Build/Unit-Test sichtbar."
  - test: "Auf application_detail: Button 'E-Mail senden' bei einem Antrag OHNE E-Mail-Adresse beobachten (disabled + Hinweistext) und bei einem Antrag MIT Adresse den Klick ausführen."
    expected: "Ohne Adresse: Button disabled, Hinweis 'Keine E-Mail-Adresse hinterlegt' (o.ä.) sichtbar. Mit Adresse: Klick navigiert zu /applications/{id}/compose."
    why_human: "Visuelle Bestätigung von disabled-Rendering + Navigation ist reines Browser-Verhalten; die zugrundeliegende Logik (is_email_empty) ist zwar unit-getestet, das End-to-End-Rendering/Navigieren aber nicht."
  - test: "Auf application_detail einen Timeline-Eintrag anklicken und das Inline-Body-Panel öffnen; zusätzlich einen Eintrag mit sehr langem/HTML-lastigem Body prüfen."
    expected: "Panel zeigt den echten gespeicherten Body (rendered_body/rendered_html_body); bei sehr langem Inhalt bleibt der Text innerhalb des max-h-96 overflow-auto-Containers ohne die Seite zu sprengen."
    why_human: "Long-text-Backstop (UI-SPEC) ist ein rein visueller Held-out-Check, nur interaktiv im Browser prüfbar."
---

# Phase 32: Frontend Compose-Dialog Verification Report

**Phase Goal:** Der Vorstand kann auf der Application-Detailseite eine Erinnerung komponieren, in Live-Vorschau mit aufgelösten Platzhaltern prüfen, bewusst bestätigen und absenden — mit sichtbarer Kommunikations-Historie, prominenter „zuletzt gesendet"-Anzeige und sauberem No-Email-Handling.
**Verified:** 2026-08-21
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | **(SC1, APMAIL-03/APUI-01)** "E-Mail senden"-Button auf `application_detail.rs` öffnet Compose-Route; bei fehlender `application.email` deaktiviert + annotiert, nie stiller Fehlversuch. | ✓ VERIFIED | `application_detail.rs:49` `let email_empty = is_email_empty(...)`; `disabled: email_empty`, `title` = Key::NoEmailAddressHint, italic-Hinweis daneben (Zeilen 196-213). Klick navigiert nur `if !email_empty` zu `Route::ApplicationCompose { id }` (Zeile 207). `is_email_empty` geteilt aus `util/email.rs`, 5 Unit-Tests grün (`nix develop --command cargo test` in `genossi-frontend/`). Route `/applications/:id/compose` registriert VOR der generischen `/applications`-Route in `router.rs:60-64`. |
| 2 | **(SC2, APUI-02)** Compose-Seite komponiert bestehende `mail_compose/*`-Bausteine (kein geforktes UI); API-Aufrufe sind dedizierte `api.rs`-Funktionen, nicht Member-umgeleitet. | ✓ VERIFIED | `application_compose.rs:28-31` importiert `MailPreviewFrame`, `plain_to_html`, `MailSubjectInput`, `TemplateSelector`, `WysiwygEditor` aus `component::mail_compose::*` (kein Fork). `api.rs:1787-1838` enthält `send_application_mail`/`preview_application_mail`/`get_application_communications` mit URLs `/api/applications/{id}/mail`, `/api/applications/{id}/mail/preview`, `/api/applications/{id}/communications` — keine `/api/mail/*`-Umleitung; lokale `SendApplicationMailRequest`/`PreviewApplicationMailRequest`-Structs (Landmine 1 geschlossen). |
| 3 | **(SC3, APMAIL-04)** Vor dem Absenden sieht der Vorstand eine Live-Vorschau mit aufgelösten Platzhaltern über den Backend-Render-Kernel (debounced); die Vorschau ist die Bestätigung (confirm-before-send). | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Code vorhanden + verdrahtet: `schedule_preview`/`bump_and_preview` (`application_compose.rs:57-131`) rufen `api::preview_application_mail` über einen Generation-Zähler-Debounce (400ms); `MailPreviewFrame` rendert das Ergebnis (Zeilen 320-339). **Aber:** kein Unit-/Integrationstest deckt die Generation-Zähler-Verwerfung ab (kein `#[cfg(test)]`-Modul in `application_compose.rs`); dies ist eine Cancellation/Ordering-Invariante, die nur WASM-Laufzeitverhalten prüfen kann. → Human-Verification-Item. |
| 4 | **(SC4, APUI-03)** Kommunikations-Historie via unveränderter, prop-getriebener `communication_timeline.rs`; prominente „zuletzt gesendet"-Anzeige (Betreff+Status+Datum). | ✓ VERIFIED | `communication_timeline.rs` additiv um `#[props(default)] on_entry_click` erweitert (Zeilen 14-15); ohne Handler bleibt exakt der bestehende `Link`-Pfad (Zeilen 117-123) — `member_details.rs:1420-1422` nutzt die Komponente weiterhin ohne `on_entry_click` (unverändert, Build grün). `last_outbound_summary` (api.rs:1859-1869, unit-getestet) speist die „zuletzt gesendet"-Zeile in `application_detail.rs:218-237` — exaktes UI-SPEC-Format `"{subject} — {status_label} am {date_str}"`. **Kleine Abweichung (nicht blockierend):** die Compose-Seite (`application_compose.rs:349-361`) zeigt dieselbe Zeile im Format `"{subj} — {status_part} ({date})"` — unübersetzter Roh-Status statt `outbound_status_label` und ohne `i18n.format_datetime`; UI-SPEC-Copywriting-Contract ("… am {Datum}") ist dort nicht exakt eingehalten (siehe Anti-Patterns). |
| 5a | **(SC5, D-05)** Kein `form onsubmit` (Dioxus-Reload-Falle); Senden-Trigger ist `button`+`onclick`+`r#type:"button"`. | ✓ VERIFIED | `application_compose.rs:367-369` `button { r#type: "button", onclick: ... }`, kein `form`-Element. Projektweiter Regressionstest `no_submit_type_buttons_in_frontend_source` (`tests/no_form_submit_regression.rs`) läuft grün — scannt den kompletten `src/`-Baum nach `r#type: "submit"` und schlägt bei Wiedereinführung fehl. Behaviorale Bestätigung, nicht nur Presence. |
| 5b | **(SC5, D-05)** Senden-Button ist während des laufenden Requests deaktiviert (kein Doppelversand). | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Code vorhanden: `disabled: *sending.read() \|\| subject.read().is_empty()` (`application_compose.rs:370`), `sending.set(true)`/`sending.set(false)` um den async `send_application_mail`-Call (Zeilen 382-415). Kein Test beweist, dass die UI während des offenen Requests tatsächlich `disabled` rendert (Zeitfenster-Verhalten, WASM-only). → Human-Verification-Item. |
| 6 | **(D-06)** Klick auf Timeline-Eintrag zeigt den echten gespeicherten Body (`rendered_body`/`rendered_html_body`), kein Re-Render. | ✓ VERIFIED | Backend-Kette vollständig verifiziert: `genossi_mail/src/dao.rs` (`CommunicationEntry.rendered_body/rendered_html_body`), `dao_sqlite.rs` (`CommunicationEntryDb` + `TryFrom` + beide SELECT-Zweige, Zeilen 1013-1170) — DAO-Tests `test_application_communications_exposes_rendered_body` (Some-Pfad) und `test_application_communications_rendered_body_none_for_legacy_row` (None-Pfad) laufen grün (einzeln nachgeführt: `cargo test -p genossi_mail test_application_communications` → 7/7 passed). Backend-TO (`communication_rest.rs:50-87`) und Frontend-TO (`rest-types/src/lib.rs:901-924`) mappen additiv, Frontend-Serde-Tests grün. UI-Wiring: `application_detail.rs:240-273` setzt `selected_entry` per `on_entry_click`, rendert `entry.rendered_body` (Fallback `rendered_html_body`) HTML-escaped in `max-h-96 overflow-auto whitespace-pre-wrap` — deterministische Signal-Set/Read-Bindung, keine Cancellation-/Ordering-Logik. Die Kern-Behauptung ("echter gespeicherter Body, kein Re-Render") ist damit serverseitig UND clientseitig behavioral bewiesen; das visuelle Long-text/HTML-Overflow-Verhalten bleibt separat als Human-Item (rein visueller Held-out-Check). |

**Score:** 5/7 truths verified (2 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `genossi_mail/src/dao.rs` | `CommunicationEntry.rendered_body/rendered_html_body` | ✓ VERIFIED | Zeilen 75, 79 (MailRecipient) + 296-297 (CommunicationEntry) |
| `genossi_mail/src/dao_sqlite.rs` | `CommunicationEntryDb` + TryFrom + beide SELECTs erweitert | ✓ VERIFIED | Zeilen 1013-1170; beide UNION-Zweige (member NULL, application r.rendered_*) konsistent |
| `genossi_mail/src/communication_rest.rs` | `CommunicationEntryTO` + From mappt rendered_* | ✓ VERIFIED | Zeilen 29-87 |
| `genossi-frontend/rest-types/src/lib.rs` | `CommunicationEntryTO.rendered_body/rendered_html_body` | ✓ VERIFIED | Zeilen 901-924 + 2 Serde-Tests |
| `genossi-frontend/src/api.rs` | `send_application_mail`/`preview_application_mail`/`get_application_communications` + lokale Structs + `filter_templates_by_type`/`last_outbound_summary` | ✓ VERIFIED | Zeilen 1757-1869 |
| `genossi-frontend/src/i18n/{mod,de,en}.rs` | `Key::LastSentSummary`/`NeverSent`/`SentMailBody` in beiden Locales | ✓ VERIFIED | mod.rs:400-402, de.rs:325-327, en.rs:323-325 |
| `genossi-frontend/src/page/application_compose.rs` (NEU) | `ApplicationCompose`-Page | ✓ VERIFIED | Existiert, 432 Zeilen, `#[component] pub fn ApplicationCompose(id: String)` |
| `genossi-frontend/src/router.rs` | `Route::ApplicationCompose { id: String }` | ✓ VERIFIED | Zeile 62-63, vor generischer `/applications`-Route (Zeile 64) |
| `genossi-frontend/src/component/mail_compose/template_selector.rs` | `filter_type` + `initial_template_id` additiv | ✓ VERIFIED | Zeilen 24, 29 als `#[props(default)]`; MailPage-Nutzung unverändert (kein `filter_type`/`initial_template_id` dort) |
| `genossi-frontend/src/util/email.rs` (NEU) | geteiltes `is_email_empty` + Tests | ✓ VERIFIED | Existiert, 5 Tests, von `application_detail.rs` und `member_details.rs` importiert |
| `genossi-frontend/src/component/communication_timeline.rs` | additiver `on_entry_click`-Prop | ✓ VERIFIED | Zeilen 14-15, 105-116; Fallback-Pfad (`Link`) unverändert |
| `genossi-frontend/src/component/application_detail.rs` | Button + last-sent + Timeline-Abschnitt + Body-Detail-Panel | ✓ VERIFIED | Zeilen 190-274 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `mail_recipients.rendered_body/rendered_html_body` | `CommunicationEntryTO` (Backend + Frontend) | SQL SELECT → CommunicationEntryDb → CommunicationEntry → TO | ✓ WIRED | DAO-Tests (Some/None) + Serde-Tests grün |
| `application_detail.rs` Button | `Route::ApplicationCompose { id }` | `nav.push(...)` bei `!email_empty` | ✓ WIRED | Code gelesen, Route registriert, kompiliert |
| `TemplateSelector.filter_type` | `filter_templates_by_type` | `api::filter_templates_by_type(&templates.read(), t)` | ✓ WIRED | `template_selector.rs:48-51` |
| `ApplicationCompose` | `preview_application_mail`/`send_application_mail`/`get_application_communications` | direkte async-Aufrufe | ✓ WIRED | Zeilen 82, 173, 393 in `application_compose.rs` |
| Erfolg (Send) | `show_toast(Key::MailJobCreated)` + `nav.push(Route::ApplicationsPage)` | onclick-Handler `Ok(())`-Zweig | ✓ WIRED | Code gelesen (Zeilen 403-410); Laufzeit-Effekt selbst Human-Item |
| `CommunicationTimeline.on_entry_click` | Body-Panel (`selected_entry`) | `selected_entry.set(Some(entry))` | ✓ WIRED | `application_detail.rs:242-244, 250-273` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| D-06 DAO liefert rendered_body (Some) | `cargo test -p genossi_mail test_application_communications` | 7 passed (inkl. exposes_rendered_body + none_for_legacy_row) | ✓ PASS |
| Frontend rest-types Serde Some/None | enumeriert via `cargo test -- --list` | Tests vorhanden (`communication_entry_deserializes_rendered_body_fields`, `communication_entry_missing_rendered_body_is_none`) | ✓ PASS (via vollem Frontend-Testlauf, s.u.) |
| `filter_templates_by_type` / `last_outbound_summary` | `cargo test test_last_outbound_summary`, `cargo test test_filter_templates_by_type` (in `genossi-frontend/`) | je 3 passed | ✓ PASS |
| `is_email_empty` (geteilt) | `cargo test is_email_empty` (in `genossi-frontend/`) | 5 passed | ✓ PASS |
| Kein `form onsubmit`/`r#type:"submit"` im gesamten Frontend | `cargo test no_submit_type_buttons_in_frontend_source` | 1 passed | ✓ PASS |
| Backend-Workspace vollständig | `cargo test --workspace` (einmalig, gesamter Workspace) | 0 failed über alle Crates (u.a. 326 e2e, 311 genossi_mail) | ✓ PASS |
| Frontend-Crate vollständig | `cargo test` in `genossi-frontend/` (einmalig) | 339 + 1 passed, 0 failed | ✓ PASS |
| Frontend-Build (Page/Route/Props/Felder) | `cargo build` in `genossi-frontend/` | kompiliert (nur unabhängige Warnings) | ✓ PASS |
| Backend-Build (Handler-Passthrough) | `cargo build -p genossi_mail -p genossi_rest` | kompiliert | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|-----------------|--------------|--------|----------|
| APMAIL-03 | 32-04 | Fehlende E-Mail-Adresse sauber behandelt, Button deaktiviert/annotiert | ✓ SATISFIED | `application_detail.rs:196-213`, `is_email_empty` getestet |
| APMAIL-04 | 32-02, 32-03 | Live-Vorschau + confirm-before-send | ✓ SATISFIED (mit PRESENT_BEHAVIOR_UNVERIFIED für Debounce-Laufzeitverhalten) | `preview_application_mail` verdrahtet; Cancellation-Invariante nur visuell prüfbar |
| APUI-01 | 32-03, 32-04 | Dedizierte Compose-Route (kein Modal-in-Modal), Button-Navigation | ✓ SATISFIED | Route registriert, `RequirePrivilege(PRIVILEGE_ADMIN)`, Button navigiert |
| APUI-02 | 32-02, 32-03 | Component-First (mail_compose/*) + dedizierte api.rs-Funktionen | ✓ SATISFIED | Keine Member-Umleitung, kein UI-Fork |
| APUI-03 | 32-01, 32-04 | Unveränderte, prop-getriebene CommunicationTimeline + echter gespeicherter Body | ✓ SATISFIED | Additive Props, Member-Pfad unverändert, D-06-Kette bewiesen |

Keine orphaned Requirements — alle 5 IDs aus REQUIREMENTS.md sind in mindestens einem Plan deklariert und hier nachgewiesen.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `genossi-frontend/src/page/application_compose.rs` | 349-361 | „zuletzt gesendet"-Zeile weicht vom UI-SPEC-Copywriting-Contract ab: Roh-Status statt `outbound_status_label`-Übersetzung, Datum ohne `i18n.format_datetime`, Format `"({date})"` statt `"am {date}"` | ℹ️ Info (nicht blockierend) | Funktional korrekt (Anti-Doppelversand-Guard ist sichtbar und wirksam), aber inkonsistent mit der identischen Zeile in `application_detail.rs` und dem UI-SPEC-Wortlaut "Zuletzt gesendet: {Betreff} — {Status} am {Datum}". Empfehlung: `outbound_status_label` + `i18n.format_datetime` auch auf der Compose-Seite verwenden (kleiner Fast-Follow, kein Gap). |

Keine TBD/FIXME/XXX/TODO/HACK/PLACEHOLDER-Marker in den 17 geprüften geänderten Dateien. Keine leeren Stub-Implementierungen, keine hartcodierten leeren Rückgaben in Render-Pfaden gefunden.

### Human Verification Required

Siehe YAML-Frontmatter `human_verification` (6 Punkte) — zusammengefasst:

1. **Compose-Seite öffnet vorbefüllt** (Zahlungserinnerung-Vorlage, Antragsteller-gefilterter TemplateSelector).
2. **Debounced Live-Vorschau** aktualisiert sich verzögert, zeigt aufgelöste Platzhalter, kein Flackern.
3. **Senden-Button disabled während Request** (kein Doppelversand sichtbar im Browser).
4. **Erfolg → Toast + Rücksprung** zur Antragsliste.
5. **application_detail-Button**: disabled+Hinweis ohne Adresse; Navigation bei vorhandener Adresse.
6. **Body-Detail-Panel**: echter gespeicherter Body sichtbar; Long-text/HTML-Backstop (kein Seiten-Überlauf).

Diese sechs Punkte sind laufzeitgebundenes WASM-Verhalten (Debounce/Generation-Zähler, async `sending`-Signal, Browser-Navigation, visuelles Overflow-Verhalten) und wurden von den Executoren selbst bereits als `human_judgment: true` in den SUMMARY-Coverage-Abschnitten (32-03 T2; 32-04 D3/D5) markiert — konsistent mit der geplanten `dx serve`-Vorstands-Smoke-Session vor Milestone-Merge. Kein Punkt deutet auf fehlenden oder kaputten Code hin; alle zugrundeliegenden Code-Pfade sind vorhanden, verdrahtet und (soweit host-testbar) grün getestet.

### Gaps Summary

Keine BLOCKER gefunden. Alle Artefakte existieren, sind substantiell (kein Stub), verdrahtet und — soweit mit `cargo test`/`cargo build` prüfbar — grün. Der komplette Backend-Workspace-Testlauf (0 fehlgeschlagen über alle Crates, inkl. 326 E2E) und der komplette Frontend-Crate-Testlauf (339+1 grün) wurden in dieser Verifikation selbst reproduziert, nicht nur aus SUMMARY.md übernommen. Die D-06-Backend-Kette (`mail_recipients` → `CommunicationEntry` → beide `CommunicationEntryTO`) ist Ende-zu-Ende mit Some/None-Testfällen bewiesen.

Der Status ist `human_needed`, nicht `passed`, weil zwei Wahrheiten (Live-Vorschau-Debounce mit Generation-Zähler-Cancellation, disabled-während-Send) echte Laufzeit-/Cancellation-Invarianten sind, die kein Unit-/Build-Check beweisen kann (WASM-only, kein Headless-Browser-Test im Projekt-Stack) — plus vier weitere rein visuelle/interaktive Bestätigungen (Prefill, Toast+Navigation, Button-Disabled-Sichtbarkeit, Body-Panel-Overflow). Dies ist keine Abweichung vom Plan: die Executoren haben diese Punkte selbst bereits als `human_judgment: true` klassifiziert und auf die geplante `dx serve`-Vorstands-Smoke-Session vor Milestone-Merge verschoben.

**Einzige Info-Findung** (nicht blockierend): die "zuletzt gesendet"-Zeile auf der Compose-Seite weicht im Copywriting leicht vom UI-SPEC-Contract ab (Roh-Status/Datum statt übersetzt/formatiert). Empfehlung: kleiner Fast-Follow, kein Gap-Plan nötig.

---

*Verified: 2026-08-21*
*Verifier: Claude (gsd-verifier)*
