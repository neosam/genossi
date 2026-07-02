# Phase 22: 8bit + Shared Mail-Body Helper - Context

**Gathered:** 2026-07-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Alle ausgehenden Mails (Bulk-Worker, Test-Mail, Digest, Reply) bauen ihre `lettre::Message` über **eine** geteilte, pure Funktion `build_message(...)` mit konsistentem `Content-Type: text/plain; charset=utf-8`. Der Text-Teil kann per Konfiguration als `8bit` statt `quoted-printable` kodiert werden (opt-in, Default bleibt quoted-printable). Der bestehende Charset-Bug im Test-Mail-/Digest-Pfad wird dadurch behoben.

**In scope:** geteilte `build_message`-Fabrik; `MailEncoding`-Enum + `smtp_encoding`-Config-Key; Test-Mail/Digest laufen durch denselben Sende-Code wie der Worker; MIME-Byte-Tests; Runbook-Doku für den 8BITMIME-Verify-Schritt.

**Out of scope (bewusst, gegen frühere Diskussion abgegrenzt):** KEINE Schema-Änderung; Test-Mails werden NICHT als persistierte `MailJob`-Rows angelegt (Option „E" verworfen — hätte Migration + async-Test-UX bedeutet); keine volle „Message-Fabrik" mit DI von `DocumentStorage` in den Service; HTML/multipart-alternative (das ist Phase 23).

</domain>

<decisions>
## Implementation Decisions

### Helfer-Zuschnitt (D+ mit build_message-Split)
- **D-01:** Eine pure, **synchrone** Funktion `build_message(...)` wird aus `worker.rs::send_mail_for_recipient` (worker.rs:627-720) an einen geteilten Ort extrahiert (z. B. neues Modul `genossi_mail/src/send.rs` — genauer Ort ist Planner-Entscheidung). Sie besitzt **beide** MIME-Bausteine: (Baustein 1) den Text-`SinglePart` mit `charset=utf-8` + konfigurierbarer CTE, und (Baustein 2) subject, `message_id(None)`, `in_reply_to`/`references`, sowie das `MultiPart::mixed()`-Wrapping für Attachments.
- **D-02:** Die **Naht** liegt bei den **bereits geladenen Attachment-Bytes**, NICHT bei `DocumentStorage`. `build_message` nimmt eine Liste geladener Attachments (`filename`, `mime`, `bytes`) entgegen — dadurch braucht es kein `DocumentStorage`, ist synchron und rein MIME-testbar.
- **D-03:** Das Attachment-**Laden** (`document_storage.load().await`, worker.rs:678) bleibt pfadspezifisch im Worker (echte async-I/O). Der Worker lädt zuerst die Bytes, ruft dann `build_message(...)`, dann `transport.send()`.
- **D-04:** Test-Mail (`send_test_mail`, service.rs:415), Test-Mail-with-body (`send_test_mail_with_body`, service.rs:447) und Digest (`digest.rs:174`, läuft über Test-Mail-with-body) rufen `build_message(..., &[], None, encoding)` (leere Attachments, kein in_reply_to) + `transport.send()`. Damit läuft die Test-Mail durch **exakt denselben Message-Konstruktions-Code** wie der echte Versand → der Charset-Bug kann sich strukturell nicht mehr verstecken.
- **D-05:** Test-Mail bleibt **synchron** (sofortiges Feedback am Button erhalten) und wird **nicht** persistiert. KEIN `DocumentStorage`-Generic am `MailServiceImpl` (kein DI-Wiring in `genossi_bin`). Das ist der bewusste Unterschied zwischen D+ und der verworfenen Option E.
- **D-06:** Bonus-Konsolidierung: Die dreifach kopierten `.parse()`-Adress-Blöcke (worker.rs:642-650, service.rs:422-434, service.rs:464-477) wandern in `build_message` (from/to als `&str`, dort geparst).

### Config-Toggle (Enum, nie Boolean)
- **D-07:** Neues internes Enum `MailEncoding { QuotedPrintable, EightBit }` (kein `bool`). Fließt als Parameter in `build_message` — der **einzige** Ort, an dem quoted-printable vs 8bit entschieden wird.
- **D-08:** Neuer optionaler KV-Config-Key `smtp_encoding` mit String-Werten `"quoted-printable"` (Default) / `"8bit"`, gelesen in `load_smtp_config` (service.rs:127) analog zum bestehenden `smtp_tls` (service.rs:163-165), unbekannte/leere Werte fallen sauber auf den Default zurück. Neues Feld in `SmtpConfig` (service.rs:118-125). Default bleibt quoted-printable, bis der Betreiber opt-in aktiviert (MAIL-03).
- **D-09:** Für 8bit muss die Body-Part-Konstruktion von `SinglePart::plain()` auf `SinglePart::builder().header(ContentType::TEXT_PLAIN).header(ContentTransferEncoding::EightBit).body(...)` umgestellt werden (quoted-printable-Zweig kann `SinglePart::plain` bleiben oder ebenfalls explizit gesetzt werden — Planner-Detail).

### Test-Strategie
- **D-10:** `build_message` wird die getestete **Single-Source**. Unit-Tests asserten auf MIME-Byte-Ebene (`email.formatted()` + `String::from_utf8_lossy`, wie bestehende worker-Tests) sowohl `charset=utf-8` ALS AUCH die `Content-Transfer-Encoding` in **beiden** Modi: `quoted-printable`/`base64` (Default) und `8bit` (opt-in). Damit ist die 8bit-CTE trotz nicht durchführbarem Prod-Relay-Test byte-genau abgesichert.
- **D-11:** Die bestehenden worker-Tests (`plain_mail_body_has_utf8_charset` worker.rs:977, `multipart_mail_body_has_utf8_charset` worker.rs:1061) sollen `build_message` **aufrufen** statt die Build-Logik zu **re-inlinen** (heute duplizieren sie sie). Charset-Abdeckung für Test-Mail/Digest-Pfad wird ergänzt (heute ungetestet).

### MAIL-04 8BITMIME-Verifikation
- **D-12:** Schlanker **Runbook-/Deployment-Doku-Abschnitt** (genauer Ort ist Planner-Entscheidung — Betreiber-Doku) mit dem konkreten `openssl s_client -starttls smtp -connect <relay>:<port>` → EHLO → Prüfung auf `250-8BITMIME`, plus expliziter Reihenfolge „**erst** am Prod-Relay verifizieren, **dann** `smtp_encoding=8bit` setzen". Verify-in-Prod, aus Dev nicht automatisierbar (Relay nur über Prod-Netz).

### Claude's Discretion
- Genauer Modul-/Dateiname für die geteilte Funktion (`send.rs` o. ä.) und exakte Signatur-Details (Struct für Attachment-Tripel vs Tupel-Slice).
- Ort der Betreiber-Doku für D-12.
- Ob der quoted-printable-Zweig `SinglePart::plain` behält oder ebenfalls explizit CTE setzt.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — MAIL-01..05 (Zeilen 15-19), Phase-22-Zuordnung (Zeilen 74-78, 100)
- `.planning/ROADMAP.md` §"Phase 22" — Goal, Success Criteria (MAIL-01..05), Dependency-Reihenfolge

### Kern-Code (die drei divergierenden Sendepfade)
- `genossi_mail/src/worker.rs:627-720` — `send_mail_for_recipient` (der „korrekte" Pfad; Quelle für den `build_message`-Extract; enthält Attachment-Loop + message-id + in-reply-to)
- `genossi_mail/src/service.rs:415-445` — `send_test_mail` (Bug-Pfad: `.body()` ohne charset, service.rs:436)
- `genossi_mail/src/service.rs:447-488` — `send_test_mail_with_body` (Bug-Pfad: `.body()` ohne charset, service.rs:479; Privacy-Defense-Kommentar service.rs:453-457)
- `genossi_mail/src/digest.rs:170-177` — `build_digest_subject`/`build_digest_body` + Aufruf von `send_test_mail_with_body` (erbt den Bug)

### Config-Plumbing
- `genossi_mail/src/service.rs:118-125` — `SmtpConfig` (neues Feld `encoding`)
- `genossi_mail/src/service.rs:127-181` — `load_smtp_config` (neuer `smtp_encoding`-Key, Muster: `smtp_tls` bei 163-165)
- `genossi_mail/src/service.rs:183-209` — `build_transport` (nur Referenz — CTE ist Message-Concern, NICHT Transport-Concern)

### Tests
- `genossi_mail/src/worker.rs:722-1097+` — bestehende MIME-Byte-Tests (Vorbild für D-10/D-11; re-inlinen heute die Build-Logik)

### Bestehende Codebase-Maps
- `.planning/codebase/INTEGRATIONS.md`, `.planning/codebase/STACK.md` — Mail-/lettre-Kontext

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SinglePart::plain(body)` (lettre 0.11.20) setzt automatisch `text/plain; charset=utf-8` — der „korrekte" Baustein, den der Worker heute schon nutzt (worker.rs:656).
- `MultiPart::mixed().singlepart(text).singlepart(attachment)` — bestehendes Attachment-Wrapping (worker.rs:673-702), wandert in `build_message`.
- `email.formatted()` + `String::from_utf8_lossy` — bestehende Test-Technik für MIME-Byte-Asserts (worker.rs:995-1006).

### Established Patterns
- **Optionale Config-Keys mit Default-Fallback:** `smtp_tls` (service.rs:163-165) ist die Vorlage für `smtp_encoding`.
- **Config ist KV-basiert** via `ConfigService::get_all()` (service.rs:139), NICHT env-Vars.
- **Immer Enum statt bool** (User-Regel) — `MailEncoding` statt `smtp_8bit: bool`.
- lettre-Default für CTE ist quoted-printable/base64 (Auto); für 8bit muss CTE explizit gesetzt werden.

### Integration Points
- `SmtpConfig` fließt bereits in alle drei Sendepfade (worker.rs:639, service.rs:418, service.rs:460) → das neue `encoding`-Feld erreicht alle Pfade ohne zusätzliches Wiring.
- `send_mail_for_recipient` ist eine eigenständige Pro-Empfänger-Funktion und braucht **keinen** persistierten Job — deshalb kann der Test-Pfad denselben Sende-Code nutzen, ohne einen Job anzulegen.

</code_context>

<specifics>
## Specific Ideas

- Kernanliegen des Users: „Eine Test-Mail soll genau das testen, was beim Versenden passiert." → gelöst durch geteilten `build_message`-Code (identisch **auf der Leitung**), bewusst OHNE Test-Mails zu persistierten Jobs zu machen.
- User-Regel für dieses Projekt: **immer Enum, nie Boolean** für umschaltbare/konfigurierbare Werte.

</specifics>

<deferred>
## Deferred Ideas

- **Option E — Test-Mails als versteckte echte `MailJob`-Rows** (mit `MailJobKind::{Normal, Test}`-Enum, vom Worker abgearbeitet, aus der UI gefiltert): verworfen für Phase 22, weil es eine Schema-Migration erfordert (Phase 22 ist „no-schema") und die Test-Mail von synchron auf asynchron umstellen würde (Verlust des sofortigen SMTP-Feedbacks). Falls später gewünscht: eigene Phase, bewusst mit Migration + Filter-Fläche (Job-Listen, Counts, ggf. Audit).
- **FMT-01 (deutsches Datumsformat DD.MM.YYYY in Template-Variablen):** gehört zu Phase 23 (`resolves_phase: 23`, `.planning/todos/pending/2026-07-02-mail-datum-deutsches-format.md`) — NICHT Phase 22.
- HTML-Mail / multipart-alternative → Phase 23. WYSIWYG-Editor → Phase 24. Antrags-Datei-Upload → Phase 25.

### Reviewed Todos (not folded)
Alle 5 Todo-Matches aus `todo.match-phase 22` waren False-Positives (score 0.6, generische Keyword-Treffer) und gehören zu anderen Phasen: HTML-Mail (Phase 23), Datums-Format FMT-01 (Phase 23), Antrags-Datei (Phase 25), RepaymentLetter-Pre-Flight/Bulk (Backlog). Keiner in Phase-22-Scope gefaltet.

</deferred>

---

*Phase: 22-8bit-shared-mail-body-helper*
*Context gathered: 2026-07-02*
