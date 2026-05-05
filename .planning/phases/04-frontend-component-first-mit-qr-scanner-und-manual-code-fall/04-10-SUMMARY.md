---
phase: 04-frontend-component-first-mit-qr-scanner-und-manual-code-fall
plan: 10
subsystem: verification
autonomous: false

tags: [verification, uat, phase-acceptance, hlpr-03, sync-01, attn-06, datenschutz, roadmap-sc]

# Dependency graph
requires:
  - phase: 04
    provides: "Plans 01-09 vollständig implementiert (Backend Helper-Endpoints + Frontend Components/Pages + ATTN-06 Component-Reuse + Component-First Anwesenheits-Tab)"
provides:
  - Automated Verification Report (04-VERIFICATION.md) — 13 PASS / 1 FAIL / 1 PENDING
  - UAT Checklist (04-UAT-CHECKLIST.md) — 173 Checkboxes über 5 Blocks (A-E) + Sign-Off
  - Klare Liste der vor Generalprobe zu behebenden Punkte (Pitfall 6 Tailwind, wasm-bindgen-cli)
affects: [04-Phase-Closure, 05-Generalprobe-Setup]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "VERIFICATION.md als Code-Side-Acceptance-Gate vor UAT — automatisierte Greps + Cargo-Tests fangen Verletzungen, BEVOR der Tester Zeit auf manueller Hardware investiert"
    - "UAT-Checkliste als Reproduktionsskript für Phase-5 Generalprobe — 173 explizite Checkboxes mit FAIL-Indikatoren + Eskalations-Tabelle"

key-files:
  created:
    - .planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-VERIFICATION.md
    - .planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-UAT-CHECKLIST.md
    - .planning/phases/04-frontend-component-first-mit-qr-scanner-und-manual-code-fall/04-10-SUMMARY.md
  modified: []

key-decisions:
  - "Pitfall 6 Tailwind-Purge wurde als FAIL dokumentiert (nicht als PASS verschleiert): `qr-card`-Print-Rules werden vom dx-Tailwind-Build aus der `tailwind.css`-Output gepurged, weil der Selektor zwar in `tailwind.config.js` safelist steht, aber nur als utility-class-name — nicht als CSS-Block-Selektor in `@media print`. Auswirkung: leere Print-Page beim Drucken der QrCard. Mitigation in 04-VERIFICATION.md dokumentiert; Quick-Fix in Phase-5 (Top-level-`.qr-card`-Stub in input.css ODER inline `<style>`-Tag in index.html)."
  - "`dx build --release` als PENDING markiert: lokale Dev-Umgebung (Nix-Profil) liefert kein wasm-bindgen-cli@0.2.104. Debug-Build läuft erfolgreich; Release-Build wird vor Phase-5-Generalprobe re-running mit installierter CLI."
  - "Plan-Grep Pattern für SYNC-01 (`on_toggle_success`) war veraltet — Implementation refactorierte zu `on_toggle` mit Parent-Bumps-Pattern (smart-parent/dumb-list). Semantisch äquivalent (no-optimistic guarantee bleibt); als PASS gewertet, weil das Mechanismus-Property erfüllt ist."
  - "Plan-Spec listete '5 Helper-Backend-E2E-Tests' — Implementation lieferte 6 (4 session + 2 logout); +1 Coverage als positiv vermerkt."

patterns-established:
  - "VERIFICATION.md als CI-Surface für Phase-Acceptance: jeder Pitfall, jedes Requirement, jedes ROADMAP-SC bekommt eine eigene Tabelle-Row + Status (PASS/FAIL/PENDING)"
  - "UAT-Checkliste als Tester-Skript: jeder Schritt liefert FAIL-Indikator, der vom Tester sofort eskaliert werden kann; explizite 'NOTIEREN' Hinweise (z.B. Helfer-Codes, weil nach Schließen unrecoverable)"

