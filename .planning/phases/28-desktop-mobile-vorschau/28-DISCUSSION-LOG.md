# Phase 28: Desktop/Mobile-Vorschau - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-07-27
**Phase:** 28-desktop-mobile-vorschau
**Areas discussed:** Sanitize-Quelle & Variablen, Bilder & iframe-Sandbox, iframe-Befüllung & CSS, UI-Integration

**Form:** Alle Fragen wurden auf Wunsch des Users als **eine Text-Liste** gestellt (kein `AskUserQuestion`). Antwort: „A2 b, rest default".

---

## A — Sanitize-Quelle & Variablen (PREV-02)

### A1 — Woher kommt das sanitisierte HTML?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) `/api/mail/preview` um `sanitize_html` erweitern | Eine Quelle; bestehende `TemplatePreview` profitiert mit | ✓ (aufgelöst) |
| (b) Neuer Endpoint `POST /api/mail/sanitize` | Schlank, kein Member-Kontext — war meine Empfehlung | |
| (c) Beides | (b) für Editor-Modus, (a) als Fix | |

**User's choice:** „rest default" ⇒ formal (b).
**Notes:** **Auflösungskonflikt.** Die Wahl A2 = (b) (Variablen gerendert) macht Option (b) hier funktional unmöglich — ein reiner Sanitize-Endpoint kann kein Jinja gegen Member-Kontext rendern. Die Kopplung war in der Fragestellung vorab angekündigt („Wenn gerendert, geht nur (a)/(c)"). Aufgelöst auf **(a)** als einzige kohärente Wahl; (c) wäre ein überflüssiger zweiter Endpoint. Dokumentiert als D-01 in CONTEXT.md.

### A2 — Template-Variablen gerendert oder roh?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Roh — `{{ first_name }}` sichtbar | Prüft Layout, kein Member nötig — war meine Empfehlung | |
| (b) Gerendert mit Beispiel-Mitglied | Realistischer, braucht Member-Auswahl in 3 Call-Sites | ✓ |
| (c) Roh als Default, gerendert falls Member gewählt | Hybrid | |

**User's choice:** (b) — die einzige bewusste Abweichung von den Empfehlungen.
**Notes:** Löst den Member-Durchreich-Bedarf aus (D-03): `WysiwygEditor` kennt heute keine `member_id`. Entscheidung: `preview_member_id` aus `TemplatePreview` in die Pages hochziehen und an beide Components reichen — sonst stünden zwei konkurrierende Member-Auswahlen auf einer Seite. Ausstiegsklausel für `reply_form.rs` dokumentiert.

### A3 — Diff-Banner bei entfernten Elementen?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Still | Vorschau zeigt das Ergebnis, Vorstand sieht selbst | ✓ |
| (b) Banner „N Elemente entfernt" | Explizit, aber eigene Komplexität | |

**User's choice:** (a) (default)

### A4 — Wann wird sanitisiert?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Nur beim Modus-Wechsel | Ein Request pro Umschaltung | ✓ |
| (b) Live/debounced | Im Preview-Modus wird ohnehin nicht getippt | |

**User's choice:** (a) (default)

---

## B — Bilder & iframe-Sandbox (PREV-03, PREV-05)

### B1 — Wer injiziert `src`?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Frontend | Spiegelt `image_insert_html()`, nutzt `config.backend` | ✓ |
| (b) Backend via `BASE_PATH` | Backend müsste Public-URL kennen; Dev hat getrennte Ports | |

**User's choice:** (a) (default)

### B2 — sandbox-Flags?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) `sandbox=""` | Maximal restriktiv — **funktional kaputt**: opaque Origin, Cookie fehlt, Bilder 401 | |
| (b) `sandbox="allow-same-origin"` | Cookies gehen mit; ohne `allow-scripts` sicher | ✓ |
| (c) + `allow-popups` | Links klickbar in neuem Tab | |

**User's choice:** (b) (default)
**Notes:** Sicherheitsrelevant. Die gefährliche Kombination wäre `allow-same-origin` **plus** `allow-scripts`; die wird ausdrücklich nicht gesetzt. `allow-popups` bewusst weggelassen — nicht-klickbare Links unterstützen PREV-04. Wird per Grep-Gate festgenagelt (C3).

### B3 — Bild lädt nicht?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Browser-Default (kaputtes Icon) | Ausreichend für PREV-03 | ✓ |
| (b) Alt-Text-Placeholder | Expliziter | |

**User's choice:** (a) (default)

---

## C — iframe-Befüllung & CSS (PREV-05)

### C1 — Wie kommt das HTML in den iframe?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) `srcdoc` | Deklarativ in RSX, erbt Parent-Origin, keine Timing-Fallen | ✓ |
| (b) `contentDocument.write()` via web-sys | Volle Kontrolle, aber Remount-/Signal-Lag-Pitfalls | |
| (c) Blob-URL | Eigene Origin — bricht B2 | |

**User's choice:** (a) (default)

### C2 — Welches CSS im iframe?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) `.mail-html-render` duplizieren | Vorschau = Editor — unterläuft aber den Phasen-Sinn | |
| (b) Nackte Browser-Defaults | Ehrlichste Variante, aber Times New Roman 16px ist unrealistisch | |
| (c) Kleines Mail-Baseline-Stylesheet | Arial/Helvetica ~14px, `img{max-width:100%}` | ✓ |

**User's choice:** (c) (default)
**Notes:** Kernentscheidung der Phase. (a) hätte PREV-02 unterlaufen — wenn Vorschau und Editor identisch aussehen, werden Diskrepanzen gerade *nicht* sichtbar.

### C3 — Grep-Gate-Test?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Ja — `sandbox`-Attribut als Invariante | Muster aus Phase 26 | ✓ |
| (b) Nein | | |

**User's choice:** (a) (default)

### C4 — Breiten fix oder konfigurierbar?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Zwei Code-Konstanten | Roadmap sagt „~640px / ~360px" | ✓ |
| (b) Über Settings konfigurierbar | Scope-Creep | |

**User's choice:** (a) (default)

---

## D — UI-Integration (PREV-01, PREV-04)

### D1 — Wo lebt der Umschalter?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) In `WysiwygEditor` selbst | Wirkt in allen 3 Call-Sites; iframe wird eigene Component | ✓ |
| (b) Eigenständige Component pro Page | Mehr Kontrolle, 3× Verdrahtung | |

**User's choice:** (a) (default)

### D2 — Toolbar im Preview-Modus?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Ausblenden | Klarstes „nicht editierbar"-Signal | ✓ |
| (b) Ausgegraut | Layout springt nicht | |

**User's choice:** (a) (default)

### D3 — Device-Rahmen?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Schlichter Rahmen + Label, zentriert auf grauem Backdrop | Reicht für PREV-04 | ✓ |
| (b) Stilisiertes Gerät mit Notch | Kosmetik | |

**User's choice:** (a) (default)

### D4 — Verhältnis zur bestehenden `TemplatePreview`?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Beide bleiben | Variablen-Prüfung vs. Layout-Prüfung — verschiedene Zwecke | ✓ |
| (b) HTML-Block durch iframe ersetzen | Eine Mechanik, aber Umbau an 3 Call-Sites | |

**User's choice:** (a) (default)

### D5 — `contenteditable` beim Umschalten?

| Option | Beschreibung | Selected |
|--------|--------------|----------|
| (a) Im DOM lassen, per CSS verstecken | Kein Remount, kein Seed-Lag | ✓ |
| (b) Unmounten + neu seeden | Risiko: Content-/Selection-Verlust | |

**User's choice:** (a) (default)

---

## Claude's Discretion

Explizit an Claude delegiert (siehe CONTEXT.md `<decisions>` → „Claude's Discretion"):

- Konkrete Werte des Mail-Baseline-Stylesheets
- Helper-Extraktion für die `src`-Injektion vs. zwei Stellen mit Grep-Gate
- Umschalt-UI-Form (Segmented-Control / Buttons / Tabs)
- Call-site-weise Anwendung der Member-Durchreich-Ausstiegsklausel
- `srcdoc`-Escaping-Strategie
- Unconditional vs. bedingter Sanitize-Aufruf im Preview-Endpoint
- i18n-Key-Namen

## Deferred Ideas

Vom User im Vorfeld als „draußen" bestätigt (keine Einsprüche auf die Ausschluss-Liste):

- Dark-Mode-Vorschau
- Echte Mail-Client-Simulation (Outlook-Quirks)
- Tablet-Breakpoint (~768 px)
- Screenshot-/PDF-Export der Vorschau

Im Verlauf ergänzt:

- Konfigurierbare Breakpoints in den Settings (gegen entschieden, C4)
- Sanitize-Diff-Banner (gegen entschieden, A3 — Wiedervorlage falls UAT Verständnisprobleme zeigt)
- `TemplatePreview` durch den iframe ersetzen (gegen entschieden, D4)

## Reviewed Todos (nicht eingefaltet)

5 Treffer aus `todo.match-phase 28`, alle generische Keyword-Matches ohne Preview-Bezug — Details in CONTEXT.md `<deferred>`.
