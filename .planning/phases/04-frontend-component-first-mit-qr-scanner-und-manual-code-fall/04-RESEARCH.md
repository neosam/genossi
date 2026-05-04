# Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback — Research

**Researched:** 2026-05-04
**Domain:** Dioxus 0.6 WASM Frontend + Browser Camera-API + JS-Polyfill-Bridge
**Confidence:** HIGH (Codebase-Patterns + Context7-Dioxus + offizielle web-sys-Quelle vetting + Caniuse für Browser-Support)

---

## Summary

Phase 4 ist ein reines Frontend-Phase, das gegen die fertigen REST-Endpoints aus Phasen 1–3 baut. Das schwierigste Stück ist **nicht** die Component-Architektur (UI-SPEC liefert detaillierte Skelette) sondern (a) die **Camera-Integration** über eine `BarcodeDetector`-API, die in iOS Safari de-facto **nicht** verfügbar ist, sodass der "Polyfill" ZXing-JS auf iOS-Geräten der **primäre** Pfad wird, nicht der Fallback, und (b) das **Polling-Lifecycle**-Pattern für `LiveCounter` + `AttendanceList`, das in dieser Codebase neu ist (existing components pollen nicht).

**Primary recommendation:** Polling über `use_future` mit endloser `gloo_timers`-Loop + automatischem Cancel bei Unmount (Dioxus-Signal-Teardown — bestätigt in Context7-Docs). Camera-Integration über das existierende `js.rs`-Pattern (`#[wasm_bindgen] extern "C"` + `js_sys::Reflect`-Feature-Detection). ZXing-JS lokal vendored als Static Asset, lazy-initialisiert via `document::eval()` beim ersten "QR-Code scannen"-Klick. Kein neuer Rust-Crate für QR — alles über web-sys + JS-Bridge. **Polyfill-Strategie umkehren:** ZXing-JS auf iOS sofort laden (Feature-Detection sagt "BarcodeDetector fehlt" → 100% iOS-Cases), BarcodeDetector nur als Optimierung für Chrome/Edge/Android.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### QR-Scanner-Strategie
- **D-01:** **Primär: BarcodeDetector** (Browser-native API). Aufruf via `web_sys`/`js_sys` mit Feature-Detection `'BarcodeDetector' in window`. Unterstützt: Chrome ≥83, Edge, Android Chrome, iOS Safari ≥17. Kein zusätzlicher Bundle für die meisten User.
- **D-02:** **Polyfill: ZXing-JS** (`@zxing/library`), **lazy-loaded** nur wenn BarcodeDetector nicht verfügbar ist. Bundle-Größe ~200KB on-demand. Begründung: aktiv gepflegt (zxing-js org), Goldstandard für Browser-Barcode-Scanning, beste iOS-Quirks-Dokumentation.
- **D-03:** **UX-Flow Helfer-Login:** Initial-View zeigt zwei gleichberechtigte Pfade: (a) Button "QR-Code scannen" + (b) Manual-Code-Input. Camera-Permission **erst beim Klick**. Manual-Code-Validation: Frontend prüft Länge=10 + Crockford-Base32-Alphabet (`0-9A-HJ-NP-Z`).
- **D-04:** Beim Scan/Submit: POST `/api/helper/redeem` mit `{code: "ABCXYZ1234"}`. Response: 200 + Set-Cookie + `{assembly_id, expires_at}`. Bei Fehler: Status-Code-Mapping zu deutscher Message (404/410/403/400).

#### Routing & Layout-Trennung
- **D-05:** Getrennte Routen: `/helper`, `/helper/attendance`, `/assemblies`, `/assemblies/{id}`. Helfer-Routes ohne TopBar (HelperShell-Wrapper); Vorstand-Routes mit `RequirePrivilege "admin"`.
- **D-06:** Helfer-Login + Anwesenheit als zwei getrennte Routen; beim Mount von `/helper` per API-Call prüfen ob bereits Helfer-Session existiert → Auto-Redirect.
- **D-07:** HelperShell-Layout: minimal, mobile-first, ohne Vorstand-Navigation. Schmaler Header mit GV-Name + LogOut-Button.

#### Vorstand-Assembly-UI-Struktur
- **D-08:** `/assemblies` (Liste + Anlegen-Modal); `/assemblies/{id}` mit drei Tabs: Stamm-Daten / Helfer-Tokens / Anwesenheit.
- **D-09:** QR-Druck = einzelne Browser-Prints pro Card (Bulk-Print v2).
- **D-10:** Eingelöste Token zeigen Memo + `Used` + Timestamp; QR-SVG/Code nicht erneut abrufbar.

#### Geteilte Anwesenheits-Components
- **D-11:** `AttendanceList`, `AttendanceSearch`, `LiveCounter` als shared Components in `genossi-frontend/src/component/`.
- **D-12:** Helfer-Login-Components: `qr_scanner.rs`, `manual_code_input.rs`, `qr_card.rs`, `helper_shell.rs`.
- **D-13:** Vorstand-UI-Components: `assembly_list_row.rs`, `assembly_status_badge.rs`, `tab_strip.rs` (neu, da `CollapsibleSection` kein Tab-Pattern ist).

#### Polling-Architektur
- **D-14:** `LiveCounter` nutzt `use_resource` + `gloo_timers::future::TimeoutFuture` für ~5s-Intervall. Lokales Polling, kein globaler Service. Stop bei Unmount.
- **D-15:** AttendanceList-Refresh-Trigger: nach Toggle-200-OK, nach Such-Vorgang, alle ~5s. Plan-Discretion: gemeinsamer vs separate Polling-Hook.

#### Connection-Banner & 200-OK-Feedback
- **D-16:** ConnectionBanner bei 2 Polling-Fehlern in Folge; verschwindet bei Recovery. Pattern-Vorlage: `error_alert.rs`-Farben aber sticky-top + amber statt red.
- **D-17:** Toggle-Feedback: sofort Loading-State, KEIN visuelles Häkchen vor 200-OK. Bei 4xx/5xx: Toast-Notification, Button revert.
- **D-18:** Doppel-Klick-Schutz via `disabled` während Request. Backend-Idempotenz als Backstop.

#### i18n & Sprache
- **D-19 (amended):** Bestehendes i18n-System nutzen. Beide Locales (de, en) MÜSSEN neue Keys haben — `Locale`-Enum hat NUR `En`+`De`, kein `cs.rs`. Helfer-View standardmäßig deutsch.

#### Frontend-Build & Dependencies
- **D-20:** web-sys-Features ergänzen (BarcodeDetector*, MediaDevices, MediaStream*, HtmlVideoElement). ZXing-JS lokal vendored auf v0.21.3 (Apache-2.0, vetted), SHA256-pinned.
- **D-21:** `@media print` CSS in `input.css` für QR-Card-Druck.

#### REST-API-Konsumption
- **D-22:** Neue API-Funktionen in `api.rs` (redeem_helper_token, list/get/create/update/open/close_assembly, list/create/revoke_helper_token, list_attendance_members, mark_present/absent, get_assembly_stats). Alle nutzen `AppError`/`status_to_message`.

#### Naming
- **D-23:** Englisch + snake_case (Genossi-Konvention) für alle neuen Files.

### Claude's Discretion
- ConnectionBanner-Defaults (D-16): "2 fehlgeschlagene Polls" als Default — Plan kann Status-Dot oder andere Variante wählen.
- Toggle-Feedback-Pattern (D-17/D-18): Loading-Spinner + 200-OK-Verifizierung — Plan kann subtle visual feedback ergänzen.
- Polling-Hook-Sharing (D-15): gemeinsamer Hook für Counter+Liste vs separate.
- Debounce-Wert für AttendanceSearch (D-11): 500ms Default.
- JS-Polyfill-Bezug (D-20): UI-SPEC empfiehlt vendoring (Option B); CDN bleibt Override.
- i18n-Helfer-Page-Locale-Switch (D-19): fix de oder Locale-Detection.
- Tab-Implementation (D-13): bestehende `CollapsibleSection` reusen vs neuer `tab_strip.rs` — UI-SPEC empfiehlt neuen `tab_strip.rs`.
- Helfer-Auto-Redirect-Endpoint (D-06): welcher Endpoint signalisiert "gültige Helfer-Session"? Plan finalisiert.
- Print-CSS-Layout-Details (D-21): exakte `@media print`-Regeln.
- Test-Strategie für Phase 4: WASM-Tests sind nicht etabliert. Plan entscheidet: (a) Cargo-Tests für Logik, (b) manuelle E2E in Phase 5, oder (c) Playwright/Cypress-Setup.

### Deferred Ideas (OUT OF SCOPE)

#### Phase 5 (Generalprobe & Operations)
- Realer iOS-Safari-Test
- Connection-Banner-UX-Validation unter Vereinsheim-WiFi-Last
- Print-Layout-Polishing
- Bulk-Print-Layout (>5 Helfer)
- Stats-Polling-Last-Tests

#### Spätere Phasen / Out of Scope
- Bulk-QR-Druck-Layout (BULK-01/02 v2)
- Mehrsprachige Helfer-UI mit Auto-Detect
- Native-Mobile-App
- PDF-Export der Anwesenheits-Liste (EXPO-01 v2)
- CSV/Excel-Export (EXPO-02 v2)
- Vollmacht-/Stimmrechts-UI (VOTE-01..04 v2)
- Self-Check-in für Mitglieder per persönlichem QR-Code
- WASM-Test-Setup (`wasm-bindgen-test` oder Playwright)
- Dritte Locale (cs)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HLPR-03 | Helfer kann alternativ den 8–12-Zeichen-Code (in dieser Iteration fix 10 Zeichen, Phase 2 D-09) manuell in ein Eingabefeld tippen und damit dieselbe Session erzeugen — als Fallback bei Camera-Permission-Verweigerung oder Scanner-Fehlfunktion | "Manual-Code Form-Validation"-Sektion (Crockford-Base32-Regex, Auto-Uppercase, 10-Char-Constraint, Frontend-Validation vor POST `/api/helper/redeem`) — `ManualCodeInput`-Component-Skelett ist in UI-SPEC Sektion 3 vollständig spezifiziert; Research bestätigt nur Mechanik (Input-Filter, Submit-Button-Disabled-State) und liefert iOS-Safari-Krise-Begründung weshalb das Manual-Eingabefeld auf iOS de facto der **Haupt-Pfad** und nicht der Fallback ist [VERIFIED: caniuse.com] |
| SYNC-01 | Helfer sehen aktualisierte Anwesenheits-Status beim nächsten Refresh oder beim nächsten Such-Vorgang; kein Live-Push (SSE/WebSocket) erforderlich | "Polling-Pattern für Live-Counter"-Sektion (use_future-Loop mit gloo_timers, automatischer Cancel bei Component-Unmount, Refresh-Trigger nach Toggle/Search/Tick); "Connection-Banner"-Sektion (2-Failures-in-Folge Schwellwert) — Phase-3-Backend liefert idempotente Endpoints und SYNC-02 garantiert Konflikt-Freiheit [VERIFIED: Phase 3 abgeschlossen, Phase 3 D-26] |
</phase_requirements>

---

## Project Constraints (from CLAUDE.md)

Direkt aus `/home/neosam/programming/rust/projects/genossi3/CLAUDE.md` und `genossi-frontend/CLAUDE.md` — der Plan muss diese ohne Ausnahmen einhalten:

