# Phase 20: Inbox-Digest — täglicher Posteingangs-Benachrichtigungs-Worker - Context

**Gathered:** 2026-06-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Ein Hintergrund-Worker verschickt **einmal pro Kalendertag** zur konfigurierten Uhrzeit eine
Zusammenfassungs-Mail aller **offenen (nicht-archivierten)** Posteingangs-Mails an eine oder
mehrere konfigurierbare Empfänger-Adressen. Jede Mail listet Titel, Absender und
Eingangszeitpunkt und enthält einen Deep-Link auf `/inbox`. Versand nur bei nicht-leerem
Posteingang und mindestens einem konfigurierten Empfänger. Empfänger und Uhrzeit werden über
die bestehende Config-Seite gepflegt (analog SMTP/IMAP-Settings).

Requirements DIGEST-01..07 sind als Scope fixiert (`.planning/REQUIREMENTS.md`).

**Nicht in dieser Phase:** Reply-Komfort (Phase 21), feineres Intervall als täglich (DIGEST-F2,
bewusst verworfen), Digest nur über neu eingegangene Mails seit letztem Versand (DIGEST-F1,
bewusst zugunsten der Workqueue-Erinnerung verworfen).

</domain>

<decisions>
## Implementation Decisions

### Scheduling & Tages-Garantie
- **D-01:** Verpasstes Zeitfenster wird **nachgeholt** — war der Server zur Uhrzeit aus und ist
  heute noch keine Digest-Mail raus, sendet der nächste Worker-Lauf nach (kein Verpassen).
- **D-02:** Die konfigurierte Versand-Uhrzeit gilt in **Server-Lokalzeit** (TZ env des
  Deployments). Kein chrono-tz / keine explizite Zeitzonen-Konfiguration.
- **D-03:** „Genau ein Versand pro Kalendertag" wird über eine **neue, dedizierte DB-Tabelle**
  (eigene Migration in `migrations/sqlite/`) persistiert — bewusst NICHT über den
  Config-KV-Store. Erwartete Form: kleine Singleton-/State-Tabelle, die das letzte
  Versanddatum hält. Braucht eine eigene DAO-Anbindung.
- **D-04:** Der Worker **pollt periodisch** (Richtwert ~60s) und vergleicht aktuelle Uhrzeit +
  letztes Versanddatum — analog zur Loop-Struktur in `timestamp_worker.rs`. KEIN
  „sleep bis nächste Uhrzeit" (robuster gegen Zeitumstellung/Config-Änderung).

### Empfänger-Format & Versand-Art
- **D-05:** Mehrere Empfänger werden in **einem komma-getrennten Textfeld** gepflegt (ein
  Config-Key, z.B. `digest_recipients='a@x.de,b@y.de'`).
- **D-06:** Versand erfolgt als **Einzelmail pro Empfänger** (To: nur dieser Empfänger).
  Empfänger sehen sich gegenseitig nicht; ein fehlerhafter Empfänger blockiert die anderen nicht.
- **D-07:** Schlägt der Versand an einen Empfänger fehl (SMTP-Fehler), wird der Fehler
  **geloggt und es geht weiter** mit den übrigen Empfängern. Das Versanddatum wird trotzdem
  gesetzt (der Tag gilt als erledigt — kein Retry/Mehrfachversand).

### Digest-Mail Inhalt & Format
- **D-08:** **Plain-Text**-Mail (keine HTML-Mail). Passt zum vorhandenen Versand-Helper.
- **D-09:** Body wird **hardcodiert im Worker** zusammengebaut (`format!`), deutscher Text fix.
  KEINE minijinja-Template-Infrastruktur für diese interne Benachrichtigung.
