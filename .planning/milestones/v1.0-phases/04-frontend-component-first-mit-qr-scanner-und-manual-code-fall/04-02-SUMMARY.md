---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 02
subsystem: ui
tags: [dioxus, wasm, web-sys, barcode-detector, zxing, tailwind, crockford, print-css]

# Dependency graph
requires:
  - phase: 04-01
    provides: keine direkte Abhängigkeit — Plan 02 ist Frontend-Foundation, parallel zu 04-01 (Backend-Helper-API)
provides:
  - web-sys Camera/Media-Features (MediaDevices, MediaStream, MediaStreamTrack, MediaStreamConstraints, MediaTrackConstraints, HtmlVideoElement)
  - JS-Bridge `BarcodeDetector` extern + `has_barcode_detector()` Feature-Detection (über `js_sys::Reflect::has`)
  - `ZXING_POLYFILL`-Manganis-Asset-Konstante für lazy-loaded Fallback
  - Lokal vendored ZXing-JS 0.21.3 (Apache-2.0) + SHA256-Pin + assets/README.md
  - `helper_code`-Modul (Pure Logic): `CROCKFORD_ALPHABET`, `is_valid_helper_code`, `sanitize_helper_code_input` mit 9 Cargo-Tests
  - Print-CSS-Block für `.qr-card` (UI-SPEC §QrCard print contract)
  - Tailwind-Safelist mit `qr-card` + amber-Banner-Klassen + `animate-spin`/`animate-pulse` + `print:hidden`
affects: [04-04, 04-05, 04-06, 04-07, 04-08, 04-09, 04-10]

# Tech tracking
tech-stack:
  added:
    - "@zxing/library 0.21.3 (vendored UMD bundle, Apache-2.0)"
    - "BarcodeDetector via wasm-bindgen extern (kein web-sys Feature — Pitfall 1)"
  patterns:
    - "Pure-Logic-Module mit cargo-testbarer #[cfg(test)] mod tests-Konvention (analog zu component/member_search.rs)"
    - "wasm-bindgen extern Block in src/js.rs als Sammelpunkt für JS-Interop (BarcodeDetector neben CodeMirror)"
    - "Manganis asset!-Konstanten für lazy-loaded Polyfills"

key-files:
  created:
    - genossi-frontend/src/helper_code.rs
    - genossi-frontend/assets/zxing.umd.min.js
    - genossi-frontend/assets/zxing.umd.min.js.sha256
    - genossi-frontend/assets/README.md
  modified:
    - genossi-frontend/Cargo.toml
    - genossi-frontend/src/js.rs
    - genossi-frontend/src/main.rs
    - genossi-frontend/input.css
    - genossi-frontend/tailwind.config.js

key-decisions:
  - "BarcodeDetector NICHT als web-sys-Feature — existiert nicht in web-sys 0.3.97 (RESEARCH Pitfall 1). Stattdessen wasm-bindgen extern Block in js.rs."
  - "ZXing-JS lokal vendored statt CDN — Vereinsheim-WiFi-Aussetzer (Phase-5-Risiko) dürfen QR-Scanning nicht blockieren. Apache-2.0-License kompatibel."
  - "Crockford-Alphabet als explizite 32-char-Whitelist `0123456789ABCDEFGHJKMNPQRSTVWXYZ` — KEINE Range-Regex `0-9A-HJ-NP-Z`, weil diese L und U fälschlich einschließen würde (RESEARCH Pitfall 9)."
  - "helper_code-Modul ist UX-Convenience, KEINE Security-Boundary — Backend D-24 ist authoritative (Modul-Doccomment dokumentiert das explizit)."
  - "Tailwind-Safelist präventiv erweitert — Plan 10 verifiziert post-build, dass die Klassen tatsächlich im dist/tailwind.css landen (RESEARCH Pitfall 6)."
  - "Modul-Registrierung in src/main.rs (nicht src/lib.rs) — das Frontend-Crate ist ein Binary; eine lib.rs existiert nicht. Plan-Frontmatter referenzierte fälschlicherweise lib.rs."

