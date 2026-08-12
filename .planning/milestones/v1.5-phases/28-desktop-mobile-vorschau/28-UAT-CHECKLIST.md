# Phase 28 UAT Checklist

**Phase:** 28-desktop-mobile-vorschau
**Coverage:** PREV-01, PREV-02, PREV-03, PREV-04, PREV-05
**Companion automated tests:** `preview_body_html_is_sanitized_before_render`, `preview_body_html_img_keeps_asset_id_strips_src` (beide in `genossi_bin/tests/e2e_tests.rs`), `preview_frame_never_allows_scripts`, `srcdoc_is_self_contained_no_external_css`, `inject_asset_src_rejects_quote_injection_payload` (alle in `genossi-frontend/src/component/mail_compose/mail_preview_frame.rs`), `editor_is_hidden_offscreen_not_display_none`, `preview_mode_switch_syncs_dom_before_switching` (beide in `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`), `test_mail_recipient_comes_from_test_address_only` (in `genossi-frontend/src/component/mail_compose/template_tester.rs`)

Diese Checkliste deckt genau die Anteile der Phase 28 ab, die sich **nicht** automatisiert
prüfen lassen: die visuelle Abgrenzung des Device-Rahmens, das tatsächliche Laden der Bilder
im sandboxed iframe samt Cookie-Verhalten, die CSS-Bleed-Gegenprobe in beide Richtungen und
die Darstellung bestehender v1.4-Templates. Alles andere ist durch die oben genannten
Companion-Tests festgenagelt (Frontend: 331 passed / 0 failed; Backend: 314 passed / 2
vorbestehende Fehlschläge aus Phase 22/24).

Analog zur Phase-26-Regelung (dort D-06) ist diese Abnahme das **Ship-Gate vor dem
v1.5-Milestone-Abschluss** (`/gsd-complete-milestone`) und **kein Merge-Gate innerhalb der
Phase 28**. Das Repo arbeitet mit jj-WIP-Changes; klassisches PR-Gating greift hier nicht.
Die Phase gilt als code-fertig, sobald die Pläne 28-01 bis 28-04 grün sind — der
Vorstands-Smoke läuft nachgelagert und muss vor dem Milestone-Archiv abgehakt sein.

## Setup

Folge dem Projekt-Skill `run-rust-backend-and-frontend` — oder manuell:

1. **Backend** — aus dem Repo-Root:
   ```bash
   cargo run --features mock_auth --bin genossi
   ```
   Läuft auf `http://localhost:3000` mit Mock-Authentifizierung (Context = DEVUSER, admin).
   Variante mit Nix und persistenter DB (wie im Skill hinterlegt):
   ```bash
   DATABASE_URL="sqlite:genossi.db?mode=rwc" SQLX_OFFLINE=true nix develop --command cargo run --bin genossi
   ```

2. **Frontend** — aus `genossi-frontend/`:
   ```bash
   npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch &
   dx serve
   ```
   Läuft auf `http://localhost:8080`. `assets/config.json` zeigt auf die Backend-URL.
   Der erste Kompilierlauf dauert 1–3 Minuten.

3. **DevTools** — `http://localhost:8080` öffnen und die Tabs **Console**, **Network** und
   **Elements** bereithalten. Ohne diese drei sind die Schritte 3, 5, 6, 7, 8 und 12 nicht
   durchführbar.

---

> ### ⚠️ DATENSCHUTZ-WARNUNG — vor dem ersten Klick lesen
>
> **Die Dev-Datenbank enthält echte Mitglieder-E-Mail-Adressen.**
>
> In dieser Checkliste wird der **Versand-Button auf der Massenmail-Seite und im
> Reply-Formular NICHT geklickt**. Kein „nur mal kurz schauen, was passiert".
>
> Der **einzige zugelassene Versandpfad** ist der Test-Empfänger im `TemplateTester`
> (Schritt 16). Dieser verwendet ausschließlich eine ausdrücklich selbst eingetippte
> Test-Adresse und niemals die Adresse des gewählten Mitglieds — das gewählte Mitglied
> liefert dort nur die Template-Variablen über seine Id. Diese Regel ist zusätzlich per
> Grep-Gate `test_mail_recipient_comes_from_test_address_only` im Quelltext festgenagelt.

