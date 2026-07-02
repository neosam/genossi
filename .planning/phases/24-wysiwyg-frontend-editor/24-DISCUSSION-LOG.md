# Phase 24: WYSIWYG Frontend Editor - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-02
**Phase:** 24-wysiwyg-frontend-editor
**Areas discussed:** Dual-Body-Auflösung, Migrations-Umfang, Preview-UX, Toolbar & Paste

---

## Dual-Body-Auflösung (der Kern-Konflikt: HTML-02 vs. EDIT-01)

| Option | Description | Selected |
|--------|-------------|----------|
| (a) Editor liefert HTML + Plain-Text wird beim Submit aus DOM (`innerText`/`textContent`) extrahiert | Pragmatisch, einzige Eingabe, aber DOM-„Ableitung" | ✓ |
| (b) Zwei Felder: WYSIWYG für `body_html` + alte Textarea bleibt für Plain-`body` | Treu zu HTML-02, aber doppelte Tipperei | |
| (c) Editor HTML-only; Plain-Text separat getrackt und mitgesendet | Variante von (a) | |

**User's choice:** (a) — „Das soll extrahiert werden."
**Notes:** Revidiert die Phase-23-Annahme (Plain-`body` separat vom Autor verfasst). Der Plain-Teil kommt im Compose-Flow jetzt aus dem Editor-`innerText`. Konsequenz für Phase 23s Planner in CONTEXT.md (D-02) festgehalten: kein separates Plain-Text-Feld bauen.

---

## Migrations-Umfang

| Option | Description | Selected |
|--------|-------------|----------|
| Alle 3 `MailBodyEditor`-Verwender migrieren | Massenmail-Compose + Inbox-Reply + Template-Tester; eine geteilte Component | ✓ |
| Nur der primäre Massenmail-Compose-Flow (EDIT-01 „Mail-Compose-Flow") zuerst | Schmalerer Rollout | |

**User's choice:** Alle `MailBodyEditor` migrieren.
**Notes:** Component-First — es ist EINE geteilte Component, Austausch zieht alle 3 Stellen mit.

---

## Preview-UX (EDIT-05)

| Option | Description | Selected |
|--------|-------------|----------|
| Member-Variablen-Substitution als Vorschau-Mehrwert (Backend-Render, HTML) | contenteditable ist schon WYSIWYG; Preview substituiert Variablen wie heutige `TemplatePreview` | ✓ |
| Rein clientseitiges Read-only-HTML-Rendering ohne Backend | Kein Variablen-Kontext | |
| Getrenntes Panel vs. Umschalt-Button; neue vs. bestehende Component | UX-Detail | |

**User's choice:** „Es ist schon WYSIWYG. Vorschau substituiert die Variablen."
**Notes:** Braucht Backend-HTML-Render (Phase-23-Seam: `preview_mail` muss gerenderte HTML-Variante liefern). Reuse/Erweiterung von `TemplatePreview` bevorzugt.

---

## Toolbar & Paste (EDIT-02, EDIT-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Toolbar-Umfang: nur EDIT-01-Minimum (fett/kursiv/Links/Listen) | Minimal | |
| Toolbar-Umfang: sämtliche gängigen Features (+ Überschriften etc.) | Breiter Funktionsumfang | ✓ |
| Link-UX: nativer `prompt()`-Dialog | Einfach, aber Blocking-Risiko | |
| Link-UX: separater Dialog | Eigener Dialog für URL-Eingabe | ✓ |
| Paste: nur Plain-Text | Simpel, sauber | ✓ |
| Paste: Whitelist-Struktur erhalten (Word-Formatierung übernehmen) | Komfortabler, komplexer | |

**User's choice:** „soll sämtliche gängige Features haben. Links mit separatem Dialog. Paste soll plain text sein."
**Notes:** Exakte Button-Liste beim Planen gegen die ammonia-Default-Whitelist verifizieren (nur Tags nehmen, die die Sanitization überleben). `styleWithCSS=false` erzwingen. Link-Dialog ggf. als In-App-`modal.rs` statt nativem `prompt()` (Dioxus-Reload/Blocking-Vorsicht).

## Claude's Discretion

- Component-Aufteilung/Benennung (Editor + Toolbar + ggf. dünner JS-Interop-Layer).
- `execCommand` direkt über web-sys/`Reflect` (Muster `js.rs`) vs. dünner `extern "C"`-JS-Layer (Muster `codemirror-bundle.js`) — beides ohne neue npm-Dependency.
- Finale Toolbar-Button-Liste (innerhalb ammonia-Whitelist-Constraint).
- Ob HTML-Vorschau `TemplatePreview` erweitert oder parallel-Render-Zweig.

## Deferred Ideas

- HTML-Mail-Bilder / Briefkopf / Logo / Inline-CSS-Branding — Future-Deferral.
- Backend `body_html`-Wire + ammonia-Gate — Phase 23 (harte Vorbedingung).
- Backend-HTML-Render im `preview_mail`-Endpoint — Phase-23/24-Seam für die Variablen-substituierte Vorschau.