requirements-completed:
  - "HLPR-03 (automatisiert): Frontend Manual-Code-Path durchgängig — ManualCodeInput in helper_login.rs, is_valid_helper_code in manual_code_input.rs, redeem_helper_token gewired (Block B2 in UAT pendend)"
  - "SYNC-01 (automatisiert): use_future + 5s Polling in LiveCounter, refresh_signal-Bump-on-200-OK Pattern in helper_attendance.rs UND assembly_details.rs (Block C1+C3 in UAT pendend)"
  - "ATTN-06 (automatisiert): identische 4 Components (AttendanceList, AttendanceSearch, LiveCounter, ConnectionBanner) in helper_attendance.rs UND assembly_details.rs (Block C4 visueller Diff in UAT pendend)"
  - "Datenschutz (automatisiert): AttendanceList ohne PII-Felder, HelperShell+Helper-Pages ohne TopBar/Footer (Block D in UAT pendend)"

# Metrics
duration: ~30min
completed: 2026-05-05

# Verification Summary

automated-checks:
  total: 15
  pass: 13
  fail: 1   # Pitfall 6 — Tailwind purgt qr-card-Print-Rules; Mitigation in VERIFICATION + UAT
  pending: 1  # dx build --release — wasm-bindgen-cli@0.2.104 fehlt im lokalen Nix-Profil

uat-checkboxes:
  total: 173
  blocks:
    - "Block A — Vorstand-Workflow (4 sub-blocks A1-A4)"
    - "Block B — Helfer-Login HLPR-03 (5 sub-blocks B1-B5)"
    - "Block C — Anwesenheits-Erfassung SYNC-01+ATTN-06 (5 sub-blocks C1-C5)"
    - "Block D — Datenschutz DSGVO (3 sub-blocks D1-D3)"
    - "Block E — GV-Lifecycle (2 sub-blocks E1-E2)"
    - "Acceptance Sign-Off (HLPR-03 + SYNC-01 + ATTN-06 + 6 ROADMAP-SCs + Datenschutz + Build-Pipeline + Auth)"

failures:
  - id: P6-FAIL
    name: "Pitfall 6 — Tailwind Purge entfernt .qr-card Print-Rules"
    severity: medium
    suspected-cause: "`@media print { .qr-card { ... } }` in input.css wird vom Tailwind-CLI gepurged, weil der Selektor `.qr-card` keine Top-Level-Definition hat (nur Print-Variante). Safelist enthält den Klassennamen, aber nicht den Block."
    mitigation: "Quick-Fix in Phase-5: Top-level `.qr-card { /* base */ }` in input.css ODER inline `<style>`-Tag in index.html. UAT Block A3 (Print-Test) deckt den visuellen Defekt ab."
  - id: WASM-PENDING
    name: "dx build --release — wasm-bindgen-cli@0.2.104 fehlt"
    severity: low (nur Dev-Env)
    suspected-cause: "Nix-Profil liefert wasm-bindgen-cli in falscher Version; debug-build erfolgreich"
    mitigation: "`cargo install wasm-bindgen-cli --version 0.2.104` ODER flake.nix updaten vor Phase-5-Generalprobe"

next-steps:
  - "Orchestrator surface User-Checkpoint (autonomous: false) mit Hinweis auf 04-UAT-CHECKLIST.md"
  - "Vor Phase-5 Generalprobe: Quick-Fix für Pitfall 6 (Tailwind .qr-card) + wasm-bindgen-cli installieren"
  - "Tester führt UAT-Checkliste auf realer Hardware aus, hakt ab, signiert; Final-Status = PASS → Phase 4 in ROADMAP als COMPLETE markieren"
  - "Bei FAIL in einem Block: Issue eskalieren via Phase-5-Plans (z.B. P6-Print-Fix als ersten Phase-5-Plan einreihen)"

acceptance:
  code-side: "PASS (13/15 automated checks; 1 FAIL ist nicht release-blockierend, Mitigation dokumentiert; 1 PENDING ist Dev-Env-only)"
  user-side: "PENDING (UAT-Checkliste 04-UAT-CHECKLIST.md erfordert manuelle Verifikation auf realer Hardware durch User)"
  orchestrator-action: "User-Checkpoint nach diesem Plan — Plan ist `autonomous: false`"
---

# Plan 04-10 — Phase 4 Verification + UAT Checklist (SUMMARY)

## Was geschah

Plan 04-10 schließt Wave 5 (Phase-Verifikation) ab. Es wurden drei Artefakte erzeugt:

1. **`04-VERIFICATION.md`** — automatisierter Verification-Report über 15 Checks:
   - 13 PASS (Build, Tests, Greps für alle Pitfalls außer 6, ATTN-06, Datenschutz, HLPR-03, SYNC-01)
   - 1 FAIL (Pitfall 6 — `qr-card`-Print-Rules werden gepurged)
   - 1 PENDING (`dx build --release` ohne wasm-bindgen-cli@0.2.104)

