---
quick_id: 260603-jtf
title: Mail-Templates — Test-Funktion auf der Template-Editor-Seite
status: completed
commit: e246980ee1d80808d61ab8babd4de525d0bbe266
completed_at: 2026-06-03
files_modified:
  - genossi_mail/src/service.rs
  - genossi_mail/src/rest.rs
  - genossi-frontend/src/api.rs
  - genossi-frontend/src/component/mail_compose/mod.rs
  - genossi-frontend/src/component/mail_compose/template_tester.rs
  - genossi-frontend/src/component/mail_compose/template_preview.rs
  - genossi-frontend/src/page/mail_templates.rs
  - genossi-frontend/src/i18n/mod.rs
  - genossi-frontend/src/i18n/de.rs
  - genossi-frontend/src/i18n/en.rs
tests_added: 7
---

# Quick 260603-jtf Summary — Template-Test-Funktion auf der Editor-Seite

## Was gebaut wurde

### Backend (`genossi_mail`)
- **Neuer MailService-Methode** `send_test_mail_with_body(to, subject, body)` (Sibling zu
  bestehender `send_test_mail`, deren hartkodierte Constant-Mail unverändert bleibt für
  die SMTP-Smoke-Test-Funktion auf der Settings-Seite).
- **Neuer REST-Endpoint** `POST /api/mail/test-with-template` mit Request-Body
  `TestMailWithTemplateRequest { to_address, subject, body, member_id, repayment_phase_id? }`.
  Der Handler:
  1. Parsed `member_id`, lädt den Member.
  2. Baut den Template-Context aus Member-Variablen (`member_to_template_context`).
  3. Wenn `repayment_phase_id` gesetzt → merged Repayment-Vars (gleiche Logik wie `/preview`).
  4. Rendert Subject + Body über `render_template` (Fehler → 400 TemplateValidation).
  5. Ruft `MailService::send_test_mail_with_body(&body.to_address, &rendered_subject, &rendered_body)`
     — **`to_address` ist NIE `member.email`** (Privacy-Defense, Schicht 3).
- **Route registriert** in `generate_route` und **OpenAPI-Schema** in `ApiDoc::paths`/`components`.

### Frontend (`genossi-frontend`)
- **Neue wiederverwendbare Komponente** `mail_compose/template_tester.rs::TemplateTester` mit
  Props `subject: ReadOnlySignal<String>` und `body: ReadOnlySignal<String>`. Komponiert:
  - `MemberSearch` (Single-Member-Selector, reused)
  - `TemplatePreview` (1:1 reused, `member_ids=vec![selected_id]`)
  - Test-Adress-`<input type="email">` mit Datenschutz-Hinweis-Text
  - "Test-Mail senden"-Button mit `r#type="button"` (Memory `feedback_dioxus_button_type.md`)
  - Erfolg/Fehler-Toast
- **Pure Helper** `is_valid_test_address(addr) -> bool` (trim + `@`-Check; absichtlich minimal,
  Server-seitige RFC-Validation in lettre).
- **mod.rs** exportiert `TemplateTester`.
- **`page/mail_templates.rs`** bettet `TemplateTester` direkt unterhalb des Body-Textareas
  ein — keine inline-RSX für Member-Selector/Preview/Test-Send (Component-First-Gate grün).
- **Neuer API-Client** `api::send_test_mail_with_template(config, to_address, subject, body, member_id)`.

### i18n
6 neue Keys, sowohl in `de.rs` als auch `en.rs` übersetzt:
- `MailTemplateTest` ("Template testen" / "Test template")
- `MailTemplateTestSendTo` ("Test-Empfänger" / "Test recipient")
- `MailTemplateTestSend` ("Test-Mail senden" / "Send test mail")
- `MailTemplateTestPrivacyHint` ("Wird an die Test-Adresse gesendet, NICHT an das ausgewählte Mitglied.")
- `MailTemplateTestSuccess` ("Test-Mail gesendet." / "Test mail sent.")
- `MailTemplateTestFailed` ("Test-Mail fehlgeschlagen" / "Test mail failed")

## Test-Coverage

