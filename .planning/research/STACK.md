# Stack Research — GV-Anwesenheits-Erfassung (QR-Code Helfer-Sessions)

**Domain:** Event-Anwesenheits-Tracking als Erweiterung einer bestehenden Rust/Axum/Dioxus-Plattform
**Researched:** 2026-05-02
**Confidence:** HIGH (Backend) / MEDIUM (Frontend Scanner — Browser-API-Fragmentierung)

---

## Scope-Hinweis

Dieses Dokument beschreibt **nur die zusätzlichen Bausteine** für das GV-Feature. Die Genossi-Basis (Rust 2021, Axum 0.8.3, SQLx 0.8 + SQLite, Tokio 1.35, axum-oidc 0.6, tower-sessions 0.14, tower-cookies 0.10, Dioxus 0.6.3, Utoipa 5.0, Mockall 0.13) bleibt unverändert und ist in `.planning/codebase/STACK.md` dokumentiert — bewusst **nicht erneut recherchiert**.

Die unten genannten Versionen sind über die crates.io-API am 2026-05-02 verifiziert und nicht aus Trainingsdaten geraten.

---

## Recommended Stack

### Core Technologies (additiv)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `qrcode` (kennytm) | 0.14.1 | Server-side QR-Code-Erzeugung als SVG/PNG | Standard-Crate im Rust-Ökosystem (~13.2 Mio. Downloads, Update 2024-07), pures Rust ohne C-Bindings, SVG-Export ohne `image`-Dependency möglich, Apache-2.0/MIT — passt zu existierender Lizenzlinie. **Defaults reichen aus**: ein Helfer-QR pro GV, keine Hochlast-Generierung. |
| Browser-`MediaDevices.getUserMedia` + `BarcodeDetector` (mit JS-Polyfill) | nativ | QR-Scan im Dioxus-WASM-Frontend | Bereits über `web-sys` 0.3 / `wasm-bindgen` 0.2.97 erreichbar (in `.planning/codebase/STACK.md` bestätigt). `BarcodeDetector` ist nativ in Chrome/Edge ≥ 83 und über Polyfill in Safari/Firefox; deckt iPad-/Android-Tablet-Helfer-Szenario ab. Kein zusätzliches WASM-Modul nötig. |
| `barcode-detector` JS-Polyfill (via `<script>` + `wasm-bindgen`) | npm 2.x | Fallback-Scanner für Safari/Firefox | Polyfill basiert auf zbar-wasm; erfüllt die `BarcodeDetector`-API überall. So bleibt der Rust-Code identisch — er ruft immer `BarcodeDetector` auf, der Browser entscheidet, ob nativ oder Polyfill. |
| `tower-sessions` 0.15 (zweite Layer-Instanz) | 0.15.0 | Helfer-Session als **separater** Cookie neben dem OIDC-Session-Cookie | Bereits im Workspace (aktuell 0.14, Upgrade auf 0.15 empfohlen — Release 2026-02-01). `SessionManagerLayer::with_name(...)` erlaubt einen zweiten, parallelen Cookie. Beide Sessions koexistieren konfliktfrei: OIDC-Session bleibt für Vorstand, Helfer-Session ist GV-gebunden. |

**Rationale für QR-Crate-Wahl (`qrcode` statt `fast_qr`/`qrcode-generator`):**

