---
phase: 31-service-rest-versand-versand-guardrails
reviewed: 2026-08-20T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
  - genossi_mail/src/backfill.rs
  - genossi_mail/src/render.rs
  - genossi_mail/src/template.rs
  - genossi_mail/src/worker.rs
  - genossi_rest/src/application.rs
  - genossi_rest_types/src/lib.rs
  - genossi_service_impl/src/application.rs
  - genossi_service/src/application.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 31: Code Review Report

**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Zusammenfassung

Die zentralen Guardrails sind sauber umgesetzt und gut getestet: CR-02 (Permission-First) gilt für `send_mail`/`preview_mail`/`last_sent_at`, der `Offen`-only 409-Guard und der 400-Guard bei fehlender Adresse greifen, der Fixed-Recipient (`application.email`, kein Free-Text) ist per e2e (`test_application_mail_send_ignores_freetext_recipient`) bewiesen, und die `application_id`/`member_id`-Namespace-Trennung ist im Render-Kernel korrekt (Application-Zweig hat Vorrang, Member-Resolver wird nicht konsultiert).

Der schwerwiegendste Fund betrifft die `body_html`-Sanitize-Boundary: die Preview umgeht die ammonia-Sanitisierung, die der Send-Pfad anwendet — damit ist die im Code dokumentierte „byte-identical"-Garantie (D-06) faktisch falsch. Dazu kommen mehrere Robustheits-/Guardrail-Lücken beim tatsächlichen Doppelversand-Schutz und bei der Enqueue-Time-Statusprüfung.

## Critical Issues

### CR-01: Preview ist NICHT byte-identisch zum Versand — `body_html` wird in der Preview nicht sanitisiert

**File:** `genossi_service_impl/src/application.rs:788-839` (preview_mail), `genossi_mail/src/render.rs:161-205` (render_application_content), `genossi_mail/src/service.rs:417-432` (create_job)