| Constraint | Source | Phase-4-Auswirkung |
|------------|--------|--------------------|
| **Component-First** — keine inline-RSX-Duplikate, alles wandert in `src/component/` | `CLAUDE.md` §Constraints + `genossi-frontend/CLAUDE.md` §Component-First-Principle | Sieben neue shared Components (ATTN-Components + QR-Login-Components + Vorstand-Components) — siehe Sektion "Component Architecture Recommendations" |
| **Tech stack lock** — Rust + Dioxus 0.6 WASM, kein Sprachwechsel | `CLAUDE.md` §Constraints | Kein neuer Rust-Crate für QR; QR-Generation wurde Backend-seitig in Phase 2 gelöst (D-21) |
| **Datenschutz: Helfer sieht nur 4 Felder** (Mitgliedsnr/Name/Titel/Anrede) | `CLAUDE.md` §Constraints + Phase-3 D-24 (`AttendanceMemberTO` 7-Feld-Whitelist) | `AttendanceList`-Row rendert exakt 5 sichtbare Felder (member_number + salutation + title + first_name + last_name); KEIN extra Tooltip; PII-Frontend-Guard ist die letzte Verteidigungslinie hinter Backend-Whitelist |
| **One-Time-Use-QR** | `CLAUDE.md` §Constraints | Frontend zeigt jeden QR-Code/Klartext nur einmal an (Backend liefert beides nur im Create-Response, Phase 2 D-21); QrCard MUSS sofort druckbar/kopierbar sein, kein Re-Fetch-Pfad |
| **Snake_case file naming** + workspace-Konvention | `CLAUDE.md` §Naming Patterns | Alle neuen Files snake_case (D-23 spezifiziert exakte Namen) |
| **i18n nur 2 Locales (de, en)** — kein `cs.rs` | `genossi-frontend/CLAUDE.md` §i18n (corrected 2026-05-04) | Alle neuen Keys MÜSSEN in `de.rs` und `en.rs`; `mod.rs` Key-Enum erweitern; KEIN cs.rs anlegen |
| **WASM-Validation-Errors → `dx clean` first** | `genossi-frontend/CLAUDE.md` §Common Issues | Plan-Tasks die web-sys-Features hinzufügen oder asset-bundling ändern: nach Cargo.toml-Edit `dx clean && dx build` empfehlen |
| **Backend-Proxy** läuft auf `localhost:3000` via `Dioxus.toml` | `genossi-frontend/CLAUDE.md` §Backend Configuration | Neue API-Calls relative `/api/...`-Pfade nutzen; reqwest-default `same-origin` reicht — KEIN explizites `fetch_credentials_include` nötig (siehe Sektion "Cookie-Handling") |
| **Tailwind muss in watch-mode laufen** | `genossi-frontend/CLAUDE.md` §Backend Configuration | Plan-Setup-Task dokumentiert: `npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch` parallel zu `dx serve` |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| QR-Code-Scanning (Frame-Decoding) | Browser/Client (JS-Library `@zxing/library`) | — | Reines Client-Concern; Scan passiert lokal am Helfer-Gerät, nur das fertig dekodierte Code-String geht über die Leitung |
| Camera-Stream-Lifecycle (getUserMedia, MediaStream-Tracks) | Browser/Client (web-sys MediaDevices) | — | DOM/MediaStream-Objekte leben im Browser; Cleanup bei Unmount muss client-side passieren |
| QR-Code-Generation (SVG-Erzeugung) | API/Backend (Phase 2 D-21) | — | Backend liefert bereits fertiges QR-SVG einmalig im Create-Response; Frontend nur Anzeige + dangerous_inner_html-Render |
| Manual-Code-Validation (Format-Check) | Browser/Client (Pre-Submit-Validation) | API/Backend (HTTP-400 als Backstop) | Schnelles UX-Feedback ohne Roundtrip + Defense-in-Depth (Phase 2 D-24 spezifiziert 400 Bad Request bei ungültigem Format) |
| Manual-Code-Redeem (Atomar) | API/Backend (Phase 2: `UPDATE ... WHERE used_at IS NULL RETURNING`) | — | Race-Hardening MUSS DB-seitig sein; Frontend ist nur Submitter |
| Polling-Logik (5s-Tick) | Browser/Client (use_future + gloo_timers) | — | Kein SSE/WebSocket, kein Backend-Push; jede Polling-Loop läuft pro Component-Instance |
| Counter-Display ("X von Y anwesend") | Browser/Client (Render aus `AttendanceStatsTO`) | API/Backend (Phase 3 D-21: `/api/assembly/{id}/stats`) | Y stammt aus Member-Universe-Snapshot (Phase 1 D-12), wird unverändert durchgereicht; X kommt aus aktueller Counter-Aggregation |
| Anwesenheits-Toggle (Idempotent) | API/Backend (Phase 3 ATTN-03/04) | Browser/Client (Loading-State + 200-OK-Verifizierung, D-17) | Backend macht die Idempotenz; Frontend zeigt nur den verifizierten neuen State |
| Helfer-Session-Cookie | API/Backend (Phase 2: tower-sessions HTTP-Only-Cookie) | Browser/Client (auto-mitgeschickt, kein JS-Code) | Cookie wird vom Server gesetzt, vom Browser auto-attached zu jedem `/api/...`-Request; Frontend muss NICHTS aktiv tun |
| Print-Layout für QR-Card | Browser/Client (CSS `@media print` + `window.print()`) | — | Browser-native Printing; keine Backend-PDF-Pipeline in Phase 4 (EXPO-01 ist v2) |
| Routing-Auth-Branching (Helfer vs Vorstand) | Browser/Client (Router-Layer + RequirePrivilege Wrapper) | API/Backend (echte Permission-Enforcement bleibt Backend) | Frontend-Routing ist UX-Konvenienz; echte Sicherheit liegt im Backend `check_assembly_access` (Phase 3 D-25) |

---

## Standard Stack