patterns-established:
  - "Pure Function + cargo test in genossi-frontend: Testen aus dem Verzeichnis ohne -p (Crate ist Binary, daher schlägt cargo test -p genossi-frontend fehl)"
  - "Vendored Asset Workflow: assets/<name> + assets/<name>.sha256 + assets/README.md mit Vendoring-Datum, Version, License, Source-URL"
  - "BarcodeDetector Feature-Detection-Pattern: js_sys::Reflect::has(&window, &JsValue::from_str('BarcodeDetector'))"

requirements-completed: []

# Metrics
duration: ~25min
completed: 2026-05-05
---

# Phase 04, Plan 02: Frontend-Foundation für QR-Scanner Summary

**Foundation-Layer für Phase-4-Frontend gebaut: web-sys Camera-Features, vendored ZXing-JS-Polyfill, JS-Bridge für nativen BarcodeDetector mit Feature-Detection, Print-CSS für QR-Karten, Tailwind-Safelist und cargo-testbares Crockford-Validation-Modul mit 9 grünen Unit-Tests.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 3 / 3 abgeschlossen
- **Files modified:** 5 modifiziert + 4 neu erstellt = 9 total

## Accomplishments
- web-sys Camera/Media-Stack erweitert (6 neue Features) ohne `BarcodeDetector` als Cargo-Feature (Pitfall 1 vermieden)
- ZXing-JS 0.21.3 lokal vendored (336 KB UMD-Bundle, Apache-2.0, SHA256-Pin: `d7cc8f69dd70bdcf3ac00c9ae572bf2acb9f4132ba379c72df842e4db918652d`)
- JS-Bridge in `src/js.rs`: `BarcodeDetector` wasm-bindgen extern + `has_barcode_detector()` + `ZXING_POLYFILL`-Asset-Konstante
- `helper_code`-Modul mit `CROCKFORD_ALPHABET`, `is_valid_helper_code`, `sanitize_helper_code_input` als Pure Functions (9 Cargo-Unit-Tests, alle grün)
- Print-CSS für `.qr-card` (UI-SPEC-konform: A4 portrait, 16mm-Margin, Visibility-Hide-Pattern, 60mm QR-SVG, 16pt Mono-Code)
- Tailwind-Safelist erweitert um `qr-card` + amber-Banner-Klassen + Animations + `print:hidden`

## Task Commits

1. **Task 1: Cargo.toml web-sys features + ZXing vendoring + assets/README** — `243cb3b` (chore)
2. **Task 2: js.rs BarcodeDetector-Bridge + helper_code-Modul + 9 Cargo-Tests** — `73c9c98` (feat)
3. **Task 3: input.css print rules + tailwind safelist** — `fa56ddf` (style)

## Files Created/Modified

**Created:**
- `genossi-frontend/src/helper_code.rs` — Pure-Logic-Modul für Crockford-Validation (9 Cargo-Tests)
- `genossi-frontend/assets/zxing.umd.min.js` — Vendored ZXing-JS 0.21.3 (336 KB)
- `genossi-frontend/assets/zxing.umd.min.js.sha256` — SHA256-Pin für Reviewer-Verifikation
- `genossi-frontend/assets/README.md` — Vendoring-Doku (Source, Version, License, Update-Procedure)

**Modified:**
- `genossi-frontend/Cargo.toml` — 6 web-sys Camera/Media-Features ergänzt
- `genossi-frontend/src/js.rs` — BarcodeDetector extern Block + `has_barcode_detector()` + `ZXING_POLYFILL` (manganis::Asset)
- `genossi-frontend/src/main.rs` — `mod helper_code;` registriert
- `genossi-frontend/input.css` — `@media print`-Block + `print-color-adjust: exact`
- `genossi-frontend/tailwind.config.js` — Safelist um 8 Phase-4-Klassen erweitert

## Decisions Made

Alle Entscheidungen folgen direkt dem Plan und RESEARCH/UI-SPEC. Schlüssel-Entscheidungen:

