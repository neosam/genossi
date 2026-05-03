---
phase: 02-helfer-token-session-authcontext-helper
plan: 03
subsystem: workspace-dependencies
tags: [cargo, dependencies, qrcode, rand, workspace]
requires:
  - 2018a4ec558e2553f8c12f2494dd9e48181eac45  # Phase 2 plans baseline (commit cad3e67 → 2018a4e)
provides:
  - qrcode 0.14 workspace dependency (D-13)
  - rand 0.8 workspace dependency with std/std_rng/getrandom features (D-10)
  - genossi_service_impl consumption of qrcode + rand via { workspace = true }
affects:
  - Plan 05 (genossi_service_impl/src/helper_token.rs) — kann jetzt qrcode + rand ohne weitere Cargo-Arbeit importieren
tech_stack:
  added:
    - qrcode 0.14.1 (pure-Rust QR-Encoder mit svg::render-Modul, MIT/Apache)
    - rand 0.8.5 (CSPRNG via OsRng, std/getrandom-Backend)
  patterns:
    - workspace.dependencies zentrale Versionsverwaltung (Phase-1-Konvention)
    - Sub-Crate-Konsumption via { workspace = true }
key_files:
  modified:
    - Cargo.toml
    - genossi_service_impl/Cargo.toml
    - Cargo.lock
decisions:
  - D-13 ratifiziert (qrcode 0.14)
  - D-10 ratifiziert (rand 0.8 mit getrandom)
  - sha2 0.10 unverändert (D-11 nutzt Bestand)
metrics:
  duration_seconds: 637
  duration_human: "~11 Minuten"
  completed: 2026-05-03
  files_changed: 3
  insertions: 15
  deletions: 0
---

# Phase 02 Plan 03: Workspace-Dependency-Setup Summary

Workspace-Cargo-Wiring abgeschlossen — `qrcode` 0.14 und `rand` 0.8 stehen ab sofort als Workspace-Dependencies bereit, `genossi_service_impl` konsumiert beide via `{ workspace = true }`, `Cargo.lock` ist regeneriert und `cargo build --workspace` ist grün.

## Objective Recap

Plan 03 verdrahtet zwei neue externe Crates für Phase 2 zentral im Workspace:
- **qrcode 0.14** — pure-Rust QR-Encoder mit SVG-Render-Modul, wird in Plan 05 für `HelperTokenServiceImpl::render_qr_svg()` verwendet (D-13).
- **rand 0.8** — kryptographisch sicherer RNG via `OsRng`, wird in Plan 05 für die Crockford-Base32-Klartext-Code-Generierung verwendet (D-10).

Plan-03 führt **keine semantische Logik** ein — er ist reines Cargo-Wiring und damit Wave-1-parallel zu Plan 01 (Migration + DAO-Trait) und Plan 02 (`AuthContext::Helper`-Variante).

## Execution Summary

| Task | Name | Status | Commit |
| --- | --- | --- | --- |
| 1 | Workspace + Service-Impl Cargo.toml Dependency-Wiring | ✓ Done | `176e558` |

### Files Modified

- `Cargo.toml` — `[workspace.dependencies]`-Block um zwei neue Einträge erweitert (alphabetisch zwischen `mockall` und `serde`):
  - `qrcode = "0.14"`
  - `rand = { version = "0.8", default-features = false, features = ["std", "std_rng", "getrandom"] }`
- `genossi_service_impl/Cargo.toml` — `[dependencies]`-Block um zwei neue Einträge erweitert (nach `sha2 = "0.10"`):
  - `qrcode = { workspace = true }`
  - `rand = { workspace = true }`
- `Cargo.lock` — automatisch regeneriert von `cargo build --workspace`. Neue Einträge:
  - `qrcode 0.14.1` (pulls in `image` als transitive Dependency — bereits im Lock-Graph präsent)
  - `genossi_service_impl` deklariert nun `qrcode` und `rand 0.8.5` als direkte Deps (siehe Lock-Diff: `+ "qrcode"`, `+ "rand 0.8.5"`)
  - `rand 0.8.5` war bereits transitiv im Lock-Graph via `ring`/`uuid` und ist jetzt zusätzlich direkter Workspace-Member; `rand 0.9.4` bleibt transitiv erhalten (von anderen Stack-Crates abhängig — kein Konflikt, da Cargo Multi-Major-Versionen erlaubt)

### Acceptance Criteria

