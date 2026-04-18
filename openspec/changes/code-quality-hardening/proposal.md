## Meta
- **Priority:** low
- **Category:** quality

## Why

Das Security Audit (2026-04-18) hat mehrere Code-Quality-Findings mit niedrigem Risiko identifiziert. Keines davon ist dringend, aber zusammen verbessern sie die Robustheit und Wartbarkeit.

## What Changes

- **N1 — `unsafe impl Send/Sync` reduzieren:** 9 Dependency-Injection-Structs in `genossi_bin/src/lib.rs` verwenden `unsafe impl Send/Sync`. Prüfen, ob ein sicheres Pattern (z.B. `Arc`-basiert oder `PhantomData`-Ansatz) möglich ist.

- **N3 — SQLite Pool explizit konfigurieren:** `genossi_bin/src/main.rs:24-28` verwendet SQLite-Pool-Defaults. Explizite `max_connections` und `acquire_timeout` setzen und dokumentieren.

- **N4 — CHECK-Constraints in DB-Schema:** Numerische Felder (`current_shares`, `shares_at_joining`, `balance`) haben keine DB-Level-Validierung. Migration mit `CHECK`-Constraints ergänzen.

- **I1 — `cargo-audit` in CI einbauen:** Automatische CVE-Prüfung der Dependencies. `cargo install cargo-audit` und als CI-Step einrichten.

## Capabilities

### New Capabilities

_(keine)_

### Modified Capabilities

_(keine — reine Implementierungsdetails ohne Spec-Level-Änderungen)_

## Impact

**Code:**
- `genossi_bin/src/lib.rs` — DI-Pattern refactoren
- `genossi_bin/src/main.rs` — Pool-Konfiguration
- CI-Pipeline — `cargo audit` Step

**Datenbank:**
- Neue Migration mit CHECK-Constraints

**Risiko:** Minimal. Keine Verhaltensänderung für Endnutzer.