1. **Modul-Registrierung in `src/main.rs` statt `src/lib.rs`**: Das Plan-Frontmatter listete `genossi-frontend/src/lib.rs`, aber dieses Crate ist ein Binary (kein Library-Crate, keine `lib.rs` existiert). Modul-Registrierung erfolgt in `main.rs`. Plan-Action-Step erkennt das implizit ("Frontend-Crate ist ein Binary"), nur das `files_modified`-Frontmatter und der Verify-grep waren inkonsistent. Tests laufen über `cd genossi-frontend && cargo test helper_code` (Pattern verifiziert, identisch zum bereits funktionierenden `member_search`).

2. **Crockford-Alphabet als String-Whitelist**: `pub const CROCKFORD_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"`. Range-Regex `0-9A-HJ-NP-Z` würde laut RESEARCH Pitfall 9 die Buchstaben L und U fälschlich erlauben. Der explizite 32-char-String ist die single source of truth, deckungsgleich mit Phase-2-Backend.

3. **`#[allow(dead_code)]` auf neue js.rs- und helper_code-Funktionen**: Bis Plan 04-05 die Konsumenten implementiert, sind die Symbole unused. Konsistent mit Pattern in js.rs (z.B. `current_datetime()` ist auch `#[allow(dead_code)]`).

## Deviations from Plan

### Deviation 1: Modul-Registrierung in `main.rs` statt `lib.rs`

- **Found during:** Task 2
- **Plan said:** "`genossi-frontend/src/lib.rs` erweitern — `pub mod helper_code;` ergänzen"
- **Reality:** `src/lib.rs` existiert nicht — das Crate ist ein Binary mit nur `src/main.rs`
- **Fix:** Modul in `src/main.rs` zwischen den existierenden `mod`-Deklarationen registriert (`mod helper_code;` statt `pub mod`, da Binary-internal-Sichtbarkeit ausreicht)
- **Verification:** `grep -q "mod helper_code" src/main.rs` → OK; 9 Tests grün
- **Rationale:** Das ist eine kosmetische Plan-Inkonsistenz. Der Plan-Action-Step erkennt explizit, dass das Crate ein Binary ist (Hinweis zu `cargo test -p genossi-frontend`), nur die `lib.rs`-Referenz im Frontmatter und Verify-grep waren stale. Funktionalität unverändert.

## Verification Results

```
=== Tests ===
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 59 filtered out
=== SHA pin ===
zxing.umd.min.js: OK
=== No BarcodeDetector feature ===
OK: no BarcodeDetector feature
=== has_barcode_detector ===
OK
=== ZXING_POLYFILL ===
OK
=== CROCKFORD_ALPHABET ===
OK
```

`cargo build` und `cargo check -p genossi-frontend` (über Verzeichnis-CD) grün — keine "unknown feature"-Errors, keine Compile-Errors aus dem neuen Modul.

## ZXing-JS Vendoring Details

- **Package:** `@zxing/library` (UMD-Bundle)
- **Version:** 0.21.3
- **License:** Apache-2.0
- **Source URL:** `https://unpkg.com/@zxing/library@0.21.3/umd/index.min.js`
- **Vendored file size:** 336 008 bytes (~328 KB)
- **SHA256:** `d7cc8f69dd70bdcf3ac00c9ae572bf2acb9f4132ba379c72df842e4db918652d`
- **Vendoring date:** 2026-05-05
- **Verification command:** `cd genossi-frontend/assets && sha256sum -c zxing.umd.min.js.sha256`

## Out-of-Scope Notes

- **`genossi-frontend/src/component/mod.rs` wurde NICHT angefasst** — Plan 04-06b ist der Single Writer für diese Datei.
- **STATE.md, ROADMAP.md, REQUIREMENTS.md** wurden NICHT modifiziert — Orchestrator-Verantwortung.
- Tatsächliches Camera-Streaming, ZXing-JS-Lazy-Loading-Logik, QR-Scanner-Component und Print-Trigger sind in Plans 04-05 / 04-06 / 04-09 zu bauen (diese Foundation-Plan stellt nur die Bausteine bereit).