| Kriterium | Erwartet | Ist | Status |
| --- | --- | --- | --- |
| `grep -c '^qrcode = "0\.14"' Cargo.toml` | == 1 | 1 | ✓ |
| `grep -c '^rand = \{ version = "0\.8"' Cargo.toml` | == 1 | 1 | ✓ |
| `grep -c 'features = \["std", "std_rng", "getrandom"\]' Cargo.toml` | ≥ 1 | 1 | ✓ |
| `grep -c 'qrcode = \{ workspace = true \}' genossi_service_impl/Cargo.toml` | == 1 | 1 | ✓ |
| `grep -c 'rand = \{ workspace = true \}' genossi_service_impl/Cargo.toml` | == 1 | 1 | ✓ |
| `grep -c 'sha2 = "0\.10"' genossi_service_impl/Cargo.toml` | == 1 (unverändert) | 1 | ✓ |
| `cargo metadata` enthält `qrcode` | ja | `qrcode 0.14.1` | ✓ |
| `cargo metadata` enthält `rand` | ja | `rand 0.8.5` (+ transitives 0.9.4) | ✓ |
| `Cargo.lock` enthält `qrcode 0.14.x` | ja | `version = "0.14.1"` | ✓ |
| `cargo build --workspace` exit 0 | grün | `Finished dev profile target(s) in 2m 40s` | ✓ |

### Verification Run

```
$ SQLX_OFFLINE=true cargo build --workspace
... (compile output)
warning: `genossi_rest` (lib) generated 2 warnings (run `cargo fix --lib -p genossi_rest` to apply 2 suggestions)
warning: `genossi_bin` (lib) generated 1 warning (run `cargo fix --lib -p genossi_bin` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2m 40s
```

Die im Build-Output sichtbaren Warnings (`unused_imports` in `genossi_rest/src/permission.rs:780`, `genossi_rest/src/lib.rs:27`, `genossi_bin/src/lib.rs:606`) sind **pre-existing**, nicht durch diesen Plan verursacht und betreffen ausschließlich Dateien, die Plan 03 nicht modifiziert. Sie liegen außerhalb des Scope-Boundary (siehe `deferred-items.md`-Konvention) und werden von einem späteren Aufräum-Plan adressiert.

`cargo build` benötigt `SQLX_OFFLINE=true`, weil das Repo SQLx-Compile-Time-Checked-Queries verwendet und der Offline-Cache (`.sqlx/`) im Repo eingecheckt ist (Standard im Genossi-Setup für CI ohne Live-DB).

## Decisions Reaffirmed

| Decision | Status | Note |
| --- | --- | --- |
| D-13 — qrcode 0.14 als QR-Render-Crate | ratifiziert | `qrcode 0.14.1` ist im Lockfile, `EcLevel::Q` + SVG-Output stehen Plan 05 zur Verfügung |
| D-10 — rand 0.8 mit getrandom-Backend | ratifiziert | `rand 0.8.5` ist im Lockfile als direkter Workspace-Member; verhindert mögliches stilles 0.9-Upgrade |
| sha2 0.10 unverändert | bestätigt | bereits in `genossi_service_impl/Cargo.toml:26`, kein Eingriff nötig |

## Deviations from Plan

Keine — Plan exakt wie geschrieben ausgeführt.

## Notes for Plan 05 (Service-Impl)

Plan 05 kann ab sofort folgende Imports ohne weitere Cargo-Arbeit verwenden:

```rust
use qrcode::{QrCode, EcLevel};
use qrcode::render::svg;

use rand::{rngs::OsRng, RngCore};
```

Die Pattern-Maps in `02-PATTERNS.md` §"Phase-2 novel sub-patterns" und `02-RESEARCH.md` §"Standard Stack" referenzieren beide direkt. Plan 05 braucht keine Cargo-Edits durchzuführen.

## Threat Flags

Keine neuen Threat-Surfaces durch diesen Plan eingeführt. Die im Plan dokumentierten Threats T-02-03-01..T-02-03-03 (Supply-Chain-Risiken via qrcode/rand-Crates, RNG-Klartext-Leak via Debug-Logs) bleiben unverändert. Der RNG-Klartext-Leak-Schutz (T-02-03-03 → mitigate) ist Plan-05-Verantwortung.

## Self-Check: PASSED

- [x] `Cargo.toml`: `qrcode = "0.14"` und `rand = { ... }` vorhanden — verifiziert via `git show 176e558:Cargo.toml | grep -E 'qrcode|rand ='`
- [x] `genossi_service_impl/Cargo.toml`: beide Workspace-Konsumtionen vorhanden — verifiziert via `git show 176e558:genossi_service_impl/Cargo.toml | grep -E 'qrcode|rand'`
- [x] `Cargo.lock`: `qrcode 0.14.1` als neuer Top-Level-Eintrag, `rand 0.8.5` als zusätzliche `genossi_service_impl`-Dep — verifiziert via Diff in `git show 176e558 -- Cargo.lock`
- [x] Commit `176e558` existiert in git log: `git log --oneline | grep 176e558` → `176e558 chore(02-03): add qrcode 0.14 and rand 0.8 workspace dependencies`
- [x] `cargo build --workspace` exit 0 mit `SQLX_OFFLINE=true`
- [x] Genau 3 Files in Commit (`Cargo.toml`, `genossi_service_impl/Cargo.toml`, `Cargo.lock`) — verifiziert via `git show 176e558 --stat`
- [x] Keine Modifikation an `STATE.md`, `ROADMAP.md` oder anderen Orchestrator-Artefakten

## Commit Hashes

- `176e558` — chore(02-03): add qrcode 0.14 and rand 0.8 workspace dependencies