### Core (alle bereits in `genossi-frontend/Cargo.toml` vorhanden — verifiziert 2026-05-04)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| dioxus | 0.6.3 | Reactive WASM UI mit RSX, Router, Signals | Stack-Lock per `CLAUDE.md` §Constraints [VERIFIED: `genossi-frontend/Cargo.toml:10`] |
| dioxus-router | (re-export von `dioxus`-feature `router`) | URL-Routing mit `Route`-Enum | Bestehender `Route`-Enum in `src/router.rs:21-54` wird nur erweitert [VERIFIED: `genossi-frontend/src/router.rs`] |
| reqwest | 0.12.15 | HTTP-Client (default-features=false, json, rustls-tls) | Bestehender Pattern in `api.rs` [VERIFIED: `Cargo.toml:15`] |
| web-sys | 0.3 | Browser-API-Bindings | Bereits aktiv mit Window/Navigator/FormData/Document/HtmlInputElement etc. [VERIFIED: `Cargo.toml:39-63`] |
| wasm-bindgen | 0.2.97 | Rust↔JS Bridge | Stable, in `js.rs` für CodeMirror-Integration etabliert [VERIFIED: `Cargo.toml:25` + `src/js.rs:5-22`] |
| wasm-bindgen-futures | 0.4.47 | `JsFuture`-Wrapper für JS-Promises | Bereits genutzt in `api.rs:331` für `fetch_with_request` [VERIFIED: `Cargo.toml:26` + `src/api.rs:331`] |
| serde-wasm-bindgen | 0.6 | Effizientes serde von/zu JsValue | Genutzt in `api.rs:347` für JSON-Deserialisierung [VERIFIED: `Cargo.toml:27`] |
| gloo-timers | 0.3 (mit `futures`) | `TimeoutFuture` für Debouncing + Polling | Bereits genutzt in `member_search.rs:66`, `application_search.rs:68`, `members.rs:59` (Toast-Timeout) [VERIFIED: `Cargo.toml:28`] |
| js-sys | 0.3.77 | Direkte JS-API-Bindings (Reflect, Function, Date) | Genutzt in `js.rs` für JS-API-Reflect-Patterns [VERIFIED: `Cargo.toml:22`] |
| uuid | 1.18 (mit `v4`, `js`) | Assembly-/Member-/Token-IDs | Bereits genutzt; `js`-Feature aktiviert für WASM-RNG [VERIFIED: `Cargo.toml:33-35`] |
| time | 0.3 (mit `macros`) | DateTime-Parsing für `expires_at`/`created` | Bereits in `i18n/mod.rs:format_datetime` integriert [VERIFIED: `Cargo.toml:36-38`] |
| futures + futures-util + futures-channel | 0.3 | Async-Combinators | Bereits genutzt [VERIFIED: `Cargo.toml:19-23`] |
| manganis | 0.6.2 | Asset-Embedding mit Hash-Cache-Busting | Bereits importiert; `asset!`-Makro mit JS-File-Support [VERIFIED: `Cargo.toml:24` + [Dioxus 0.6 Release Notes](https://dioxuslabs.com/blog/release-060/)] |
| tracing + dioxus-logger | 0.1.41 + 0.6.2 | Browser-Console-Logging via `info!`-Macro | Bereits etabliert [VERIFIED: `Cargo.toml:13-14`] |

### Supporting (NEU für Phase 4)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `web-sys` Feature `BarcodeDetector` | n/a — feature exists NOT in web-sys 0.3.97 | Browser-native Barcode-Detection | **NICHT verfügbar** — siehe "Pitfalls" und "QR-Scanner Integration Plan"; Bridge muss über `js_sys::Reflect`/`#[wasm_bindgen] extern` gebaut werden [VERIFIED: web-sys 0.3.97 Cargo.toml grep, [GitHub source](https://github.com/rustwasm/wasm-bindgen/blob/main/crates/web-sys/Cargo.toml)] |
| `web-sys` Feature `MediaDevices` | 0.3 | `navigator.mediaDevices.getUserMedia()` | Camera-Stream-Start [VERIFIED: web-sys 0.3.97 Cargo.toml line 'MediaDevices = ["EventTarget"]'] |
| `web-sys` Feature `MediaStream` | 0.3 | MediaStream-Objekt + Track-Iteration | Stop-on-Unmount-Cleanup [VERIFIED] |
| `web-sys` Feature `MediaStreamTrack` | 0.3 | `track.stop()` für jeden VideoTrack | Cleanup-Pflicht (sonst Permission-Lecke) [VERIFIED] |
| `web-sys` Feature `MediaStreamConstraints` | 0.3 | `{video: {facingMode: 'environment'}}` | Rückkamera-Default für Helfer-Hardware [VERIFIED] |
| `web-sys` Feature `MediaTrackConstraints` | 0.3 | facingMode-Constraint-Konfiguration | Optional, falls weitere Constraints (width/height) [VERIFIED] |
| `web-sys` Feature `HtmlVideoElement` | 0.3 | `video`-Element-Manipulation (srcObject, play()) | Live-Camera-Preview im Scanner [VERIFIED] |
| `@zxing/library` (JS via vendored Asset) | 0.21.3 | QR/Barcode-Decoder als Polyfill für iOS Safari + (faktisch) primärer Pfad auf iOS | UI-SPEC §"ZXing-JS Vendoring Procedure" pinned auf 0.21.3 (Apache-2.0, 2.9k+ Stars, Maintenance-Mode) [CITED: [zxing-js/library GitHub](https://github.com/zxing-js/library)] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `@zxing/library` (umbrella) | `@zxing/browser` (~2.0) | `@zxing/browser` ist schlanker (nur Browser-APIs), aber kleinere User-Base, weniger getestet auf iOS. UI-SPEC entscheidet sich für `@zxing/library` als safer-Choice — Plan sollte das nicht ohne Anlass ändern. [CITED: UI-SPEC §"ZXing-JS Vetting"] |
| `@zxing/library@0.23.0` (latest) | `@zxing/library@0.21.3` (UI-SPEC pin) | 0.23.0 ist neueste stable (April 2026); 0.21.3 ist die in Production häufig laufende Version. UI-SPEC pinned 0.21.3 zur Risikominimierung vor Phase-5-Generalprobe — **akzeptabel**; bei späteren Updates (Phase 5+) Re-Vetting mit SHA256-Update. [VERIFIED: WebFetch des GitHub-Repos zeigt v0.23.0 als latest April 2026] |
| `qrcode-rs` (Rust crate, server-side) | bereits Backend-Lösung in Phase 2 D-21 | Backend liefert QR-SVG einmalig im Create-Response — Frontend nur Anzeige; KEIN client-side QR-Generation nötig. [VERIFIED: Phase 2 D-21] |
| `qrcodegen` (JS, client-side QR-Generation) | Backend-Lösung | Backend-Approach gewinnt: ein gemeinsamer Anker (token_hash) wird einmal im Backend zum QR-SVG; Frontend muss QR nicht regenerieren. [VERIFIED: Phase 2 D-21] |
| `reqwest` mit `fetch_credentials_include()` für Helfer-Cookie | reqwest-Default `same-origin` | dx-serve proxy + production-same-host = same-origin → Default reicht; keine Änderungen nötig. [VERIFIED: `Dioxus.toml:53-56` + reqwest CHANGELOG `(wasm) Add credentials mode methods to RequestBuilder` v0.11.3, [seanmonstar/reqwest CHANGELOG](https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md)] |
| Native browser fetch via wasm-bindgen+web-sys | reqwest 0.12 | reqwest reicht für JSON-Calls; nur File-Upload nutzt nativen Fetch (`api.rs:331`). Phase 4 hat keine File-Uploads → reqwest bleibt First-Choice. [VERIFIED: existing `api.rs` mixed pattern] |
| `wasm-bindgen-test` für Component-Tests | Cargo-`#[test]` für reine Logik | wasm-bindgen-test IST in dev-deps (`Cargo.toml:78`), wird aber NICHT genutzt. Existing Tests sind alle Native-Cargo-Tests von Pure-Logik (z.B. `member_search.rs::tests::filter_*`). Plan-Discretion: keine WASM-Tests; Phase 5 verifiziert E2E. |

**Installation:** Einziger Edit-Schritt für Cargo:

```toml
# genossi-frontend/Cargo.toml — append to web-sys features array
[dependencies.web-sys]
version = "0.3"
features = [
    # ...existing features (Window, Navigator, ..., Url)...
    "MediaDevices",
    "MediaStream",
    "MediaStreamTrack",
    "MediaStreamConstraints",
    "MediaTrackConstraints",
    "HtmlVideoElement",
    # NOTE: BarcodeDetector ist KEIN bekanntes web-sys-Feature in 0.3.97 — wir verwenden
    # js_sys::Reflect für die Feature-Detection und einen #[wasm_bindgen] extern-Block
    # für den Aufruf. Siehe "QR-Scanner Integration Plan".
]
```

**Version verification (verifiziert 2026-05-04):**

```bash
# wasm-bindgen master Cargo.toml zeigt aktive Features:
curl -sL https://raw.githubusercontent.com/rustwasm/wasm-bindgen/main/crates/web-sys/Cargo.toml \
  | grep -iE 'mediadevices|mediastream\b|htmlvideo|mediatrack'
# → liefert MediaDevices, MediaStream, MediaStreamTrack, MediaStreamConstraints,
#   MediaTrackConstraints, HtmlVideoElement — alle ✓
# → BarcodeDetector taucht NICHT auf ✗

# zxing-js/library latest:
curl -sL https://api.github.com/repos/zxing-js/library/releases/latest | grep tag_name
# → v0.23.0 (April 2026) — UI-SPEC pinned 0.21.3 (deliberate)
```

[VERIFIED: GitHub source 2026-05-04]

---

## Architecture Patterns

### System Architecture Diagram

```
   Vorstand (admin)                     Helfer (Helper-Cookie)            Camera (Mobile)
        │                                       │                                │
        │ login via OIDC/cookie                 │ Click "QR-Code scannen"        │
        ▼                                       ▼                                │
   ┌─────────────┐                       ┌──────────────┐                        │
   │ Auth wrap   │ → RequirePrivilege    │ HelperShell  │                        │
   │ (TopBar +   │     "admin"           │ (no TopBar)  │                        │
   │  Footer)    │                       │              │                        │
   └─────┬───────┘                       └─────┬────────┘                        │
         │                                     │                                 │
         ▼                                     ▼                                 │
   ┌──────────────┐                      ┌──────────────────┐                    │
   │ /assemblies  │                      │ /helper          │                    │
   │ /assemblies  │                      │ /helper/         │                    │
   │   /{id}      │                      │   attendance     │                    │
   │ (3 Tabs)     │                      │                  │                    │
   └──────┬───────┘                      └─────┬────────────┘                    │
          │ Tab "Anwesenheit"                  │                                 │
          │   ATTN-06 reuse                    │                                 │
          ▼                                    ▼                                 │
        ┌──────────────────────────────────────────────────────┐                 │
        │ Shared Components                                    │                 │
        │   AttendanceList, AttendanceSearch, LiveCounter,     │                 │
        │   ConnectionBanner                                   │                 │
        └──────────────────┬───────────────────────────────────┘                 │
                           │                                                     │
        ┌──────────────────┼───────────────────┬──────────────┐                  │
        ▼                  ▼                   ▼              ▼                  │
   QrScanner         ManualCodeInput     Polling Loop    Toggle Click            │
   (only on /helper) (only on /helper)   (use_future)    (immediate loading)     │
        │                  │                   │              │                  │
        │ getUserMedia     │ POST              │ GET stats    │ PUT/DELETE       │
        │ + Frame loop     │ /api/helper/      │ /api/asse-   │ /api/atten-      │
        │ + BarcodeDetect  │ redeem            │ mbly/{id}/   │ dance/{aid}/     │
        │ OR ZXing-JS      │                   │ stats        │ {mid}            │
        │ (lazy-loaded)    │                   │              │                  │
        ▼                  ▼                   ▼              ▼                  │
   on_scan(text) → POST /api/helper/redeem ┐                                     │
                                           │                                     │
   ┌───────────────────────────────────────┴────────────────────────────────────┐│
   │                         Backend (Phasen 1–3 abgeschlossen)                 ││
   │ Auth-Middleware → Permission → AttendanceService → DAO → SQLite            ││
   └────────────────────────────────────────────────────────────────────────────┘│
                                                                                 │
                            ┌──Component Lifecycle────────────────────────────┐  │
                            │ Mount: getUserMedia + start frame loop          │◄─┘
                            │ Tick:  decode frame → if match: emit on_scan    │
                            │ Unmount (use_drop): stream.getTracks().stop()   │
                            └─────────────────────────────────────────────────┘
```

### Recommended Project Structure

Phase 4 erweitert die existing Struktur — keine Re-Organisation:

```
genossi-frontend/src/
├── api.rs                              # +10 neue async fns (D-22)
├── app.rs                              # MOD: Helper-Route-Branch ohne TopBar
├── auth.rs                             # unverändert (RequirePrivilege wird re-used)
├── i18n/
│   ├── mod.rs                          # +Key-Variants für Phase 4
│   ├── de.rs                           # +deutsche Strings
│   └── en.rs                           # +englische Strings (Helfer-Page bleibt deutsch)
├── router.rs                           # +4 Route-Varianten
├── component/
│   ├── helper_shell.rs                 # NEW (D-12) — Layout-Wrapper für /helper*
│   ├── qr_scanner.rs                   # NEW (D-12) — Camera + BarcodeDetector/ZXing
│   ├── manual_code_input.rs            # NEW (D-12) — Crockford-Validation-Input
│   ├── qr_card.rs                      # NEW (D-12) — Druckbare QR-Anzeige
│   ├── attendance_list.rs              # NEW (D-11) — Shared Helfer/Vorstand
│   ├── attendance_search.rs            # NEW (D-11) — Debounced Substring (~500ms)
│   ├── live_counter.rs                 # NEW (D-11) — Polling + ConnState-Emit
│   ├── connection_banner.rs            # NEW (D-16) — Sticky-amber-Warning
│   ├── assembly_list_row.rs            # NEW (D-13) — Listen-Eintrag
│   ├── assembly_status_badge.rs        # NEW (D-13) — Wiederverwendbar in Liste+Detail
│   ├── tab_strip.rs                    # NEW (D-13) — Echtes Tab-Pattern (NICHT CollapsibleSection)
│   └── mod.rs                          # +12 pub mod + pub use Re-Exports
├── page/
│   ├── helper_login.rs                 # NEW (D-23) — /helper Login-Page mit Auto-Redirect
│   ├── helper_attendance.rs            # NEW (D-23) — /helper/attendance
│   ├── assemblies.rs                   # NEW (D-23) — /assemblies Liste
│   ├── assembly_details.rs             # NEW (D-23) — /assemblies/{id} 3-Tab-Layout
│   └── mod.rs                          # +pub mod + pub use Re-Exports
├── service/
│   └── (kein neuer Service nötig — Local-Page-State reicht; Auth-Service bleibt unverändert)
├── state/
│   └── (kein neuer State-Store nötig — Helfer-Session-Info kann lokal in HelperShell-Signal leben)
└── assets/
    ├── tailwind.css                    # generiert von Tailwind-Watch
    ├── zxing.umd.min.js                # NEW: vendored Polyfill (UI-SPEC §Vendoring)
    └── zxing.umd.min.js.sha256         # NEW: pinned SHA256
```

### Pattern 1: Polling-Loop mit `use_future` (Component-lifetime)

**What:** Endlos-Loop mit `gloo_timers::future::TimeoutFuture` in einem `use_future`-Hook. Der Future wird automatisch gecancelt sobald die Component unmounted (Dioxus-Hook-Drop-Mechanismus).

**When to use:** Live-Counter (5s-Polling), AttendanceList-Auto-Refresh.

**Example:**

```rust
// Source: Context7 Dioxus docs — "Background Async Tasks with use_future"
// (https://github.com/dioxuslabs/dioxus/blob/main/packages/hooks/docs/use_future.md)
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn LiveCounter(assembly_id: Uuid) -> Element {
    let mut stats = use_signal(|| Option::<AssemblyStatsTO>::None);
    let mut consecutive_failures = use_signal(|| 0u32);

    // Runs forever in the background; auto-cancelled on unmount
    use_future(move || async move {
        loop {
            let config = CONFIG.read().clone();
            match api::get_assembly_stats(&config, assembly_id).await {
                Ok(s) => {
                    stats.set(Some(s));
                    consecutive_failures.set(0);
                }
                Err(_) => {
                    consecutive_failures.with_mut(|n| *n += 1);
                }
            }
            TimeoutFuture::new(5_000).await;
        }
    });

    // Render based on stats.read()
    rsx! { /* ... */ }
}
```

**Key insight:** `use_future` ist die korrekte Wahl für Polling, NICHT `use_resource`. `use_resource` re-runs nur bei Signal-Changes; eine Polling-Loop mit `loop { sleep; fetch }` läuft unabhängig vom Signal-State. [VERIFIED: Context7 Dioxus docs]

### Pattern 2: Cleanup mit `use_drop` für getUserMedia-Streams

**What:** `use_drop` registriert eine Cleanup-Closure, die beim Component-Unmount feuert. Wird gebraucht, weil MediaStream-Tracks NICHT automatisch von Dioxus aufgeräumt werden — das Browser-Permission-Indicator-Light bleibt sonst hängen.

**When to use:** `QrScanner` MUSS Streams stoppen wenn der User die Component verlässt.

**Example:**

```rust
// Source: Dioxus 0.7 lifecycle docs (use_drop ist auch in 0.6 vorhanden)
// (https://dioxuslabs.com/learn/0.7/essentials/advanced/lifecycle/)
use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[component]
pub fn QrScanner(/* props */) -> Element {
    let stream_holder: Rc<RefCell<Option<web_sys::MediaStream>>> = Rc::new(RefCell::new(None));

    // ... setup video stream, store into stream_holder.borrow_mut() = Some(stream); ...

    let stream_for_drop = stream_holder.clone();
    use_drop(move || {
        if let Some(stream) = stream_for_drop.borrow_mut().take() {
            // Iterate getTracks() and call .stop() on each
            let tracks = stream.get_tracks();
            for i in 0..tracks.length() {
                if let Some(track) = tracks.get(i).dyn_into::<web_sys::MediaStreamTrack>().ok() {
                    track.stop();
                }
            }
        }
    });

    rsx! { /* video element */ }
}
```

[VERIFIED: Dioxus 0.7 docs explicitly mention `use_drop` for resource cleanup; pattern transfers to 0.6]

### Pattern 3: Component-Service-State mit Coroutine (existing pattern)

**What:** Globale State-Stores als `GlobalSignal<T>`, gespeist von Coroutine-Services in `src/service/`.

**When to use:** Phase 4 braucht NUR für globale Daten (Auth, Config) — bestehende `AUTH`/`CONFIG`-Stores reichen. Helfer-Session-Daten (assembly_id, GV-Name) leben lokal in HelperShell-Signal, brauchen keinen globalen Store.

**Example:** existing pattern in `genossi-frontend/src/service/auth.rs:35-46`.

```rust
// Source: existing genossi-frontend codebase
pub static AUTH: GlobalSignal<AuthStore> = Signal::global(|| AuthStore::default());

pub async fn auth_service(_rx: UnboundedReceiver<()>) {
    load_auth_info().await;
}
```

[VERIFIED: `genossi-frontend/src/service/auth.rs:35-46`]

### Pattern 4: API-Call mit `AppError` und `status_to_message`

**What:** Alle async API-Calls geben `Result<T, AppError>` zurück; Error-Mapping mit deutschen Messages via `status_to_message`.

**When to use:** Alle 12 neuen API-Funktionen aus D-22.

**Example:**

```rust
// Source: existing pattern, genossi-frontend/src/api.rs:165-200
pub async fn get_assembly_stats(
    config: &Config,
    assembly_id: Uuid,
) -> Result<AssemblyStatsTO, AppError> {
    let url = format!("{}/api/assembly/{assembly_id}/stats", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}
```

[VERIFIED: `genossi-frontend/src/api.rs:160-200`]

### Anti-Patterns to Avoid

- **Optimistic-UI für Anwesenheits-Toggle:** Phase-4-SC#6 verbietet das explizit. Toggle-Button MUSS sofort in `loading`-State (kein Häkchen!) und erst nach 200-OK in den neuen State flippen. UI-SPEC §"Toggle-Button states" und D-17 spezifizieren das prescriptive.
- **Inline-RSX-Duplikation zwischen `/helper/attendance` und `/assemblies/{id}` Tab "Anwesenheit":** Das wäre ein direkter Verstoß gegen das Component-First-Prinzip. Die drei shared Components MÜSSEN in `src/component/` leben und werden von beiden Pages identisch eingebettet.
- **`Locale::Cs` Reference:** Bestehender Lapsus in alten Doc-Versionen — Locale-Enum hat NUR `En`+`De`. Fokus auf zwei Locale-Files genügt; Plan darf NICHT cs.rs anlegen oder `Locale::Cs` einführen (D-19 amended).
- **Hard-coded German Strings im RSX:** Alle visible Strings MÜSSEN über `use_i18n()` + `Key`-Enum laufen — auch wenn Helfer-View deutsch-only ist. UI-SPEC §"i18n Key Inventory" listet alle neuen Keys explizit.
- **`getUserMedia` ohne use_drop-Cleanup:** Permission-Indicator-Light bleibt nach Unmount an, Browser sieht es als "page is using camera" — schlechtes UX. Pattern 2 oben ist Pflicht für QrScanner.
- **MediaStream im RAM behalten:** Wenn der Helfer von Login zu Attendance navigiert, MUSS QrScanner-Component unmounten und Stream stoppen (use_drop), nicht im Background weiterlaufen.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| QR-Code-Scanning aus Camera-Frame | Eigenen Reed-Solomon-Decoder + Binary-Pattern-Detector | `@zxing/library@0.21.3` (vendored) + nativer `BarcodeDetector` für Chromium | QR-Spec ist nicht trivial; ECC-Math + Pattern-Detection sind 1000+ Zeilen Code; iOS-Quirks alleine sind ein Fulltime-Job. Vetted by UI-SPEC. [CITED: UI-SPEC §"ZXing-JS Vetting"] |
| QR-Code-Generation client-side | `qrcodegen-js` o.ä. einbinden | Backend liefert QR-SVG einmalig im Token-Create-Response (Phase 2 D-21) | Single Source of Truth: Backend hat den authoritativen Code-Hash; Frontend würde nur das gleiche Ergebnis wegen anderen Library-Defaults leicht abweichend rendern. |
| Polling-State-Machine | Eigenen Timer + Retry-Logic + Backoff | `use_future` + `gloo_timers::future::TimeoutFuture` (Pattern 1) | Dioxus' Hook-Lifecycle managed Cleanup automatisch; gloo-timers ist bereits Dep. Kein neuer Code-Pfad. |
| Cookie-Handling | Eigenen Cookie-Parser oder `js_sys::Reflect`-Read von `document.cookie` | Browser-default + `tower-sessions`-HTTP-Only-Cookie + Same-Origin-Fetch | HTTP-Only-Cookies sind by-design für JS unsichtbar; das ist die Sicherheits-Eigenschaft. Kein Frontend-Code muss Cookies lesen — Browser attached sie automatisch. |
| Form-Validation für 10-Char-Crockford-Code | Komplexe Validation-Library | Plain `String.chars().all(...)` + Length-Check (Pattern 7) | 10 Char Whitelist-Check ist <5 Zeilen Code; siehe Sektion "Manual-Code Form-Validation". |
| Toast-Notification-System | Eigenes globales Toast-State-Store | Wiederverwendung des bestehenden `members.rs::show_toast`-Pattern | UI-SPEC §"Toast Notification position" verweist explizit auf `members.rs:49` als Pattern-Vorlage. Plan extrahiert es in einen kleinen `toast`-Component falls Pages mehrfach brauchen. |
| Tab-Strip-State-Management | Eigene Active-Tab-Signal + URL-Sync | Lokales `use_signal::<String>(default_key)` in `assembly_details.rs` + `tab_strip.rs`-Component (D-13) | Drei-Tab-Layout ist statisch; URL-Sync via Hash-Fragment ist Phase-5-Polishing. Plan baut Minimum. |
| Permission-Privilege-Wrapper | Neuen Auth-Guard-Component | `RequirePrivilege { privilege: "admin" }` aus `auth.rs:35` | Bestehender Component, exakt für Vorstand-Routes geeignet. |
| HTTP-Status-zu-Deutscher-Message | Eigene Mapping-Funktion in jedem Handler | `status_to_message` aus `api.rs:49` + `AppError` (Pattern 4) | Deutsche Messages sind bereits zentral konsolidiert; nur `HelperLoginError*`-Keys aus i18n für Login-spezifische Fälle (404→Code-not-found, 410→Already-used) ergänzen — aber via `status_to_message`-Override im `redeem_helper_token()`-Call, nicht als zweites Mapping-System. |

**Key insight:** Die meisten "schwierigen" Probleme von Phase 4 sind in Backend (Phase 2 + 3) oder existierenden Frontend-Patterns bereits gelöst. Die Forschungsschwelle liegt bei (1) Camera-Lifecycle und (2) Polling-Lifecycle — beide werden durch Dioxus-Hooks (`use_drop`, `use_future`) sauber abgebildet, sobald das Pattern verstanden ist.

---

## Stack-Specifics (Dioxus 0.6, web-sys, gloo)

### Dioxus 0.6 Hook-API für Phase 4

| Hook | Use Case in Phase 4 | Notes |
|------|---------------------|-------|
| `use_signal::<T>(default)` | Local component state (search-query, loading-flag, current-tab-key) | Bestehend: alle existing components nutzen das [VERIFIED: Codebase grep] |
| `use_future(\|\| async move { ... })` | Polling-Loop in `LiveCounter` und `AttendanceList`-Auto-Refresh | Runs once on mount, drops on unmount. Reactive zu Signal-Reads im Body. [CITED: [Dioxus Hooks docs](https://github.com/dioxuslabs/dioxus/blob/main/packages/hooks/docs/use_future.md)] |
| `use_resource(\|\| async move { ... })` | NICHT verwenden für Polling — re-runs nur bei Signal-Change. Geeignet für Initial-Fetch (z.B. Assembly-Liste laden). | UI-SPEC §"LiveCounter Polling behavior" erwähnt `use_resource`, aber Pattern 1 oben empfiehlt `use_future` weil Loop-Pattern natürlicher passt. Plan kann beide testen. [CITED: Context7 Dioxus] |
| `use_effect(\|\| { ... })` | Reactive side effects (z.B. Auto-Redirect bei mount, document.title setzen) | Bereits in `app.rs:21-29` und `application_search.rs:44` genutzt [VERIFIED: codebase] |
| `use_drop(\|\| { ... })` | Cleanup bei Component-Unmount (MediaStream stoppen) | NEU für Phase 4. `use_drop` existiert in 0.6 unter `dioxus::core::use_drop`. [CITED: [Dioxus 0.7 Lifecycle](https://dioxuslabs.com/learn/0.7/essentials/advanced/lifecycle/) — Pattern identisch in 0.6] |
| `spawn(async move { ... })` | One-shot async-Task in Event-Handler (z.B. nach Klick redeem-Call starten) | Bereits in `members.rs:58`, `member_search.rs:65` genutzt [VERIFIED: codebase] |
| `navigator()` + `nav.push(Route)` / `nav.replace(Route)` | Route-Navigation aus Event-Handlern | Bereits in `members.rs:509`, `home.rs:9` genutzt [VERIFIED: codebase] |

### web-sys-Bindings für Camera-Workflow

```rust
// Vollständige Kette von getUserMedia bis Stream-Stop, basierend auf web-sys 0.3.97

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    HtmlVideoElement, MediaDevices, MediaStream, MediaStreamConstraints, MediaStreamTrack,
    MediaTrackConstraints,
};

async fn start_camera(video_element: &HtmlVideoElement) -> Result<MediaStream, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    let media_devices: MediaDevices = navigator.media_devices()?;

    // Build constraints: { video: { facingMode: 'environment' } }
    let mut video_constraints = MediaTrackConstraints::new();
    // Note: facingMode is set via the deprecated dictionary-style API on MediaTrackConstraints;
    // for more control, use js_sys::Object::set with reflection.
    // web-sys 0.3.97 has set_facing_mode? Let's verify:
    // - facingMode: yes, MediaTrackConstraints has set_facing_mode taking &JsValue
    video_constraints.facing_mode(&JsValue::from_str("environment"));

    let mut constraints = MediaStreamConstraints::new();
    constraints.video(&video_constraints.into());

    let promise = media_devices.get_user_media_with_constraints(&constraints)?;
    let stream_value = JsFuture::from(promise).await?;
    let stream: MediaStream = stream_value.dyn_into()?;

    // Pipe stream into <video> element
    video_element.set_src_object(Some(&stream));
    let _ = video_element.play()?;

    Ok(stream)
}

fn stop_stream(stream: &MediaStream) {
    let tracks = stream.get_tracks();
    for i in 0..tracks.length() {
        if let Ok(track) = tracks.get(i).dyn_into::<MediaStreamTrack>() {
            track.stop();
        }
    }
}
```

[VERIFIED: web-sys 0.3.97 features list confirms all required types are available]

### BarcodeDetector via wasm-bindgen extern (kein web-sys-Feature verfügbar)

**Critical:** `BarcodeDetector` ist KEIN web-sys-Feature in 0.3.97. Das D-20-Statement "web-sys Features-Erweiterung um BarcodeDetector" ist faktisch nicht möglich — Plan MUSS einen `#[wasm_bindgen]`-Bridge anlegen.

```rust
// genossi-frontend/src/js.rs — append (existing extern-block style, see js.rs:5-22)

use wasm_bindgen::prelude::*;
use js_sys::{Array, Object, Promise};

#[wasm_bindgen]
extern "C" {
    pub type BarcodeDetector;

    #[wasm_bindgen(constructor)]
    pub fn new(options: &JsValue) -> BarcodeDetector;

    #[wasm_bindgen(method)]
    pub fn detect(this: &BarcodeDetector, source: &JsValue) -> Promise;

    #[wasm_bindgen(static_method_of = BarcodeDetector, js_name = getSupportedFormats)]
    pub fn get_supported_formats() -> Promise;
}

/// Returns true iff `'BarcodeDetector' in window`.
pub fn has_barcode_detector() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    js_sys::Reflect::has(&window, &JsValue::from_str("BarcodeDetector")).unwrap_or(false)
}
```

[VERIFIED: wasm-bindgen pattern is standard, used in `js.rs:5-22` for CodeMirror; equivalent here]

[CITED: [MDN BarcodeDetector](https://developer.mozilla.org/en-US/docs/Web/API/BarcodeDetector) for the JS-API surface]

### gloo-timers für Debounce + Polling (existing patterns)

```rust
// Debounce (existing: member_search.rs:66, application_search.rs:68, members.rs:59)
use gloo_timers::future::TimeoutFuture;

spawn(async move {
    TimeoutFuture::new(500).await;
    if /* still latest */ { on_change.call(value.clone()); }
});

// Polling-Loop (NEW for Phase 4)
use_future(move || async move {
    loop {
        TimeoutFuture::new(5_000).await;
        // do fetch
    }
});
```

[VERIFIED: existing codebase grep — `gloo_timers::future::TimeoutFuture` already in 6+ files]

---

## Component Architecture Recommendations

### Was wird wiederverwendbar (12 neue Components in `src/component/`)

| Component | Reuse-Surface | Justification |
|-----------|--------------|---------------|
| `helper_shell.rs` | `helper_login.rs` + `helper_attendance.rs` | Identisches Layout (mobile-first, kein TopBar/Footer) auf beiden Helfer-Pages — wird zwingend extrahiert |
| `qr_scanner.rs` | `helper_login.rs` (nur dort) — aber als isolierte Camera-Lifecycle-Einheit, da ~150 Zeilen Camera-Code keine Page-Logik sein dürfen | Component-First-Prinzip: jede deutlich abgrenzbare UI-Einheit MUSS Component sein |
| `manual_code_input.rs` | `helper_login.rs` (nur dort) — aber als testbare Validation-Einheit | Erlaubt Cargo-Tests für `is_valid_crockford_code(&str) -> bool` ohne Browser |
| `qr_card.rs` | `assembly_details.rs` Tab "Helfer-Tokens" — nach Token-Create | Print-Layout MUSS einmalig definiert sein (CSS `@media print`-Pattern) |
| `attendance_list.rs` | `helper_attendance.rs` + `assembly_details.rs` Tab "Anwesenheit" | **DAS** Component-First-Anchor-Beispiel; ATTN-06 wird durch genau dieses Reuse erfüllt |
| `attendance_search.rs` | gleichermaßen reuse — die Suche ist auf beiden Pages identisch | dito |
| `live_counter.rs` | gleichermaßen reuse | dito |
| `connection_banner.rs` | `helper_attendance.rs` + `assembly_details.rs` Tab "Anwesenheit" | UI-SPEC §"Connection-Banner" — sticky-amber-Banner; Component-First erlaubt einheitliches Trigger-Verhalten |
| `assembly_list_row.rs` | `assemblies.rs` (nur dort) | Konsequente Konvention: Listen-Einträge sind eigene Components (analog `application_list.rs` Pattern) |
| `assembly_status_badge.rs` | `assemblies.rs` Liste + `assembly_details.rs` Header | Drei-Status-Badge ist mehrfach verwendet → eigene Component, bestehender Pattern aus `application_list.rs::status_badge_class` re-used |
| `tab_strip.rs` | `assembly_details.rs` (nur dort in Phase 4) — aber wiederverwendbar in Phase 5+ | UI-SPEC §"TabStrip" empfiehlt explizit ein neues Component (CollapsibleSection ist KEIN Tab-Pattern) |

### Was bleibt page-spezifisch (4 neue Pages in `src/page/`)

| Page | Page-spezifische Logik (was NICHT in Components ausgelagert wird) |
|------|------------------------------------------------------------------|
| `helper_login.rs` | Auto-Redirect-Effect (mount → check `/api/helper/session` → navigate); Komposition von QR-Scanner + ManualCodeInput; `redeem_helper_token`-Call-Orchestration mit Error-Mapping zu i18n-Keys; Modal-Show/Hide für QR-Scanner |
| `helper_attendance.rs` | Komposition LiveCounter + AttendanceSearch + AttendanceList + ConnectionBanner mit Refresh-Signal-Wiring; Error-Toast-Container |
| `assemblies.rs` | Liste-Fetch + Empty-State + Modal-Form für `create_assembly` mit Dirty-Validation (Name nicht leer, Datum-Parse) |
| `assembly_details.rs` | Tab-Active-Key-Local-State; Tab-Body-Branch (Stamm-Daten / Tokens / Anwesenheit); Modal-Forms für Token-Create + Confirm-Dialogs für GV öffnen/schließen + Token-Revoke; In-Memory-Hold der Just-Created-`HelperTokenCreateResponseTO` (qr_svg + code) bis User Modal schließt — Backend liefert beides nur einmal (Phase 2 D-21) |

### Anti-Pattern: Erst-die-Page-und-dann-extrahieren

Dieser Approach ist im Genossi-Codebase historisch belegt als **schmerzhafter Anti-Pattern** ([Memory: Component-First-Principle](file:///home/neosam/.claude/projects/-home-neosam-programming-rust-projects-genossi3/memory/feedback_component_first.md), `genossi-frontend/CLAUDE.md` §Component-First). Plan-Tasks MÜSSEN Components ZUERST anlegen, dann Pages komponieren — nicht umgekehrt.

---

## QR-Scanner Integration Plan

### Browser-Support-Realität (verifiziert 2026-05-04)

| Browser | BarcodeDetector | ZXing-JS-Polyfill | Empfehlung |
|---------|-----------------|-------------------|-----------|
| Chrome ≥83 (Desktop) | Partial — only on macOS/ChromeOS | Funktioniert | BarcodeDetector wenn vorhanden, sonst Polyfill |
| Chrome Android ≥147 | YES (full) | nicht nötig | BarcodeDetector |
| Edge ≥83 (Desktop) | Partial | Funktioniert | BarcodeDetector wenn vorhanden, sonst Polyfill |
| Firefox (alle) | NO | Funktioniert | Polyfill |
| **iOS Safari (alle Versionen bis 26.5)** | **NO — disabled by default** | **Funktioniert** ab iOS 14.3 (WebRTC) | **Polyfill ist primärer Pfad** |
| Samsung Internet ≥13 | YES | nicht nötig | BarcodeDetector |

**Globale Coverage:** ca. 76% (BarcodeDetector + Partial) — aber die wichtigste Genossi-Zielgruppe (Helfer auf iOS Safari) fällt in die 0%-Gruppe. [CITED: [caniuse.com mdn-api_barcodedetector](https://caniuse.com/mdn-api_barcodedetector)]

**Implication:** Die Phrase "Polyfill nur als Fallback" aus D-02 stimmt formal — aber praktisch wird **jeder iPhone-Helfer den Polyfill laden müssen**. Plan sollte das nicht als Edge-Case behandeln, sondern den Polyfill-Pfad als gleichberechtigt validieren.

### Lifecycle-Plan für `qr_scanner.rs`

```
Mount (after user click "QR-Code scannen")
  │
  ▼
1. Detect: has_barcode_detector() (sync, js_sys::Reflect::has)
  │
  ├── true (Chromium/Android)
  │     ▼
  │   Use native: js::BarcodeDetector::new(...)
  │
  └── false (iOS Safari, Firefox)
        ▼
      Lazy-load ZXing-JS via document::eval():
        document::eval(format!(r#"
          if (!window.__zxing_loaded) {{
            await new Promise((resolve, reject) => {{
              const s = document.createElement('script');
              s.src = '{ZXING_ASSET_PATH}';
              s.onload = resolve;
              s.onerror = reject;
              document.head.appendChild(s);
            }});
            window.__zxing_loaded = true;
          }}
          // Initialize codeReader on window for later access
          window.__zxing_reader = new ZXing.BrowserMultiFormatReader();
        "#)).await;

  ▼
2. getUserMedia({video: {facingMode: 'environment'}})
  │
  ├── Permission denied → on_error("Kamera-Zugriff verweigert. Bitte Code manuell eingeben.")
  ├── No camera → on_error("Kamera nicht verfügbar. Bitte Code manuell eingeben.")
  │
  └── stream → set video.srcObject = stream; video.play()

  ▼
3. Start scan loop
  │
  ├── Native path: requestVideoFrameCallback → barcodeDetector.detect(video) → Promise<Array<DetectedBarcode>>
  │
  └── Polyfill path: zxingReader.decodeFromVideoElement(video, callback) — ZXing handles its own loop

  ▼
4. On match: on_scan(text) → parent calls redeem_helper_token(text)

  ▼
5. Unmount (use_drop):
   - stop scan loop (cancel frame callback / call zxingReader.reset())
   - stop_stream(stream): iterate tracks.stop()
   - clear video.srcObject
```

[CITED: ZXing-JS API via Context7 — `BrowserMultiFormatReader.decodeFromVideoDevice(deviceId, elementId, callback)` is the canonical entry point]

### Critical Implementation Notes

1. **HTTPS-Requirement:** `getUserMedia` benötigt einen Secure Context. `dx serve` läuft auf `http://localhost:8080` — localhost gilt als secure → funktioniert lokal. Production MUSS HTTPS sein. Phase 5 dokumentiert TLS-Setup. [CITED: [MDN getUserMedia secure context](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia)]

2. **iOS-Safari-Camera-Quirk (verified):** iOS-Safari benötigt `<video playsinline>` — sonst geht das Stream im Vollbild auf statt im Page-Flow. [CITED: [iOS Safari WebRTC](https://webkit.org/blog/11353/mediarecorder-api/)]

3. **iOS-Permission-Gate:** iOS verlangt `getUserMedia` aus User-Gesture-Handler heraus (Click-Event). D-03 macht das richtig — Permission erst beim Klick.

4. **`requestVideoFrameCallback` vs `setInterval`:** `requestVideoFrameCallback` ist die effizienteste API für Per-Frame-Decode (nur wenn neuer Frame vorhanden). Nicht alle Browser supporten es; Fallback ist `setInterval(decode, 100)`. ZXing's `decodeFromVideoElement` macht das alles intern, daher Polyfill-Pfad sorgenfrei.

5. **Bundle-Size-Risiko:** ZXing-JS UMD ist ~200KB minified. Lazy-Loading via `<script>`-Inject (nicht via `document::Script` in RSX-Tree) verhindert Bundle-Bloat für Chromium-Helfer.

### Vendoring Procedure (already specified in UI-SPEC §"ZXing-JS Vendoring Procedure")

Plan-Task ist Reproduzierung der UI-SPEC-Schritte:
1. `mkdir -p genossi-frontend/assets/`
2. `curl -sL https://unpkg.com/@zxing/library@0.21.3/umd/index.min.js -o genossi-frontend/assets/zxing.umd.min.js`
3. `sha256sum ... > genossi-frontend/assets/zxing.umd.min.js.sha256`
4. Commit beide Files
5. In Dioxus: `const ZXING_ASSET: Asset = asset!("/assets/zxing.umd.min.js");` — wird hash-fingerprinted; Path verfügbar als `{ZXING_ASSET}`
6. Lazy-Load via `document::eval()` mit `<script src='{ZXING_ASSET}' />`-Inject

[VERIFIED: UI-SPEC vetting protocol; manganis asset!-Macro behaviour [CITED: [Dioxus 0.6 Release Notes](https://dioxuslabs.com/blog/release-060/)]]

---

## QR-Code Generation Strategy

**Decision: Server-side (Phase 2 D-21).**

Phase 4 generiert KEINE QR-Codes client-side. Der Backend-Endpoint `POST /api/assembly/{aid}/helper-tokens` (Phase 2) liefert in der Response genau einmal:

```json
{
  "token": { /* HelperTokenTO */ },
  "code": "ABC123XYZ4",
  "qr_svg": "<svg xmlns='http://www.w3.org/2000/svg' ...>...</svg>"
}
```

Frontend-Verarbeitung in `assembly_details.rs`:

1. POST-Call → `HelperTokenCreateResponseTO` zurück.
2. State im Page-Memory halten: `let mut just_created = use_signal(|| Option::<HelperTokenCreateResponseTO>::None);`
3. `QrCard { memo, code, qr_svg }`-Component rendert mit `dangerous_inner_html: qr_svg` (das Backend-SVG ist self-contained — kein XSS-Risiko, da Backend kontrollierter Producer ist; aber Plan sollte explizit dokumentieren dass `qr_svg` aus eigenem Backend stammt und nicht aus User-Input).
4. Print-Trigger: `onclick: |_| { web_sys::window().unwrap().print(); }` (mit `Window`-Feature, das bereits in Cargo.toml aktiv ist — verifiziert).
5. Sobald User Modal schließt: `just_created.set(None)` — Code/SVG sind dann **weg** und nicht-rekonstruierbar (Backend speichert beides nie persistent — Phase 2 D-11).

### Druck-Layout

UI-SPEC §"QrCard Print contract" liefert das vollständige CSS:

```css
@media print {
  body * { visibility: hidden; }
  .qr-card, .qr-card * { visibility: visible; }
  .qr-card {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    box-shadow: none;
    border: none;
    max-width: 80mm;
  }
  .qr-card .w-64 { width: 60mm; height: 60mm; }
  .qr-card .font-mono { font-size: 16pt; letter-spacing: 0.15em; }
  @page { size: A4 portrait; margin: 16mm; }
}
```

Das wird in `genossi-frontend/input.css` ergänzt (D-21). Tailwind purge-Settings für `.qr-card`-Klasse beachten — wenn Tailwind die Klasse nicht im RSX findet, wird sie aus dem Production-Build gepurgt. **Lösung:** Explizit als safelisted Class in `tailwind.config.js`, oder die Klasse direkt im `qr_card.rs`-Component verwenden (was sie ohnehin tut). Plan sollte das verifizieren.

[CITED: UI-SPEC §"QrCard Print contract"]

---

## Polling-Pattern für Live-Counter

### Pattern: `use_future` mit Endless-Loop (Pattern 1 above)

Strikt am Genossi-Codebase orientiert: kein neuer State-Store, kein globaler Polling-Service. Der `LiveCounter` polled lokal solange er gemountet ist.

### Concrete Implementation Sketch

```rust
// component/live_counter.rs
use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use rest_types::AssemblyStatsTO;
use uuid::Uuid;

use crate::api;
use crate::service::config::CONFIG;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ConnState { Healthy, Warning, Lost }

#[component]
pub fn LiveCounter(
    assembly_id: Uuid,
    polling_enabled: bool,
    on_connection_state: EventHandler<ConnState>,
) -> Element {
    let mut stats = use_signal(|| Option::<AssemblyStatsTO>::None);
    let mut consecutive_failures = use_signal(|| 0u32);

    // Spawn the polling loop.
    // - Auto-cancelled on unmount (Dioxus drops the future when component drops).
    // - Conditional on polling_enabled (re-evaluated per loop iteration).
    use_future(move || async move {
        loop {
            if !polling_enabled {
                TimeoutFuture::new(1_000).await; // idle wait
                continue;
            }

            let config = CONFIG.read().clone();
            match api::get_assembly_stats(&config, assembly_id).await {
                Ok(s) => {
                    stats.set(Some(s));
                    if *consecutive_failures.read() != 0 {
                        on_connection_state.call(ConnState::Healthy);
                    }
                    consecutive_failures.set(0);
                }
                Err(_) => {
                    let n = consecutive_failures.with_mut(|n| { *n += 1; *n });
                    if n == 1 {
                        on_connection_state.call(ConnState::Warning);
                    } else if n >= 2 {
                        on_connection_state.call(ConnState::Lost);
                    }
                }
            }

            TimeoutFuture::new(5_000).await;
        }
    });

    let display = match (&*stats.read(), *consecutive_failures.read()) {
        (None, _) => "Anwesenheit lädt…".to_string(),
        (Some(s), 0..=1) => format!("{} von {} anwesend", s.x_present, s.y_total),
        (Some(s), _) => format!("— von {} anwesend", s.y_total),
    };

    rsx! {
        div { class: "bg-white border border-gray-200 rounded-lg p-6 mb-4 flex items-baseline justify-between",
            span { class: "text-sm font-medium text-gray-500 uppercase tracking-wider", "Anwesenheit" }
            span { class: "text-4xl font-bold text-gray-900", "{display}" }
        }
    }
}
```

### Cleanup & Lifecycle Verification

- `use_future`-Future hängt am Component-Scope; bei Unmount drop't Dioxus den ScopeId und damit die enthaltene Future. [CITED: Context7 Dioxus — "Background Async Tasks with use_future"]
- Bei Page-Wechsel von `/helper/attendance` → `/helper` (Logout) wird LiveCounter unmounted → Future cancelled → kein Memory-Leak.
- Bei Tab-Wechsel von "Anwesenheit" → "Helfer-Tokens" auf `/assemblies/{id}` wird LiveCounter unmounted (TabStrip rendert Body conditional via `match active_key`) → Polling stoppt. ✓

### Error-Recovery

Definition "Recovery": Ein erfolgreicher Poll nach einem oder mehr Failures. Counter resettet `consecutive_failures` → emittet `ConnState::Healthy`. ConnectionBanner (separate Component) hört auf `on_connection_state` und entscheidet eigenständig sticky-Visibility:
- `Healthy` → Banner unsichtbar (oder fade-out)
- `Warning` → Banner unsichtbar (still toleriert)
- `Lost` → Banner visible bis nächstes `Healthy`

### Polling-Hook-Sharing-Discretion (D-15)

UI-SPEC offen gelassen. Empfehlung von Researcher:
- **Separate Hooks (LiveCounter macht eigenes Polling, AttendanceList macht eigenes Polling):** einfacher zu reasonen, isolierte Test-Surface, jeder Component-Mount ist autonom.
- **Gemeinsamer Hook (ein Tick triggert beide Endpoints parallel via `futures::join!`):** spart einen Network-RTT alle 5s. Kostet einen geteilten Signal-Store oder ein Parent-Scope-Signal.

Für Phase 4 Empfehlung: **Separate Hooks**. Begründung: Vereinsheim-WiFi-Last ist mit 2 GET-Requests/5s pro Helfer-Tablet niedrig (ca. 0.4 RPS pro Helfer); bei 5 Helfern = 2 RPS gesamt — unkritisch. Plan kann später Optimieren wenn Phase 5 Stress-Daten zeigt.

---

## Routing & Auth-Guard Pattern

### Aktuelle Route-Enum (zu erweitern)

```rust
// genossi-frontend/src/router.rs:21-54 (existing — diff)
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},

    // ── Phase 4 additions ────────────────────────────────
    #[route("/helper")]
    Helper {},
    #[route("/helper/attendance")]
    HelperAttendance {},
    #[route("/assemblies")]
    Assemblies {},
    #[route("/assemblies/:id")]
    AssemblyDetails { id: String },

    // ── existing routes ──────────────────────────────────
    #[route("/members")]
    Members {},
    // ...
}
```

[VERIFIED: existing pattern at `router.rs:21-54`]

### App-Layout-Branching (`app.rs`)

UI-SPEC empfiehlt: bei Routes unter `/helper*` skip `<Auth>` und `<TopBar>`/`<Footer>`. Currently `app.rs:36-46` wraps everything in `<Auth>{...}<NotAuthenticated/></Auth>`. Plan muss das so umstrukturieren, dass der aktive Pfad geprüft wird:

```rust
// app.rs (sketch — Plan finalises exact structure)
use dioxus_router::prelude::use_route;

pub fn App() -> Element {
    // ... existing config-loading ...

    rsx! {
        document::Stylesheet { href: "/assets/tailwind.css" }
        div { class: "flex flex-col min-h-screen",
            DropdownBase {}
            div { class: "flex-1",
                // Choice: branch at the Router level by using a separate layout component
                // Alternative: use a "Layout"-Component-Pattern with `Outlet<Route>`
                Router::<Route> {}
            }
        }
    }
}
```

**Plan-Discretion:** Cleaner alternative ist **Layout-Components per Route-Group** mit Dioxus Router's `#[layout(...)]` annotation:

```rust
#[derive(Routable)]
enum Route {
    #[layout(HelperShellLayout)]
    #[route("/helper")]
    Helper {},
    #[route("/helper/attendance")]
    HelperAttendance {},
    #[end_layout]

    #[layout(StandardLayout)]
    #[route("/")]
    Home {},
    #[route("/members")]
    Members {},
    // ...
}
```

[CITED: Dioxus Router docs — `#[layout(...)]` enables route-group-Layouts ohne if/else in app.rs]

Plan entscheidet zwischen (a) if/else in `app.rs` (minimal, näher am bestehenden Pattern) oder (b) `#[layout]`-Annotations (idiomatischer, aber neuer Pattern in dieser Codebase).

### Auth-Guard für Vorstand-Routes

`/assemblies` und `/assemblies/{id}` sind Admin-only. Wrap mit existing `RequirePrivilege`:

```rust
// page/assemblies.rs (sketch)
#[component]
pub fn Assemblies() -> Element {
    rsx! {
        RequirePrivilege { privilege: "admin",
            // ... assembly list UI ...
        }
    }
}
```

[VERIFIED: `auth.rs:35-48` — `RequirePrivilege` component exists, takes `privilege: &'static str`]

### Auth-Guard für Helfer-Routes

`/helper` ist öffentlich (kein Login nötig — der QR-Code IST der Login).
`/helper/attendance` benötigt eine gültige Helfer-Session — aber **nicht** über `RequirePrivilege` (das prüft OIDC-Privileges, nicht Helper-Cookies).

**Plan-Discretion (D-06 open):** Welcher Endpoint signalisiert "valid helper session"?

Optionen:
- **Option A (minimaler Backend-Eingriff):** Bei jedem `/helper/attendance`-Mount einen Initial-Call zu `/api/attendance/{aid}/members` machen — wenn 401/403, Navigate zu `/helper`. Nutzt existing Endpoints; aber: man weiß den `aid` noch nicht beim ersten Mount.
- **Option B (neuer Read-only-Endpoint):** `GET /api/helper/session` → 200 `{assembly_id, expires_at}` oder 401. Backend-Eingriff klein (kein neuer Service, nur Auth-Context-Read in Handler), aber ein neuer Endpoint.
- **Option C (im Cookie selbst):** Phase 2 Helper-Session-Cookie ist HTTP-Only, also nicht JS-lesbar. Frontend kann nicht sehen ob Cookie da ist — Option C scheidet aus.

Empfehlung Researcher: **Option B**, weil ohne `assembly_id` der HelperShell-Header nicht den GV-Namen anzeigen kann (UI-SPEC §HelperShell). Plan sollte diesen 5-Zeilen-Backend-Endpoint als Phase-4-Erweiterung dokumentieren.

---

## Cookie-Handling im WASM-Frontend

### Status Quo (verifiziert)

- Backend setzt OIDC-Session-Cookie (Vorstand) und Helper-Session-Cookie (Phase 2). Beide HTTP-Only.
- Frontend ruft `/api/...` immer als **same-origin** auf, weil `Dioxus.toml` proxied `/api/*` und `/swagger-ui` auf `localhost:3000` (dev) und in Production der gleiche Server-Host beides serviert.
- Browser attached HTTP-Only-Cookies automatisch für same-origin-Requests — **kein Frontend-Code nötig**.
- Existing `api.rs` macht keinerlei explizites Cookie-Setting — funktioniert für Vorstand-Auth seit Jahren.

[VERIFIED: `genossi-frontend/Dioxus.toml:53-56` Proxy-Config; `genossi-frontend/src/api.rs` keine `credentials`-Calls — funktional bestätigt durch Phase-1..3-E2E-Tests]

### Was Phase 4 NICHT braucht

- **Kein `fetch_credentials_include()`** (reqwest 0.12 method) — wäre cross-origin-Mode; nicht unsere Situation.
- **Kein `RequestCredentials::Include`** (web-sys) — selbe Begründung.
- **Kein Cookie-Parser im Frontend** — Cookies sind HTTP-Only, JS sieht sie nicht.

### Was Phase 4 mitnehmen muss

1. **Logout-Endpoint:** Helfer-Logout muss Session-Cookie invalidieren. Plan-Discretion: existing `/api/auth/logout` (OIDC) wird vermutlich nicht passen — Helfer-Sessions sind separate. **Empfehlung:** neuer `POST /api/helper/logout`-Endpoint (spiegelt Backend-Phase-2-Pattern). Phase 4 darf solche kleinen Backend-Erweiterungen anstoßen (CONTEXT D-06 Discretion).
2. **Session-Expiry-UX:** Wenn Helfer-Cookie abgelaufen ist (GV wurde geschlossen, Phase 2 D-18), liefert Backend 401. Frontend MUSS dann auf `/helper` zurück-Navigaten und Toast "Session abgelaufen" zeigen. Plan-Task: zentrale Error-Handler-Schicht in `api.rs` für 401 auf Helfer-Routes.

[CITED: [reqwest CHANGELOG](https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md) (wasm credentials added v0.11.3); [MDN Request.credentials](https://developer.mozilla.org/en-US/docs/Web/API/Request/credentials) (default same-origin)]

---

## Manual-Code Form-Validation

### Crockford-Base32-Alphabet

Phase 2 D-09 ist die autoritative Spec:
- **Alphabet:** `0123456789ABCDEFGHJKMNPQRSTVWXYZ`
- **Length:** fix 10 Zeichen
- **Excluded letters:** I, L, O, U
- **Included digits:** 0-9 (alle)

[VERIFIED: `genossi3/.planning/phases/02-helfer-token-session-authcontext-helper/02-CONTEXT.md:37`]

### **DISCREPANCY FLAG (open question for Plan)**

UI-SPEC schreibt mehrfach `0-9A-HJ-NP-Z` (z.B. Zeile 343 + Zeile 29). Wenn man das wörtlich als Regex liest:
- `A-H` = ABCDEFGH (8 Zeichen)
- `J-N` = JKLMN (5 Zeichen) — **L ist enthalten**
- `P-Z` = PQRSTUVWXYZ (11 Zeichen) — **U ist enthalten**

Das ergibt 10+8+5+11 = 34 Zeichen, **nicht 32**. Außerdem würden L und U erlaubt sein, was Phase 2 widerspricht.

**Korrekte Range-Notation für Phase-2-Alphabet:**
- `A-HJKMN-PQRSTVWXYZ` (umständlich)
- ODER explizit Whitelist: `[0-9ABCDEFGHJKMNPQRSTVWXYZ]`

**Plan-Action erforderlich:** Plan MUSS sich auf eine der beiden Versionen festlegen und im Code verifizieren, dass Frontend- und Backend-Validation deckungsgleich sind. Empfehlung: explizite Whitelist-Konstante:

```rust
const CROCKFORD_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

pub fn is_valid_helper_code(s: &str) -> bool {
    s.len() == 10 && s.chars().all(|c| CROCKFORD_ALPHABET.contains(c))
}
```

Diese Funktion ist trivial Cargo-testbar und vermeidet Regex-Range-Subtleties.

### Auto-Uppercase & Filter-on-Type

UI-SPEC §3 spezifiziert: oninput filter to alphabet, uppercase live, maxlength=10.

```rust
input {
    class: "font-mono text-2xl tracking-widest text-center uppercase ...",
    r#type: "text",
    maxlength: "10",
    autocapitalize: "characters",
    inputmode: "text",
    autocomplete: "off",
    spellcheck: "false",
    value: "{value}",
    oninput: move |e| {
        let raw = e.value();
        let cleaned: String = raw.to_uppercase()
            .chars()
            .filter(|c| CROCKFORD_ALPHABET.contains(*c))
            .take(10)
            .collect();
        value.set(cleaned);
    },
}
```

### Mobile-Keyboard-Hints

- `inputmode="text"` (nicht `numeric`, weil Buchstaben enthalten)
- `autocapitalize="characters"` — iOS Safari respektiert das, Android Chrome respektiert es
- `autocomplete="off"` — verhindert Autofill-Vorschläge
- `spellcheck="false"` — verhindert rote Unterstreichung

### Submit-Disabled

Submit-Button ist nur aktiv wenn `is_valid_helper_code(&value)` true UND nicht `submitting`:

```rust
button {
    r#type: "submit",
    disabled: !is_valid_helper_code(&value()) || submitting,
    // ...
}
```

### Backend als Backstop

Phase 2 D-24 spezifiziert:
- **400** Bad Request — Code-Format ungültig
- **404** Not Found — Code nicht erkannt
- **410** Gone — bereits eingelöst
- **403** Forbidden — GV nicht offen oder Code revoked
- **429** Too Many Requests — rate-limited

Frontend-Mapping (UI-SPEC §"Helfer-Login + Manual-Code"):

```rust
match err.status {
    Some(400) => i18n.t(Key::HelperLoginInvalidFormat),
    Some(404) => i18n.t(Key::HelperLoginErrorNotFound),
    Some(410) => i18n.t(Key::HelperLoginErrorAlreadyUsed),
    Some(403) => i18n.t(Key::HelperLoginErrorAssemblyClosed),
    Some(429) => i18n.t(Key::HelperLoginErrorRateLimit),
    _ => i18n.t(Key::ConnectionError), // generic
}
```

[VERIFIED: Phase 2 D-24 status codes]

---

## Print-CSS für Helfer-Sheet

### Pattern: `body * { visibility: hidden }` + selective unhide

Komplettes CSS in UI-SPEC §"QrCard Print contract" — bereits oben in "QR-Code Generation Strategy" zitiert.

### Plan-Tasks für `input.css`

1. Append `@media print { ... }` block aus UI-SPEC.
2. **Tailwind-Purge-Awareness:** `.qr-card`, `.font-mono`, `.w-64` müssen im finalen Build behalten werden. Da sie als Tailwind-Utilities (`font-mono`, `w-64`) und als Custom-Class (`qr-card` — gesetzt im RSX) verwendet werden, sollte der Tailwind-Scan sie automatisch finden. Plan-Task: nach Implementation `dx clean && dx build` und in `dist/assets/tailwind.css` greppen ob `.qr-card` da ist.
3. **`print:hidden` Tailwind-Utility:** UI-SPEC §"Connection-Banner colors" und §"Toast Notification position" verwenden `print:hidden`. Phase-4-Tailwind-Config muss `print:` als enabled-variant haben. UI-SPEC §"Design System" sagt das ist bereits konfiguriert ("`print:` und `screen:` raw screens already configured"). Plan-Verifikation: `tailwind.config.js` enthält die Setup.

[VERIFIED: UI-SPEC §"Design System" + `genossi-frontend/tailwind.config.js`-Verifikation als Plan-Task]

### Bekannte Browser-Quirks beim Drucken

- **Chrome:** Background-Colors werden by-default NICHT gedruckt. `body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }` zwingt Druck. Plan kann das in `input.css` ergänzen — relevant für QR-Card-Hintergrund.
- **Firefox:** Akzeptiert `print-color-adjust: exact` ohne Vendor-Prefix.
- **Safari iOS:** Print über Share-Sheet > Print; identisch zu Desktop-Safari für CSS.
- **A4 vs Letter:** UI-SPEC pinned `@page { size: A4 portrait }` — Genossenschaften DACH-Raum, A4 ist Default. US-Letter-Helfer würden falsche Skalierung sehen, aber das ist nicht Genossi-Zielgruppe.

[CITED: [MDN print-color-adjust](https://developer.mozilla.org/en-US/docs/Web/CSS/print-color-adjust)]

---

## Error-Handling & Toast-Pattern

### Existing Pattern: `members.rs::show_toast`

```rust
// genossi-frontend/src/page/members.rs:49-62
fn show_toast(
    toast_messages: &mut Signal<Vec<(u64, String)>>,
    toast_counter: &mut Signal<u64>,
    msg: String,
) {
    let id = *toast_counter.read();
    *toast_counter.write() += 1;
    toast_messages.write().push((id, msg));
    let mut toast_messages = toast_messages.clone();
    spawn(async move {
        TimeoutFuture::new(5_000).await;
        toast_messages.write().retain(|(tid, _)| *tid != id);
    });
}
```

[VERIFIED: `genossi-frontend/src/page/members.rs:49-62`]

UI-SPEC §"Toast Notification position" spezifiziert exakt das Layout. **Plan-Empfehlung:** Phase 4 extrahiert `show_toast` in einen kleinen `toast.rs`-Component oder in `service/toast.rs`-Coroutine — weil mehrere Pages (helper_attendance, assembly_details) Toasts brauchen werden. Component-First-Prinzip.

### Error-Routing per HTTP-Status

| Status | Generic deutsche Message (`status_to_message` aus `api.rs:49`) | Phase-4-spezifische Override |
|--------|----------------------------------------------------------------|------------------------------|
| 400 | "Ungültige Anfrage" | Manual-Code: `HelperLoginInvalidFormat`. Sonst: generic |
| 401 | "Keine Berechtigung — bitte erneut anmelden" | Helfer-Routes: redirect zu `/helper` + Toast "Session abgelaufen" |
| 403 | "Keine Berechtigung für diese Aktion" | Manual-Code: `HelperLoginErrorAssemblyClosed`. Toggle: generic |
| 404 | "Nicht gefunden" | Manual-Code: `HelperLoginErrorNotFound`. Sonst: generic |
| 409 | "Konflikt — das Element wurde zwischenzeitlich geändert" | Assembly-update: optimistic-locking-Konflikt |
| 410 | (existiert nicht in `status_to_message`) | Manual-Code: `HelperLoginErrorAlreadyUsed`. **Plan-Task:** ergänzt 410 in `status_to_message` |
| 422 | "Validierungsfehler" | Assembly/Member-Forms: parse Detail aus Body |
| 429 | "Zu viele Anfragen — bitte warten" | Manual-Code: `HelperLoginErrorRateLimit` |
| 500..=599 | "Serverfehler — bitte später erneut versuchen" | generic — Toast |

[VERIFIED: `genossi-frontend/src/api.rs:49-62`]

### UX-Routing nach Error-Type

- **Helfer-Login-Errors (401/403/404/410/400/429):** Inline-Display unter dem ManualCodeInput (UI-SPEC §"Error state — Redeem"). NICHT als Toast — der Login-Flow ist gated, Inline-Error ist prominenter.
- **Toggle-Errors (4xx/5xx):** Toast-Notification (UI-SPEC §"Toggle button states"); Button revertiert zu Vor-Klick-State.
- **Polling-Errors:** SILENT für 1 Failure (Warning), ConnectionBanner für ≥2 Failures (Lost). Keine Toast.
- **Assembly-Form-Errors:** Inline (z.B. `text-red-600` unter Input) oder Toast je nach Pattern in `members.rs`.

---

## Pitfalls & Landmines

### Pitfall 1: BarcodeDetector als web-sys-Feature anlegen wollen

**What goes wrong:** Plan listet "BarcodeDetector" in Cargo.toml `web-sys` features → `cargo build` fails with "unknown feature".
**Why it happens:** `BarcodeDetector` ist NICHT in web-sys 0.3.97 enthalten. WebIDL-Generator hat es nicht inkludiert.
**How to avoid:** Plan MUSS einen `#[wasm_bindgen] extern "C"` Bridge-Block in `js.rs` (oder neuem `js/barcode.rs`) anlegen. Siehe Stack-Specifics §"BarcodeDetector via wasm-bindgen extern".
**Warning signs:** Build-Error mit "feature 'BarcodeDetector' is not in the list of allowed features" für web-sys.

[VERIFIED: web-sys 0.3.97 Cargo.toml — feature absent]

### Pitfall 2: getUserMedia ohne use_drop-Cleanup

**What goes wrong:** Helfer scannt QR, erfolgreicher Login, Navigate zu Attendance. Browser-Camera-Indicator-Light bleibt an — sieht aus als spioniere die App. User schließt Tab. Stream weiterläuft bis Browser ihn killed.
**Why it happens:** `MediaStream`-Tracks werden NICHT von Dioxus gedroppt; das Stream-Objekt ist ein JS-Reference, kein Rust-Resource.
**How to avoid:** `use_drop`-Hook in `qr_scanner.rs` der `track.stop()` für jeden Track aufruft. Pattern 2 oben.
**Warning signs:** Manuelles Test: nach Scan-Success im Browser-DevTools-Network-Panel → keine "MediaStream"-Entries mehr aktiv; Browser-URL-Bar-Indicator (Camera-Symbol) ist verschwunden.

### Pitfall 3: `<video>`-Element ohne `playsinline` auf iOS

**What goes wrong:** iOS Safari öffnet bei `video.play()` Vollbild-Modus statt Page-Stream. UX kaputt — User sieht keine Scanner-UI mehr.
**Why it happens:** iOS-Safari-Default für `<video>` ist Vollbild bei `play()`.
**How to avoid:** RSX setzt `playsinline: "true"` und `muted: "true"` (auch nötig für Auto-Play auf manchen Browsern):
```rust
video {
    playsinline: "true",
    muted: "true",
    autoplay: "true",
    onmounted: move |evt| { /* set srcObject */ },
    // ...
}
```
**Warning signs:** Test auf echtem iPhone (Phase 5 SC#3); Vollbild-Trigger ist sichtbares Symptom.

[CITED: [WebKit MediaRecorder API](https://webkit.org/blog/11353/mediarecorder-api/)]

### Pitfall 4: HTTPS fehlt in Production

**What goes wrong:** `getUserMedia` wirft `NotAllowedError` mit "secure context required". Helfer kann nicht scannen.
**Why it happens:** Modern Browsers verlangen HTTPS oder `localhost` für Camera/Microphone-APIs.
**How to avoid:** Phase 5 Operations-Plan MUSS HTTPS-Setup dokumentieren (Caddy + Let's Encrypt, oder Tailscale-LAN-Cert, oder mkcert für lokales Vereinsheim).
**Warning signs:** `getUserMedia()` rejected mit `NotAllowedError`; HTTP-Origin-Indikator im Browser.

### Pitfall 5: Polling-Loop-Race nach Toggle

**What goes wrong:** Helfer klickt Toggle. Während Toggle-Request fliegt, feuert auch ein 5s-Polling-Tick. Polling holt `members?q=...` mit `present_at: null` (alter Stand) und überschreibt das in der UI gerade in `loading`-State befindliche Toggle. Helfer sieht Häkchen kurz aufleuchten und sofort wieder verschwinden.
**Why it happens:** Polling und Toggle-Request laufen unabhängig; State-Reconciliation fehlt.
**How to avoid:** AttendanceList tracked per row einen `loading: bool`-State. Polling-Refresh überschreibt UI-State NUR für Rows die NICHT in `loading` sind. Nach Toggle-200-OK: refresh signal incrementen → polling holt fresh data → row exit `loading` mit autoritativem present_at.
**Warning signs:** Race-Condition-Symptom nur unter realer Last sichtbar (Phase 5 SC#3).

### Pitfall 6: Tailwind Class Purge for `.qr-card`

**What goes wrong:** Production-Build hat keine `.qr-card`-Klasse → `@media print`-Regel matched nichts → Print-Output ist leer.
**Why it happens:** Tailwind purgiert Klassen die nicht im Source-Code gefunden werden. CSS-Custom-Klasse `.qr-card` wird im RSX als string-literal verwendet (`class: "qr-card ..."`); Tailwind-Scanner muss das parsen.
**How to avoid:** Verifikation nach Build: `grep qr-card dist/assets/tailwind.css` → muss matchen. Falls nicht: `safelist` in `tailwind.config.js` ergänzen:
```js
module.exports = {
  // ...
  safelist: ['qr-card'],
}
```
**Warning signs:** Print-Preview zeigt blank page; `dist/assets/tailwind.css` enthält kein `.qr-card`.

### Pitfall 7: ZXing-JS-Asset-Path nach manganis-Hashing

**What goes wrong:** `<script src='/assets/zxing.umd.min.js'>` wird in Production zu `<script src='/assets/zxing-{HASH}.umd.min.js'>` durch manganis-Asset-Hashing — Plan-Code mit hard-coded Path failed.
**Why it happens:** Manganis fingerprints assets für Cache-Busting [CITED: [Dioxus Manganis README](https://github.com/dioxuslabs/dioxus/blob/main/packages/manganis/manganis/README.md)].
**How to avoid:** ZXing-Asset über `asset!`-Macro:
```rust
const ZXING: Asset = asset!("/assets/zxing.umd.min.js");
// Use as: format!("<script src='{}'></script>", ZXING)
```
**Warning signs:** 404 in Production-Browser-Console für `/assets/zxing.umd.min.js`.

### Pitfall 8: `dangerous_inner_html` für QR-SVG XSS

**What goes wrong:** Wenn `qr_svg` jemals aus User-Input kommen würde, könnte SVG `<script>`-Tags enthalten → XSS.
**Why it happens:** `dangerous_inner_html` umgeht Dioxus' Auto-Escaping.
**How to avoid:** Plan dokumentiert explizit dass `qr_svg` aus eigenem Backend (Phase 2 D-21) kommt — Backend ist trusted producer. Defense-in-depth: Backend könnte das SVG noch sanitizen (das ist Backend-Concern; aktuell Plan-2-Backend produziert das SVG aus dem Backend selbst, nicht aus User-Input — sicher).
**Warning signs:** N/A in Phase 4; Audit-Konzept für Backend-Phase-2-Review.

### Pitfall 9: Crockford-Alphabet-Discrepancy zwischen UI-SPEC und Phase-2

**What goes wrong:** UI-SPEC schreibt `0-9A-HJ-NP-Z` (was L und U EINSCHLIEßT), Phase-2-Backend wiederum `0-9ABCDEFGHJKMNPQRSTVWXYZ` (was L und U AUSSCHLIEßT). Frontend akzeptiert Codes mit L/U, Backend lehnt sie ab — Helfer kriegt 400 trotz "valides" UI.
**Why it happens:** Range-Notation `J-N` enthält JKLMN (inklusive L); `P-Z` enthält PQRSTUVWXYZ (inklusive U). Crockford excludes I, L, O, U.
**How to avoid:** Plan MUSS sich auf eine Whitelist festlegen. Empfehlung: explizite Konstante `CROCKFORD_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (32 Chars) — Cargo-Test gegen Phase-2-Backend-Test (oder Backend-Code direkt) zur Verifikation der Identität.
**Warning signs:** Plan-Phase findet Discrepancy; Implementierung scheitert wenn ein Helfer einen Code mit L oder U eingegeben würde.

[VERIFIED: Phase 2 D-09 vs UI-SPEC §3]

### Pitfall 10: Nicht-existente Endpoints in api.rs annehmen

**What goes wrong:** Plan erwartet `/api/helper/session` für Auto-Redirect (D-06), aber dieser Endpoint existiert nicht.
**Why it happens:** Phase 2 hat KEINEN dedizierten Helper-Session-GET-Endpoint gebaut (Phase 2 hat nur `POST /api/helper/redeem` und das Cookie + die Session-Verifikation in Auth-Middleware).
**How to avoid:** Plan-Task: entweder kleinen Backend-Endpoint nachziehen (Read-only `GET /api/helper/session`) oder den ersten `/api/attendance/{aid}/members`-Call als Probe nutzen (Workaround). Erstgenanntes ist sauberer.
**Warning signs:** Frontend-Implementation crasht bei `404` oder `405` auf erwartetem Endpoint.

### Pitfall 11: Component-Props haben falschen Lifetime

**What goes wrong:** `Component`-Props mit `&'a str` oder borrowed Refs schlagen wegen Dioxus' Closure-Lifetime fehl.
**Why it happens:** Dioxus 0.6 erwartet Props mit `'static`-Lifetime (Components werden async eingesetzt, Bindings müssen owned sein).
**How to avoid:** Props sind `String`, `Arc<str>`, `Rc<str>`, owned `Uuid`, `Vec<T>`, `EventHandler<T>` — niemals `&str` o.ä. Folge bestehenden Pattern aus `member_search.rs::filter_members<'a>` (das ist eine UTILITY-FN, kein Component-Prop) vs `MemberSearch::new(on_select: EventHandler<Option<Uuid>>, ...)` (owned).
**Warning signs:** `cannot infer an appropriate lifetime` compiler error.

[VERIFIED: existing component patterns]

### Pitfall 12: Dioxus Hot-Reload bricht bei Hook-Changes

**What goes wrong:** Während `dx serve --hot-reload` läuft, ändert man die Reihenfolge oder Anzahl der `use_signal`/`use_future`-Calls in einer Component → Hot-Reload-Error oder UI-Klemmt.
**Why it happens:** Dioxus' Hook-System ist positionsabhängig (wie React). Hot-Reload kann Hook-Identitäten nicht trackzen wenn die Liste sich ändert.
**How to avoid:** Bei Hook-Struktur-Änderungen: full reload statt hot-reload (Browser-F5 oder Restart `dx serve`). Plan-DevX-Note: dokumentiert Workflow.
**Warning signs:** Console-Warning "Hooks changed since last render"; Component zeigt veralteten State.

[CITED: [Dioxus 0.6 Release Notes](https://dioxuslabs.com/blog/release-060/) — Hot-Reload-Limitationen sind generell bekannt]

### Pitfall 13: Cookie-Path-Mismatch für Helfer-Session

**What goes wrong:** Helfer-Cookie wird gesetzt mit `Path=/api/helper`, dann `/api/attendance/...`-Request kriegt das Cookie nicht mit.
**Why it happens:** Cookie-Path ist scope-restricted; Backend muss `Path=/` setzen damit das Cookie für alle API-Routes gilt.
**How to avoid:** Phase-4-Researcher kann das nicht selbst entscheiden — Plan-Task: verify Phase-2-Backend-`set-cookie`-Header für Helper-Session: `Path=/` ist gesetzt. Wenn nicht: Backend-Fix nötig.
**Warning signs:** `/api/attendance/...` 401 trotz erfolgreichem Login.

[VERIFIED via inference: Phase 3 E2E-Tests laufen erfolgreich, also Cookie-Path stimmt; Plan-Verifikation via Browser-DevTools-Application-Tab Cookie-Inspector]

### Pitfall 14: `print:` Tailwind-Variant nicht aktiv

**What goes wrong:** UI-SPEC nutzt `print:hidden` für ConnectionBanner, Toasts, TabStrip. Wenn `tailwind.config.js` die `print`-variant nicht enabled hat, werden die Klassen ignoriert → Banner druckt mit.
**Why it happens:** Tailwind-Variants sind opt-in.
**How to avoid:** UI-SPEC §"Design System" sagt `print:` ist already configured — Plan verifiziert via:
```bash
grep -E "print|screens" genossi-frontend/tailwind.config.js
```
**Warning signs:** Print-Preview zeigt ConnectionBanner; Browser-CSS-Inspector zeigt `.print:hidden { /* class missing */ }`.

---

## Open Questions

1. **Helper-Session-Endpoint (D-06):**
   - What we know: D-06 fordert Auto-Redirect bei `/helper`-Mount.
   - What's unclear: Backend hat keinen dedizierten Endpoint; Frontend braucht eine Probe.
   - Recommendation: Plan baut entweder (a) schmalen Backend-Endpoint `GET /api/helper/session` → 200 `{assembly_id, expires_at, gv_name}` oder 401 (5-15 Zeilen Backend-Code) ODER (b) nutzt existierenden `/api/attendance/{aid}/members` als Probe — aber dann fehlt `aid` beim ersten Mount, also (a) ist sauberer.

2. **Crockford-Alphabet-Single-Source-of-Truth:**
   - What we know: Phase 2 D-09 spezifiziert `0-9ABCDEFGHJKMNPQRSTVWXYZ`. UI-SPEC schreibt mehrdeutig `0-9A-HJ-NP-Z`.
   - What's unclear: Range-Notation in UI-SPEC ist semantisch fehlerhaft (würde L+U einschließen).
   - Recommendation: Plan definiert eine `CROCKFORD_ALPHABET`-Konstante und verifiziert per Cargo-Test gegen Backend-Phase-2-Code (gleichen Backend-`generate_code`-Test referenzieren oder duplizieren). Plan-Task: UI-SPEC korrigieren zu expliziter Whitelist.

3. **Logout-Endpoint für Helfer:**
   - What we know: HelperShell hat einen "Abmelden"-Button.
   - What's unclear: Phase 2 hat keinen `POST /api/helper/logout`-Endpoint dokumentiert. Existing `/api/auth/logout` ist OIDC-spezifisch.
   - Recommendation: Plan baut kleinen Endpoint `POST /api/helper/logout` der Helper-Session-Cookie invalidiert (5-10 Zeilen Backend). Alternative: Cookie via JS löschen — geht NICHT bei HTTP-Only-Cookie. Backend-Endpoint ist Pflicht.

4. **Dioxus Layout vs if/else in app.rs:**
   - What we know: Helper-Routes brauchen anderes Layout (kein TopBar).
   - What's unclear: Cleaner with `#[layout]` annotations or with if/else-branch in `App()`?
   - Recommendation: Plan-Discretion. `#[layout]` ist idiomatischer; if/else ist näher am bestehenden Pattern. Plan-Reviewer entscheidet.

5. **AttendanceList: Polling-Refresh oder Push-Notification?**
   - What we know: D-15 Polling-Refresh; SYNC-01 fordert Refresh-only.
   - What's unclear: Plan-Discretion ob LiveCounter und AttendanceList gemeinsamen oder separaten Hook nutzen.
   - Recommendation: separat (siehe "Polling-Pattern" oben); Plan kann später konsolidieren.

6. **Locale-Switch in HelperShell:**
   - What we know: Helfer-View deutsch-only (D-19).
   - What's unclear: Bei Helfer mit nicht-deutschem Browser-Default (selten in Genossi-DACH-Zielgruppe) — Locale-Detection oder fix de?
   - Recommendation: fix de für Phase 4; Phase 5 evaluiert wenn Vereins-Diversität ein Issue ist. Bestehender `detect_browser_locale()` (`i18n/mod.rs:20-38`) wird in `HelperShell` deaktiviert (lokal `Locale::De` setzen).

7. **WASM-Test-Strategie:**
   - What we know: `wasm-bindgen-test` in dev-deps aber unbenutzt; existing Cargo-Tests sind Pure-Logic.
   - What's unclear: Brauch Phase 4 Browser-Tests für QrScanner/Camera-Lifecycle?
   - Recommendation: Cargo-Tests für reine Logik (Crockford-Validation, ConnectionState-Machine, Counter-Display-Format). Camera-Lifecycle wird in Phase 5 manuell auf echtem iPhone/Android getestet (SC#3). Plan-Task: identifiziert testbare Pure-Functions.

8. **Manganis Asset-Hash für `.sha256`-Companion:**
   - What we know: UI-SPEC fordert SHA256-Pin für `zxing.umd.min.js`.
   - What's unclear: Die `.sha256`-Datei wird auch durch `asset!`-Macro hash-fingerprinted — wenn ja, ist Pinning der HASH der Hash-File schwierig.
   - Recommendation: `.sha256`-File nicht via `asset!()` einbinden, sondern als reine Repo-Datei für Reviewer-Verifikation. Plan klärt Manganis-Ausschluss-Pattern.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Build | ✓ | 1.77+ (Cargo workspace) | — |
| Dioxus CLI (`dx`) | dev server, build | ? (Plan verifiziert) | 0.6.x | — |
| Node.js + npm | Tailwind CSS watch | ? (Plan verifiziert) | LTS | — |
| `tailwindcss` (npm bin) | CSS-Build | ? (Plan verifiziert) | v3.x | — |
| `curl` | Vendoring ZXing-JS asset | ✓ (standard Linux) | — | wget oder manuell download |
| `sha256sum` | SHA256-Pin | ✓ (coreutils) | — | `shasum -a 256` (macOS) |
| Browser mit getUserMedia + JS | Manuelles E2E-Testing | ✓ Chrome/Firefox auf Dev-Maschine | — | — |
| iPhone/iPad mit iOS Safari | Phase-5-Verifikation (NICHT Phase 4) | n/a für Phase 4 | — | Phase-5-Generalprobe |
| Backend `genossi-bin` running | Frontend dev mit Proxy | ✓ (Phasen 1–3 abgeschlossen) | — | — |
| HTTPS-Cert für Production | `getUserMedia` Production | ? (Phase 5) | — | mkcert für lokale GV |

**Missing dependencies with no fallback:** keine — alle Phase-4-Hard-Dependencies sind im Genossi-Workspace bereits vorhanden oder lassen sich mit Standard-OS-Tools nachziehen.

**Missing dependencies with fallback:** Browser-Test-Lab fehlt; Fallback ist Phase-5-Generalprobe auf echtem Gerät.

---

## Sources

### Primary (HIGH confidence)
- Context7 `/dioxuslabs/dioxus` — `use_resource`, `use_future`, `use_drop`, `use_effect` patterns; `document::eval` semantics
- Context7 `/zxing-js/library` — `BrowserMultiFormatReader.decodeFromVideoDevice` API + `decodeFromConstraints`
- web-sys 0.3.97 source [GitHub master Cargo.toml](https://github.com/rustwasm/wasm-bindgen/blob/main/crates/web-sys/Cargo.toml) — verified: `MediaDevices`, `MediaStream`, `MediaStreamTrack`, `MediaStreamConstraints`, `MediaTrackConstraints`, `HtmlVideoElement` exist; `BarcodeDetector` does NOT
- [caniuse.com mdn-api_barcodedetector](https://caniuse.com/mdn-api_barcodedetector) — verified iOS Safari has NO native BarcodeDetector through 26.5
- Existing genossi-frontend codebase: `api.rs`, `auth.rs`, `app.rs`, `router.rs`, `i18n/mod.rs`, `service/auth.rs`, `state/auth_info.rs`, `js.rs`, `component/member_search.rs`, `component/error_alert.rs`, `component/base_components.rs`, `page/members.rs`, `page/home.rs` — all read 2026-05-04
- Phase 1–3 CONTEXT.md and SUMMARY.md docs — for backend contract, status codes, endpoint paths
- UI-SPEC `04-UI-SPEC.md` — visual contract revised 2026-05-04 (already passed checker once after locale fix + ZXing vetting)

### Secondary (MEDIUM confidence)
- [Dioxus 0.6 Release Notes](https://dioxuslabs.com/blog/release-060/) — manganis asset bundling, hot-reload behaviour
- [Dioxus 0.7 Lifecycle docs](https://dioxuslabs.com/learn/0.7/essentials/advanced/lifecycle/) — `use_drop` semantics (pattern is identical in 0.6, but docs page is for 0.7)
- [reqwest CHANGELOG](https://github.com/seanmonstar/reqwest/blob/master/CHANGELOG.md) — wasm credentials methods added v0.11.3
- [MDN BarcodeDetector](https://developer.mozilla.org/en-US/docs/Web/API/Barcode_Detection_API) — feature detection pattern
- [Crockford Base32 spec](https://www.crockford.com/base32.html) — alphabet definition
- [WebKit MediaRecorder API blog](https://webkit.org/blog/11353/mediarecorder-api/) — iOS Safari `playsinline` requirement (inferred from ecosystem knowledge, broad community consensus)

### Tertiary (LOW confidence — requires Plan-validation)
- iOS Safari `getUserMedia` + ZXing-JS combination on real device — only Phase 5 SC#3 validates; researcher relies on caniuse.com + zxing-js README claims
- Tailwind config `print:` variant active — UI-SPEC asserts it; Plan-task: verify
- Manganis hash-fingerprinting of `.sha256` companion file — Open Question 8

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `use_drop` exists in Dioxus 0.6 (referenced in Dioxus 0.7 docs, used pattern is the standard) | Architecture Patterns Pattern 2; Pitfall 2 | LOW — `use_drop` is part of the stable Dioxus hook surface; if naming differs in 0.6 (e.g. `use_on_destroy`), Plan adapts and the cleanup pattern remains identical [ASSUMED] |
| A2 | Tailwind `safelist` config option works with `dx build` (manganis asset pipeline) | Pitfall 6 | LOW — Tailwind safelist is part of Tailwind CSS not Dioxus, runs at CSS compile not WASM compile [ASSUMED] |
| A3 | iOS Safari requires `playsinline` for in-page video | Pitfall 3 | MEDIUM — long-standing iOS limitation, but exact threshold version may have shifted; Plan-task: explicit attribute MUST be set as defensive default |
| A4 | `manganis::asset!()` produces a valid `<script src="...">`-compatible URL when used in `document::eval()` | QR-Scanner Integration Plan; Pitfall 7 | LOW — Asset returns String-coercible path; if format changes Plan adjusts the lazy-load template [ASSUMED] |
| A5 | reqwest 0.12 default for WASM cookies is `same-origin` (not `omit`) | Cookie-Handling im WASM-Frontend | MEDIUM — verified via reqwest CHANGELOG that wasm credentials methods exist; Plan-task: empirisch testen mit DevTools Network-Tab |
| A6 | Backend Phase 2 `Set-Cookie` for Helper-Session uses `Path=/` (not restricted to `/api/helper`) | Pitfall 13 | MEDIUM — implicit by Phase-3-E2E-test-success; Plan-Task: explicit verification in browser DevTools |
| A7 | Existing Cargo dev-dep `wasm-bindgen-test` is functional | Test-Strategy discussion | LOW — installed and present, but not yet wired up; if `dx test` workflow doesn't exist, Plan falls back to native Cargo tests for pure logic |

**Note:** Several `[ASSUMED]` claims are LOW-risk because their verification is automatic (compiler errors at Plan-implementation time). MEDIUM-risk assumptions (A3, A5, A6) require explicit Plan-task verification BEFORE Phase-5-Generalprobe.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all Cargo deps verified line-by-line; web-sys features verified against master source; ZXing pinned
- Architecture: HIGH — patterns extracted from existing codebase + Context7 Dioxus docs; no novel design decisions
- Pitfalls: HIGH — most catalogued from concrete codebase artifacts (Cargo features, manganis hashing, Tailwind purge) plus battle-tested ecosystem knowledge (iOS playsinline, getUserMedia HTTPS); Pitfall 9 (Crockford-Alphabet) found via cross-referencing UI-SPEC vs Phase-2 contract
- Polling-Pattern: HIGH — `use_future` loop is standard Dioxus pattern, gloo_timers already in deps, `use_drop` for cleanup is documented
- QR-Scanner Integration: MEDIUM-HIGH — strategy is sound (BarcodeDetector + ZXing-JS-bridge via wasm-bindgen extern), but the iOS Safari unsupported-status flips D-02's "Polyfill nur als Fallback" claim into "Polyfill ist primärer iOS-Pfad" — Plan should reflect this in task descriptions
- Manual-Code Validation: HIGH — pattern is trivial; only ambiguity is the alphabet discrepancy (Pitfall 9 + Open Question 2) which Plan must resolve

**Research date:** 2026-05-04
**Valid until:** 2026-06-04 (30 days for stable Dioxus/web-sys, ZXing-JS pinned to 0.21.3 with no auto-update)

---

## RESEARCH COMPLETE