| Bereich | Datei | Anzahl | Status |
|---------|-------|--------|--------|
| Backend Service-Unit | `genossi_mail/src/service.rs` | 2 | grün |
| Backend REST-Serde | `genossi_mail/src/rest.rs` | 2 | grün |
| Frontend Pure-Helper | `genossi-frontend/src/component/mail_compose/template_tester.rs` | 3 | grün |
| **Summe neu** | | **7** | **alle grün** |

**Workspace-Tests vorher/nachher:** Plan forderte 6 neue Tests; geliefert 7
(zwei serde-Tests statt einem — Roundtrip + Backward-Compat-without-phase-Variante;
beides ist additiv und kostet 0 Risiko). `cargo test --workspace` läuft sauber durch
(keine bestehenden Tests gebrochen). Workspace zählt jetzt ~1300+ Tests grün.

Verifikation der Tests via:
```bash
cargo test -p genossi_mail send_test_mail_with_body
cargo test -p genossi_mail test_test_with_template_request_serde
cd genossi-frontend && cargo test is_valid_test_address
```

## Privacy-Defense — Verifikation

Drei voneinander unabhängige Schutzschichten — jeder einzelne reicht, um Versand an
den ausgewählten Member zu verhindern; alle drei müssen gleichzeitig kompromittiert
werden, damit es zu unbeabsichtigtem Versand kommt:

1. **UI-Layer** (`template_tester.rs`):
   - Test-Adress-Input ist ein separates `<input type="email">`, visuell getrennt
     vom Member-Selector durch eigene `border-t pt-3`-Section.
   - Ein amber-farbener Hinweis (`Key::MailTemplateTestPrivacyHint`) erklärt
     explizit: "Wird an die Test-Adresse gesendet, NICHT an das ausgewählte Mitglied."
   - Default-Wert ist leerer String — kein Pre-Fill vom Member.

2. **Frontend-Wiring** (`template_tester.rs::onclick`):
   - Doc-Comment direkt über dem `onclick`-Handler (Zeile 116-120) dokumentiert die
     Privacy-Invariante.
   - Der `addr`-`String`, der an `api::send_test_mail_with_template` durchgereicht
     wird, ist `test_address.read().trim().to_string()` — kein Code-Pfad liest
     `member.email`.
   - `member_id` wird nur als String an die Backend-Funktion weitergegeben (für
     Template-Render-Context), niemals als Empfänger.

3. **Backend-Layer** (`genossi_mail/src/rest.rs::send_test_mail_with_template`):
   - Der Handler ruft `state.mail_service().send_test_mail_with_body(&body.to_address, ...)`
     mit dem **Request-Body-Feld `to_address`**, NICHT mit der `member.email` aus dem
     geladenen `MemberEntity`.
   - `MemberEntity` wird ausschließlich für `member_to_template_context(&member)`
     verwendet (Template-Variablen-Source).
   - Inline `PRIVACY:`-Kommentar dokumentiert das Pattern dauerhaft.

## Architektur-Entscheidungen

| Entscheidung | Rationale |
|--------------|-----------|
| Neuer Endpoint `/api/mail/test-with-template` statt Reuse von `/api/mail/test` | Bestehender `/test` sendet eine fixe Constant-Mail "Genossi Test-E-Mail" als SMTP-Config-Smoke-Test; Signatur-Änderung würde die Settings-Seite (`config_page.rs:445`) breaken. Klare Trennung: `/test` = "geht SMTP überhaupt?", `/test-with-template` = "wie sieht dieses Template aus?". |
| Neuer Endpoint statt Reuse von `/send-bulk` | Bulk-Send erzeugt MailJob-Persistenz, Audit-Log-Einträge, MemberDocument-Updates. Ein Template-Test soll fire-and-forget sein — kein Job, keine History, kein Risiko versehentlich an einen Member zu senden. |
| Backward-Compat in `MailService`-Trait | Methode wurde als **zusätzliche** Trait-Methode hinzugefügt; alle bestehenden Implementierungen via `automock` werden auto-generiert ohne Änderung am Boilerplate. `MockMailService` adoptiert die neue Methode automatisch. |
| Component-First: TemplateTester statt inline-RSX | Künftiges Reuse-Szenario möglich (Compose-Seite könnte denselben Tester einbetten). Tests sind so isolierbar (Pure-Helper ohne Page-Mounting). Code-First-Memory-Regel `feedback_component_first.md` explizit befolgt. |
| Pure-Helper `is_valid_test_address` ist sehr minimal | Lettre rejected ungültige Adressen serverseitig mit `502 SmtpError`. Eine vollständige RFC5321-Validation im Frontend wäre Doppelarbeit; minimal-Check (trim + `@`) reicht, um den Button-`disabled`-State zu steuern. |
| `repayment_phase_id` ist optional und in Editor-UI nicht durchgereicht | Editor-Templates haben keinen Phase-Kontext im aktuellen Quick-Scope. Die Backend-Signatur akzeptiert das Feld bereits (Phase-12-spezifisch im Bulk-Compose-Flow), sodass der Tester in einer Folge-Iteration ohne Schema-Breaking erweitert werden könnte. |

