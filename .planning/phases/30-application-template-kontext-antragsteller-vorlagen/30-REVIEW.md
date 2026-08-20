---
phase: 30-application-template-kontext-antragsteller-vorlagen
reviewed: 2026-08-20T00:00:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - genossi_service/src/euro.rs
  - genossi_service/src/lib.rs
  - genossi_service_impl/src/application.rs
  - genossi_mail/src/dao.rs
  - genossi_mail/src/dao_sqlite.rs
  - genossi_mail/src/rest_templates.rs
  - genossi_mail/src/mail_template_service.rs
  - genossi_mail/src/template.rs
  - genossi_bin/tests/e2e_tests.rs
  - migrations/sqlite/20260820000000_mail_templates_add_template_type.sql
  - migrations/sqlite/20260820000001_seed_zahlungserinnerung_template.sql
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 30: Code Review Report

**Reviewed:** 2026-08-20
**Depth:** standard
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Reviewt wurden die Phase-30-Änderungen (diff-Basis `5698d12`): der kanonische
Euro-Formatter `format_eur_de`, die additive `template_type`-Spalte samt
Migration + Seed, der DSGVO-getrennte Antragsteller-Template-Kontext
(`application_to_template_context`) und die additive
Antragsteller-Template-Validierung an create/update.

Gesamteindruck: solide umgesetzt. Die vier Fokusbereiche halten der Prüfung
weitgehend stand:

- **Euro-Formatter:** Runden (cent-exakt, kein Verlust), Negativ-Behandlung und
  die `i64::MIN`-Overflow-Normierung über `i128` sind korrekt und gut getestet.
- **DSGVO-Trennung:** `application_to_template_context` exponiert
  ausschließlich Antragsteller-Felder plus die *eigenen* Zahlungsdaten der
  Genossenschaft; member-only-Keys fehlen und lassen Strict-Render sauber
  fehlschlagen statt Mitgliederdaten zu leaken. Der Dummy-Kontext für die
  Author-Zeit-Validierung deckt exakt dieselbe Schlüsselmenge ab. Kein Leak
  gefunden.
- **Migration:** `NOT NULL DEFAULT 'member'` + `INSERT OR IGNORE` sind
  abwärtskompatibel und (für den Seed) idempotent.
- **SQL:** durchgehend parametrisiert (`bind`), keine Injection; UPDATE schreibt
  `template_type` bewusst nicht (Immutabilität).

Die gefundenen Punkte betreffen Robustheit/Eingabevalidierung, keine akuten
Sicherheits- oder Datenverlust-Defekte.

## Warnings

### WR-01: `template_type` wird bei create nicht gegen eine Allowlist validiert

**File:** `genossi_mail/src/mail_template_service.rs:80-101`
**Issue:** `create()` verzweigt nur auf `template_type == "application"` (Strict-
Validierung) und persistiert andernfalls jeden beliebigen String unverändert.
Werte wie `"garbage"`, `""` oder ein via JSON gesetztes `"Member"`
(Groß-/Kleinschreibung) werden akzeptiert. Solche Zeilen sind weder
Antragsteller-validiert noch (im Phase-32-Selektor `template_type = 'member'`)
im Mitglieder-Pool sichtbar — die Vorlage verschwindet still aus beiden Pools.
Das untergräbt genau die Pool-Trennung, die die Spalte einführen soll.
`default_template_type()` schützt nur den Fall des *fehlenden* Feldes, nicht den
eines *falschen* Feldwertes.
**Fix:** Diskriminator gegen eine Allowlist prüfen, bevor persistiert wird:
```rust
if template_type != "member" && template_type != "application" {
    return Err(MailTemplateError::BadRequest(Arc::from(format!(
        "Invalid template_type '{template_type}' (expected 'member' or 'application')"
    ))));
}
```

### WR-02: Ungeschützte i64-Multiplikation vor `format_eur_de` (Overflow)