- `fast_qr` 0.13.1 (Update 2025-06): performant, aktiv. Aber: 6–7× schneller ist für **eine Handvoll QR-Codes pro GV** völlig irrelevant. `fast_qr` zieht zusätzliche Builder-API-Komplexität und ist im Ökosystem deutlich weniger verbreitet (260k vs. 13M Downloads).
- `qrcode-generator` 5.0.0: PNG/SVG/RAW. Aktiv, aber kleinere Community. Kein technischer Vorteil gegenüber `qrcode`.
- `qrcode` 0.14.1: Größte Verbreitung → mehr StackOverflow/Beispiele für Audit/Review-Phase, Dokumentation in `.planning/codebase/`-Stil dokumentierbar, SVG/PNG/Unicode out-of-the-box, `default-features = false` möglich, falls `image` nicht gebraucht wird (für reinen SVG-Output empfohlen — kleinerer Build).

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `rand` 0.8 (bereits im Workspace prüfen) oder `getrandom` 0.2 | latest | Kryptographisch sichere Pre-Token-Erzeugung (256 Bit) | Ein-Zeiler im Service-Layer für `QrToken::new()`. Falls `rand` nicht vorhanden, `getrandom::getrandom` direkt — minimaler Footprint. UUID-v4 (bereits via `uuid` 1.6) ist **nicht ausreichend** als Auth-Token (122 Bit Entropie + Layout-Bias möglich). |
| `subtle` | 2.6 | Konstantzeit-Vergleich beim Token-Redeem | Verhindert Timing-Side-Channels beim Lookup, insbesondere wenn Tokens kurzzeitig in einer Map gehalten werden. Optional, wenn Tokens nur per indizierter SQLite-Spalte verglichen werden. |
| `time` 0.3 (bereits da) | bereits im Workspace | Ablauf-Zeitstempel Helfer-Session = `Assembly.closed_at` | Wiederverwendung der bestehenden ISO8601-Konvention; kein neues Datums-Lib. |
| `wasm-bindgen` / `wasm-bindgen-futures` / `js-sys` / `web-sys` | bereits im Workspace (0.2.97 / 0.4.47 / 0.3.77 / 0.3) | Brücke zu `getUserMedia` + `BarcodeDetector` | Keine neuen Frontend-Crates nötig. Component-First: ein Component `QrScanner` in `genossi-frontend/src/component/qr_scanner.rs`. |
| `manganis` 0.6.2 (bereits da) | bereits im Workspace | Auslieferung des `barcode-detector`-Polyfills als Static Asset | Polyfill-JS wird als Asset eingebunden, in `index.html` per `<script>`-Tag geladen, vor dem ersten Scan-Aufruf vorhanden. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo sqlx prepare` (bereits im Workflow) | Offline-Query-Generierung für neue `assemblies`/`assembly_attendances`/`qr_tokens`-Tabellen | Existierende Migrations-/Prepare-Pipeline ohne Anpassung wiederverwenden. |
| Dioxus CLI (bereits da) | WASM-Build inkl. Polyfill-Asset | `dx serve` muss HTTPS-tauglich für Kamera-Test sein (entweder via mkcert lokal oder via Reverse-Proxy in Staging) — `getUserMedia` braucht Secure Context. |
| Browser-DevTools | BarcodeDetector-Verfügbarkeit testen | `'BarcodeDetector' in window` als Boolean-Check; Polyfill nur laden, wenn `false`. |

---

## Installation

```toml
# genossi_service_impl / genossi_rest (oder neues genossi_assembly Crate)
[dependencies]
qrcode = { version = "0.14", default-features = false, features = ["svg"] }   # SVG-Output reicht
# Falls PNG nötig (z. B. PDF-Druck-Liste):
# qrcode = { version = "0.14", features = ["image", "svg"] }

# Für Pre-Token (32 Bytes, base64url-kodiert, ~43 Zeichen)
rand = "0.8"
base64 = "0.22"
subtle = "2.6"  # optional, für konstantzeit-Vergleich

# tower-sessions Upgrade von 0.14 → 0.15 (Workspace-weit prüfen)
tower-sessions = "0.15"
```

```toml
# genossi-frontend
# Keine neuen Crates — bestehende web-sys / wasm-bindgen reichen.
```

```bash
# Polyfill für BarcodeDetector (Safari/Firefox-Fallback)
# via npm in genossi-frontend (oder direkt als Asset-Datei vendoren):
npm install --save barcode-detector
# Alternativ: gehostet als statisches Asset im /assets/ Ordner und per <script> einbinden.
```

---

## Integration mit bestehender Genossi-Auth

**Problem:** Wie liegt eine Helfer-Session **neben** der OIDC-Session, ohne sich gegenseitig zu invalidieren?

**Lösung — Zwei `SessionManagerLayer`-Instanzen, unterschiedliche Cookie-Namen:**

```rust
// Bestehender OIDC-Session-Layer (unverändert)
let oidc_session_layer = SessionManagerLayer::new(oidc_store.clone())
    .with_name("genossi.session")           // bestehender Name beibehalten
    .with_secure(true)
    .with_expiry(Expiry::OnInactivity(Duration::days(30)));

// NEU: Helfer-Session-Layer
let helper_session_layer = SessionManagerLayer::new(helper_store.clone())
    .with_name("genossi.helper")            // separater Cookie
    .with_secure(true)
    .with_same_site(SameSite::Strict)
    .with_expiry(Expiry::AtDateTime(assembly_close_time));