---

## Testdaten-Vorbereitung

- [ ] **V1. Mitglied mit Umlauten.** Über die Mitglieder-Seite ein Mitglied anlegen oder
  auswählen, das im Vor- **oder** Nachnamen einen Umlaut trägt (z. B. „Jürgen Müller" oder
  „Anna Schäfer"). Wird in Schritt 11 gebraucht.

- [ ] **V2. v1.4-Alt-Template ohne Bilder.** Ein bestehendes Mail-Template aus der Zeit vor
  Phase 27 heraussuchen, das ausschließlich Text und Fettung enthält. Existiert keines mehr,
  eines neu anlegen: mehrere Absätze, mindestens ein `<b>`-Bereich, **kein** Bild, **keine**
  Liste, **keine** Überschrift. Wird in Schritt 9 gebraucht.

- [ ] **V3. v1.5-Template mit voller Formatierung.** Ein Template anlegen mit mindestens einer
  Aufzählungs- oder Nummernliste, einer Überschrift (H2 oder H3) und einem per Toolbar-Button
  oder per Drag-and-Drop eingefügten Bild. Wird in den Schritten 7, 8 und 10 gebraucht.

## Verifikationsschritte

Nach jedem erledigten Schritt die Checkbox setzen. Für jeden Fehlschlag festhalten: (a) die
getroffene Requirement-Id, (b) einen DevTools-Beleg (Screenshot des Elements- oder
Network-Panels), (c) den vermuteten Fix-Ort.

- [ ] **1. Drei Modus-Buttons sichtbar und beschriftet [PREV-01].** Massenmail-Compose-Seite
  öffnen. Erwartet: über der Formatier-Toolbar steht eine zusammenhängende Button-Gruppe mit
  drei Schaltflächen („Bearbeiten", „Desktop-Vorschau", „Mobile-Vorschau"). Der aktive Modus
  ist farblich hervorgehoben (blau hinterlegt, weiße Schrift), die beiden anderen sind weiß mit
  grauer Schrift. Die Console zeigt beim Mount keine roten Fehler. **Fehlschlag:** Buttons
  fehlen oder der aktive Modus ist optisch nicht unterscheidbar — Fix-Ort ist der
  Segmented-Control-Block in `wysiwyg_editor.rs`.

- [ ] **2. Umschalten blendet die Toolbar aus [PREV-04, D-14].** Auf „Desktop-Vorschau"
  klicken. Erwartet: die Formatier-Toolbar (B/I/U/S/Listen/Überschriften/Link/Bild)
  verschwindet **vollständig** aus dem Layout — nicht ausgegraut, nicht durchgestrichen,
  nicht klickbar-aber-wirkungslos. Zurück auf „Bearbeiten": die Toolbar ist wieder da, und ein
  Test-Klick auf den Fett-Button wirkt weiterhin auf den markierten Text. **Fehlschlag:** die
  Toolbar bleibt sichtbar — Fix-Ort ist die `is_preview()`-Bedingung um den Toolbar-Block in
  `wysiwyg_editor.rs`.

- [ ] **3. Hinweiszeile ohne gewähltes Mitglied [PREV-02, D-03].** Ohne ein Mitglied
  auszuwählen in „Desktop-Vorschau" schalten. Erwartet: eine lesbare Hinweiszeile („Mitglied
  für die Vorschau wählen"), **kein** leerer Rahmen und **kein** leeres weißes Rechteck. Im
  Network-Tab, gefiltert auf `preview`, wurde **KEIN** Request an `/api/mail/preview`
  abgesetzt. **Fehlschlag:** leerer Rahmen oder ein Request trotz fehlender Auswahl — Fix-Ort
  ist der `preview_member_id.is_none()`-Zweig in `wysiwyg_editor.rs`.

- [ ] **4. Device-Rahmen und Label [PREV-04, D-15].** ⚠️ **HARD-FAIL-GATE.** Ein Mitglied
  auswählen, dann „Desktop-Vorschau". Erwartet: grauer Hintergrund (Backdrop), darauf ein
  zentrierter, deutlich abgesetzter Rahmen mit weißem Inhalt, und darüber ein Label, das die
  Breite in Klammern nennt („Desktop-Vorschau (640 px)"). Auf „Mobile-Vorschau" umschalten:
  der Rahmen wird **sichtbar** schmaler, das Label wechselt auf „Mobile-Vorschau (360 px)".
  **Fehlschlag** bedeutet, dass ein versehentliches Tippen im Vorschau-Modus nicht
  offensichtlich folgenlos wirkt — der Vorstand könnte glauben, er bearbeite noch. Fix-Ort ist
  der Rahmen-/Backdrop-Block in `mail_preview_frame.rs`.

- [ ] **5. Genau ein Request beim Wechsel in die Vorschau [D-05].** Network-Tab öffnen und auf
  `preview` filtern, Liste leeren. Dann: von „Bearbeiten" auf „Desktop-Vorschau" —
  erwartet **genau ein** Request. Von „Desktop-Vorschau" auf „Mobile-Vorschau" — erwartet
  **KEIN** weiterer Request, die Rahmenbreite ändert sich trotzdem sofort und der Inhalt
  flackert nicht. Von „Mobile-Vorschau" zurück auf „Bearbeiten" und wieder auf
  „Mobile-Vorschau" — erwartet **genau ein** weiterer Request. **Fehlschlag:** mehr Requests
  als hier beschrieben — Fix-Ort ist `preview_needs_fetch` bzw. dessen Aufruf in
  `switch_preview_mode`.

- [ ] **6. Vorschau zeigt die sanitisierte Fassung [PREV-02].** ⚠️ **HARD-FAIL-GATE.** Im
  Bearbeiten-Modus Text eingeben. Dann im DevTools-Elements-Panel den
  `<div id="wysiwyg-editor" contenteditable="true">` aufsuchen und in dessen Inhalt ein
  Attribut einschleusen, das die ammonia-Allowlist nicht passiert — etwa per
  „Edit as HTML" ein `<p onclick="alert(1)" style="color:red">Test</p>` einsetzen. Danach in
  die Desktop-Vorschau schalten. Erwartet: die Vorschau zeigt „Test" **ohne** das
  `onclick`-Attribut und **ohne** die Inline-Farbe. Zum Gegenbeweis das iframe-Dokument im
  Elements-Panel aufklappen (`#document` unterhalb des `<iframe>`) und den `<p>`-Tag dort
  prüfen. **Fehlschlag** — das Attribut ist im iframe-Dokument vorhanden — heißt, dass die
  Vorschau das rohe Editor-DOM zeigt und der Phasenzweck verfehlt ist. Fix-Ort ist der
  Sanitize-Schritt in `genossi_mail/src/rest.rs` (`sanitize_body_html_opt` vor
  `render_html_template`).

- [ ] **7. Bilder laden im iframe [PREV-03, Annahme A2].** ⚠️ **HARD-FAIL-GATE.** Das
  v1.5-Template aus V3 in den Editor laden, ein Mitglied wählen, in die Desktop-Vorschau
  schalten. Erwartet: das Bild wird **angezeigt** (kein kaputtes Bild-Symbol). Im Network-Tab
  erscheint ein Request auf die Asset-Bytes-Route (`/api/mail/assets/<uuid>/bytes`) mit
  **Status 200**, und in dessen Request-Headern ist das Session-Cookie (`app_session`)
  enthalten. **Fehlschlag mit Status 401 oder ohne mitgesendetes Cookie** bedeutet, dass die
  Same-Origin-Erlaubnis der Sandbox nicht greift und das `SameSite=Strict`-Cookie unterdrückt
  wird — genau die Annahme A2 aus dem Research, die nur empirisch belegbar ist. Fix-Ort ist
  das Sandbox-Attribut in `mail_preview_frame.rs` (muss `allow-same-origin` enthalten und darf
  `allow-scripts` **niemals** enthalten).

- [ ] **8. Kaputtes Bild bleibt beim Browser-Default [D-08].** Im DevTools-Elements-Panel den
  Wert von `data-genossi-asset-id` im Editor-Inhalt auf eine gültige, aber unbekannte UUID
  ändern (z. B. `00000000-0000-4000-8000-000000000000`), dann die Vorschau neu aufrufen
  (zurück auf „Bearbeiten" und wieder auf „Desktop-Vorschau"). Erwartet: das
  **Standard-Symbol des Browsers** für ein nicht ladbares Bild. **Kein** eigener Platzhalter,
  **kein** roter Fehlerblock, **kein** Absturz oder Leerlaufen der Vorschau; der übrige
  Mail-Inhalt bleibt sichtbar.

- [ ] **9. v1.4-Template ohne Bilder rendert unverändert [Erfolgskriterium 5].** Das
  Alt-Template aus V2 laden und beide Vorschau-Modi durchschalten. Erwartet: Text, Absätze und
  Fettung erscheinen korrekt und lesbar; keine Fehlermeldung, kein leerer Rahmen, keine
  verschluckten Absätze. **Fehlschlag** wäre eine Rückwärtskompatibilitäts-Regression —
  Fix-Ort ist `inject_asset_src` (muss HTML ohne `<img>` byte-identisch durchreichen) bzw.
  `preview_srcdoc`.

- [ ] **10. v1.5-Template mit Listen, Überschriften und Bild [Erfolgskriterium 5].** Das
  Template aus V3 laden, beide Vorschau-Modi durchschalten. Erwartet: Aufzählungszeichen bzw.
  Nummern sind sichtbar und eingerückt; Überschriften sind erkennbar größer als der Fließtext;
  das Bild ist auf die Rahmenbreite begrenzt und läuft **nicht** über den Rahmen hinaus —
  insbesondere im 360-px-Mobile-Modus. **Fehlschlag:** horizontaler Überlauf — Fix-Ort ist die
  `img { max-width:100% }`-Regel im Baseline-Stylesheet (`MAIL_PREVIEW_BASELINE_CSS` in
  `mail_preview_frame.rs`).

- [ ] **11. Umlaute erscheinen korrekt [Annahme A1, Pitfall 8].** Einen Text mit `Grüße` und
  `Mitgliedschaftserklärung` in den Editor schreiben und das Mitglied aus V1 (Umlaut im Namen)
  auswählen; im Text zusätzlich einen Namens-Platzhalter verwenden, damit der Umlaut auch aus
  den Mitgliedsdaten kommt. Erwartet: in **beiden** Vorschau-Modi keine Ersatzzeichen (`�`)
  und keine Mojibake (`GrÃ¼ÃŸe`). **Fehlschlag** — Fix-Ort ist die `<meta charset="utf-8">`-
  Angabe im `<head>` von `preview_srcdoc`.

- [ ] **12. CSS-Bleed-Gegenprobe in beide Richtungen [PREV-05, Erfolgskriterium 4].** ⚠️
  **HARD-FAIL-GATE.** In der Desktop-Vorschau mit sichtbarem Inhalt:
  **Richtung eins (App → Vorschau):** im DevTools-Styles-Panel auf dem äußeren `<body>` der
  App temporär eine Regel `font-family: cursive !important` setzen. Erwartet: die App-Oberfläche
  ändert ihre Schrift sichtbar, der Inhalt **IM Rahmen** bleibt unverändert in Arial/Helvetica.
  **Richtung zwei (Vorschau → App):** im Elements-Panel in das iframe-Dokument hineingehen
  (`#document` unter dem `<iframe>`) und dort auf dessen `body` eine Regel
  `background: magenta !important` setzen. Erwartet: **nur** der Rahmeninhalt wird magenta, die
  App außen bleibt vollständig unverändert. Beide Änderungen danach wieder verwerfen.
  **Fehlschlag** bedeutet, dass die Browser-Isolation selbst unterlaufen wurde (etwa durch ein
  `<link>` oder `@import` im Vorschau-Dokument) — Fix-Ort ist der Dokument-Aufbau in
  `preview_srcdoc` (`mail_preview_frame.rs`).

- [ ] **13. Links in der Vorschau sind nicht klickbar [PREV-04, D-07].** Ein Template mit einem
  Link (`https://example.com`) laden, in die Desktop-Vorschau schalten, auf den Link klicken.
  Erwartet: **nichts** passiert — kein neuer Tab, keine Navigation der Hauptseite, keine
  Konsolenmeldung. **Fehlschlag:** ein Tab öffnet sich — Fix-Ort ist das Sandbox-Attribut in
  `mail_preview_frame.rs` (es darf weder `allow-popups` noch `allow-top-navigation` tragen).

- [ ] **14. Zweimaliges Hin- und Herschalten zeigt aktuellen Inhalt [Annahme A3].** In die
  Desktop-Vorschau schalten, zurück auf „Bearbeiten", den Text **sichtbar** ändern (etwa einen
  Satz anhängen), erneut in die Desktop-Vorschau schalten. Erwartet: die Vorschau zeigt die
  **geänderte** Fassung, nicht die alte. Das belegt Annahme A3 (das Dokument-Attribut wird
  reaktiv gesetzt und der iframe navigiert neu). **Fehlschlag:** die alte Fassung bleibt
  stehen — Fix-Ort ist die `srcdoc`-Bindung in `mail_preview_frame.rs` bzw. die
  `preview_doc`-Signal-Zuweisung in `switch_preview_mode`.

- [ ] **15. Der Editor-Inhalt überlebt einen Mitglieds-Wechsel [Annahme A4].** Im
  Bearbeiten-Modus Text tippen und **ohne** das Template zu wechseln ein anderes Mitglied
  auswählen. Erwartet: der getippte Text steht unverändert im Editor (Re-Render ohne Remount).
  **Zusatzprüfung für Pitfall 6:** danach ein anderes Template auswählen. Erwartet und
  ausdrücklich **KEIN Fehler**: der Modus springt auf „Bearbeiten" zurück und der Editor zeigt
  das neue Template. Das ist bewusst akzeptiertes Verhalten (T-28-18, dokumentiert in
  28-03-SUMMARY.md) und darf **nicht** als Bug gemeldet werden.

- [ ] **16. Zeilenumbrüche überleben den Vorschau-Ausflug [Pitfall 1, PREV-01].** ⚠️
  **HARD-FAIL-GATE.** Auf der Mail-Template-Seite ein Template mit mehreren Absätzen **und**
  einer Liste anlegen. In die Desktop-Vorschau schalten und wieder zurück auf „Bearbeiten".
  Danach im `TemplateTester` ein Mitglied wählen, im Test-Empfänger-Feld eine **eigene
  Test-Adresse** eintippen (nicht die Adresse des Mitglieds — siehe Datenschutz-Warnung) und
  die Test-Mail senden. Die empfangene Mail im Mail-Client als Rohtext ansehen. Erwartet: der
  `text/plain`-Teil trägt die Absatz- und Listenumbrüche. **Fehlschlag mit einer einzigen
  Zeile Fließtext** bedeutet, dass der contenteditable-Container im Vorschau-Modus aus dem
  Rendering genommen wurde und `inner_text()` auf `textContent` zurückgefallen ist; Fix-Ort ist
  `editor_container_style` in `wysiwyg_editor.rs` (muss off-screen positionieren, niemals
  `display:none` oder `visibility:hidden`).

## Nebeneffekt aus D-01 gegenprüfen

Plan 28-01 sanitisiert die Preview-Response im Backend. Dadurch zeigt die **bestehende**
`TemplatePreview` (der Block mit Plain-Text und HTML nebeneinander, nicht der neue
Device-Rahmen) ab jetzt ebenfalls die sanitisierte Fassung statt des ungefilterten
Editor-DOMs. Das ist die Behebung eines vorbestehenden Defekts, kein Kollateralschaden.

- [ ] **N1. TemplatePreview auf unerwartete Unterschiede prüfen.** Ein Template mit
  Copy-Paste-Fremdmarkup, mit einem Attribut-Platzhalter (`<a href="{{ link }}">`) und mit
  einem Bild in der bestehenden `TemplatePreview` betrachten. Bekannte, **erwartete**
  Unterschiede gegenüber der Zeit vor Phase 28: (a) Inline-Styles und nicht-allowlistete Tags
  verschwinden, (b) Jinja-Platzhalter in Attribut-Position verlieren das Attribut (seit
  Phase 24 out-of-contract), (c) `<img>` ohne injizierte `src` bleibt in der `TemplatePreview`
  leer — dort läuft die Frontend-`src`-Injektion bewusst nicht, die gibt es nur im
  Device-Rahmen. Jeden weiteren Unterschied **im Protokollblock notieren**, nicht als Bug
  melden.

## Wiederholungsflächen

Der `WysiwygEditor` ist in allen drei Compose-Flächen dieselbe Component (D-13). Die Schritte
**1 bis 5** gelten daher überall und sind in jeder Fläche zu wiederholen.

| Fläche | Datei | Zu wiederholende Schritte | Besonderheit |
|---|---|---|---|
| Massenmail-Compose | `page/mail_page.rs` | 1–5 (plus 6–16 als Hauptfläche) | Einzige Fläche mit Rückzahlungs-Kontext; Repayment-Platzhalter lösen hier auf |
| Mail-Template-Editor | `page/mail_templates.rs` | 1–5, zusätzlich 16 | Kein Rückzahlungs-Kontext — ein Template mit Repayment-Platzhaltern zeigt hier bewusst den roten Fehler-Block. Genau **eine** Mitglieds-Auswahl (MemberSearch); das Auswahlfeld der `TemplatePreview` zieht dank `value`-Bindung mit |
| Inbox-Reply | `component/inbox/reply_form.rs` | 1–5 | Per D-03-Ausstiegsklausel gibt es dort **keine** Mitgliedsauswahl — das zugeordnete Mitglied gilt implizit. Schritt 3 (Hinweiszeile) ist dort nur bei einer Mail **ohne** zugeordnetes Mitglied auslösbar |

- [ ] **W1. Schritte 1–5 auf der Massenmail-Compose-Seite durchlaufen.**
- [ ] **W2. Schritte 1–5 im Mail-Template-Editor durchlaufen** (dort zusätzlich prüfen, dass
  nur noch **eine** Mitglieds-Auswahl sichtbar ist und beide Vorschauen dasselbe Mitglied
  zeigen).
- [ ] **W3. Schritte 1–5 im Inbox-Reply-Formular durchlaufen** (Schritt 3 an einer Mail ohne
  zugeordnetes Mitglied; Versand-Button dort **nicht** klicken).

## Bekannte Einschränkungen

- **Feste iframe-Höhe (640 px, Pitfall 2).** Der Vorschau-Rahmen hat eine feste Höhe mit
  internem Scrolling; er wächst **nicht** mit dem Inhalt. Das ist für eine *Device*-Vorschau
  die semantisch richtige Simulation (echte Mail-Clients haben auch einen festen Viewport) und
  vermeidet den `allow-scripts`-Zwang. Wird die Höhe im UAT als zu klein empfunden, ist die
  Auto-Höhe ein eigener Quick-Task, kein Phase-28-Fehlschlag — im Protokollblock vermerken.

- **Modus-Rücksprung beim Template-Wechsel (T-28-18, Pitfall 6).** Beim Wechsel des
  bearbeiteten Templates wird der Editor per `key`-Bump remountet und der Vorschau-Modus fällt
  auf „Bearbeiten" zurück. Bewusst akzeptiertes Verhalten, siehe Schritt 15. Die
  Mitglieds-Auswahl überlebt den Remount, weil sie seit Plan 28-04 in der Page liegt.

- **Zwei vorbestehende Backend-Testfehlschläge.** `test_mail_preview_repayment_no_entries_does_not_default_to_one`
  (`errors must be array`) und `preview_body_html_round_trips_to_response`
  (`left: "Hallo **Max**"` / `right: "Hallo Max"`) waren bereits vor Phase 28 rot
  (zuletzt geändert in Phase 24, Commit `cfa3794`). Keine Phase-28-Regression; erfasst in
  `deferred-items.md` Punkt 1 und 2.

- **Attribut-Platzhalter werden gestrippt (D-02, D-04).** `{{ … }}` in Attribut-Position
  (`<a href="{{ link }}">`) überlebt ammonia nicht. Das ist seit Phase 24 out-of-contract; die
  Vorschau macht es erstmals sichtbar. Bewusst ohne Diff-Banner (D-04). Kein UAT-Fehlschlag.

## Regressions-Gegenprobe

Vor dem Abzeichnen die automatisierten Suiten laufen lassen:

```bash
cd genossi-frontend && cargo test
cargo test -p genossi_mail
cargo test -p genossi_bin --test e2e_tests
```

Erwartete Ergebnisse:
- `genossi-frontend`: **331 passed, 0 failed**
- `genossi_mail`: **279 passed, 0 failed**
- `genossi_bin --test e2e_tests`: **314 passed, 2 failed** — exakt die beiden oben genannten
  vorbestehenden Fehlschläge, keine weiteren.

## Protokollblock

- **Ausführende Person:** _______________
- **Datum:** _______________
- **Testdaten-Vorbereitung V1–V3 erledigt:** ☐ Ja  ☐ Nein
- **Alle 16 Verifikationsschritte abgehakt:** ☐ Ja  ☐ Nein — siehe Notizen
- **Wiederholungsflächen W1–W3 durchlaufen:** ☐ Ja  ☐ Nein

**Ergebnis je Hard-Fail-Gate:**

| Gate | Schritt | Requirement | Ergebnis |
|---|---|---|---|
| Device-Rahmen und Label | 4 | PREV-04 | ☐ bestanden  ☐ fehlgeschlagen |
| Vorschau zeigt sanitisierte Fassung | 6 | PREV-02 | ☐ bestanden  ☐ fehlgeschlagen |
| Bilder laden mit Session-Cookie | 7 | PREV-03 (Annahme A2) | ☐ bestanden  ☐ fehlgeschlagen |
| CSS-Bleed-Gegenprobe beide Richtungen | 12 | PREV-05 | ☐ bestanden  ☐ fehlgeschlagen |
| Zeilenumbrüche im text/plain-Teil | 16 | PREV-01 (Pitfall 1) | ☐ bestanden  ☐ fehlgeschlagen |

**Fehlschläge** — für jeden Fehlschlag die drei geforderten Angaben:

| # | Getroffene Requirement-Id | DevTools-Beleg | Vermuteter Fix-Ort |
|---|---|---|---|
|   |                           |                |                    |
|   |                           |                |                    |
|   |                           |                |                    |

**Notizen / im Protokoll vermerkte Beobachtungen** (auch: Unterschiede aus N1, Empfinden zur
festen Rahmenhöhe):

_______________________________________________________________________________

_______________________________________________________________________________

**Einordnung:** Schlägt eines der fünf Hard-Fail-Gates fehl, blockiert das den
v1.5-Milestone-Abschluss (`/gsd-complete-milestone`); ein Ticket mit der betroffenen
PREV-Requirement-Id und dem DevTools-Beleg anlegen. Nicht-kritische Fehlschläge in den übrigen
Schritten dürfen dokumentiert aufgeschoben werden. Kann die Vorstands-Session jetzt nicht
stattfinden, wird die Abnahme ausdrücklich als aufgeschoben protokolliert und in
`.planning/STATE.md` unter „Deferred Verification" mit dem Zustand
`verification_deferred_human` eingetragen — zusammen mit den bereits offenen Posten aus
Phase 24 und Phase 26. Ein stillschweigendes Überspringen ist nicht zulässig.