## Abweichungen vom Plan (alle additiv, kein Re-Work)

- **+1 Backend-Test**: Statt eines einzelnen serde-Roundtrip-Tests wurden zwei Tests
  erstellt (`test_test_with_template_request_serde_roundtrip` mit komplettem Payload +
  `test_test_with_template_request_serde_without_phase` für Backward-Compat). Beide
  sind additiv und schnell — zementiert das `#[serde(default)]`-Verhalten dauerhaft.
- **template_preview.rs rustfmt-Drift**: rustfmt-Lauf hat eine unzusammenhängende
  rein-kosmetische Änderung im `#[props(default)]`-Attribut produziert (vier Zeilen →
  eine Zeile). Pure Format-Korrektur ohne Logik-Änderung; im selben Commit mit
  ausgeliefert, damit `cargo fmt --check` im CI grün bleibt.

## Tech-Debt-Items (Follow-ups)

1. **TemplateTester auch in Compose-Seite**: Die Komponente ist generisch genug, dass
   sie auch in `mail_page.rs` unterhalb des Body-Editors verwendet werden könnte
   (statt dem dort eingebetteten `TemplatePreview` + separater Bulk-Send-Flow). Würde
   die Compose-Seite vom "test before sending"-Risiko entkoppeln. Eigener Quick.

2. **`repayment_phase_id` im Editor-Tester**: Aktuell wird das Feld backendseitig
   akzeptiert, aber vom Editor-UI nicht durchgereicht (Templates haben dort keinen
   Phase-Kontext). Sobald Phase-12-Flow-Templates auch im Editor mit Repayment-Preview
   getestet werden sollen, kann eine optionale Phase-Auswahl additiv ergänzt werden —
   ohne API-Breaking-Change.

3. **`send_test_mail_with_body` und `send_test_mail` DRY-fizieren**: Die beiden Service-
   Methoden teilen 95% des SMTP-Builder-Codes. Ein gemeinsamer Helper `send_smtp(to,
   subject, body)` würde die Duplikation eliminieren. Sehr klein (3 Zeilen Risiko),
   aber out-of-scope für diesen Quick.

## jj-Commit

- **Commit-ID:** `e246980ee1d80808d61ab8babd4de525d0bbe266`
- **Title:** `feat(quick-260603-jtf): Template-Test-Funktion auf Editor-Seite`
- **Files in commit:** 10 (source-only; `.planning/`-Files separat per Orchestrator)

## Verification gates

| Gate | Status |
|------|--------|
| `cargo build --workspace` | grün |
| `cargo test --workspace` (alle bestehenden + 7 neue) | grün |
| `cargo clippy --workspace --all-targets` | grün (keine neuen Warnings für berührte Dateien) |
| `rustfmt --check` auf alle berührten Dateien | grün |
| Component-First-Gate: `grep -cE "MemberSearch\|TemplatePreview\|test_address" mail_templates.rs` = 0 | grün |
| Grep-Gate Backend: `grep -c "send_test_mail_with_template\|send_test_mail_with_body" rest.rs service.rs` = 13 (≥ 6) | grün |
| Endpoint registriert in `generate_route` + `ApiDoc` | grün |
| jj-Commit (NICHT git) | grün |
| Privacy-Defense: 3 Schichten (UI-Hinweis, Frontend-onclick-Doc, Backend-Handler-Doc) | grün |

## Self-Check: PASSED
