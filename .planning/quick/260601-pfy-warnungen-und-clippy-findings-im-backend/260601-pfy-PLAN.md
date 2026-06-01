---
quick_id: 260601-pfy
description: Warnungen und Clippy-Findings im Backend beheben
mode: quick
scope: pragmatic-cleanup
created: 2026-06-01
---

# Quick Task 260601-pfy — Backend-Warnungen und Clippy-Findings beheben

## Objective

Alle `cargo build` Warnings und `cargo clippy --all-targets --all-features` Findings im Rust-Workspace (10 Pakete, ohne `genossi-frontend`) adressieren. Cleanup ist **verhaltensneutral** — keine API-Änderungen, keine Refactors mit Test-Breaks.

## Scope (Pragmatisch)

**In Scope:**
- Alle `unused imports`, `unused variables`, `unused fields`, `dead code` entfernen
- Sichere `cargo clippy --fix` Anwendungen (redundant closures, useless conversions, manual prefix stripping, `repeat().take()`, `iter().any()`)
- `Default`-Impls hinzufügen wo clippy es vorschlägt (PackageCache, PdfGenerator, UuidServiceImpl)
- `map_or` Vereinfachungen
- Doc-List Indentation Findings beheben
- `else { if .. }` Block kollabieren

**Out of Scope (per `#[allow(...)]` unterdrücken):**
- `too_many_arguments` — kein Funktions-Refactor
- `type_complexity` — keine type-alias-Extraktion
- `from_str` collisions in DAO-Layern — `from_str` als inhärente Methode erhalten (API-Stabilität)
- `should_implement_trait` für `From<T>` mit Derive-Konflikt

## Bekannte Findings (Vor-Analyse)

| Paket | Build Warnings | Clippy Findings |
|-------|---------------|----------------|
| genossi_dao | 0 | 14 (mostly from_str + derivable impls) |
| genossi_dao_impl_sqlite | 0 | 0 |
| genossi_service | 0 | 1 (from_str) |
| genossi_service_impl | 0 | 23 (mixed) |
| genossi_rest | 2 | many (unused imports, useless conv, redundant closures, prefix strip) |
| genossi_rest_types | 0 | 1 (derivable impl) |
| genossi_bin | 1 | 0 |
| genossi_config | 0 | 0 |
| genossi_mail | 2 | 9 (mixed) |
| genossi_backup | 3 | 6 (too_many_args, else-if collapse, dead code) |

## Tasks

1. **`cargo fix --lib` pro Paket** — automatische Auto-Fixes für unused imports
2. **`cargo clippy --fix`** — sichere Suggestion-Fixes anwenden
3. **Default-Impls** manuell hinzufügen (`PackageCache`, `PdfGenerator`, `UuidServiceImpl`)
4. **Dead Code entfernen** — `format_datetime`, `interval_hours`, `audit_log_exported`
5. **Allow-Attribute** für strukturelle Findings
6. **Verifizieren** — `cargo build`, `cargo clippy -- -D warnings`, `cargo test`

## Verification

```bash
cargo build --workspace 2>&1 | grep -E "^(warning|error)" | wc -l    # → 0
cargo clippy --workspace --all-targets --all-features 2>&1 | grep -E "^warning" | wc -l    # → 0
cargo test --workspace    # → all pass
```

## must_haves

- **truth:** Backend baut ohne Warnings (Build und Clippy)
- **truth:** Alle Tests bleiben grün
- **truth:** Keine API-Änderungen in öffentlichen Traits oder Services
- **artifact:** SUMMARY.md mit Liste der gefixten Findings
- **key_link:** `cargo build --workspace` — clean output

## Rollback

Falls Tests brechen: `git reset --hard HEAD` vor jedem atomaren Commit. Da pro Logik-Block einzeln committet wird, kann jeder Schritt isoliert zurückgerollt werden.
