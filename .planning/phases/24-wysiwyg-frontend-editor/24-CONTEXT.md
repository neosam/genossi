# Phase 24: WYSIWYG Frontend Editor - Context

**Gathered:** 2026-07-02
**Status:** Ready for planning

<domain>
## Phase Boundary

Eine wiederverwendbare Dioxus-`contenteditable`-Component ersetzt die bestehende `MailBodyEditor`-Textarea und lässt einen Vorstand ohne HTML-Kenntnisse formatierte Mails (fett/kursiv/Links/Aufzählungs- und nummerierte Listen sowie weitere gängige Formatierung) verfassen. Der Editor erzeugt semantisches, sanitisierbares HTML (`styleWithCSS=false` → `<b>/<i>` statt Inline-`style`-Spans), das die serverseitige `ammonia`-Sanitization (Phase 23) überlebt. Beim Absenden werden **HTML** (`body_html`) UND der sichtbare **Plain-Text** (`innerText`/`textContent`, als `body`) aus dem DOM gelesen und mit dem Dioxus-State synchronisiert. Eingefügter Inhalt wird beim Paste auf reinen Text reduziert. Eine Live-Vorschau zeigt das gerenderte HTML **mit substituierten Member-Variablen**.

**In scope:** Neue wiederverwendbare WYSIWYG-Dioxus-Component (`contenteditable` + `execCommand` über vorhandenes `web-sys`/`js-sys`, ggf. dünner JS-Interop-Layer analog `js.rs`/`codemirror-bundle.js`); Toolbar mit gängigen Formatierungs-Features; Link-Einfügen über separaten Dialog; Plain-Text-Paste-Cleanup; DOM→State-Sync von `body_html` + extrahiertem Plain-`body` beim Submit; Migration **aller 3** bestehenden `MailBodyEditor`-Verwender (Massenmail-Compose, Inbox-Reply, Template-Tester); HTML-Live-Vorschau mit Member-Variablen-Substitution (erweitert das `TemplatePreview`-Konzept).

**Out of scope (bewusst):** KEINE neuen Frontend-Dependencies (EDIT-02 — kein Quill/TipTap/Editor-Framework); KEINE eingebetteten Bilder / Briefkopf / Logo / Inline-CSS-Branding (Future-Deferral aus Phase 23); Frontend-Sanitization ist ausdrücklich KEINE Sicherheitsgrenze (die liegt bei ammonia serverseitig, Phase 23); KEINE Backend-/Schema-Arbeit (der `body_html`-Wire + die HTML-Preview-Render-Naht gehören zu Phase 23 — siehe Abhängigkeiten unten).

</domain>

<decisions>
## Implementation Decisions

### Dual-Body-Auflösung (EDIT-01, EDIT-03) — Kern-Entscheidung
- **D-01:** Variante **(a)** gewählt. Der WYSIWYG-Editor ist die **einzige** Eingabe im Compose-Flow (die separate Plain-Text-Textarea entfällt). Beim Submit wird aus dem contenteditable-DOM gelesen: (1) das **HTML** → `body_html`, und (2) der **sichtbare Text** (`innerText`/`textContent`) → Plain-`body`. Beide werden mit dem Dioxus-State synchronisiert (kein Datenverlust beim Submit).
- **D-02 (revidiert Phase-23-Annahme):** Phase 23 (HTML-02) formulierte, der Plain-`body` sei „vom Autor separat verfasst, keine Ableitung aus dem HTML". Mit D-01 kommt der Plain-Text im **Compose-Flow** jetzt aus dem Editor-`innerText` — im Geist weiterhin der getippte Autoren-Text (nur ohne Markup), aber mechanisch DOM-extrahiert. **Konsequenz für Phase 23s Planner:** im Compose-UI KEIN separates Plain-Text-Feld bauen; der Plain-Teil für `multipart/alternative` entsteht aus der Editor-Extraktion. `innerText` (nicht `textContent`) bevorzugen, da es Zeilenumbrüche/Listen lesbarer erhält (beim Planen verifizieren).

