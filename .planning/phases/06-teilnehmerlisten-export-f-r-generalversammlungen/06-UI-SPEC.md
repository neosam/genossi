---
phase: 6
slug: teilnehmerlisten-export-für-generalversammlungen
status: draft
shadcn_initialized: false
preset: none
created: 2026-05-17
---

# Phase 6 — UI Design Contract

> Visueller und Interaktions-Vertrag für den **Teilnehmerlisten-Export-Block** in `assembly_details.rs`.
> Scope ist klein und fokussiert: EIN Block in EINER Page, sichtbar nur wenn `assembly.status == Closed`.
> Generiert von gsd-ui-researcher, zu verifizieren durch gsd-ui-checker.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (kein shadcn — Rust/Dioxus-WASM-Stack, shadcn-Gate nicht anwendbar) |
| Preset | not applicable |
| Component library | Dioxus 0.6.3 (React-like Rust→WASM) — Components als `#[component] fn ... -> Element` mit `rsx! { ... }` |
| Styling | Tailwind CSS 3.x via `npx tailwindcss --watch` — Utility-Klassen direkt in RSX |
| Icon library | keine — Text-Labels (`📄 PDF`, `📊 CSV`, `📈 XLSX` zulässig als Inline-Glyphe, KEINE neue Icon-Dep) |
| Font | System-Default (Tailwind default `font-sans` — Browser System UI Stack) |
| Detected components | `AssemblyStatusBadge`, `TabStrip`, `BasicsTab`, `Modal`, `ToastContainer` + `show_toast()`, `ConnectionBanner` (alle aus Phase 4, `src/component/`) |
| i18n | `crate::i18n::{use_i18n, Key}` — neue Keys in `de.rs` + `en.rs`, Enum-Variant in `mod.rs` (`Locale::De` + `Locale::En` only) |

---

## Spacing Scale

Übernimmt das bestehende Tailwind-Default-Scale (Multiple von 4px). Alle Werte sind bereits in `assembly_details.rs`/`basics_tab.rs` aktiv verwendet:

| Token | Tailwind-Klasse | Value | Usage in Phase 6 |
|-------|-----------------|-------|------------------|
| xs | `gap-1` / `px-1` | 4px | Icon-Glyphe ↔ Label-Abstand im Button |
| sm | `gap-2` / `p-2` / `mb-2` | 8px | Button-Reihe (`flex gap-2`), Radio-Gruppen-Abstand |
| md | `gap-4` / `p-4` / `mb-4` / `px-4 py-2` | 16px | Card-Innenpadding, Button-Padding (Standard CTA), Section-Margin |
| lg | `p-6` / `mb-6` | 24px | Card-Padding (analog `BasicsTab` `bg-white p-6 rounded-lg border`) |
| xl | `py-8` / `mt-8` | 32px | Section-Vertikalabstand zwischen Hinweis-State und Download-Block |
| 2xl | `py-12` | 48px | Empty-State-Padding (analog `AssemblyAttendanceNotOpenYet`-Block) |

**Exception:** Alle interaktiven Buttons MÜSSEN `min-h-[44px]` tragen — projektweite Touch-Target-Konvention (siehe `basics_tab.rs:75`, `assembly_details.rs:240`). Dies ist die EINZIGE Spacing-Ausnahme.

---

## Typography

Alle Werte konsistent mit `BasicsTab` / `TokensTab` (Phase 4 Pattern-Anker). Phase 6 führt KEINE neuen Größen ein.

