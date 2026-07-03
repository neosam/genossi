# Phase 23: HTML Mail Backend - Context

**Gathered:** 2026-07-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Eine Mail kann mit Text- UND HTML-Teil als `multipart/alternative` versendet werden (Text zuerst, mit Anhang korrekt als `mixed{ alternative{plain, html}, attachments }` verschachtelt). Der Plain-Text-Teil bleibt der bestehende, vom Autor verfasste `body` (keine Ableitung aus HTML). Der optionale HTML-Body (`body_html`) wird auf Templates + Jobs gespeichert (forward-only `ADD COLUMN … NULL`, Legacy-Zeilen = NULL = weiterhin reine Textmails). Template-Variablen werden in Text- UND HTML-Body interpoliert; die HTML-Variante nutzt eine **separate autoescapende** minijinja-Env, sodass mitglieds-/nutzergelieferte Werte HTML-escaped werden, während vom Autor verfasste Markup-Struktur erhalten bleibt. Vom Vorstand verfasstes HTML wird an allen Eintritts-Punkten serverseitig mit `ammonia` saniert.

**In scope:** MIME-`multipart/alternative`-Bau im geteilten `build_message`-Helfer (aus Phase 22); `body_html`-Spalten (Templates + Jobs) + `rendered_html_body`-Spalte (mail_recipients); separate autoescapende HTML-Render-Env; `ammonia`-Sanitization an allen Eintritts-Punkten (`create_job`, template create/update, Test-Mail-Pfad); deutsches Datumsformat (FMT-01) im gemeinsamen Context-Builder; API-Wire von `body_html` (Backend, damit Phase 24 posten kann); Unit-/MIME-Byte-Tests.

