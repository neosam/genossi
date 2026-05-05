# Phase 4 Vendored Assets

## zxing.umd.min.js

- **Why:** Phase 4 D-02 Polyfill für QR-Scanning auf Browsern ohne native BarcodeDetector
  (de facto: alle iOS Safari < 26.5 — siehe RESEARCH §Browser-Support-Realität).
  Lokal vendored statt CDN, damit Vereinsheim-WiFi-Aussetzer (Phase-5-Risiko) das
  QR-Scanning nicht blockieren.
- **Pinned version:** 0.21.3 — siehe `04-UI-SPEC.md` §"ZXing-JS Vetting" (vetted 2026-05-04).
- **License:** Apache-2.0 (kompatibel mit Genossi-Commercial-Use).
- **Source:** https://unpkg.com/@zxing/library@0.21.3/umd/index.min.js
- **Vendoring date:** 2026-05-05
- **SHA256-Verifikation:**
  ```bash
  cd genossi-frontend/assets && sha256sum -c zxing.umd.min.js.sha256
  ```
- **Update-Procedure:** wenn upgraden auf eine neuere Version, neue SHA256 berechnen
  und REVISION-Eintrag hier ergänzen. NIEMALS `@latest` — bleeding-edge-Releases müssen
  geprüft werden.