**Issue:**
Der Send-Pfad sanitisiert `body_html` an der Store-Boundary: `create_job` ruft `sanitize::sanitize_html` (ammonia `clean()`, entfernt `<script>` etc.) BEVOR der Wert in der DAO landet; der Worker rendert danach das bereits-sanitisierte HTML (`resolve_rendered_content` → `render_application_content`, D-05 „NOT re-sanitize").

Die Preview (`preview_mail`) ruft `render_application_content` aber direkt auf dem ROHEN `draft.body_html` auf — `create_job`/`sanitize_html` läuft hier nie. `render_html_template` macht nur Autoescape der interpolierten Werte, sanitisiert aber das Autor-Markup NICHT.

Folge:
1. Die im Kernel-Doc (`render.rs:150-160`) und im Service-Trait-Doc (`application.rs:207-211`) explizit versprochene Garantie „preview output is byte-identical to what the recipient receives" ist falsch, sobald `body_html` Inhalte enthält, die ammonia strippt. Der Vorstand approved in der Preview etwas anderes, als tatsächlich versendet wird.
2. Die Preview-Response liefert unsanitisiertes Autor-HTML an das Admin-Frontend zurück (`PreviewApplicationMailResponse.body_html`). Wenn das Frontend diese HTML als Raw-Markup rendert (WYSIWYG-Preview), ist das ein reflektierter XSS im Admin-Kontext — trotz Admin-Autorenschaft ein echter Sink, da genau der sanitisierte Pfad hier fehlt.

Es gibt keinen e2e-Test, der Preview vs. Send für ein `body_html` mit strip-baren Elementen vergleicht (`test_application_mail_preview_renders_open_amount` nutzt `body_html: None`).

**Fix:**
Die Preview muss dieselbe Sanitize-Boundary durchlaufen wie der Send-Pfad, damit die byte-identical-Zusage hält:
```rust
// preview_mail, vor dem Aufruf von render_application_content:
let sanitized_html = draft
    .body_html
    .as_deref()
    .map(genossi_mail::sanitize::sanitize_html);

let rendered = genossi_mail::render::render_application_content(
    &app,
    &draft.subject,
    &draft.body,
    sanitized_html.as_deref(),
    &cfg,
)?;
```
Alternativ die Sanitisierung in `render_application_content` selbst ziehen (dann aber auch im Worker-Pfad das doppelte Sanitize vermeiden). Zusätzlich einen e2e-Test ergänzen, der ein `<script>`/disallowed-Attribut in `body_html` durch Preview UND Send schickt und identische, gestrippte Ausgabe assertet.

## Warnings

### WR-01: „Anti-double-send guard" ist rein anzeigend — `send_mail` erzwingt nichts server-seitig

**File:** `genossi_service/src/application.rs:219-227`, `genossi_service_impl/src/application.rs:711-786`, `genossi_rest_types/src/lib.rs:1138-1142`

**Issue:**
`last_sent_at` ist im Doc als „server-side anti-double-send guard" (APHIST-02, D-07) bezeichnet, und auch das TO-Feld trägt diesen Namen. Tatsächlich konsultiert `send_mail` `last_sent_at` NIE — es gibt keine server-seitige Deduplizierung, keinen Cooldown und kein Rate-Limit auf der `/{id}/mail`-Route. Der „Guard" ist reine Frontend-Anzeige (GET liefert den Wert). Zwei schnelle Klicks / ein Retry erzeugen zwei Mails. Die Bezeichnung „server-side guard" ist irreführend; falls Enforcement beabsichtigt war, fehlt sie.

**Fix:** Entweder das Doc/Naming auf „display-only advisory" korrigieren, oder in `send_mail` tatsächlich erzwingen (z. B. `last_sent_at` innerhalb desselben Flows prüfen und bei einem Send innerhalb eines Cooldown-Fensters `Conflict` zurückgeben).

### WR-02: `Offen`-only Legal-Basis-Guard greift nur zur Enqueue-Zeit — Worker versendet auch nach Ablehnung

**File:** `genossi_service_impl/src/application.rs:733-740`, `genossi_mail/src/render.rs:243-273`

**Issue:**
Der 409-Guard (`app.status != Offen`) läuft bei `send_mail` zur Enqueue-Zeit. Der Worker-Application-Zweig (`resolve_rendered_content`) lädt die Application per `application_id` und rendert OHNE erneute Statusprüfung. Zwischen Enqueue (Status `Offen`) und Worker-Pickup (Default-Intervall `DEFAULT_SEND_INTERVAL_SECONDS = 36`) kann ein Admin die Application `reject`/`confirm`. Die bereits eingereihte Mail wird trotzdem versendet — die als „DSGVO transactional legal basis boundary" bezeichnete Grenze ist damit umgehbar über ein Race.

**Fix:** Beim Worker-Render im Application-Zweig den aktuellen Status re-prüfen und einen nicht-`Offen`-Recipient via `mark_recipient_failed` überspringen; oder dokumentieren, dass der Guard bewusst nur Enqueue-Time ist und das Race akzeptiert wird.

### WR-03: `get_application` koppelt den Erfolg an `last_sent_at` — Communication-DB-Fehler bricht die Detailansicht

**File:** `genossi_rest/src/application.rs:339-355`

**Issue:**
`get_application` ruft nach `get()` zusätzlich `last_sent_at(id, auth)` und propagiert dessen Fehler (`?`). Ein Fehler in `communication_dao.get_application_communications` (DataAccess → 500) lässt jetzt den kompletten GET der Application-Detailseite fehlschlagen, obwohl der Kern-Datensatz verfügbar wäre. Die Anti-Double-Send-Anzeige ist ein Nice-to-have und sollte den Kern-GET nicht killen. Zusätzlich werden zwei volle Permission-Checks + zwei `find_by_id` für einen GET ausgeführt (redundant).

**Fix:** `last_sent_at`-Fehler best-effort behandeln (loggen, `to.last_sent_at = None`) statt zu propagieren; ggf. Existenz/Permission nur einmal prüfen.

### WR-04: `render_application_content` rendert den Plain-`body` auch dann strikt, wenn er verworfen wird

**File:** `genossi_mail/src/render.rs:180-198`

**Issue:**
`body_rendered = render_template(body, &ctx)?` läuft immer und schlägt unter Strict-Env fehl, wenn `body` einen undefinierten Platzhalter referenziert — selbst wenn `body_html` gesetzt ist und der gerenderte Plain-Text danach via `plain_from_html(html)` abgeleitet und `body_rendered` verworfen wird (Zeilen 195-198). Ein valides `body_html` kann so an einem irrelevanten Fehler im ungenutzten `body`-Template scheitern (Send → `mark_recipient_failed`, Preview → 422). Verhalten ist zwar konsistent mit dem Member-Pfad (`render.rs:375-398`), aber im Application-Kernel unnötig.

**Fix:** Bei `body_html.is_some()` das Plain-`body`-Template nicht rendern (oder nur best-effort), da das Ergebnis ohnehin durch `plain_from_html` ersetzt wird.

## Info

### IN-01: `SendApplicationMailRequest`/`PreviewApplicationMailRequest` ohne `deny_unknown_fields`

**File:** `genossi_rest_types/src/lib.rs:1171-1192`

**Issue:** Unbekannte Felder werden still ignoriert. Hier unkritisch (kein Recipient-Feld existiert, e2e beweist Ignorieren von `to`/`address`/`recipient`), aber ein `#[serde(deny_unknown_fields)]` würde Tippfehler in `template_id`/`body_html` sichtbar machen und die „no free-text recipient"-Invariante explizit machen.

### IN-02: Best-effort Carry-over-Fehler in `confirm` nur geloggt

**File:** `genossi_service_impl/src/application.rs:570-581`

**Issue:** `link_application_recipients_to_member` läuft post-commit, Fehler werden nur geloggt (dokumentiert, D2 Option A). Konsequenz: Antragsteller-Kommunikation kann bei DB-Fehler dauerhaft nicht in die Member-Timeline übernommen werden, ohne Retry-Mechanismus. Bewusste Design-Entscheidung; erwähnt zur Vollständigkeit.

### IN-03: `strip_img_tags`/`rewrite_img_cids` Substring-Parsing auf `<img`

**File:** `genossi_mail/src/render.rs:441-465, 480-529`

**Issue:** `html[i..].starts_with("<img")` matcht auch `<image`/`<imgfoo` als Tag-Start. Da der Input bereits ammonia-sanitisiert ist (nur `<img>` als void-Element mit `data-genossi-asset-id`), praktisch unkritisch — im Application-Preview-Pfad (CR-01) läuft ammonia jedoch NICHT, sodass hier zusätzlich unsaubere Tags durchlaufen könnten. Mit Fix von CR-01 entfällt das Risiko.

---

_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