**File:** `genossi_mail/src/template.rs:88` (auch `genossi_service_impl/src/application.rs:99`)
**Issue:** `share_value_cents * app.shares as i64` wird unchecked ausgeführt. Der
Euro-Formatter selbst dokumentiert und garantiert Panic-Freiheit bei
pathologischer Eingabe (bis `i64::MIN`) — dieser Anspruch wird aber schon
*vor* dem Aufruf durch die Multiplikation gebrochen: in Debug-Builds
panict `*` bei Overflow, in Release-Builds wrappt es still zu einem falschen
Euro-Betrag (in `open_amount` einer echten Zahlungserinnerung an einen
Antragsteller). Realistische Werte überlaufen nicht (deshalb Warning, nicht
Blocker), aber die Overflow-Härtung des Moduls ist inkonsistent, wenn die
einzige Multiplikations-Call-Site sie umgeht.
**Fix:** `checked_mul` verwenden und den Overflow-Fall neutral behandeln:
```rust
let total_cents = share_value_cents
    .checked_mul(app.shares as i64)
    .unwrap_or(i64::MAX); // oder: Fehler/Log statt wrap
let open_amount = genossi_service::euro::format_eur_de(total_cents);
```

### WR-03: `application_id` bleibt bei der Bestätigungs-Mail `None` — Timeline-Carry-over greift nicht für die Submit-Mail

**File:** `genossi_service_impl/src/application.rs:130-137`
**Issue:** `send_confirmation_mail` (aus `submit()`) setzt
`RecipientInput { member_id: None, application_id: None }`. Die Mail geht an
einen Antragsteller (zum Submit-Zeitpunkt existiert noch kein Mitglied), wird
aber ohne `application_id` gespeichert. Damit erfasst der Phase-29-Carry-over
(`link_application_recipients_to_member`, `WHERE application_id = ?`) diese Mail
bei einer späteren `confirm()` nie — die Beitritts-Bestätigungsmail taucht in
der Mitglieder-Timeline nicht auf. Der Inline-Kommentar begründet dies mit
"real Application-Send kommt in Phase 31", der Recipient trägt aber faktisch eine
Antragsteller-Adresse. Grenzfall zwischen bewusster Vertagung und Defekt; da
`app.id` an dieser Stelle verfügbar ist, wäre `application_id: Some(app.id)` die
konsistente Wahl.
**Fix:** Prüfen, ob die Submit-Bestätigungsmail den Antragsteller-Namespace
tragen soll:
```rust
let recipient = genossi_mail::service::RecipientInput {
    address: email,
    member_id: None,
    application_id: Some(app.id),
};
```
Falls die Vertagung auf Phase 31 gewollt ist: als expliziten TODO mit
Requirement-Verweis kennzeichnen, damit die Carry-over-Lücke nicht unbemerkt
bleibt.

## Info

### IN-01: Fremd-Pool-Templates im Member-Selektor bis Phase 32 sichtbar (by design)

**File:** `migrations/sqlite/20260820000000_mail_templates_add_template_type.sql:10-12`
**Issue:** Der Mitglieder-Selektor-Filter (`template_type = 'member'`) ist laut
Migration bewusst auf Phase 32 vertagt. Bis dahin kann ein Vorstand im
Member-Bulk-Send weiterhin eine `'application'`-Vorlage auswählen. Kein
Datenleck: der Member-Kontext definiert `open_amount`/`shares`/`bank_iban` nicht,
also schlägt der Strict-Render fehl (kein Versand mit Fehldaten) — aber es ist
eine UX-Falle bis der Filter greift.
**Fix:** Kein Code-Change nötig; sicherstellen, dass Phase 32 den Selektor-Filter
tatsächlich liefert (Requirement-Tracking).

### IN-02: `format_eur_de` fügt jetzt Tausenderpunkte hinzu — Formatänderung in der Bestätigungsmail

**File:** `genossi_service_impl/src/application.rs:100`
**Issue:** Der ersetzte Inline-Code (`format!("{},{:02} €", …)`) gab keine
Tausendertrennzeichen aus; `format_eur_de` tut das (`1234,56 €` → `1.234,56 €`).
Das ist beabsichtigt und dokumentiert, aber eine sichtbare Änderung am Wortlaut
bereits produktiver Mails. Nebeneffekt-positiv: die alte Inline-Variante hatte
bei negativen Beträgen einen Vorzeichen-Bug (`euros == 0` verlor das Minus), der
mit `format_eur_de` behoben ist.
**Fix:** Keiner — nur zur Kenntnis für Release-Notes/Erwartungssteuerung.

---

_Reviewed: 2026-08-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
