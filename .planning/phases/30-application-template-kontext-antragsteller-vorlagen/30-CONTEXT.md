# Phase 30: Application-Template-Kontext (Antragsteller-Vorlagen) - Context

**Gathered:** 2026-08-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Vorlagen können gegen einen **eigenen Application-Kontext** gerendert werden, und der
Vorstand hat eine mitgelieferte deutsche **„Zahlungserinnerung"** mit korrekt berechnetem,
korrekt formatiertem offenem Betrag.

Konkret: neuer „Antragsteller"-Vorlagentyp getrennt vom Member-Pool (D1),
`application_to_template_context`-Kontext-Builder, `format_eur_de`-Helper, generischer
`validate_rendered`-Kern für Antragsteller-Vorlagen, und ein Seed für die Standard-Vorlage.

**Nicht in dieser Phase:** Der eigentliche Versand + REST-Endpoints (Phase 31), der
Frontend-Compose-Dialog (Phase 32), die `application_id`-Historie-Linkage (bereits Phase 29
abgeschlossen). Kein neues Zahlungsstatus-Feld auf `ApplicationEntity` — der offene Betrag
wird zur Laufzeit berechnet, nie gespeichert. Keine neue Backend-Dependency („add nothing").

</domain>

<decisions>
## Implementation Decisions

### Vorlagentyp-Trennung (D1-Mechanismus)
- **D-01:** Trennung via **neuer Spalte `template_type TEXT NOT NULL DEFAULT 'member'`** auf
  `mail_templates` (Werte `'member'` / `'application'`). Additive, forward-only Migration.
  Kein separates Tabellen-Set (vermeidet Duplikation von DAO/Service/REST/Frontend-CRUD),
  kein Bool-Flag (bessere Erweiterbarkeit für spätere Typen, vgl. APTPL-FUT-01 mehrstufige
  Erinnerungen). Die 2 bestehenden Seeds („Formelle/Informelle Anrede") bleiben `'member'`.
- **D-02:** **PITFALL — SQL-Spaltenlisten:** Analog zum Phase-29-`mail_recipients`-Muster
  müssen ALLE `mail_templates`-SQL-Spaltenlisten (SELECT/INSERT/UPDATE, inkl. Test-DDL) um
  `template_type` erweitert werden. NULL-/Default-Legacy-Roundtrip absichern: bestehende
  Zeilen ohne expliziten Typ lesen als `'member'` zurück.
- **D-03:** Der Frontend-Member-Template-Selector filtert auf `template_type = 'member'`,
  damit im Member-Massenmail keine Antragsteller-Vorlage wählbar ist (Kern von D1). Die
  Datengrundlage (Typ-Feld im Read-Pfad/TO) entsteht in Phase 30; die Filterung im UI greift
  in Phase 32.

### Kontext-Umfang & Platzhalter
- **D-04:** Der Application-Kontext enthält die **Application-Felder** unter member-kompatiblen
  Variablennamen: `first_name`, `last_name`, `salutation`, `title` (damit die bestehenden
  Anrede-Bausteine `{% if salutation == "Herr" %}…` 1:1 funktionieren). Anzahl Anteile als
  `shares` (das Application-Feld heißt `shares: i32`, NICHT `current_shares`).
- **D-05:** Der Kontext enthält zusätzlich die **Genossenschafts-Bankdaten aus der Config**
  (dieselbe Quelle wie `send_confirmation_mail`): `bank_iban`, `bank_name`, `bank_bic`,
  `genossenschaft_name`. Begründung: Die Zahlungserinnerung nennt UNSERE IBAN — der
  Antragsteller überweist an die Genossenschaft. Die Application hat kein eigenes Bankfeld;
  Antragsteller-Bankdaten sind irrelevant. Optional zusätzlich ein Verwendungszweck-Baustein
  im Template-Text (`Beitritt {{ first_name }} {{ last_name }}`), kein separater Config-Wert.
- **D-06:** Der **offene Betrag** kommt als **vorformatierter String** unter `open_amount`
  (z. B. `"1.234,56 €"`) in den Kontext — direkt renderbar ohne Jinja-Filter. Berechnung:
  `shares × share_value_cents` (D3), formatiert via `format_eur_de`.
- **D-07:** **Design des Kontext-Builders:** `application_to_template_context` ist eine **pure,
  synchrone Funktion**, die die aufgelösten Config-Werte (`share_value_cents`, `bank_iban`,
  `bank_name`, `bank_bic`, `genossenschaft_name`) als **Parameter** entgegennimmt — sie macht
  KEINEN eigenen `config.get().await`-Lookup. So bleibt sie unit-testbar wie
  `member_to_template_context`. Der Service-Layer (Phase 31) löst die Config einmal auf und
  reicht die Werte hinein.

### Validierung (APTPL-04)
- **D-08:** Probe-Render gegen **einen Dummy-Application-Kontext** (fester Sentinel, analog
  `validate_template_with_repayment` / `dummy_repayment_context`) — kein DB-Zugriff,
  deterministisch, deckt „unbekannte/Member-only-Platzhalter → kontrollierter Fehler" ab.
- **D-09:** Ein generischer **`validate_rendered(subject, body, &[Value])`-Kern** wird
  extrahiert; das bestehende `validate_template` (Member) UND ein neues
  `validate_application_template` rufen beide diesen Kern. Die **Signatur von
  `validate_template` bleibt unverändert** → die ~40 bestehenden Member-Template-Tests bleiben
  grün.
- **D-10:** Die Antragsteller-Vorlagen-Validierung greift am selben Punkt wie beim
  Member-Flow: bei **Create/Update** der Vorlage (`rest_templates.rs`), damit ein kaputtes
  Template gar nicht erst gespeichert wird. Kein `strict`-Render-Crash beim späteren Versand.

### format_eur_de + Seed-Content
- **D-11:** `format_eur_de(cents: i64) -> String` lebt in **`genossi_service`** (neben
  `iban::mask_iban`) — reine Domänen-Formatierung, von mehreren Callern nutzbar. Deutsches
  Format: Tausenderpunkt, Dezimalkomma, `€`-Suffix.
- **D-12:** `send_confirmation_mail` wird **auf `format_eur_de` umgestellt** (ersetzt das naive
  `format!("{},{:02} €")` ohne Tausenderpunkt) — Konsistenz, ein einziger korrekter
  Euro-Formatter. Kleiner Blast-Radius (eine Formatier-Stelle), berührt getesteten Code →
  bestehende Erwartung ggf. mit-anpassen.
- **D-13:** `format_eur_de` behandelt **Null (`0,00 €`) und Negativ (`-1.234,56 €`)** korrekt
  und wird direkt getestet (APTPL-02 verlangt es explizit), obwohl der offene Betrag praktisch
  nie negativ wird (`shares ≥ 0`, `share_value_cents > 0`).
- **D-14:** Seed-Vorlage **„Zahlungserinnerung"**: formeller Ton (Sie-Form), `template_type =
  'application'`, **fixe UUID `00000000-0000-0000-0000-000000000003`** (Reihe der bestehenden
  `…0001`/`…0002`-Seeds fortsetzen), `INSERT OR IGNORE` in eigener Seed-Migration. Betreff
  z. B. „Zahlungserinnerung — Ihre Beitrittserklärung". Body: Anrede-Baustein + Hinweis auf
  offenen Betrag (`{{ open_amount }}`) + Anzahl Anteile + Bankverbindung
  (`{{ bank_iban }}`/`{{ bank_name }}`/`{{ bank_bic }}`/`{{ genossenschaft_name }}`) +
  Verwendungszweck `Beitritt {{ first_name }} {{ last_name }}` + freundlicher Gruß. Rendert den
  Haupt-Use-Case ohne manuelle Konfiguration.

### Claude's Discretion
- Exakter Wortlaut/Formatierung des Seed-Vorlagen-Textes (Betreff + Body), solange formell,
  deutsch, alle in D-14 genannten Platzhalter enthalten und strict-render-sicher.
- Genaue Sentinel-Werte des Dummy-Application-Kontexts (D-08), solange auffällig/deterministisch.
- Ob `body_html` für den Seed gesetzt wird oder text-only (NULL-Legacy = text-only ist ok).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/ROADMAP.md` §"Phase 30" — Goal + 4 Success Criteria + Load-bearing D1/D3
- `.planning/REQUIREMENTS.md` — APTPL-01..04 (Zeilen 21-24), Entscheide D1/D3 (Zeilen 69-71)

### Template-/Mail-Subsystem (Wiederverwendung — „add nothing")
- `genossi_mail/src/template.rs` — `member_to_template_context` (Vorbild für den neuen
  Application-Builder, Zeile 16), `validate_template` (Zeile 126, Signatur bleibt),
  `validate_template_with_repayment` + `dummy_repayment_context` (Vorbild für Dummy-Probe,
  Zeilen 258/300), `strict_env`/`html_env`/`render_template` (Render-Pfad)
- `genossi_mail/src/rest_templates.rs` — Create/Update-Validierungs-Call-Site (D-10)
- `genossi_service_impl/src/application.rs` §`send_confirmation_mail` (Zeile 44) —
  Config-Quelle `share_value_cents`/`bank_*`/`genossenschaft_name` (D3/D-05), Euro-Format-Stelle
  die auf `format_eur_de` umgestellt wird (D-12), Application-Feldzugriff (`app.shares`,
  `app.salutation`, `app.title`, …)
- `genossi_service/src/application.rs` — `Application`-Struct-Felder (Zeile 12) für den Kontext

### Migrationen (Muster)
- `migrations/sqlite/20260416100000_create_mail_templates_table.sql` — heutiges Schema (keine
  Typ-Spalte)
- `migrations/sqlite/20260416100001_seed_mail_templates.sql` — Seed-Muster mit fixen UUIDs
  (Vorbild für „Zahlungserinnerung"-Seed, D-14)
- `migrations/sqlite/20260702000000_mail_templates_add_body_html.sql` — Muster für additive
  `ALTER TABLE mail_templates ADD COLUMN … NULL/DEFAULT` forward-only Migration (D-01)

### Formatierung
- `genossi_service/src/iban.rs` (`mask_iban`) — Nachbarschaft/Muster für `format_eur_de` (D-11)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `member_to_template_context` (`genossi_mail/src/template.rs:16`): direktes Vorbild — gleicher
  `minijinja::context!`-Aufbau, member-kompatible Variablennamen übernehmen (D-04).
- `validate_template_with_repayment` + `merge_repayment_context` + `dummy_repayment_context`:
  etabliertes Muster „Dummy-Context als Validierungs-Probe" → 1:1 auf Application übertragbar
  (D-08).
- `send_confirmation_mail` (`genossi_service_impl/src/application.rs:44`): liefert die exakte
  Config-Kette (`share_value_cents`, `bank_iban`, `bank_name`, `bank_bic`,
  `genossenschaft_name`) UND die zu ersetzende Euro-Format-Stelle (Zeilen 99-102).

### Established Patterns
- **Strict minijinja env** (`UndefinedBehavior::Strict`): unbekannte Platzhalter erroren beim
  Render — genau darauf baut die kontrollierte Fehlschlag-Semantik (APTPL-04/D-08).
- **Additive forward-only Migrationen** mit `DEFAULT`/NULL-Legacy-Roundtrip (Phase-29-Muster,
  `mail_templates_add_body_html`).
- **PITFALL (aus Phase 29):** Beim Hinzufügen einer Spalte alle SQL-Spaltenlisten synchron
  ziehen (D-02).

### Integration Points
- DAO/Entity/TO für `mail_templates` bekommen das `template_type`-Feld (Read-Pfad exponiert
  es für den Phase-32-Selector-Filter).
- Der neue Kontext-Builder + `validate_application_template` werden in Phase 31 vom
  `ApplicationService::send_mail` konsumiert (Config-Auflösung dort).
- `format_eur_de` in `genossi_service` wird von Application-Kontext (D-06) UND
  `send_confirmation_mail` (D-12) genutzt.

</code_context>

<specifics>
## Specific Ideas

- Die Zahlungserinnerung geht inhaltlich um DIE Bankverbindung DER GENOSSENSCHAFT (unsere
  IBAN), an die der Antragsteller überweist — nicht um Antragsteller-Bankdaten. Deshalb kommen
  die Bank-Felder aus der Config, nicht aus der Application (D-05).
- Variablennamen bewusst member-kompatibel, damit Anrede-Logik unverändert wiederverwendbar
  bleibt (D-04) — trotz getrenntem Pool (D1).

</specifics>

<deferred>
## Deferred Ideas

- **Mehrstufige Erinnerungs-Vorlagen** (1./2. Erinnerung mit Eskalationstext) — APTPL-FUT-01,
  eigene Zukunfts-Phase. Die `template_type`-Spalte (D-01) hält den Weg dafür offen.
- Versand + REST-Endpoints + Guardrails → Phase 31.
- Frontend-Compose-Dialog + Template-Selector-Filterung im UI → Phase 32.

</deferred>

---

*Phase: 30-application-template-kontext-antragsteller-vorlagen*
*Context gathered: 2026-08-12*