**Out of scope (bewusst):** KEIN Frontend / WYSIWYG-Editor (UI hint: no → Phase 24); KEINE eingebetteten Bilder / Briefkopf / Logo / Inline-CSS-Branding (REQUIREMENTS.md „Future Requirements — HTML-Mail-Bilder/Branding", bewusst deferred; der ammonia-Filter *lässt* einen `<img src="https://…">`-Tag durch, aber es wird keine Bild-Upload-/Embedding-**Funktion** gebaut); Text-aus-HTML-Ableitung (HTML-02 verbietet sie); zusätzliche Crates für den Text-Teil.

</domain>

<decisions>
## Implementation Decisions

### Ammonia-Sanitization (HTML-05)
- **D-01:** `ammonia` wird als neue Dependency aufgenommen (heute nicht im Workspace). Verwendet wird der **permissive Default-Filter** (`ammonia::clean()` bzw. `Builder::default()`), NICHT eine enge Custom-Whitelist. Begründung (User-Entscheidung): mehr Formatierungs-Freiheit für den Vorstand (Fett/Kursiv/Links/Listen/Absätze **plus** Überschriften/Tabellen), weniger Custom-Code. Der Default strippt trotzdem zuverlässig alles Gefährliche: `<script>`, Event-Handler (`onclick` etc.), `javascript:`/`data:`-URL-Schemata; `target=_blank`-Links bekommen `rel=noopener` erzwungen.
- **D-02:** ammonia ist ein **Sicherheitsnetz** (Sanitizer), kein Mail-Versender — Versand bleibt `lettre`. Der `<img>`-Tag wird vom Default-Filter durchgelassen (nur handgeschriebenes externes `<img src="https://…">`); echte Bild-/Branding-Funktion bleibt eine spätere Phase (siehe Deferred).
- **D-03:** Sanitization läuft an **allen** Eintritts-Punkten, wo Autor-HTML in die Persistenz gelangt: `create_job` (service.rs:268), Template-Create + Template-Update, sowie der Test-Mail-Pfad (`send_test_mail_with_body`, service.rs:447). Frontend-Sanitization gilt ausdrücklich NICHT als Sicherheitsgrenze (harte Ordering-Constraint: ammonia-Gate MUSS vor/mit Phase 24 landen).

### HTML-Render & Escaping (HTML-04)
- **D-04:** Neue **separate autoescapende** minijinja-Env für den HTML-Body (z. B. `html_env()` mit `set_auto_escape` bzw. HTML-Autoescape aktiviert). Die bestehende `strict_env()` (template.rs:61) bleibt **unverändert** für Text-Body UND Subject. Ein Mitglied namens `<script> & Co` erscheint im HTML-Body als `&lt;script&gt; &amp;`, die Autor-Markup-Struktur bleibt erhalten.
- **D-05:** **Sanitize-on-store + autoescape-on-render**, KEIN Re-Sanitize des gerenderten Outputs. Autor-HTML wird beim Speichern **einmal** durch ammonia gesäubert; Mitgliedswerte werden beim Rendern durch die autoescapende Env neutralisiert. Doppeltes Sanitizen wäre redundant; Legacy-HTML existiert nicht (HTML-03 → NULL = text-only).

### Schema / Persistenz (HTML-01, HTML-03) — 3 forward-only Migrationen
- **D-06:** `ALTER TABLE mail_templates ADD COLUMN body_html TEXT NULL` (forward-only).
- **D-07:** `ALTER TABLE mail_jobs ADD COLUMN body_html TEXT NULL` (forward-only).
- **D-08:** `ALTER TABLE mail_recipients ADD COLUMN rendered_html_body TEXT NULL` (forward-only). **User-Entscheidung:** Der gerenderte HTML-Body wird pro Empfänger persistiert (nicht nur on-the-fly), analog zum bestehenden `rendered_body` (Quick 260614-9zf) — damit byte-genau dokumentiert ist, was jeder Empfänger wirklich bekommen hat („wir müssen aufbewahren, was verschickt wurde"). Der Worker befüllt `rendered_html_body` beim Versand parallel zu `rendered_subject`/`rendered_body`. `body_html`-Feld auf `MailJob`/`MailTemplate`-Structs (dao.rs) + `rendered_html_body` auf `MailRecipient` ergänzen.
- **D-09:** Legacy-Verhalten: `body_html IS NULL` → reine Textmail (kein `alternative`-Teil). `body_html IS NOT NULL` → `multipart/alternative` (Text zuerst, dann HTML) via geteiltem `build_message`-Helfer (Phase 22). Mit Anhang: `mixed{ alternative{plain, html}, attachments }`.

### MIME-Bau (HTML-01, HTML-02)
- **D-10:** Die `multipart/alternative`-Verschachtelung wird im geteilten `build_message(...)`-Helfer aus Phase 22 ergänzt (der bereits das `MultiPart::mixed()`-Attachment-Wrapping besitzt). Der Text-`SinglePart` bleibt Baustein 1 (unverändert, aus dem vom Autor verfassten `body`), der HTML-`SinglePart` kommt als optionaler zweiter `alternative`-Zweig dazu. Kein zusätzliches Crate für den Text-Teil (HTML-02).

### Deutsches Datumsformat (FMT-01)
- **D-11:** Kleiner geteilter Helfer `format_de(date) -> String` in `genossi_mail` mit `time::format_description`-Vorlage `"[day].[month].[year]"` (z. B. `02.07.2026`). Angewandt auf `join_date` und `exit_date` im **gemeinsamen** Context-Builder `member_to_template_context` (template.rs:17-18) → automatisch konsistent in Text- UND HTML-Body (beide nutzen denselben Context). Ersetzt das heutige `.to_string()`. Unit-Test analog `test_exit_date_null` (template.rs:481), der ein gesetztes `exit_date` als `DD.MM.YYYY` prüft. Weitere Datums-Variablen gibt es im Member-Context aktuell nicht.

### Test-Strategie
- **D-12:** MIME-Byte-Tests (`email.formatted()` + `String::from_utf8_lossy`, bestehende Technik worker.rs) asserten die `multipart/alternative`-Struktur in beiden Fällen: nur-Text (`body_html` NULL) UND Text+HTML; mit Anhang die korrekte `mixed{ alternative{…}, attachments }`-Verschachtelung. Escaping-Test: Mitgliedswert mit `<script> & Co` erscheint im HTML-Body escaped, im Text-Body roh. ammonia-Test: Autor-HTML mit `<script>`/`onclick`/`javascript:`-Link wird gestrippt.

### Claude's Discretion
- Genaue Namen/Orte: HTML-Render-Env-Funktion (`html_env()` o. ä.), `format_de`-Helfer-Modul, Signatur-Erweiterung von `build_message` um den optionalen HTML-Teil.
- Ob der ammonia-Aufruf ein geteilter Helfer (`sanitize_html()`) wird oder inline an den 3 Eintritts-Punkten (empfohlen: geteilter Helfer gegen Divergenz).
- Exakte minijinja-Autoescape-Konfiguration (Version-abhängig; siehe canonical_refs).

### Folded Todos
- **`2026-06-28-html-mail-support-statt-nur-textmails.md`** (`resolves_phase: 23`) — „HTML-Mail-Support statt nur Textmails": Kern dieser Phase. `lettre` `multipart/alternative` (Text+HTML), minijinja-HTML-Auto-Escaping, beide Varianten getrennt gepflegt (KEINE Text-aus-HTML-Ableitung — HTML-02). **Der Frontend-/WYSIWYG-Teil des Todos gehört zu Phase 24**, nicht hierher.
- **`2026-07-02-mail-datum-deutsches-format.md`** (`resolves_phase: 23`) — FMT-01, siehe D-11. `time::format_description`-Vorlage `[day].[month].[year]` statt `.to_string()` in template.rs:17-18, geteilter `format_de`-Helfer, Unit-Test analog `test_exit_date_null`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — HTML-01..05 (Zeilen 23-27), FMT-01 (Zeile 47), Future-Deferral „HTML-Mail-Bilder/Branding" (Zeile 53), Phase-23-Zuordnung (Zeilen 79-84, 101)
- `.planning/ROADMAP.md` §"Phase 23: HTML Mail Backend" (Zeilen 97-109) — Goal, Success Criteria (HTML-01..05 + FMT-01), Dependency auf Phase 22, harte Ordering-Constraint zu Phase 24

### Vorherige Phase (geteilter Helfer — Fundament)
- `.planning/phases/22-8bit-shared-mail-body-helper/22-CONTEXT.md` — der `build_message(...)`-Helfer (D-01..D-06 dort), in den die `alternative`-Verschachtelung ergänzt wird; MIME-Byte-Test-Muster
- `genossi_mail/src/worker.rs:627-720` — `send_mail_for_recipient` / Quelle des `build_message`-Extracts (Attachment-Loop, `MultiPart::mixed()`-Wrapping)

### Kern-Code (HTML-Render & Escaping)
- `genossi_mail/src/template.rs:61-83` — `strict_env()` + `render_template` (bleibt für Text/Subject; neue autoescapende Env kommt daneben)
- `genossi_mail/src/template.rs:15-47` — `member_to_template_context` (FMT-01 Fix an Zeilen 17-18; gemeinsamer Context für Text + HTML)
- `genossi_mail/src/render.rs:43-152` — `resolve_rendered_content` → `(subject, body)`; muss zusätzlich den gerenderten HTML-Body liefern (für `rendered_html_body`-Persistenz)

### Eintritts-Punkte für ammonia-Sanitization (HTML-05)
- `genossi_mail/src/service.rs:268` — `create_job`
- `genossi_mail/src/service.rs:447-488` — `send_test_mail_with_body` (Test-Mail-Pfad)
- Template-Create + Template-Update (`MailTemplateDao`-Aufrufer in `service.rs` — genaue fn beim Planen lokalisieren)

### Schema / Structs
- `genossi_mail/src/dao.rs:28-49` — `MailJob` (+ `body_html`)
- `genossi_mail/src/dao.rs:219-224` — `MailTemplate` (+ `body_html`)
- `genossi_mail/src/dao.rs:52-73` — `MailRecipient` (+ `rendered_html_body`; Muster: `rendered_body`/`rendered_reconstructed`)
- `migrations/sqlite/20260403000003_create_mail_jobs_table.sql` — mail_jobs (`body TEXT NOT NULL`)
- `migrations/sqlite/20260403000004_create_mail_recipients_table.sql` — mail_recipients
- `migrations/sqlite/20260416100001_seed_mail_templates.sql` — mail_templates-Referenz
- Migrations-Vorbild (forward-only ADD COLUMN): `migrations/sqlite/20260603100000_mail_job_attach_repayment_letter.sql`

### Bestehende Codebase-Maps
- `.planning/codebase/INTEGRATIONS.md`, `.planning/codebase/STACK.md` — Mail-/lettre-/minijinja-Kontext

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`build_message(...)`** (Phase 22) — der einzige Message-Bau-Ort; besitzt bereits `MultiPart::mixed()`-Attachment-Wrapping → die `alternative`-Verschachtelung kommt hier rein, alle Sendepfade profitieren automatisch.
- **`member_to_template_context`** (template.rs:15) — ein gemeinsamer Context für Text + HTML; FMT-01-Fix an einer Stelle wirkt in beide Varianten.
- **`rendered_body`/`rendered_reconstructed`** auf `MailRecipient` (dao.rs:67-72, Quick 260614) — exaktes Muster für die neue `rendered_html_body`-Persistenz durch den Worker.
- **MIME-Byte-Test-Technik** `email.formatted()` + `String::from_utf8_lossy` (worker.rs) — direkt wiederverwendbar für `alternative`-Struktur-Asserts.

### Established Patterns
- **Getrennte minijinja-Envs pro Behavior:** `strict_env()` ist die Vorlage; die HTML-Env wird analog daneben gebaut (nur mit Autoescape).
- **Forward-only `ADD COLUMN … NULL`-Migrationen** mit NULL-Legacy-Semantik (Vorbild: `attach_repayment_letter`, `rendered_reconstructed`).
- **Optionale Felder als `Option<Arc<str>>`** auf den DAO-Structs (wie `rendered_body`).
- **Immer Enum statt bool** (Projekt-Regel) — falls irgendwo ein „HTML ja/nein"-Zustand modelliert wird, kein `bool`. (Hier primär via `Option<body_html>` = None/Some abgebildet.)

### Integration Points
- `SmtpConfig`/`MailEncoding` (Phase 22) fließt bereits in alle Sendepfade → der HTML-Teil braucht kein zusätzliches Config-Wiring.
- `resolve_rendered_content` (render.rs) ist die Naht zwischen Job-Daten und Worker-Versand → hier entsteht der zweite Rückgabewert (HTML-Body) für Persistenz + `build_message`.

</code_context>

<specifics>
## Specific Ideas

- **User-Entscheidung Filter:** ammonias **permissiver Default** (inkl. Tabellen/Überschriften; durchgelassene externe `<img>`), bewusst KEINE enge Custom-Whitelist — „ammonias permissiver Default ist gut, so machen".
- **User-Anforderung Persistenz:** Gerendertes HTML MUSS pro Empfänger aufbewahrt werden — „wir müssen aufbewahren, was verschickt wurde" → `rendered_html_body`-Spalte (D-08), nicht nur on-the-fly.
- **Researcher-Stolperstein (verify):** Wenn eine Template-Variable *innerhalb* eines Attributs stünde (z. B. `<a href="{{ link }}">`), könnte ammonia beim Store-Sanitize den `{{ link }}`-Platzhalter als ungültige URL strippen und das Template beschädigen. Constraint: Variablen erscheinen nur im **Textinhalt**, nicht in `href`/Attributen (deckt sich mit dem Phase-24-Editor, wo Link-URLs autorfest sind). Vor Implementierung verifizieren, dass ammonia `{{ }}`-Platzhalter im Textinhalt unangetastet lässt.

</specifics>

<deferred>
## Deferred Ideas

- **HTML-Mail-Bilder / Briefkopf / Logo / Inline-CSS-Branding** — eingebettete Bilder (Upload + CID-Einbettung) und Branding. REQUIREMENTS.md „Future Requirements (deferred — nicht in v1.4)". Der ammonia-Default lässt zwar einen externen `<img>`-Tag durch, aber es wird in Phase 23 keine Bild-**Funktion** gebaut.
- **WYSIWYG-Frontend-Editor** (EDIT-01..05) → Phase 24. Braucht den `body_html`-API-Wire + ammonia-Gate aus Phase 23; harte Ordering-Constraint (Gate MUSS vor/mit dem Editor landen).
- **Antrags-Datei-Upload + auditierter Carryover** (APDOC-01..05) → Phase 25 (unabhängig von der Mail-Strecke).

### Reviewed Todos (not folded)
- `2026-06-27-originalen-mitgliedsantrag-als-datei-attachment-an-applicati.md` — False-Positive (score 0.6, generische Keywords); gehört zu **Phase 25** (Application-Datei-Upload), nicht zur Mail-Strecke.
- `backend-pre-flight-check-attach-repayment-letter.md` — RepaymentLetter-Pre-Flight-Check; **Backlog**, nicht Phase 23.
- `frontend-bulk-no-repayment-letter-action.md` — Bulk-Generierung von no_repayment_letter-Briefen; **Backlog**, nicht Phase 23.

</deferred>

---

*Phase: 23-html-mail-backend*
*Context gathered: 2026-07-02*