2. **`04-UAT-CHECKLIST.md`** — 173 Checkboxes für manuelle Verifikation:
   - Block A: Vorstand-Workflow (GV anlegen → Token → öffnen)
   - Block B: Helfer-Login HLPR-03 (Manual-Code als Hauptpfad + QR optional)
   - Block C: Anwesenheits-Erfassung SYNC-01 + ATTN-06 (Live-Counter, 2-Helfer-Race, visueller Diff, ConnectionBanner)
   - Block D: Datenschutz (PII-Whitelist, keine Vorstand-Navigation, Helfer-API-Coverage)
   - Block E: GV-Lifecycle (schließen + Cascade-Invalidierung)
   - Acceptance Sign-Off mit allen 6 ROADMAP-SCs + 3 Requirements + Datenschutz

3. **`04-10-SUMMARY.md`** (diese Datei).

## Was funktioniert (PASS)

- Frontend-Build (`cargo build`) — 17 unschädliche Warnings
- Workspace-Build (`cargo build --workspace`)
- 9/9 Helper-Code Unit-Tests (Crockford Alphabet)
- 4/4 Helper-Session E2E-Tests + 2/2 Helper-Logout E2E-Tests = 6 Helper-Backend-E2E-Tests
- 239/239 gesamtes E2E-Suite (alle Phasen 1-4)
- Crockford-Alphabet identisch FE↔BE (`"0123456789ABCDEFGHJKMNPQRSTVWXYZ"`)
- ATTN-06 Component-Reuse: 4 identische Components in helper_attendance.rs UND assembly_details.rs
- Datenschutz: keine PII in AttendanceList; keine TopBar/Footer in Helper-Layer
- Pitfall 2: use_drop + track.stop in qr_scanner.rs vorhanden
- HLPR-03: ManualCodeInput + is_valid_helper_code + redeem_helper_token alle gewired
- SYNC-01: use_future + POLL_INTERVAL_MS=5_000 + refresh_signal-Bump-Pattern in beiden Pages

## Was offen ist (FAIL/PENDING)

| ID | Schweregrad | Beschreibung | Mitigation |
|----|-------------|--------------|------------|
| P6-FAIL | mittel | `qr-card` Print-CSS-Rules werden von Tailwind gepurged | Quick-Fix in Phase-5: Top-level-`.qr-card`-Stub in input.css |
| WASM-PENDING | niedrig (Dev-Env) | wasm-bindgen-cli@0.2.104 fehlt im Nix-Profil | `cargo install wasm-bindgen-cli --version 0.2.104` |

Beide Punkte sind nicht release-blockierend; sie müssen aber **vor Phase-5 Generalprobe** behoben sein, damit (a) Print-Output funktioniert und (b) `dx build --release` durchläuft.

## User-Checkpoint

Plan 04-10 ist `autonomous: false`. Nach diesem SUMMARY surft der Orchestrator dem User die UAT-Checkliste hoch:

> "Phase 4 Code-Implementation ist abgeschlossen (13/15 automated checks PASS). Vor Phase-Closure führe bitte die UAT-Schritte in `04-UAT-CHECKLIST.md` aus (geschätzt 30-60min). Pflicht-Blocks: A (Vorstand), B2 (HLPR-03 Manual-Code), C1+C3+C4 (SYNC-01 + ATTN-06), C2+C5 (No-Optimistic + Banner), D (Datenschutz). Optional bei Hardware: A3-Print, B4 (QR), B5 (Camera-Lifecycle). Hake ab, signiere; bei FAIL eskaliere mit Block-ID."

Resume-Signal: "approved" ODER "FAIL: <Block-ID>: <Beschreibung>".

## Phase-4-Closure-Checkliste (orchestrator-Aufgabe)

- [ ] User-Checkpoint surfacen
- [ ] Auf "approved" warten (oder FAIL-Eskalation handhaben)
- [ ] Bei "approved": Phase 4 in ROADMAP.md als COMPLETE markieren
- [ ] Pre-Phase-5 Setup-Plan einreihen: P6-Print-Fix + wasm-bindgen-cli-Install
