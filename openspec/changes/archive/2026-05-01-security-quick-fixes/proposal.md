## Meta
- **Priority:** high
- **Category:** security

## Why

Das Security Audit (2026-04-18) hat mehrere kleine, aber sicherheitsrelevante Code-Probleme identifiziert, die jeweils in wenigen Zeilen behebbar sind. Zusammengefasst als ein Change, um den Overhead klein zu halten.

## What Changes

- **H3: `eprintln!` im Auth-Pfad ersetzen:** `genossi_rest/src/auth_middleware.rs:26` verwendet `eprintln!` statt `tracing`. Debug-Formatierung von Auth-Fehlern landet unkontrolliert in stderr — nicht filterbar, nicht rotierbar, potenziell sensible Daten.
  → Durch `tracing::warn!` ersetzen.

- **M1: CORS Methods/Headers einschränken:** `genossi_rest/src/lib.rs:366-367` erlaubt `AllowMethods::any()` und `AllowHeaders::any()`. Origins sind korrekt eingeschränkt, aber die offenen Methods/Headers erweitern die Angriffsfläche unnötig.
  → Methods auf `[GET, POST, PUT, DELETE, OPTIONS]`, Headers auf `[Content-Type, Authorization, Cookie]` einschränken.

- **N2: `unwrap()` in Serialisierungs-Pfaden entfernen:** Mehrere Stellen (z.B. `genossi_config/src/rest.rs:115`) verwenden `.unwrap()` bei `serde_json::to_string()`. Bei unerwarteten Daten würde der Server paniken.
  → Durch `map_err` mit HTTP 500-Response ersetzen.

## Capabilities

### New Capabilities

_(keine)_

### Modified Capabilities

- `http-perimeter`: CORS-Konfiguration wird restriktiver (Methods/Headers-Whitelist statt Any)

## Impact

**Code:**
- `genossi_rest/src/auth_middleware.rs` — 1 Zeile: `eprintln!` → `tracing::warn!`
- `genossi_rest/src/lib.rs` — CORS-Builder: `AllowMethods`/`AllowHeaders` auf explizite Listen
- `genossi_config/src/rest.rs` + weitere REST-Handler — `unwrap()` → Error-Handling

**Risiko:** Gering. Reine Code-Quality-Fixes ohne Verhaltensänderung für Endnutzer.