### Migrations-Umfang (EDIT-01)
- **D-03:** **Alle 3** Verwender des heutigen `MailBodyEditor` werden auf die neue Component umgestellt — es ist EINE geteilte Component (Component-First, `genossi-frontend/CLAUDE.md`): Massenmail-Compose (`page/mail_page.rs`), Inbox-Reply (`component/inbox/reply_form.rs`), Template-Tester (`component/mail_compose/template_tester.rs`, via `body`-Signal). Kein separater „nur der Compose-Flow zuerst"-Rollout.

### Live-Vorschau (EDIT-05)
- **D-04:** Der contenteditable ist selbst bereits live-WYSIWYG (zeigt Formatierung sofort). Der Mehrwert der „Vorschau" ist die **Member-Variablen-Substitution** — analog zur heutigen `TemplatePreview` (die bereits Backend-`preview_mail` aufruft und member-substituierten Text rendert). Für Phase 24 wird diese Vorschau als **gerendertes HTML** dargestellt (nicht mehr als `<pre>`-Text). Reuse/Erweiterung des bestehenden `TemplatePreview`-Musters bevorzugt (Component-First), keine parallele zweite Preview-Component.

### Toolbar & Paste (EDIT-02, EDIT-04)
- **D-05:** Toolbar bekommt **sämtliche gängigen** Formatierungs-Features (mindestens fett, kursiv, Aufzählungs-/nummerierte Listen; darüber hinaus die üblichen wie Überschriften etc.). **Constraint:** die exakte Button-Liste beim Planen gegen die **ammonia-Default-Whitelist** (Phase 23 D-01) verifizieren — nur Features aufnehmen, deren erzeugte Tags die serverseitige Sanitization überleben (z. B. `<u>` nur, falls im ammonia-Default enthalten). `styleWithCSS=false` erzwingen, damit semantische `<b>/<i>`-Tags statt Inline-`style`-Spans entstehen (EDIT-02).
- **D-06:** **Link-Einfügen über separaten Dialog** (URL-Eingabe in einem Dialog, kein Inline-Toolbar-Feld). ⚠️ Native `window.prompt()`-basierte Dialoge sind mit dem Dioxus-Reload-Bug/Blocking-Verhalten vorsichtig zu behandeln — beim Planen prüfen, ob ein In-App-Dialog-Modal (bestehende `modal.rs`-Component) statt eines nativen `prompt()` passender ist.
- **D-07:** **Paste = nur Plain-Text.** Eingefügter Inhalt (z. B. aus Word/Browser) wird beim `paste`-Event auf reinen Text reduziert (kein verschmutztes Markup gelangt in den Body). Da ammonia serverseitig ohnehin saniert, ist das reine UX/Sauberkeit, keine Sicherheitsgrenze.

### Claude's Discretion
- Genaue Aufteilung/Benennung der neuen Component(s) (Editor + Toolbar + ggf. dünner JS-Interop-Layer in `js.rs`).
- Ob `execCommand` direkt über `web-sys`/`js-sys::Reflect` aufgerufen wird (Muster: `js.rs::copy_with_exec_command`) oder ein kleiner `extern "C"`-JS-Layer analog `create_typst_editor`/`codemirror-bundle.js` genutzt wird — **ohne** neue npm-/Frontend-Dependency (EDIT-02).
- Exakte finale Toolbar-Button-Liste (innerhalb D-05-Constraint gegen ammonia-Whitelist).
- Ob die HTML-Vorschau `TemplatePreview` erweitert oder ein paralleler Render-Zweig in derselben Component wird.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & Roadmap
- `.planning/REQUIREMENTS.md` — EDIT-01..05 (Zeilen 31-35), Phase-24-Zuordnung (Zeilen 85-89, 102)
- `.planning/ROADMAP.md` §"Phase 24: WYSIWYG Frontend Editor" — Goal, Success Criteria (EDIT-01..05), Dependency auf Phase 23, harte Ordering-Constraint (ammonia-Gate MUSS vor/mit Phase 24 landen; Frontend-Sanitization ist keine Sicherheitsgrenze)

