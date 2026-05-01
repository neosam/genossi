## Meta
- **Priority:** low
- **Category:** quality

## Why

Die Backend-Crates (`genossi_rest`, `genossi_config`, `genossi_mail`, `genossi_service`, DAO-Crates) definieren ihre Error-Enums handschriftlich: manuelle `From`-Impls pro Conversion, keine `std::error::Error`-Trait-Implementierung. Display-Impls fehlen weitgehend — nur `ServiceError` hat einen (`genossi_service/src/lib.rs:40`); `RestError`, `ConfigServiceError` und `MailServiceError` haben keinen. Als Konsequenz greifen die `error_handler` auf die inneren `String`-Felder der Enum-Varianten zu und loggen diese via `{}` (z. B. `genossi_rest/src/lib.rs:145`, `genossi_config/src/rest.rs:64`) — das Error-Enum selbst ist nirgends Display-formatierbar, wodurch generisches Error-Handling (etwa eine zentrale Log-Helper-Funktion) nicht möglich ist. Das Frontend-Crate (`genossi-frontend`) verwendet bereits `thiserror` und demonstriert, wie kompakt das sein kann (`genossi-frontend/src/error.rs`).

Während die Call-Sites funktionieren, fehlt ein einheitliches Error-Source-Chaining, und jede neue Fehlerquelle kostet 5-10 Zeilen Boilerplate. Die Migration reduziert wiederkehrenden Code, bringt die Backend-Crates auf denselben Stand wie das Frontend und fügt als Nebeneffekt konsistente `Display`-Impls für bessere Log-Ausgaben hinzu.

## What Changes

- `thiserror` als Workspace-Dependency ergänzen.
- Backend-Error-Enums auf `#[derive(thiserror::Error, Debug)]` umstellen:
  - `RestError` in `genossi_rest/src/lib.rs`
  - `ConfigServiceError` in `genossi_config/src/service.rs`
  - `MailServiceError` in `genossi_mail/src/service.rs`
  - `ServiceError` in `genossi_service/src/lib.rs`
  - Weitere hand-rolled Error-Enums in DAO- und Service-Impl-Crates (vollständige Inventarisierung im Design)
- `Display` konsistent über alle Error-Types via `#[error("...")]`-Attribute einführen (bzw. für `ServiceError` die bestehende manuelle `Display`-Impl ablösen).
- Manuelle `impl From<X> for Y` durch `#[from]`-Attribute auf den jeweiligen Varianten ersetzen, wo die Semantik direkt passt.
- Weiterhin manuelle `From`-Impls behalten, wo die Mapping-Logik nicht-trivial ist (z. B. `ServiceError::ValidationError` nach `RestError::BadRequest` mit Feld-Aggregation).
- Keine API-Vertragsänderungen: die öffentlichen HTTP-Responses bleiben identisch.

## Capabilities

### New Capabilities

_(keine)_

### Modified Capabilities

_(keine — reines internes Refactoring, keine spec-relevante Verhaltensänderung)_

## Impact

**Code:**
- `Cargo.toml` (Workspace) — `thiserror` ergänzen
- Alle Backend-Crates mit eigenem Error-Enum

**Tests:** existierende Tests müssen weiter grün sein; keine neuen Tests erforderlich.

**Risiko:** Gering bis mittel. Compiler fängt alle Regressionen, aber Mapping-Entscheidungen (welche Variante für welchen Fehler) müssen pro Call-Site geprüft werden.

**Abhängigkeit:** Wird idealerweise **nach** `security-quick-fixes` gemerged, um Konflikte mit den dort ergänzten manuellen `From<serde_json::Error>`-Impls zu vermeiden. Diese Impls wandern dann in `#[from]`-Attribute.
