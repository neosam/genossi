# Phase 28: Desktop/Mobile-Vorschau - Context

**Gathered:** 2026-07-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Der bestehende `WysiwygEditor` bekommt einen **3-Modi-Umschalter** (Bearbeiten / Desktop-Vorschau ~640 px / Mobile-Vorschau ~360 px). In den beiden Vorschau-Modi rendert eine neue Component `MailPreviewFrame` das **ammonia-sanitisierte und Jinja-gerenderte** HTML in einem `sandbox="allow-same-origin"`-`<iframe>` fester Breite, mit aus `data-genossi-asset-id` re-injizierten Bild-URLs.

**In scope:**
- Backend: `POST /api/mail/preview` sanitisiert `body_html` (heute NICHT — `genossi_mail/src/rest.rs:769` sagt explizit „no sanitization here"). Reihenfolge: **sanitize → render**, siehe D-02.
- Frontend: neue Component `genossi-frontend/src/component/mail_compose/mail_preview_frame.rs` (iframe + Device-Rahmen + Label).
- Frontend: Modus-State + Umschalt-UI in `wysiwyg_editor.rs`; Toolbar im Preview-Modus ausgeblendet.
- Frontend: `src`-Re-Injektion aus `data-genossi-asset-id` (Spiegel von `image_insert_html()`).
- Frontend: Mail-Baseline-Stylesheet als Konstante, in den `srcdoc`-Kopf injiziert.
- Member-Durchreichung an den Editor, damit Template-Variablen in der Vorschau aufgelöst werden (D-03).
- i18n-Keys in `de.rs` UND `en.rs`.
- Grep-Gate-Test auf das `sandbox`-Attribut (Sicherheits-Invariante).

**Out of scope (bewusst):**
- KEINE Änderung an `sanitize.rs`-Regeln — der Sanitizer wird nur *aufgerufen*, nicht umgebaut.
- KEIN Ersetzen der bestehenden `TemplatePreview` (D-11).
- KEIN Dark-Mode-Preview, keine Outlook-Quirk-Simulation, kein Tablet-Breakpoint, kein Screenshot-Export.
- KEINE konfigurierbaren Breakpoints — 640/360 sind Code-Konstanten (D-09).
- KEIN Diff-Banner „Sanitizer hat N Elemente entfernt" (D-04).

</domain>

<decisions>
## Implementation Decisions

### Sanitize-Quelle & Variablen-Rendering (PREV-02)

**D-01 (A1 → aufgelöst auf Option a): `/api/mail/preview` wird um `sanitize_html` erweitert — KEIN neuer Endpoint.**

Der User hat A2 = **(b)** gewählt (Variablen gerendert, nicht roh). Meine ursprüngliche A1-Empfehlung war ein schlanker `POST /api/mail/sanitize`-Endpoint — der kann aber kein Jinja mit Member-Kontext rendern und ist damit mit A2 = (b) unvereinbar. Die Kopplung war in der Fragestellung angekündigt („Wenn gerendert, geht nur (a)/(c)").

**Aufgelöst auf (a):** `preview_mail` (`genossi_mail/src/rest.rs:659`) rendert bereits Jinja gegen den vollen Member-/Repayment-Kontext. Es fehlt nur der Sanitize-Schritt. Damit:
- kein neuer Endpoint, keine neue Request/Response-Form,
- die bestehende `TemplatePreview` zeigt als Nebeneffekt endlich das, was wirklich gespeichert wird (heute zeigt sie un-sanitisiertes HTML — das ist gegenüber dem Geist von PREV-02 ein Defekt).

**D-02: Reihenfolge ist `sanitize_html(body_html)` ZUERST, dann `render_html_template(…, ctx)`.**

Das spiegelt exakt die Produktion: ammonia greift am Store-Boundary (Phase 23 D-03), das Jinja-Rendering passiert erst beim Versand im Send-Worker. Render-dann-sanitize wäre asymmetrisch — Member-Werte werden in Produktion *autoescaped*, nicht *sanitisiert*.

**Unbedenklich, weil dokumentiert:** Jinja-Platzhalter in **Text-Content** (`<p>Hallo {{ first_name }}</p>`) überleben ammonia intakt (`genossi_mail/src/sanitize.rs:30-34`, RESEARCH Pitfall 1). Platzhalter in **Attributen** (`<a href="{{ link }}">`) sind bereits seit Phase 24 explizit *out of contract* — ammonia strippt sie. Die Vorschau macht dieses vorbestehende Verhalten damit erstmals sichtbar, statt es zu verstecken. Das ist erwünscht.

**D-03 (A2 = b): Die Device-Vorschau rendert Template-Variablen gegen ein Beispiel-Mitglied — nicht roh.**

Konsequenz: `WysiwygEditor` braucht Zugriff auf eine `member_id`. Der Editor kennt heute nur `value: String` und `on_change` (`wysiwyg_editor.rs:39`).

**Entscheidung — Member-Auswahl wird in die Pages hochgezogen:** `preview_member_id` lebt heute als privates `use_signal` **innerhalb** `TemplatePreview` (`template_preview.rs`). Es wird in die drei Call-Sites hochgezogen und an **beide** Components gereicht (`WysiwygEditor` + `TemplatePreview`).

*Warum nicht der minimalinvasive Weg (Editor wählt selbst den ersten Member)?* Weil sonst zwei konkurrierende Member-Auswahlen auf derselben Seite stehen und der Vorstand nicht erkennt, welche gilt.

**Fallback-Verhalten:** Ist kein Member gewählt (`None`), zeigt der Preview-Modus **nicht** den iframe, sondern eine Hinweiszeile („Mitglied für die Vorschau wählen"). Kein Request, kein leerer Rahmen.

**Planner-Ausstiegsklausel:** Falls sich der Hochzieh-Refactor an einer der drei Call-Sites als unverhältnismäßig erweist (Verdacht: `reply_form.rs` hat ohnehin nur genau einen Member), darf dort stattdessen der einzige/erste Member implizit genutzt werden. Die Entscheidung gilt call-site-weise, nicht global.

**Request-Form:** Der Editor ruft `/api/mail/preview` mit `subject: ""`, `body: ""`, `body_html: <editor-innerHTML>`, `member_id: <gewählt>`, `repayment_phase_id: <durchgereicht>`. `subject`/`body` sind Pflichtfelder der `PreviewRequest` (`rest.rs:258`), leere Strings sind zulässig — `rendered_body` wird bei gesetztem `body_html` ohnehin via `plain_from_html` überschrieben (`rest.rs:785`).

**D-04 (A3 = a): Kein Diff-Banner.** Wenn ammonia etwas entfernt, zeigt die Vorschau schlicht das Ergebnis. PREV-02 verlangt „Diskrepanzen werden sichtbar" — das leistet die Darstellung selbst. Ein Element-Diff ist eigene Komplexität.

**D-05 (A4 = a): Sanitize+Render laufen nur beim Wechsel in einen Preview-Modus.** Ein Request pro Umschaltung, kein Debounce-Live-Rendering. Im Preview-Modus wird ohnehin nicht getippt.

### Bilder & iframe-Sandbox (PREV-03, PREV-05)

**D-06 (B1 = a): Das Frontend injiziert `src` — nicht das Backend.**

Rewrite `data-genossi-asset-id="X"` → zusätzliches `src="{config.backend}/api/mail/assets/X/bytes"`. Spiegelt exakt das bereits existierende `image_insert_html()` (`wysiwyg_toolbar.rs:44`). Das Backend müsste sonst seine öffentliche URL kennen — in Dev laufen Frontend (:8080) und Backend (:3000) getrennt.

**Hinweis an den Planner:** `image_insert_html()` und die Preview-Injektion sind dieselbe Regel an zwei Stellen. Wenn eine gemeinsame Helper-Funktion sinnvoll ist, extrahieren (Component-First-Geist); sonst Grep-Gate.

**D-07 (B2 = b): `sandbox="allow-same-origin"` — ohne `allow-scripts`.**

- `sandbox=""` (maximal restriktiv) wäre funktional kaputt: opaque Origin ⇒ Session-Cookie geht bei SameSite=Lax nicht mit ⇒ 401 auf `/bytes` ⇒ alle Bilder tot.
- `allow-same-origin` **ohne** `allow-scripts` ist sicher. Die gefährliche Kombination ist beides zusammen (das Dokument könnte sich dann selbst aus der Sandbox nehmen). Scripts werden zusätzlich schon von ammonia gestrippt — zwei unabhängige Schichten.
- `allow-popups` wurde bewusst NICHT gewählt: Links in der Vorschau sind nicht klickbar. Das unterstützt PREV-04 („Klicks tun nichts").

**D-08 (B3 = a): Bricht ein Bild (404/401), bleibt es beim Browser-Default** (kaputtes Bild-Icon). Kein eigener Placeholder.

### iframe-Befüllung & CSS (PREV-05)

**D-09 (C1 = a): Befüllung via `srcdoc`.** Deklarativ in RSX setzbar, kein web-sys-Handstand, keine Timing-Fallen mit Remount/Signal-Lag (bekannter Pitfall aus `24-RESEARCH.md`). Mit `allow-same-origin` erbt `srcdoc` die Parent-Origin — genau das, was D-07 braucht. Der HTML-Inhalt muss fürs Attribut escaped werden.

**D-10 (C2 = c): Eigenes kleines „Mail-Client-Baseline"-Stylesheet als Frontend-Konstante**, in den `srcdoc`-`<head>` injiziert. Richtwert: Arial/Helvetica sans-serif, ~14 px, `img { max-width: 100% }`.

**Warum ausdrücklich NICHT `.mail-html-render` duplizieren:** Dann sähe die Vorschau exakt wie der Editor aus — und der Sinn der Phase (Diskrepanzen zwischen Editor-DOM und Empfänger-Sicht sichtbar machen, PREV-02) wäre unterlaufen. Nackte Browser-Defaults wären andererseits Times New Roman 16 px, was kein realer Mail-Client so zeigt.

**D-11 (C3 = a): Grep-Gate-Test auf die `sandbox`-Invariante.** `include_str!`-Muster analog `wysiwyg_editor.rs:392` und `template_preview.rs:236` — der Test nagelt fest, dass die Preview-Component ein `sandbox`-Attribut setzt und `allow-scripts` NICHT enthält. Das ist eine Sicherheits-Invariante, kein Stil-Check.

**D-12 (C4 = a): 640 px / 360 px sind zwei Code-Konstanten.** Keine Settings-Konfiguration (wäre Scope-Creep; Roadmap sagt „~640 px / ~360 px").

### UI-Integration (PREV-01, PREV-04)

**D-13 (D1 = a): Der Modus-Umschalter lebt in `WysiwygEditor` selbst.** Damit wirkt er automatisch in allen drei Call-Sites (`mail_page.rs`, `mail_templates.rs`, `reply_form.rs`) ohne Verdrahtung pro Page. Der iframe selbst wird eine eigene Component `MailPreviewFrame` — Component-First auf beiden Ebenen.

**D-14 (D2 = a): Im Preview-Modus wird die Toolbar ausgeblendet** (nicht ausgegraut). Klarstes Signal für PREV-04.

**D-15 (D3 = a): Schlichter Device-Rahmen.** Rahmen + Label „Desktop-Vorschau (640 px)" / „Mobile-Vorschau (360 px)" über dem iframe, iframe zentriert auf grauem Backdrop. Kein stilisiertes Phone-Mockup mit Notch — reine Kosmetik.

**D-16 (D4 = a): Die bestehende `TemplatePreview` bleibt unverändert bestehen.** Zwei verschiedene Zwecke:
- `TemplatePreview` = member-aufgelöste **Variablen**-Prüfung (Plain + HTML nebeneinander),
- neuer Editor-Modus = **Layout/Device**-Prüfung.

Beide profitieren von D-01 (dem gemeinsamen Sanitize im Preview-Endpoint). Ein Ersetzen hätte drei Call-Sites umgebaut, ohne PREV-01..05 besser zu erfüllen.

**D-17 (D5 = a): Beim Umschalten bleibt das `contenteditable` im DOM und wird nur per CSS versteckt.** Kein Unmount, kein Remount, kein Re-Seeding. Der Seed-Lag beim Remount ist ein dokumentierter Pitfall aus Phase 24 (`24-RESEARCH.md`) und wird so vollständig umgangen. Wichtig: `EDITOR_ID` ist eine Konstante (`wysiwyg_editor.rs:36`) — ein zweiter Editor-Knoten im DOM würde die `get_element_by_id`-Lookups der Toolbar brechen.

### Claude's Discretion

- Exakte Werte des Baseline-Stylesheets (D-10) — Schriftgröße, Zeilenhöhe, Margins.
- Ob `image_insert_html()` und die Preview-`src`-Injektion (D-06) zu einer gemeinsamen Helper-Funktion extrahiert werden oder als zwei Stellen mit Grep-Gate bestehen bleiben.
- Konkrete Umschalt-UI: Segmented-Control vs. drei Buttons vs. Tabs.
- Call-site-weise Anwendung der Ausstiegsklausel aus D-03 (insbesondere `reply_form.rs`).
- Escaping-Strategie für den `srcdoc`-Attributwert.
- Ob der Sanitize-Aufruf in D-01 unconditional läuft oder nur bei gesetztem `body_html` (funktional äquivalent, da `body_html: None` ⇒ kein HTML-Pfad).
- Genaue i18n-Key-Namen (Konvention: `MailEditorMode*`).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — PREV-01..PREV-05 (Zeilen 32-36), Phase-28-Zuordnung (Zeilen 86-90)
- `.planning/ROADMAP.md` §„Phase 28: Desktop/Mobile-Vorschau" — Goal, 5 Success Criteria, Depends-on Phase 27
- `.planning/STATE.md` — v1.5-Kontext, Phase-27-Abschluss, deferred UAT-Restposten

### Backend — was geändert wird
- `genossi_mail/src/rest.rs:659-805` — `preview_mail`-Handler. **Ziel von D-01/D-02.** Zeile 769 trägt heute den Kommentar „Read-only preview — no sanitization here"; der wird durch D-02 obsolet und muss mit ersetzt werden.
- `genossi_mail/src/rest.rs:258-281` — `PreviewRequest`-Struct (`subject`/`body` Pflicht, `body_html`/`repayment_phase_id` optional). Wird NICHT geändert.
- `genossi_mail/src/sanitize.rs:1-70` — `sanitize_html` + Builder-Policy. **Wird nur aufgerufen, nicht geändert.** Zeilen 30-34 dokumentieren den Jinja-Contract (Text-Content überlebt, Attribute nicht) — Grundlage von D-02.
- `genossi_mail/src/render.rs:280-345` — `rewrite_img_cids` / Asset-ID-Extraktion. **Nicht Teil dieser Phase**, aber die Vorlage dafür, wie `data-genossi-asset-id` gelesen wird (D-06 macht das Frontend-Pendant).

### Frontend — was geändert/erstellt wird
- `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` — `WysiwygEditor`-Signatur (Zeile 39), `EDITOR_ID`-Konstante (36), `mail-html-render`-Klasse (73), Grep-Gate-Muster (392-412). **Ziel von D-13/D-14/D-17.**
- `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs:33-50` — `image_insert_html()`. **Vorlage für D-06.**
- `genossi-frontend/src/component/mail_compose/template_preview.rs` — `TemplatePreview` (bleibt, D-16); `preview_member_id`-Signal wird hochgezogen (D-03); `dangerous_inner_html`-Block Zeile 193; Grep-Gate-Muster ab Zeile 236.
- `genossi-frontend/src/component/mail_compose/mod.rs` — Re-Export-Stelle für die neue `MailPreviewFrame`.
- `genossi-frontend/input.css:10-31` — `.mail-html-render`-Regeln. **Bewusst NICHT dupliziert** (D-10) — hier nachlesen, wogegen abgegrenzt wird.
- `genossi-frontend/src/api.rs` — `preview_mail`-Client + `PreviewResponse`.

### Call-Sites (D-03-Refactor + D-13-Wirkung)
- `genossi-frontend/src/page/mail_page.rs:401ff` — Bulk-Mail, viele Member
- `genossi-frontend/src/page/mail_templates.rs:333ff` — Template-Editor, via `TemplateTester`
- `genossi-frontend/src/component/inbox/reply_form.rs:239ff` — Reply, vermutlich genau ein Member (Ausstiegsklausel D-03)

### Vorbild aus früheren Phasen
- `.planning/milestones/v1.4-phases/24-wysiwyg-frontend-editor/24-RESEARCH.md` — Pitfalls: styleWithCSS-Persistenz, Selection-Range-Verlust, **Signal-Sync-Lag/Remount** (Grundlage von D-09 und D-17)
- `.planning/phases/26-editor-formatierung-vervollstaendigen/26-CONTEXT.md` — D-02 dort etabliert das `include_str!`-Grep-Gate-Muster (Grundlage von D-11); D-06 dort regelt UAT als Ship-Gate
- `.planning/phases/27-*/` — `mail_asset`-Entität, `/api/mail/assets/{id}/bytes`, ammonia-`<img>`-Härtung

### Projekt-Konventionen
- `CLAUDE.md` (Root) — Layered DAO/Service/REST, Component-First, Audit-Scope (mail_asset: kein Audit)
- `genossi-frontend/CLAUDE.md` — Component-First, i18n zweisprachig (`de.rs` + `en.rs`)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`image_insert_html(backend, id)`** (`wysiwyg_toolbar.rs:44`) — erzeugt exakt die `<img data-genossi-asset-id="X" src="{backend}/api/mail/assets/X/bytes">`-Form, die D-06 in der Vorschau rekonstruieren muss. Direkt wiederverwendbar oder als gemeinsamer Helper extrahierbar.
- **`preview_mail`-Endpoint + `api::preview_mail`-Client** — komplette Jinja-Render-Pipeline inkl. Member-Kontext, Repayment-Merge und Dummy-Fallback existiert bereits. D-01 hängt genau **einen** Funktionsaufruf davor.
- **`include_str!`-Grep-Gate-Muster** — zweimal etabliert (`wysiwyg_editor.rs:392`, `template_preview.rs:236`), inklusive „Self-Reference-Hazard"-Abwehr (Source vor dem Test-Modul abschneiden, Needles zur Laufzeit zusammensetzen). D-11 kopiert das Muster.
- **`.mail-html-render`-Regelblock** (`input.css:10-31`) — als *Referenz* für das Baseline-Stylesheet in D-10, ausdrücklich nicht als Copy-Vorlage.

### Established Patterns
- **Component-First** — `MailPreviewFrame` wird eigene Component, kein Inline-RSX im Editor.
- **i18n zweisprachig** — jeder neue Label-Key in `de.rs` UND `en.rs`; `Locale` hat nur `En`/`De`.
- **`r#type: "button"`** an allen Buttons (Dioxus-Reload-Bug, Memory) — gilt auch für die drei Modus-Buttons.
- **Enum statt Boolean** (Projekt-Regel) — der Modus-State ist ein 3-wertiges Enum (`Edit`/`Desktop`/`Mobile`), kein `preview: bool` + `mobile: bool`.
- **Native Event-Listener statt Dioxus-Handler bei DOM-Randfällen** (Memory `dioxus-drag-drop-native-listeners`) — relevant falls `srcdoc` wider Erwarten Timing-Probleme macht; D-09 wählt bewusst den deklarativen Weg, um das gar nicht erst zu brauchen.
- **jj statt git** — Commits via `jj`.

### Integration Points
- **Sanitize-Einbau:** `genossi_mail/src/rest.rs` — `sanitize_html` ist crate-intern verfügbar, kein neuer Import über Crate-Grenzen.
- **Test-Ebenen:** Sanitize-Unit-Tests `cargo test -p genossi_mail --lib`; Preview-Endpoint-Verhalten `cargo test --test e2e_tests`; Grep-Gate nativ im Frontend-Crate (reiner String-Test, läuft ohne wasm32-Target).
- **Bekannter Pre-existing Failure:** `test_mail_preview_repayment_no_entries_does_not_default_to_one` (Phase 22, in STATE.md dokumentiert) — **keine** Phase-28-Regression. Achtung: liegt im Preview-Pfad, den D-01 anfasst — der Verifier darf ihn nicht neu Phase 28 zuschreiben, muss aber prüfen, dass D-01 ihn nicht *verschlimmert*.
- **UAT-Setup:** Backend `cargo run --features mock_auth --bin genossi` (:3000), Frontend `dx serve` (:8080), Skill `run-rust-backend-and-frontend`. **Warnung:** Dev-DB enthält echte Mitglieder-E-Mails — Send-Button im Smoke-Test NICHT klicken.

</code_context>

<specifics>
## Specific Ideas

- **User-Entscheidung A2 = (b):** Template-Variablen werden in der Device-Vorschau **gerendert**, nicht roh angezeigt. Das ist die einzige Abweichung von den Empfehlungen und hat A1 auf Option (a) gezwungen (D-01) sowie den Member-Durchreich-Bedarf ausgelöst (D-03).
- **Alle übrigen Bereiche:** „rest default" — die markierten Empfehlungen gelten (A1→aufgelöst, A3a, A4a, B1a, B2b, B3a, C1a, C2c, C3a, C4a, D1a, D2a, D3a, D4a, D5a).
- **Diskussionsform:** Der User will alle Fragen auf einmal als Text-Liste, keine `AskUserQuestion`-Popups. Als Memory festgehalten (`discuss-questions-as-text-batch`). Gilt auch für Research/Plan-Phasen — dort möglichst gar nicht rückfragen, sondern Discretion-Punkte selbst entscheiden.

</specifics>

<deferred>
## Deferred Ideas

- **Dark-Mode-Vorschau** — eigene Phase, nicht v1.5.
- **Echte Mail-Client-Simulation** (Outlook-Quirks, Gmail-CSS-Stripping) — eigenes Thema, deutlich größerer Umfang.
- **Tablet-Breakpoint** (~768 px) — wenn die zwei Breiten sich als zu wenig erweisen; heute nicht gefordert.
- **Screenshot-/PDF-Export der Vorschau** — neue Fähigkeit, eigene Phase.
- **Konfigurierbare Breakpoints in den Settings** — bewusst gegen entschieden (D-12), nur bei konkretem Bedarf.
- **Sanitize-Diff-Banner** („N Elemente entfernt") — gegen entschieden (D-04); wieder aufgreifen, falls der Vorstand im UAT nicht versteht, warum etwas fehlt.
- **`TemplatePreview` durch den iframe ersetzen** — gegen entschieden (D-16); sinnvoll erst, wenn beide Previews nachweislich redundant wirken.

### Reviewed Todos (not folded)

Alle fünf Treffer aus `todo.match-phase 28` waren generische Keyword-Matches (Score 0.6, Reasons wie „keywords: phase, genossi") ohne Preview-Bezug:

- `2026-06-27-originalen-mitgliedsantrag-als-datei-attachment-an-applicati.md` — Application-Attachments, nicht Mail-Preview
- `2026-06-28-html-mail-support-statt-nur-textmails.md` — bereits durch v1.4 Phase 23/24 geliefert
- `2026-07-02-mail-datum-deutsches-format.md` — Template-Variablen-Formatierung, eigenes Thema
- `backend-pre-flight-check-attach-repayment-letter.md` — Repayment-Flow
- `frontend-bulk-no-repayment-letter-action.md` — Repayment-Flow

</deferred>

---

*Phase: 28-desktop-mobile-vorschau*
*Context gathered: 2026-07-27*