- **D-10:** Mails werden **neueste zuerst** (absteigend nach Eingangszeitpunkt) gelistet; der
  **Digest-Betreff nennt die Anzahl** der offenen Mails (z.B. „Posteingang: 5 offene Mails").
- **D-11:** Deep-Link = `{APP_URL}/inbox`, gebaut aus der **`APP_URL`-Env-Variable** mit
  Fallback `http://localhost:3000/` — exakt das Pattern aus `helper_token.rs`. KEIN eigener
  Config-Key für die Basis-URL.

### Config-Seite UI & Validierung
- **D-12:** Auf der Config-Seite entsteht ein **eigener Abschnitt** „Posteingangs-Benachrichtigung"
  im Stil der bestehenden SMTP/IMAP-Blöcke in `config_page.rs` (Empfänger-Feld + Uhrzeit +
  Speichern-Button). NICHT in den IMAP-Block integriert.
- **D-13:** Vor dem Speichern werden **Empfänger-Adressen (grobes E-Mail-Format, je Adresse)
  und die Uhrzeit (HH:MM)** validiert; Fehler inline gemeldet.
- **D-14:** **Deaktivierung über leeres Empfänger-Feld** — keine Adressen konfiguriert ⇒ Worker
  sendet nicht (kein Fehler, DIGEST-07 direkt abgedeckt). KEIN separater Enabled-Toggle.

### Claude's Discretion
- Konkrete Config-Key-Namen (z.B. `digest_recipients`, `digest_send_time`), Tabellen-/
  Spaltennamen der State-Tabelle, exakter Poll-Intervall-Wert, genaue Betreff-/Body-Formulierung
  und die exakte E-Mail-Format-Prüfung liegen im Ermessen von Research/Planning, solange die
  obigen Entscheidungen eingehalten werden.

### Reviewed Todos (nicht eingefoldet)
- `backend-pre-flight-check-attach-repayment-letter.md` — geprüft, **nicht relevant** (False-Positive-Keyword-Match auf „Empfänger"; betrifft RepaymentLetter-Versand, nicht den Inbox-Digest).
- `frontend-bulk-no-repayment-letter-action.md` — geprüft, **nicht relevant** (False-Positive; Bulk-Action für RepaymentLetter-Briefe).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Scope
- `.planning/REQUIREMENTS.md` — DIGEST-01..07 (Scope), DIGEST-F1/F2 (bewusst verworfen)
- `.planning/ROADMAP.md` §"Phase 20" — Goal + Success Criteria (1–4)

### Worker-Vorbild (Pattern, das nachzubauen ist)
- `genossi_service_impl/src/timestamp_worker.rs` — config-getriebener Loop: `get_all()` lesen,
  enabled/Intervall ableiten, Aktion ausführen, sleepen, Dedup. Vorlage für den Digest-Worker.

### Config-System
- `genossi_config/src/dao.rs` — `ConfigEntry{key,value,value_type}`, `ConfigDao` (KV-Store)
- `genossi_config/src/service.rs` — `ConfigService::get_all/get/set` + `validate_value`
- `genossi-frontend/src/page/config_page.rs` — SMTP/IMAP-Block-Pattern für den neuen
  Digest-Abschnitt (Signals, `get_config_value`/`has_config_key`, Speichern-Flow)

### Inbox / Mails
- `genossi_mail/src/inbox.rs` — `InboxService::list()` → `Arc<[InboundMail]>`; Filter
  `archived == false` ergibt offene Mails. `InboundMail`-Felder (Titel/Absender/Eingangszeit).
- `genossi_mail/src/dao.rs` — `InboundMail`-Struct (`archived: bool` u.a.)

### Versand & Deep-Link
- `genossi_mail/src/service.rs` — `send_test_mail_with_body(to, subject, body)`,
  `load_smtp_config`, SMTP-Transport-Aufbau (lettre). Versand-Mechanismus zum Wiederverwenden.
- `genossi_rest/src/helper_token.rs:36-40` — `APP_URL`-Deep-Link-Pattern (Fallback-Semantik),
  Vorlage für den `/inbox`-Link.

### Worker-Wiring
- `genossi_bin/src/lib.rs` (~Zeile 1383–1517) — bestehende `tokio::spawn`-Worker-Starts; hier
  wird der neue Digest-Worker analog gespawnt.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `timestamp_worker::start_timestamp_worker` — Loop-Skelett (config-read + Aktion + sleep) als
  direkte Vorlage für den Digest-Worker.
- `MailService::send_test_mail_with_body` + `load_smtp_config` — fertiger Plain-Text-Versandweg.
- `InboxService::list()` + `archived`-Filter — liefert die offenen Mails.
- `config_page.rs` SMTP/IMAP-Blöcke + `get_config_value`/`has_config_key`-Helper — Vorlage für
  den neuen Config-Abschnitt.
- `helper_token.rs` `APP_URL`-Linkbau — Vorlage für den `/inbox`-Deep-Link.

### Established Patterns
- Config = generischer KV-Store (`ConfigEntry`), Settings liegen als Einzel-Keys; Validierung
  über `value_type` + `validate_value`.
- Hintergrund-Worker werden in `genossi_bin/src/lib.rs` per `tokio::spawn` beim Serverstart
  gestartet (mehrere bestehende Beispiele).
- Soft-Delete/Migrationen: neue Tabelle ⇒ Migration in `migrations/sqlite/`, DAO-Trait + SQLite-Impl.

### Integration Points
- **Neuer Worker-Spawn** in `genossi_bin/src/lib.rs` (DI-Wiring: ConfigService, InboxService/
  MailService, neue Digest-State-DAO).
- **Neue Migration + State-DAO** (D-03) für das letzte Versanddatum.
- **Config-Seite** (`config_page.rs`) bekommt den neuen Abschnitt; Config-Keys werden über das
  bestehende Config-REST/Service gespeichert.
- **APP_URL**-Env muss im Worker-Kontext lesbar sein (analog helper_token).

</code_context>

<specifics>
## Specific Ideas

- Worker-Loop **exakt nach dem Vorbild `timestamp_worker.rs`** strukturieren — der User hat dieses
  Muster (config-read im Loop, periodisches Polling, Dedup) bewusst als Referenz bestätigt.
- Deep-Link-Aufbau **exakt nach `helper_token.rs`** (`APP_URL` + Fallback `http://localhost:3000/`,
  trailing slash trimmen).
- Bewusste Abweichung vom einfachsten Weg: Dedup-State liegt in einer **eigenen DB-Tabelle**, nicht
  im Config-KV-Store (D-03).

</specifics>

<deferred>
## Deferred Ideas

- **Reply-Komfort / Antwort-Modal** — eigene Phase 21 (REPLY-01..04).
- **Feineres Versand-Intervall als täglich** (DIGEST-F2) — bewusst verworfen.
- **Digest nur über neu eingegangene Mails seit letztem Versand** (DIGEST-F1) — bewusst zugunsten
  der Workqueue-Erinnerung (alle offenen Mails) verworfen.
- **HTML-Mail / minijinja-Template für den Digest** — nicht in dieser Phase (D-08/D-09 → Plain-Text,
  hardcodiert); könnte später nachgezogen werden, falls gewünscht.
- **Expliziter Enabled-Toggle / eigener Config-Key für Basis-URL** — bewusst verworfen (D-14/D-11).

### Reviewed Todos (nicht eingefoldet)
- `backend-pre-flight-check-attach-repayment-letter.md` — nicht relevant (RepaymentLetter, nicht Inbox-Digest).
- `frontend-bulk-no-repayment-letter-action.md` — nicht relevant (RepaymentLetter-Bulk-Action).

</deferred>

---

*Phase: 20-inbox-digest-t-glicher-posteingangs-benachrichtigungs-worker*
*Context gathered: 2026-06-26*
