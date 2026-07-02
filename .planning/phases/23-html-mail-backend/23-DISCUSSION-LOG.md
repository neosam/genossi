# Phase 23: HTML Mail Backend - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-02
**Phase:** 23-html-mail-backend
**Areas discussed:** Ammonia-Whitelist-Zuschnitt, Per-Recipient-HTML-Persistenz, Sanitization-Timing, FMT-01

---

## Ammonia-Whitelist-Zuschnitt

| Option | Description | Selected |
|--------|-------------|----------|
| Enge Custom-Whitelist | Nur `b, strong, i, em, a, ul, ol, li, p, br`; matcht Phase-24-Editor-Output; keine Tabellen/Bilder | |
| Ammonia permissiver Default | `ammonia::clean()`-Default: erlaubt Fett/Kursiv/Links/Listen/Absätze **+ Überschriften/Tabellen/Bilder**, strippt Scripts/Event-Handler/`javascript:` | ✓ |

**User's choice:** Ammonia permissiver Default — „ammonias permissiver Default ist gut, so machen".
**Notes:** User hinterfragte die enge Liste („Warum so wenig Tags? Keine Tabellen? Keine Bilder?"). Klärung: ammonia = Sanitizer/Sicherheitsnetz, kein Mail-Versender. Tabellen sind in HTML-Mails klassisches Layout-Mittel und sicher durchlassbar. Bilder: externes `<img src>` wird von Clients oft geblockt/als Tracking gewertet; echtes Bild-/Branding-Feature (Upload + CID) ist in REQUIREMENTS.md bewusst deferred. Default-Filter lässt `<img>`-Tag durch, baut aber keine Bild-Funktion. User wählte bewusst mehr Freiheit + weniger Custom-Code.

---

## Per-Recipient-HTML-Persistenz

| Option | Description | Selected |
|--------|-------------|----------|
| Nur `body_html` auf Templates+Jobs, HTML on-the-fly beim Send | Minimales Schema (2 ADD COLUMN), gerendertes HTML nicht persistiert | |
| Zusätzlich `rendered_html_body` pro Empfänger | 3 ADD COLUMN; Worker persistiert gerenderten HTML-Body analog `rendered_body` | ✓ |

**User's choice:** Zusätzlich `rendered_html_body` pro Empfänger — „Doch, wir brauchen wenn das das gerenderte HTML. Wir müssen ja aufbewahren, was verschickt wurde."
**Notes:** Reconstruction-/Audit-Parität zum bestehenden `rendered_body` (Quick 260614). Ergibt 3 forward-only Migrationen (mail_templates.body_html, mail_jobs.body_html, mail_recipients.rendered_html_body).

---

## Sanitization-Timing

| Option | Description | Selected |
|--------|-------------|----------|
| Sanitize-on-store, autoescape-on-render, kein Re-Sanitize | Autor-HTML einmal beim Speichern durch ammonia; Mitgliedswerte per Autoescape beim Render neutralisiert | ✓ |
| Zusätzliches Re-Sanitize des gerenderten Outputs (Defense-in-Depth) | Doppelte Säuberung beim Send | |

**User's choice:** Sanitize-on-store + autoescape-on-render (Claude's Empfehlung übernommen).
**Notes:** User: „Ich versteh nur Bahnhof." → in Klartext erklärt (Vorstand-HTML einmal säubern beim Speichern; Mitgliedsdaten automatisch als reiner Text beim Einsetzen). User muss hier nichts technisch entscheiden — Claude's Discretion. Researcher-Stolperstein notiert: Variablen dürfen nicht in Attributen (`href`) stehen, sonst könnte ammonia den Platzhalter strippen.

---

## FMT-01 (deutsches Datumsformat)

| Option | Description | Selected |
|--------|-------------|----------|
| `format_de`-Helfer, `[day].[month].[year]`, join_date + exit_date | Geteilter Helfer im gemeinsamen Context-Builder, Unit-Test analog test_exit_date_null | ✓ |

**User's choice:** „Klingt gut." — wie in REQUIREMENTS.md/Todo beschrieben.
**Notes:** Einziger date-Fix nötig an template.rs:17-18; automatisch konsistent in Text + HTML durch gemeinsamen Context.

---

## Claude's Discretion

- Genaue Namen/Orte: HTML-Render-Env-Funktion, `format_de`-Helfer-Modul, `build_message`-Signatur-Erweiterung um den optionalen HTML-Teil, geteilter `sanitize_html()`-Helfer vs inline.
- Exakte minijinja-Autoescape-Konfiguration (version-abhängig).
- Sanitization-Timing-Umsetzung (D-05) — User delegierte den Punkt vollständig.

## Deferred Ideas

- HTML-Mail-Bilder / Briefkopf / Logo / Inline-CSS-Branding — REQUIREMENTS.md Future (nicht v1.4).
- WYSIWYG-Frontend-Editor (EDIT-01..05) → Phase 24.
- Antrags-Datei-Upload + Carryover (APDOC-01..05) → Phase 25.