### Harte Abhängigkeit: Phase 23 (Backend-Wire + Preview-Naht)
- `.planning/phases/23-html-mail-backend/23-CONTEXT.md` — liefert den `body_html`-API-Wire (D-06..D-10 dort), die ammonia-Sanitization (D-01..D-03, permissiver Default), die autoescapende HTML-Render-Env (D-04). **Ohne diesen Wire kann der Editor nichts posten.**
  - **Preview-Naht (neu für Phase 24):** Die member-substituierte HTML-Vorschau (D-04 hier) braucht einen Backend-Render mit autoescapender minijinja-Env. Der heutige `preview_mail`-Endpoint gibt nur Text zurück → er muss eine **gerenderte HTML-Variante** mitliefern. Diese Backend-Erweiterung ist beim Planen von Phase 23/24 als Seam zu berücksichtigen.

### Zu ersetzende / zu migrierende Frontend-Stellen
- `genossi-frontend/src/component/mail_compose/body_editor.rs` — heutige `MailBodyEditor` (`value: String` / `on_change` Textarea), wird durch die WYSIWYG-Component ersetzt
- `genossi-frontend/src/component/mail_compose/mod.rs` — Re-Exports der Mail-Compose-Components
- `genossi-frontend/src/page/mail_page.rs:401` — Verwender 1 (Massenmail-Compose)
- `genossi-frontend/src/component/inbox/reply_form.rs:201` — Verwender 2 (Inbox-Reply)
- `genossi-frontend/src/component/mail_compose/template_tester.rs:45,83-89` — Verwender 3 (Template-Tester, teilt `body`-Signal + nutzt `TemplatePreview`)

### Preview-Vorbild
- `genossi-frontend/src/component/mail_compose/template_preview.rs` — `TemplatePreview` ruft Backend `api::preview_mail` und rendert member-substituierten Text in `<pre>`; wird für HTML-Render erweitert/wiederverwendet
- `genossi-frontend/src/api.rs` — `preview_mail(...)` + `PreviewResponse { subject, body, errors, used_dummy_repayment }` (braucht HTML-Feld für D-04)

### JS-Interop-/web-sys-Vorbilder (kein neues Framework — EDIT-02)
- `genossi-frontend/src/js.rs:100-155` — `copy_with_exec_command`: bestehendes Muster für `execCommand`-Aufruf über `js_sys::Reflect` + `web-sys` (DOM-Zugriff, `create_element`, `dyn_into`)
- `genossi-frontend/src/js.rs:5-22` + `genossi-frontend/assets/codemirror-bundle.js` — Vorbild für dünnen `extern "C"`-JS-Interop-Layer (`create_typst_editor`/`get_editor_content`/`set_editor_content`), falls contenteditable-Handling über einen kleinen `window.*`-Layer sauberer ist (weiterhin ohne npm-Dependency)
- `genossi-frontend/src/component/modal.rs` — bestehende In-App-Modal-Component (Kandidat für den Link-Dialog D-06 statt nativem `prompt()`)

### Projekt-/Frontend-Konventionen
- `genossi-frontend/CLAUDE.md` — Component-First-Prinzip (Pflicht: geteilte Component, keine Inline-RSX-Duplikate); i18n in BEIDEN Locales (`de.rs` + `en.rs`, Keys in `i18n/mod.rs`)
- `.planning/codebase/CONVENTIONS.md`, `.planning/codebase/STRUCTURE.md` — Frontend-Struktur/Muster

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`MailBodyEditor`** (`body_editor.rs`) — die eine Stelle, die ersetzt wird; ihr `value: String` / `on_change: EventHandler<String>`-Contract ist das Minimum, das die neue Component nach außen erfüllen muss (plus HTML/`body_html`-Kanal).
- **`js.rs::copy_with_exec_command`** — fertiges Muster für `execCommand` über `web-sys`/`Reflect`; direkt adaptierbar für `bold`/`italic`/`insertUnorderedList` etc.
- **`codemirror-bundle.js` + `js.rs` `extern "C"`-Bindings** — Vorbild für einen dünnen JS-Interop-Layer (Editor-Init, `getEditorContent`, Debounce-`on_change`), falls contenteditable-State-Sync über `window.*` sauberer wird als reines web-sys.
- **`TemplatePreview`** — bestehende member-substituierende Backend-Preview; für die HTML-Vorschau (D-04) erweitern statt neu bauen (Component-First).
- **`modal.rs`** — In-App-Modal für den Link-Dialog (D-06).

