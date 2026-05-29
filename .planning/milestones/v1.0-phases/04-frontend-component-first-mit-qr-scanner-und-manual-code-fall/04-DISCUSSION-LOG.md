# Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-04
**Phase:** 4-Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback
**Areas discussed:** QR-Scanner-Strategie, Helfer-Routing & Layout-Trennung, Vorstand-Assembly-UI-Umfang, Connection-Banner & 200-OK-Feedback (Claude's Discretion)

---

## QR-Scanner-Strategie

### Frage 1 — QR-Scanner-Implementierung

| Option | Description | Selected |
|--------|-------------|----------|
| BarcodeDetector + Polyfill (Recommended) | Native Browser-API + dynamisch geladener Polyfill für ältere iOS | ✓ |
| rxing-wasm überall | Pure Rust-WASM, ein Code-Pfad, ~500KB-1MB Bundle-Overhead | |
| html5-qrcode (JS-Lib) | Etablierte JS-Library mit Camera-UI-Helpers | |
| Manual-only | Nur Manual-Code-UI als Default, QR-Scan als optional Add-on | |

**User's choice:** BarcodeDetector + Polyfill — minimaler Footprint im Standardpfad.

### Frage 2 — Polyfill-Auswahl

User fragte zunächst nach Stabilitäts-Einschätzung der Libraries (rxing-wasm/jsQR/ZXing-JS). Claude lieferte Vergleich; User entschied danach.

| Option | Description | Selected |
|--------|-------------|----------|
| jsQR (~50KB) | Stagnant aber funktional komplett, kleinster Footprint | |
| ZXing-JS (~200KB) | Aktiv gepflegt, Goldstandard, beste iOS-Kompatibilität | ✓ |
| rxing-wasm (~500KB) | Rust-konsistent, aber kleinere Community | |
| Kein Polyfill | Manual-Code-Fallback bei nicht-Support | |

**User's choice:** ZXing-JS lazy-loaded.

### Frage 3 — Camera-Permission-Flow

| Option | Description | Selected |
|--------|-------------|----------|
| Per Button-Klick „QR-Code scannen" (Recommended) | iOS-User-Gesture-konform, kein Surprise-Dialog | ✓ |
| Automatisch beim Page-Load | Ein Klick weniger, aber iOS Safari blockt ohne User-Gesture | |
| QR-Scan als Default-View | Scanner sofort sichtbar, Manual-Code als sekundärer Link | |

**User's choice:** Button-Klick.
**Notes:** Manual-Code-Input bleibt sichtbar als gleichberechtigter Pfad; Permission-Verweigerung führt nicht zu Error-Wall.

---

## Helfer-Routing & Layout-Trennung

### Frage 1 — Routing-Struktur Helfer vs Vorstand

| Option | Description | Selected |
|--------|-------------|----------|
| Getrennte Routen, geteilte Components (Recommended) | /helper + /assemblies/{id}/attendance, gleiche Components, sauber getrennte Auth-Pfade | ✓ |
| Eine geteilte /attendance/{id}-Route mit conditional Top-Bar | Eine Route für beide Rollen, mehr Branching-Logik | |
| Helfer-Sub-Tab in Vorstand-Assembly-Detail-Page | Zwei Layout-Welten | |

**User's choice:** Getrennte Routen, geteilte Components.
**Notes:** ATTN-06 wird durch Component-Reuse erfüllt, nicht durch Route-Sharing.

### Frage 2 — Helfer-Page-State-Flow

| Option | Description | Selected |
|--------|-------------|----------|
| Single-Page mit State-Machine (Recommended) | /helper rendert eine Page, State je nach Login/Loading/View/Error | |
| Zwei Routen: /helper (Login) + /helper/attendance (View) | Saubere URL-Trennung; Browser-Back-Edge-Case durch Auto-Redirect | ✓ |
| QR öffnet automatisch /helper?code=X, Login transparent | Scanner-zentrierter Flow ohne expliziten Login-Button | |

**User's choice:** Zwei Routen mit Auto-Redirect.
**Notes:** Beim Mount von /helper prüft die Page via API ob bereits gültige Helfer-Session existiert; wenn ja → Navigate zu /helper/attendance. Vermeidet Browser-Back-Probleme. Genauer Endpoint für „gültige Session vorhanden" = Claude's Discretion (Plan finalisiert).

---

## Vorstand-Assembly-UI-Umfang

### Frage 1 — Pages-Struktur

| Option | Description | Selected |
|--------|-------------|----------|
| Liste + Detail-Page mit Tabs (Recommended) | /assemblies + /assemblies/{id} mit Tabs (Stamm, Tokens, Anwesenheit) | ✓ |
| Flache Pages für jede Funktion | Eigene Routes, jede Page fokussiert | |
| Minimal-Scope — Liste + Token-Management, Anwesenheit via Helfer-View Reuse | Schlankester Scope | |

**User's choice:** Liste + Detail-Page mit Tabs.

### Frage 2 — QR-Druck-Pfad

| Option | Description | Selected |
|--------|-------------|----------|
| Browser-Print mit Print-CSS auf eigener Print-Route (Recommended) | /assemblies/{id}/tokens/print mit @media print Layouts | |
| PDF-Generierung im Browser (jspdf o.ä.) | Frontend baut PDF zusammen | |
| Backend-PDF-Endpoint via Typst | Würde v2 BULK-Scope vorziehen | |
| Kein dedizierter Druck-View — einzelne QR-Cards mit Browser-Print | Vorstand klickt einzeln, simpelst | ✓ |

**User's choice:** Kein dedizierter Druck-View, einzelne QR-Cards mit Browser-Print pro Stück.
**Notes:** Pragmatisch für erwartete Helfer-Anzahl pro GV (typisch 2–5). Bulk-Print kann in Phase 5 nachgezogen werden, wenn Generalprobe zeigt dass Einzel-Druck zu mühsam ist.

---

## Connection-Banner & 200-OK-Feedback (Claude's Discretion)

User unterbrach die Frage zu diesem Bereich und bat um Fortsetzung. Claude wählte sinnvolle Defaults; Plan-Phase darf verfeinern.

### Connection-Banner-Trigger (Default)
**Selected:** Banner erscheint bei 2 fehlgeschlagenen Polls in Folge; verschwindet bei nächstem Poll-Erfolg. Toleriert kurze 4G-Wackler. Nutzt status_bar.rs als Pattern-Vorlage. Alternative (Status-Dot via online_indicator.rs) bleibt offen für Phase 5 Polishing.

### Toggle-Feedback-Pattern (Default)
**Selected:** Klick → Spinner + disabled; Anwesend-Häkchen erst nach 200-OK. Bei 4xx/5xx → deutscher Toast-Error, Button kehrt in Vor-Klick-State. Doppel-Klick-Schutz durch disabled-State; Backend-Idempotenz (ATTN-03/04) ist Backstop.

---

## Claude's Discretion

Plan-Phase darf folgende Defaults verfeinern oder ändern:

- Connection-Banner-Trigger-Threshold (D-16) — 2 Polls vs Status-Dot-Variante
- Toggle-Feedback-Animation-Details (D-17/D-18)
- Polling-Hook-Sharing (D-15) — gemeinsamer vs separate Hooks für Counter+List
- Debounce-Wert für AttendanceSearch (D-11) — 500ms-Default
- JS-Polyfill-Bezug (D-20) — CDN vs lokaler Asset
- i18n-Helfer-Page-Locale-Switch (D-19) — fix de vs Locale-Detection
- Tab-Implementation (D-13) — CollapsibleSection-Reuse vs neuer tab_strip.rs
- Helfer-Auto-Redirect-Endpoint (D-06) — neuer /api/helper/session vs whoami-Erweiterung
- Print-CSS-Layout-Details (D-21)
- Test-Strategie (kein WASM-Test-Setup in Codebase) — manuelle E2E in Phase 5 Generalprobe vs Playwright-Setup

---

## Deferred Ideas

### Phase 5 (Generalprobe)
- Realer iOS-Safari-Test mit echter Hardware
- Connection-Banner-Threshold-Validation unter Vereinsheim-WiFi
- Print-Layout-Polishing auf echtem Drucker
- Bulk-Print-Evaluation falls >5 Helfer
- Stats-Polling-Last unter realer Helfer-Anzahl

### Spätere Phasen / Out of Scope
- Bulk-QR-Druck-Layout (BULK-01/02, v2)
- Mehrsprachige Helfer-UI mit Auto-Detect
- PDF-Export Anwesenheits-Liste (EXPO-01, v2)
- CSV/Excel-Export (EXPO-02, v2)
- Vollmacht-/Stimmrechts-UI (VOTE-01..04, v2)
- Self-Check-in für Mitglieder per persönlichem QR (Out of Scope)
- WASM-Test-Setup (wasm-bindgen-test oder Playwright)