```

`SessionManagerLayer::with_name(...)` ist seit tower-sessions 0.x explizit unterstützt (verifiziert auf docs.rs/tower-sessions/0.15.0). Beide Layer koexistieren als getrennte Tower-Layer, ohne dass sich die Session-Extractoren mischen — Axum-Handler entscheiden anhand des Routen-Scopes, welcher Session-Typ relevant ist.

**Alternative — Single Layer mit Discriminator-Key:** Möglich, aber **nicht empfohlen**: vermischt zwei Berechtigungs-Lebensdauern in einem Cookie und macht Logout/Invalidierung der GV nicht atomar.

**Pre-Token-Redeem-Flow:**
1. Vorstand: `POST /api/assembly/{id}/qr-tokens` mit `{name: "Anna"}` → Server erzeugt 32-Byte-Token, speichert hash (z. B. SHA256, bereits im Workspace via `sha2`) in `qr_tokens` mit `redeemed_at NULL`, gibt Token + QR-SVG zurück.
2. Helfer scannt QR → Frontend POSTet auf `POST /api/assembly/{id}/redeem?token=<...>`.
3. Server: Transaktion → SELECT WHERE token_hash = ? AND redeemed_at IS NULL → wenn vorhanden: UPDATE redeemed_at = NOW(), session_id = NEW. Wenn nicht: 401.
4. Bei Erfolg: `helper_session.insert("assembly_id", ...)`, `helper_session.insert("token_id", ...)`. Der Cookie wird im Response gesetzt, bei späteren Requests greift der `helper_session_layer`.

**Helfer-Session-Lebensdauer = bis GV-Schluss:**
- `Expiry::AtDateTime(assembly.closed_at)` setzt das Cookie-Max-Age direkt. Sobald der Vorstand die GV schließt:
  - Bestehende Sessions werden serverseitig durch ein zusätzliches `assembly_id`-gebundenes Lookup invalidiert (Service-Layer prüft `Assembly.closed_at IS NOT NULL` → 401).
  - Cookie verfällt automatisch, sobald die GV geschlossen wurde (Browser respektiert `Max-Age`).
- Server-seitig: Periodisches Cleanup-Job nicht zwingend nötig, da SQLx-Query beim Session-Lookup die `assembly.closed_at`-Bedingung mitprüft. Optional: Tokio-Task einmal pro Tag für Aufräumarbeiten alter `qr_tokens`.

**Vorstand-Bypass-Pattern (laut PROJECT.md `Active`-Liste):**
- Vorstand mit gültiger OIDC-Session kann den Helfer-View direkt aufrufen. Implementierung: REST-Handler prüft erst OIDC-Session (privileged), dann Helfer-Session (fallback). Wenn OIDC vorhanden + Permission `assembly.helper-view` → grant. Kein separater QR nötig. Component-First: gleiches `MemberAttendanceList`-Component, anderer Datenpfad.

---

## Frontend-Scanner-Strategie (WASM)

**Empfohlener Pfad:**

1. Component `QrScanner` in `genossi-frontend/src/component/qr_scanner.rs` (Component-First-Prinzip aus `CLAUDE.md`).
2. Beim Mount: `web-sys` ruft `navigator.media_devices().get_user_media(...)` mit `video: { facingMode: "environment" }`.
3. Stream → `<video>`-Element. Pro Frame (ca. alle 200 ms via `gloo-timers` 0.3, bereits im Workspace) wird `BarcodeDetector::detect(video)` aufgerufen.
4. Erkennt der Detector einen `qr_code`, wird `on_scan(token)` als Callback ausgelöst → POST an `/api/assembly/{id}/redeem`.

**Polyfill-Logik:**
```javascript
// In index.html, nach Dioxus-Bootstrap:
if (!('BarcodeDetector' in window)) {
  const { BarcodeDetector } = await import('barcode-detector/pure');
  window.BarcodeDetector = BarcodeDetector;
}
```
Ab dann ist `BarcodeDetector` global verfügbar und der Rust-Code (via `web-sys`) ruft immer den gleichen Pfad auf — Browser entscheidet transparent zwischen nativ und Polyfill.

**Browser-Reichweite (Stand 2026-05):**
- Chrome/Edge ≥ 83: nativ (~76 % global).
- Safari (Desktop + iOS) 17+: nativ verfügbar, aber per Default deaktiviert → Polyfill greift.
- Firefox: keine native Implementierung → Polyfill greift.
- **Mit Polyfill: faktisch 100 % der Helfer-Devices** (alle modernen Browser auf Tablet/Laptop/Handy). Polyfill-Größe ca. 200 kB gzipped — akzeptabel für GV-Use-Case.

**HTTPS-Anforderung:**
- `getUserMedia` benötigt Secure Context. Genossi muss in Produktion ohnehin über HTTPS laufen (OIDC + Auth-Cookies). In `dx serve` lokal: entweder via mkcert oder Reverse-Proxy.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `qrcode` 0.14 | `fast_qr` 0.13.1 | Bei Massengenerierung (>1000 QRs/s) im Hot-Path — bei Genossi (≤ 50 QRs pro GV) **nicht der Fall**. |
| `qrcode` 0.14 | `qrcode-generator` 5.0 | Wenn Multi-Segment-Encoding (numerisch + alphanumerisch) gewollt — für Pre-Tokens irrelevant, da reine base64url-Strings. |
| Browser-`BarcodeDetector` + Polyfill | `html5-qrcode` (pure JS) | Wenn größere Polyfill-Lasten vermieden werden sollen und Maintenance-Aufwand für eigenen JS-Wrapper akzeptabel ist. Nachteil: zusätzliche API-Oberfläche, anderes Lifecycle-Modell. **Polyfill-Pfad ist API-konsistenter.** |
| Browser-`BarcodeDetector` + Polyfill | `zbar-wasm` direkt | Wenn der Polyfill nicht ausreicht (z. B. exotische Symbologien). Für reine QR-Codes auf modernen Browsern Overkill. |
| Browser-`BarcodeDetector` + Polyfill | Scanbot Web SDK | Kommerziell, schnell zu integrieren — aber **kostenpflichtig** und externe Abhängigkeit, passt nicht zur Self-Hosted-Genossi-Linie. |
| `rqrr` 0.10.1 (server-side decoder) | — | **Nicht zutreffend**: Decoding läuft im Browser, Server bekommt nur den decodeten Token-String. `rqrr` wäre nur relevant, wenn Helfer Foto-Uploads machen würden — ist `Out of Scope`. |
| Zwei `SessionManagerLayer` | Eigene Auth-Header (`X-Helper-Token`) statt Cookie | Wenn der Helfer-Frontend bewusst stateless sein soll. Nachteil: token muss bei jedem Request mitgesendet werden, XSS-Surface höher als HttpOnly-Cookie. **Cookie-Pfad ist sicherer.** |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `bardecoder` 0.5 (letzter Release 2023-07) | Inaktiv, keine Releases seit 2.5 Jahren | `rqrr` 0.10.1 — falls server-seitiges Decoding **doch** mal nötig wird |
| `qrc` (sebastienrousseau) | Nische, geringere Community-Reichweite | `qrcode` (kennytm) |
| `axum-sessions` (deprecated) | Migration zu `tower-sessions` ist offiziell empfohlen (siehe maxcountryman/axum-sessions Discussion #56) | `tower-sessions` 0.15 |
| UUID v4 als Pre-Token | 122 Bit Entropie, vorhersagbares Layout (Versions-/Variant-Bits), nicht für Auth-Tokens vorgesehen | 32 Byte aus `rand::rngs::OsRng` → base64url |
| Klartext-Speicherung des Tokens in DB | DB-Leak = Massenkompromittierung der Helfer-Tokens (auch wenn sie kurzlebig sind) | SHA256-Hash in DB; Klartext wird nur einmal beim Erzeugen ausgegeben (analog zu API-Keys). `sha2` ist bereits im Workspace. |
| Hard-Delete von `qr_tokens` nach Redeem | Verhindert Forensik bei Streitfall ("Wer hat wann gescannt?") | Soft-State: `redeemed_at` setzen, plus `redeemed_by_session_id`-Spalte. Konsistent mit Genossi-Soft-Delete-Muster. |
| Live-Push via SSE/WebSocket für Anwesenheits-Counter | In `PROJECT.md` explizit `Out of Scope` — vermeidet Komplexität | Polling alle 5–10 s vom Vorstand-Live-Counter (einfacher REST-Call) |
| BarcodeDetector ohne Polyfill | Safari/Firefox-Helfer schlagen fehl | `barcode-detector` npm-Paket als Polyfill |
| `getUserMedia` ohne HTTPS | Secure-Context-Requirement → Browser blockt | Lokal mkcert; Produktion ohnehin HTTPS |

---

## Stack Patterns by Variant

**Wenn Helfer-Devices iPad/iPhone-lastig sind:**
- Polyfill ist **Pflicht** (Safari hat BarcodeDetector default-disabled).
- HTTPS in Staging zwingend testen — iOS Safari ist hier strikter als Chrome.

**Wenn QR-Codes in PDF-Helfer-Liste eingebettet werden sollen:**
- `qrcode = { version = "0.14", features = ["image", "svg"] }` (PNG via `image`).
- Integration mit bestehendem Typst-PDF-Generator (`typst-pdf` 0.14): SVG-Embed in Typst-Template ist sauberer als PNG, da vektoriell.

**Wenn QR-Codes nur am Bildschirm (Vorstand zeigt Helfer den QR) angezeigt werden:**
- `qrcode = { version = "0.14", default-features = false, features = ["svg"] }` — schlankerer Build, kein `image`-Crate nötig.

**Wenn der Vorstand mehrere parallele GVs verwaltet (z. B. zwei Sektionen gleichzeitig):**
- Helfer-Session muss `assembly_id` enthalten (nicht nur `helper_token_id`), damit Anwesenheits-Markierung dem richtigen `Assembly` zugeordnet wird. Kein Stack-Wechsel nötig — Service-Layer-Logik.

---

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `qrcode` 0.14.1 | `image` 0.25 (transitive bei `image`-Feature) | Falls Workspace `image` schon benutzt: Versionen abgleichen, sonst zwei Image-Crate-Versionen im Build. |
| `tower-sessions` 0.15 | `axum` 0.8.x, `tower-cookies` 0.10 | 0.14 → 0.15 bringt Breaking Changes im `Session::insert/get`-Pfad — Migration prüfen. Bestehender OIDC-Session-Code muss ggf. mitaktualisiert werden. **Falls Risiko zu hoch: bei 0.14 bleiben, `with_name` ist auch dort verfügbar.** |
| `axum-oidc` 0.6.0 | `tower-sessions` 0.14 / 0.15 | axum-oidc nutzt tower-sessions intern. Bei tower-sessions-Upgrade → axum-oidc-Kompatibilität prüfen (release-notes). |
| `rand` 0.8 | bereits in vielen transitive Deps | `OsRng` ist OS-RNG, ausreichend für Auth-Tokens. |
| `base64` 0.22 | — | URL-safe Variante (`URL_SAFE_NO_PAD`) für QR-codierte Tokens (kürzere Strings = einfacher zu scannen). |

**Verifiziert per crates.io-API am 2026-05-02:**
- qrcode 0.14.1 (2024-07-05, 13.179.786 Downloads) — HIGH confidence
- fast_qr 0.13.1 (2025-06-13, 260.105 Downloads) — HIGH confidence
- qrcode-generator 5.0.0 (2024-10-23, 3.810.284 Downloads) — HIGH confidence
- tower-sessions 0.15.0 (2026-02-01, 2.270.994 Downloads) — HIGH confidence
- rqrr 0.10.1 (2026-01-27, 3.402.933 Downloads) — HIGH confidence
- bardecoder 0.5.0 (2023-07-29, 128.622 Downloads) — als deprecated markiert — HIGH confidence
- axum-oidc 0.6.0 (2026-02-28, 44.695 Downloads) — HIGH confidence

---

## Open Questions for Roadmap-Phase

1. **`tower-sessions` 0.14 → 0.15-Upgrade jetzt oder separater PR?** — Empfehlung: separater Vor-Phase-Task, damit das GV-Feature auf einem stabilen Session-Stack landet.
2. **QR-Format: SVG inline im JSON oder als binärer Endpoint?** — SVG-String in JSON ist einfacher; bei späteren PDF-Listen reicht serverseitiges Re-Encoding.
3. **Polyfill als npm-Dep (Build-Step) oder vendored Asset?** — Vendored ist robuster (kein npm-Bruch), npm wäre sauberer für Updates. `manganis` 0.6.2 unterstützt beide Wege.
4. **Audit-Log für `qr_tokens` ja/nein?** — `PROJECT.md` schließt Audit-Hashchain für **Anwesenheit** aus, sagt aber nichts zu Token-Erzeugung. Entscheidung im nächsten Phase-Boundary.

---

## Sources

- [crates.io API — qrcode](https://crates.io/api/v1/crates/qrcode) — Version, Update-Datum, Download-Zahlen (verifiziert 2026-05-02) — HIGH
- [crates.io API — fast_qr](https://crates.io/api/v1/crates/fast_qr) — HIGH
- [crates.io API — qrcode-generator](https://crates.io/api/v1/crates/qrcode-generator) — HIGH
- [crates.io API — tower-sessions](https://crates.io/api/v1/crates/tower-sessions) — HIGH
- [crates.io API — rqrr](https://crates.io/api/v1/crates/rqrr) — HIGH
- [crates.io API — axum-oidc](https://crates.io/api/v1/crates/axum-oidc) — HIGH
- [docs.rs — SessionManagerLayer 0.15.0](https://docs.rs/tower-sessions/0.15.0/tower_sessions/service/struct.SessionManagerLayer.html) — `with_name`, `with_expiry`, `with_same_site` Builder-Methoden — HIGH
- [GitHub — kennytm/qrcode-rust](https://github.com/kennytm/qrcode-rust) — Output-Formate, `default-features = false`-Pfad — HIGH
- [GitHub — erwanvivien/fast_qr](https://github.com/erwanvivien/fast_qr) — WASM-Tauglichkeit, Performance-Claims — MEDIUM
- [Can I use — BarcodeDetector](https://caniuse.com/mdn-api_barcodedetector) — Browser-Support 75.9 % nativ, Safari/Firefox via Polyfill — HIGH
- [MDN — Barcode Detection API](https://developer.mozilla.org/en-US/docs/Web/API/Barcode_Detection_API) — API-Form, Polyfill-Hinweis — HIGH
- [Scanbot — Dioxus Barcode Scanner Tutorial](https://scanbot.io/techblog/dioxus-barcode-scanner-rust-tutorial/) — Bestätigung der wasm-bindgen + JS-Bridge-Pattern für Dioxus (auch wenn Scanbot SDK kommerziell ist, das Integrationsmuster ist übertragbar) — MEDIUM
- [Barkey Wolf — ZBar in Browser via WebAssembly](https://barkeywolf.consulting/posts/barcode-scanner-webassembly/) — Hintergrund zur Polyfill-Implementierung — MEDIUM
- [tower-sessions GitHub Repo](https://github.com/maxcountryman/tower-sessions) — Multi-Store-Architektur, `SessionStore`-Trait, `with_name` zur Cookie-Trennung — HIGH
- [GSD `.planning/codebase/STACK.md`](.planning/codebase/STACK.md) — Bestehender Workspace-Stand (axum-oidc 0.6, tower-sessions 0.14, web-sys/wasm-bindgen schon vorhanden) — HIGH
- [GSD `.planning/codebase/INTEGRATIONS.md`](.planning/codebase/INTEGRATIONS.md) — OIDC-Flow, Session-Tabelle, Cookie-Konfiguration — HIGH

---

## Confidence Assessment

| Area | Confidence | Rationale |
|------|------------|-----------|
| Server-side QR-Generierung | HIGH | crates.io-API verifiziert, `qrcode` ist seit Jahren De-facto-Standard, kein architektonisches Risiko |
| Pre-Token / Redeem-Pattern | HIGH | Klassisches Single-Use-Token-Muster (vgl. Email-Verify, Magic-Link). `rand` + `sha2` sind im Workspace, keine neuen Crates |
| Tower-Sessions Multi-Layer-Setup | HIGH | `with_name` per docs.rs verifiziert, Pattern in maxcountryman-Repos dokumentiert. Risiko nur beim 0.14→0.15-Upgrade |
| WASM QR-Scanner (BarcodeDetector + Polyfill) | MEDIUM | Pattern funktioniert technisch, aber: Browser-Fragmentierung 2026 ist real. Safari-iOS-Verhalten nicht in jedem Edge-Case getestet, sollte in einer kurzen Browser-Test-Phase validiert werden, bevor das Feature live geht |
| Helfer-Session-Lebensdauer-Bindung an GV-Schluss | HIGH | `Expiry::AtDateTime` ist erstklassig in tower-sessions 0.15; Server-Side-Validierung der `assembly.closed_at`-Spalte ist Trivial-SQL |