### Established Patterns
- **Component-First** (`genossi-frontend/CLAUDE.md`): eine geteilte Component ersetzt alle 3 Verwender; keine Inline-RSX-Duplikate.
- **web-sys/js-sys-Interop ohne neue npm-Deps** — bereits etabliert (`qr_scanner.rs`, `js.rs`, `helper_login.rs`); EDIT-02 verlangt genau das (keine neue Frontend-Dependency).
- **Dioxus Button-Reload-Bug** (Projekt-Memory): Toolbar-Buttons als `r#type: "button"` + `onclick` mit `prevent_default`, nie form-`onsubmit`, sonst Page-Reload.
- **i18n zweisprachig** — neue Toolbar-/Dialog-Labels in `de.rs` UND `en.rs` + Key in `i18n/mod.rs`.

### Integration Points
- **DOM→State-Sync beim Submit:** der contenteditable-Inhalt (`.innerHTML` → `body_html`, `.innerText` → `body`) muss beim Absende-Zeitpunkt zuverlässig ausgelesen und in die Dioxus-Signale (`body`, neu `body_html`) geschrieben werden (EDIT-03). Naht liegt in den 3 Verwendern (Compose/Reply/Tester).
- **Backend-Post:** der Send-/Job-Create-Pfad muss `body_html` zusätzlich zu `body` mitschicken — hängt am Phase-23-Wire (`api.rs`, `rest-types`).
- **Preview-Backend:** `api::preview_mail` / `PreviewResponse` braucht ein gerendertes-HTML-Feld (Phase-23-Seam) für D-04.

</code_context>

<specifics>
## Specific Ideas

- **User-Entscheidung Dual-Body:** ausdrücklich Variante (a) — „Das soll extrahiert werden." Der sichtbare Text wird beim Submit aus dem DOM extrahiert; kein separates Plain-Text-Feld.
- **User-Entscheidung Toolbar:** „soll sämtliche gängige Features haben" — breiter Funktionsumfang, nicht nur das EDIT-01-Minimum (gebounded durch ammonia-Whitelist, D-05).
- **User-Entscheidung Links:** „mit separatem Dialog" (D-06).
- **User-Entscheidung Paste:** „soll plain text sein" (D-07).
- **User-Entscheidung Vorschau:** „Es ist schon WYSIWYG. Vorschau substituiert die Variablen." → Preview-Mehrwert = Member-Variablen-Substitution (D-04).

</specifics>

<deferred>
## Deferred Ideas

- **HTML-Mail-Bilder / Briefkopf / Logo / Inline-CSS-Branding** — eingebettete Bilder + Branding; Future-Deferral (siehe Phase 23 CONTEXT), keine Bild-Upload-Funktion in diesem Milestone.
- **Backend `body_html`-Wire + ammonia-Gate** — gehört zu **Phase 23** (harte Vorbedingung für Phase 24). Muss vor/mit Phase 24 landen.
- **Backend-HTML-Render im `preview_mail`-Endpoint** — Seam zwischen Phase 23 (Backend) und 24 (Frontend); vom Planner beider Phasen zu berücksichtigen (nicht separate Fähigkeit, sondern Voraussetzung für D-04).

None weiter — Diskussion blieb im Phasen-Scope.

</deferred>

---

*Phase: 24-wysiwyg-frontend-editor*
*Context gathered: 2026-07-02*