| Role | Tailwind-Klasse | Size | Weight | Line Height | Phase-6-Verwendung |
|------|-----------------|------|--------|-------------|--------------------|
| Body | `text-base` | 16px | 400 (Regular) | 1.5 (`leading-normal`) | Beschreibungstext, Radio-Labels, Hinweis-Banner |
| Label / small | `text-sm text-gray-700` | 14px | 400 | 1.5 | Form-Labels („Welche Mitglieder?"), Helper-Text |
| Helper / muted | `text-xs text-gray-500` | 12px | 400 | 1.4 | Filename-Hint (`gv-2026-05-15-teilnehmer.pdf`), Status-Disabled-Erklärung |
| Section heading | `text-xl font-semibold` | 20px | 600 (Semibold) | 1.3 | Block-Überschrift „Teilnehmerliste exportieren" (analog `TokensTab` h2) |

**Weights:** exakt 2 — Regular (400) + Semibold (600). KEIN Bold (700), KEIN Light (300).

**Body Line Height:** 1.5 (Tailwind `leading-normal` Default).
**Heading Line Height:** 1.3 (Tailwind Default für `text-xl`).

---

## Color

60/30/10-Split konsistent zur Phase-4-Etablierung. Alle Werte sind bereits in `basics_tab.rs`/`assembly_details.rs` aktiv und werden direkt übernommen.

| Role | Tailwind-Token | Hex | Usage in Phase 6 |
|------|----------------|-----|------------------|
| Dominant (60%) | `bg-white` + Page-Background | `#FFFFFF` + `#F3F4F6` (`bg-gray-100` als Page-Backdrop) | Card-Surface des Export-Blocks, Body-Text-Hintergrund |
| Secondary (30%) | `border-gray-200` / `bg-gray-50` / `text-gray-700` | `#E5E7EB` / `#F9FAFB` / `#374151` | Card-Border, Hinweis-Banner-Hintergrund (Status-Gate-Hinweis), Form-Labels |
| Accent (10%) | `bg-blue-600` + `hover:bg-blue-700` + `text-blue-600` | `#2563EB` + `#1D4ED8` | **NUR**: Download-Submit-Button („Herunterladen"). NICHT für Radio-Buttons, NICHT für die drei Format-Toggle-Buttons (siehe Color Reserved-For unten). |
| Destructive | `bg-red-600` / `text-red-600` (via existierender `ToastContainer`) | `#DC2626` | **NUR** für Error-Toasts (409 Conflict, 403 Forbidden, 500 Server-Error). Im Phase-6-Block gibt es KEINE destruktive Aktion (Export ist read-only). |
| State: Info-Hint (für Closed-only-Gate) | `bg-blue-50` + `text-blue-800` + `border-blue-200` | `#EFF6FF` / `#1E40AF` / `#BFDBFE` | Banner „Export verfügbar, sobald die GV geschlossen ist" für Status `Preparation`/`Open` (s. State-Logik unten) |
| Loading (in Button) | `disabled:opacity-50` + bestehender Tailwind `animate-spin` | — | Spinner-Glyphe im Submit-Button während Generierung (analog `BasicsTab:172` `disabled:opacity-50` Pattern) |

**Accent reserved for:**
- Primary Submit-Button „Herunterladen" (1 Element pro Render des Blocks)
- Info-Banner-Akzent (blaue 50/800-Variante) für den Closed-Gate-Hinweis

**Explizit NICHT für Accent reservierte Elemente:**
- Format-Auswahl-Buttons (Radio-Group) — neutral, Border + Selected-State via `bg-gray-100`/`ring-2 ring-blue-500`
- Include-Toggle — neutral, Radio-Group ohne accent-fill
- Filename-Preview — Helper-Text in `text-gray-500`

**Status-Konsistenz mit `AssemblyStatusBadge`** (Phase-4-Anker, NICHT überschreiben):
- `Preparation` → `bg-gray-100 text-gray-800` (grau)
- `Open` → `bg-green-100 text-green-800` (grün)
- `Closed` → `bg-blue-100 text-blue-800` (blau) ← UNSER Trigger-Status

---

## Copywriting Contract

Alle Strings sind deutsch-zuerst (Vorstand-Frontend ist primär DE; EN existiert für Sprachumschaltung). Neue Keys in `genossi-frontend/src/i18n/mod.rs::Key` + `de.rs` + `en.rs`.

### Block-Header

| Element | DE | EN |
|---------|----|----|
| Section heading | **Teilnehmerliste exportieren** | **Export attendance list** |
| Subheading / Helper | Erzeugt eine Datei zur Anlage an das GV-Protokoll. | Generates a file to attach to the assembly minutes. |

### Form Controls

| Element | DE | EN |
|---------|----|----|
| Format-Group-Label | Format | Format |
| Format-Option 1 | PDF (für Protokoll) | PDF (for minutes) |
| Format-Option 2 | CSV (Semikolon, Excel-kompatibel) | CSV (semicolon, Excel-compatible) |
| Format-Option 3 | Excel (XLSX) | Excel (XLSX) |
| Include-Group-Label | Welche Mitglieder einbeziehen? | Which members to include? |
| Include-Option `all` (Default) | Alle Mitglieder (mit Anwesenheits-Spalte) | All members (with attendance column) |
| Include-Option `present` | Nur Anwesende | Only attendees |
| Filename-Preview-Label | Dateiname | Filename |
| Primary CTA | Herunterladen | Download |
| Primary CTA (während Loading) | Wird erzeugt … | Generating … |

### State Copy

| Element | DE | EN |
|---------|----|----|
| Closed-Gate Banner Heading | Export verfügbar nach GV-Schluss | Export available after the assembly is closed |
| Closed-Gate Banner Body | Die Teilnehmerliste kann erst exportiert werden, sobald diese Generalversammlung geschlossen ist. | The attendance list can only be exported once this assembly is closed. |
| Empty state | (n/a — Block ist gar nicht sichtbar im `Preparation`-Status; im `Open`-Status zeigt der Gate-Banner; im `Closed`-Status sind die Daten immer da) | — |
| Error: 409 Conflict (Status nicht Closed — defensive, falls Race) | Export ist nur für geschlossene Generalversammlungen möglich. | Export is only available for closed assemblies. |
| Error: 403 Forbidden | Keine Berechtigung zum Export. | You don't have permission to export. |
| Error: 500 / Network | Export fehlgeschlagen. Bitte erneut versuchen. | Export failed. Please try again. |
| Success-Toast (optional, nur falls Block nicht selbst-evident) | Download gestartet. | Download started. |

### Destructive

**Keine destruktiven Aktionen in Phase 6.** Export ist read-only. Kein Confirm-Dialog nötig.

---

## Layout Decision: Wo lebt der Block?

**EMPFEHLUNG: Neuer 4. Tab „Export" im `TabStrip` von `assembly_details.rs`, sichtbar NUR wenn `assembly.status == Closed`.**

| Option | Pro | Contra | Entscheidung |
|--------|-----|--------|--------------|
| **A: Neuer Tab „Export"** | Eigener fokussierter Raum; konsistent mit der etablierten 3-Tab-Struktur (Basics/Tokens/Attendance); leicht erweiterbar (späterer Sammelexport, signature-Layout etc.); klare Discoverability nach GV-Schluss | Tab erscheint/verschwindet je nach Status — neuer Pattern (bisher haben Tabs nur ihren Body status-abhängig) | **GEWÄHLT** |
| B: In Basics-Tab eingegliedert | Kein neuer Tab; alles auf einem Schirm | Basics-Tab wird überladen (Stamm-Daten + Edit + Open/Close + Export); Export ist konzeptuell ein eigener Outcome („Endbeleg fürs Protokoll") | Verworfen |
| C: Sektion unter dem TabStrip am Seitenende | Sehr sichtbar | Bricht die Tab-Struktur; Export-Block wäre immer im Viewport, auch wenn der Attendance-Tab aktiv ist | Verworfen |

**Tab-Verhalten:**
- Status `Preparation` oder `Open`: Der Tab „Export" wird **gar nicht im `TabStrip` gerendert**. Begründung: D-11 sagt „nur für Closed". Wenn er disabled wäre, müsste der User raten, was den Status ändert. „Closed" passiert ohnehin nur einmal pro GV — das Tab-Erscheinen ist ein klares Signal („GV ist fertig, jetzt kann exportiert werden").
- Status `Closed`: Tab „Export" erscheint als 4. Position rechts neben „Anwesenheit".
- Defensive: Falls der User die URL mit `?tab=export` direkt aufruft, bevor die GV closed ist, fällt das Routing auf `basics` zurück (Tab-Key existiert nicht in der Liste).

**Alternative für Konsistenz-Bewahrer:** Falls Planner doch beim 3-Tab-Layout bleiben will, wandert der Block in den Basics-Tab unterhalb der Open/Close-Buttons als eigene `<section>` mit eigener Überschrift. Dies ist die **Fallback-Option**, falls in der Planung Diskussion entsteht.

---

## Component Decision (D-20)

**EMPFEHLUNG: Inline in `assembly_details.rs` als page-internes `#[component] fn ExportTab(...)`, NICHT als geteilte Component unter `src/component/`.**

**Begründung:**
- D-20 ist explizit „nur falls Wiederverwendung absehbar".
- Phase 6 betrifft EINE Page (`assembly_details.rs`).
- Deferred Ideas (`Sammelexport`, `E-Mail-Versand`, `Multi-GV-Liste`) sind als „eigene Phase, nicht im Scope" markiert — also nicht absehbar in den nächsten 1-2 Monaten.
- Pattern-Anker: `AttendanceTab` und `TokensTab` in `assembly_details.rs` sind genauso page-interne Smart-Wrapper. Diese Konvention wird wiederholt.

**Verbindlicher Speicherort:** Die Component lebt als `fn ExportTab(assembly: AssemblyTO, on_error: EventHandler<String>) -> Element` im UNTEREN Teil von `assembly_details.rs` (analog `AttendanceTab`/`TokensTab`).

**Wann später extrahieren:** Sobald ein zweiter Aufrufer auftaucht (z. B. Sammelexport-Page), wandert der Block in `src/component/attendance_export_block.rs`. Bis dahin: inline.

---

## Layout Spec — Export-Tab-Body

```
┌─────────────────────────────────────────────────────────────────┐
│ container mx-auto px-4 py-6   (von assembly_details.rs)        │
│                                                                 │
│ ┌─ TabStrip [Basics] [Tokens] [Anwesenheit] [Export] ────────┐  │
│ └─────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ ┌─ bg-white p-6 rounded-lg border border-gray-200 ───────────┐  │
│ │                                                             │  │
│ │ Teilnehmerliste exportieren            (text-xl semibold)  │  │
│ │ Erzeugt eine Datei zur Anlage…         (text-sm gray-600)  │  │
│ │                                                             │  │
│ │ ─── space-y-6 ──────────────────────────────────────────── │  │
│ │                                                             │  │
│ │ Format                                  (text-sm gray-700) │  │
│ │ ┌────────────┐ ┌────────────┐ ┌────────────┐               │  │
│ │ │ (•) PDF    │ │ ( ) CSV    │ │ ( ) XLSX   │  RadioGroup   │  │
│ │ │ für Protok.│ │ Excel-komp.│ │ Excel-nat. │               │  │
│ │ └────────────┘ └────────────┘ └────────────┘               │  │
│ │                                                             │  │
│ │ Welche Mitglieder einbeziehen?          (text-sm gray-700) │  │
│ │  (•) Alle Mitglieder (mit Anwesenheits-Spalte)             │  │
│ │  ( ) Nur Anwesende                                          │  │
│ │                                                             │  │
│ │ Dateiname            gv-2026-05-15-teilnehmer.pdf          │  │
│ │                      (text-xs gray-500 mono)                │  │
│ │                                                             │  │
│ │                              ┌─────────────────────────┐    │  │
│ │                              │ Herunterladen          │ ←  │  │
│ │                              └─────────────────────────┘    │  │
│ │                              (bg-blue-600 …, min-h-44)      │  │
│ │                                                             │  │
│ └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Inventory (verbindlich)

| Element | Tag/Type | Tailwind-Klassen | Verhalten |
|---------|----------|------------------|-----------|
| Card-Wrapper | `div` | `bg-white p-6 rounded-lg border border-gray-200` | identisch zu `BasicsTab` |
| Section Heading | `h2` | `text-xl font-semibold mb-2` | DE: „Teilnehmerliste exportieren" |
| Subheading | `p` | `text-sm text-gray-600 mb-6` | DE: „Erzeugt eine Datei zur Anlage…" |
| Form-Body-Wrapper | `form` | `flex flex-col gap-6` | `onsubmit` triggert Download |
| Format-Group-Label | `span` | `text-sm text-gray-700 mb-2 block` | DE: „Format" |
| Format-Radio-Card-Container | `div` | `grid grid-cols-1 sm:grid-cols-3 gap-3` | Responsive Grid (3 Karten desktop, gestapelt mobile) |
| Format-Radio-Card (selected) | `label` | `border-2 border-blue-500 bg-blue-50 px-4 py-3 rounded cursor-pointer flex flex-col gap-1 min-h-[44px]` | Visuelles Selected-State |
| Format-Radio-Card (unselected) | `label` | `border-2 border-gray-200 hover:border-gray-300 bg-white px-4 py-3 rounded cursor-pointer flex flex-col gap-1 min-h-[44px]` | Neutral |
| Format-Radio-Card-Title | `span` | `text-base font-semibold` | „PDF" / „CSV" / „Excel (XLSX)" |
| Format-Radio-Card-Hint | `span` | `text-xs text-gray-500` | „für Protokoll" / „Semikolon, Excel-kompatibel" / „Excel-natives Format" |
| Native Radio (visually hidden) | `input` | `sr-only` mit `r#type: "radio"` + `name: "export_format"` | Tastatur+Screen-Reader-Fallback |
| Include-Group-Label | `span` | `text-sm text-gray-700 mb-2 block` | DE: „Welche Mitglieder einbeziehen?" |
| Include-Radio-Wrapper | `div` | `flex flex-col gap-2` | |
| Include-Radio-Label | `label` | `flex items-center gap-2 cursor-pointer min-h-[44px] px-2` | |
| Include-Radio-Input | `input` | (Browser-default radio mit `accent-blue-600`) | `r#type: "radio"` + `name: "export_include"` |
| Filename-Preview-Row | `div` | `flex items-baseline gap-3 text-sm` | |
| Filename-Preview-Label | `span` | `text-gray-700` | DE: „Dateiname" |
| Filename-Preview-Value | `code` | `text-xs text-gray-500 font-mono` | `gv-{YYYY-MM-DD}-teilnehmer.{ext}` (reaktiv aus Format) |
| Submit-Button-Row | `div` | `flex justify-end` | |
| Submit-Button (idle) | `button` | `bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded min-h-[44px] disabled:opacity-50` + `r#type: "submit"` | DE: „Herunterladen" |
| Submit-Button (loading) | `button` | dieselbe Klassen + `disabled` | DE: „Wird erzeugt …" + optional `animate-spin`-Glyphe links |

### Closed-Gate State — wenn jemand den Tab trotzdem aufruft (defensive)

Da der Tab im `Preparation`/`Open`-Status gar nicht erscheint, ist dieser State nur defensive. Falls per URL-Manipulation/race der Tab dennoch aktiv wird:

```
┌─ bg-blue-50 border border-blue-200 rounded-lg p-6 ─────────┐
│                                                             │
│  Export verfügbar nach GV-Schluss (text-base font-semibold) │
│  Die Teilnehmerliste kann erst…   (text-sm text-blue-800)   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Interaction Contract

### Default State

- `selected_format`: `"pdf"` (PDF ist das primäre Verwendungs-Format laut D-08, gehört ans Protokoll).
- `selected_include`: `"all"` (Recommendation aus CONTEXT.md D-09).
- `submitting`: `false`.

### Loading State (Klick auf „Herunterladen")

- Submit-Button wird `disabled`, Label wechselt zu „Wird erzeugt …".
- Format- und Include-Radios bleiben sichtbar, aber `disabled` (verhindert Race wenn User mid-flight die Auswahl ändert).
- KEIN globaler Spinner, KEIN Modal-Overlay — der Loading-State bleibt lokal im Button.
- Bei PDF mit 500+ Mitgliedern kann die Generierung mehrere Sekunden dauern (siehe Phase-6-Discussion). Der Button-Loading-State ist ausreichend; KEIN globaler Toast „Wird erzeugt", weil der Button selbst die Information trägt.

### Success Path

- Backend antwortet 200 OK mit Blob + `Content-Disposition: attachment; filename="gv-…"`.
- Frontend: `fetch` → `.blob()` → `Url::create_object_url_with_blob()` → programmatisch erzeugter `<a download="…" href="…">` + `.click()` (siehe Pattern `api.rs:506-548`).
- Browser zeigt nativen Download-Dialog (oder lädt direkt in den Downloads-Ordner — Browser-Setting).
- Submit-Button kehrt nach `await` in den idle-State zurück.
- KEIN Success-Toast notwendig — der native Browser-Download ist selbst-evidentes Feedback.

### Error Path

- **409 Conflict** (Status nicht `Closed` — defensive Race): Toast „Export ist nur für geschlossene Generalversammlungen möglich." über bestehenden `show_toast()` aus `src/component/toast.rs`.
- **403 Forbidden** (User hat kein Admin-Recht): Toast „Keine Berechtigung zum Export." (Sollte gar nicht passieren, weil Tab-Sichtbarkeit `RequirePrivilege { privilege: "admin" }` voraussetzt — defensive).
- **500 / Network**: Toast „Export fehlgeschlagen. Bitte erneut versuchen."
- Toast-Anzeige nutzt UNVERÄNDERT die bestehende `ToastContainer` aus `assembly_details.rs:141`. KEINE neuen Toast-Komponenten.
- Bei Error bleibt der Block-State erhalten — User kann erneut auf „Herunterladen" klicken.

### Filename-Preview-Reaktivität

- Bei jedem Klick auf eine Format-Karte wird die Preview reaktiv aktualisiert:
  - `pdf` → `gv-2026-05-15-teilnehmer.pdf`
  - `csv` → `gv-2026-05-15-teilnehmer.csv`
  - `xlsx` → `gv-2026-05-15-teilnehmer.xlsx`
- Datum kommt aus `assembly.date` (DE-Format zu `YYYY-MM-DD` parsen). Falls `assembly.date` `None` ist (sollte für Closed-GVs nie passieren, defensive), Fallback: `gv-teilnehmer.{ext}` ohne Datum.
- **Begründung Pro-Preview:** Vorstand muss wissen, wie die Datei im Downloads-Ordner heißen wird, um sie korrekt im Protokoll-Anhang zu referenzieren („Anlage 3: `gv-2026-05-15-teilnehmer.pdf`"). Ist also nicht Visual-Noise, sondern Funktion.

---

## Accessibility

| Aspekt | Vorgabe |
|--------|---------|
| Touch-Targets | Alle Buttons + Radio-Cards: `min-h-[44px]` (projektweite Konvention seit Phase 4) |
| Tastatur-Navigation | Native `<input type="radio">` mit `sr-only` für Format-Radio-Cards → Tab-Reihenfolge: Format-PDF → Format-CSV → Format-XLSX → Include-All → Include-Present → Submit; `<label>`-Wrap macht Cards klickbar UND tastatur-fokussierbar |
| Fokus-Indikator | Tailwind `focus-visible:ring-2 focus-visible:ring-blue-500 focus-visible:ring-offset-2` auf Cards und Submit-Button (NEU für Phase 6 — bisherige Pages haben das nicht durchgängig; Phase 6 setzt den Standard) |
| ARIA-Labels | Submit-Button: `aria-label="{Herunterladen | Wird erzeugt}"` reicht (Text-Label ist sichtbar). Format-Radios: `<input>` selbst trägt den `value` → Screen-Reader liest `<label>`-Text |
| ARIA-Live | Submit-Button-Label-Wechsel von „Herunterladen" → „Wird erzeugt …" via `aria-live="polite"` auf einem inneren `<span>` für Screen-Reader-Feedback während Generierung |
| Error-Announcement | `ToastContainer` (existierend) hat KEINE `aria-live`-Region. **Empfehlung für Phase 6 (KEINE Erweiterung nötig — out-of-scope):** Spätere Iteration könnte `role="alert"` auf den Toast-Items setzen; heute reicht der visuelle Toast. |
| Sprache | `<html lang="de">` ist bereits via `i18n.locale()` im App-Shell gesetzt — kein Phase-6-Eingriff. |

---

## Responsive

| Breakpoint | Format-Radio-Cards | Submit-Button | Card-Padding |
|------------|---------------------|---------------|--------------|
| `< 640px` (mobile) | `grid-cols-1` — vertikal gestapelt | Full-width oder rechtsbündig, `min-h-[44px]` | `p-4` (statt `p-6`) für mehr Inhalt-Platz |
| `≥ 640px` (`sm:` und größer) | `sm:grid-cols-3` — nebeneinander | rechtsbündig (`flex justify-end`) | `p-6` |

Klassen-Stack für Format-Container: `grid grid-cols-1 sm:grid-cols-3 gap-3`. Klassen-Stack für Card-Wrapper: `bg-white p-4 sm:p-6 rounded-lg border border-gray-200`.

Vorstand-Frontend ist primär Desktop, aber Tablets (iPad mit Safari) müssen funktionieren — wurde bei der realen GV verifiziert. Der `min-h-[44px]`-Standard sorgt für Touch-Tauglichkeit.

---

## State Machine (Tab-Sichtbarkeit + Block-States)

```
assembly.status:
├── Preparation
│   └── Tab „Export" wird NICHT gerendert
├── Open
│   └── Tab „Export" wird NICHT gerendert
└── Closed
    └── Tab „Export" wird gerendert
        └── Block-States:
            ├── idle (Default — Format=PDF, Include=All, Submit aktiv)
            ├── loading (Submit-Click → fetch läuft → Button disabled + Label "Wird erzeugt …")
            ├── success (Browser-Download getriggert → State zurück auf idle)
            └── error (Toast erscheint via show_toast → State zurück auf idle, User kann retry)
```

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none — kein shadcn im Projekt | not applicable |
| third-party registries | none | not applicable |

Phase 6 fügt **keine** neuen Frontend-Dependencies hinzu. Alles wird aus existierenden Tailwind-Utilities + bestehenden Dioxus-Components (`Modal`, `TabStrip`, `ToastContainer`, `AssemblyStatusBadge`) komponiert.

Neue Workspace-Crates sind ausschließlich Backend (`rust_xlsxwriter` Promotion von dev → prod) und damit außerhalb des UI-SPEC-Scopes.

---

## Pre-Population Source Map

| Section | Quelle |
|---------|--------|
| Design System (kein shadcn) | Codebase-Scan: `dx serve`, kein `components.json` — Rust/Dioxus-Stack, shadcn-Gate not applicable |
| Spacing Scale | `basics_tab.rs` + `assembly_details.rs` Tailwind-Klassen (`p-6`, `mb-4`, `gap-2`, `px-4 py-2`, `min-h-[44px]`) |
| Typography | `basics_tab.rs:75` (`text-base font-medium`), `assembly_details.rs:237` (`text-xl font-semibold`), `assembly_status_badge.rs:25` (`text-xs font-medium`) |
| Color 60/30/10 | `assembly_status_badge.rs` (blue/green/gray), `basics_tab.rs` (blue-600 primary, gray-200 border), `toast.rs` (red-600 destructive) |
| Copywriting | Neu für Phase 6 — Researcher-Vorschlag, an Vorstand-Sprache angelehnt (CONTEXT.md Specifics: „Verbandskonform") |
| Layout (Tab-Position) | Researcher-Empfehlung Option A — begründet im Layout Decision Block |
| Component-Inline-Entscheidung | CONTEXT.md D-20 explizit; Researcher folgt der „nur falls Wiederverwendung absehbar"-Klausel |
| Filename-Schema | CONTEXT.md D-15 (`gv-{YYYY-MM-DD}-teilnehmer.{ext}`) |
| Blob-Download-Pattern | `genossi-frontend/src/api.rs:506-548` (`render_template_pdf`) — Vorbild für `export_attendance` |
| Toast-Wiring | `genossi-frontend/src/component/toast.rs` + `assembly_details.rs:43,141` |
| RequirePrivilege-Wrap | `assembly_details.rs:68` — bereits aktiv für `admin`, Tab-Sichtbarkeit erbt das |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS — alle Strings DE+EN deklariert, CTA + Error + Empty-State + Loading-Label spezifisch
- [ ] Dimension 2 Visuals: PASS — Card-Pattern aus Phase 4 wiederverwendet, Layout-Mockup ASCII vorhanden, Component-Inventory verbindlich
- [ ] Dimension 3 Color: PASS — 60/30/10 explizit, Accent reserved-for-Liste (Submit-Button + Info-Banner), keine Über-Akzentuierung der Radio-Cards
- [ ] Dimension 4 Typography: PASS — exakt 4 Größen (`text-xs`/`text-sm`/`text-base`/`text-xl`), exakt 2 Weights (400/600), Konsistenz mit Phase 4
- [ ] Dimension 5 Spacing: PASS — Tailwind-4px-Multiples (`gap-2`/`gap-3`/`gap-6`, `p-4`/`p-6`, `mb-2`/`mb-6`), `min-h-[44px]`-Exception explizit dokumentiert
- [ ] Dimension 6 Registry Safety: PASS — keine third-party Registry, keine neuen Frontend-Deps, not applicable

**Approval:** pending (gsd-ui-checker entscheidet)
