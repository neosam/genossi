## Why

Benutzer sollen in ihren Typst-Templates das volle Typst-Package-Ökosystem nutzen können — z.B. `#import "@preview/letter-pro:3.0.0": letter-simple` für professionelle DIN-5008-Brieflayouts. Aktuell löst die `TemplateWorld`-Implementierung nur lokale Dateien auf und scheitert bei Package-Imports.

## What Changes

- `TemplateWorld` in `genossi_service_impl/src/pdf_generation.rs` wird erweitert, um `FileId`s mit `PackageSpec` zu erkennen und aufzulösen
- Ein Package-Cache wird eingeführt, der Packages beim ersten Zugriff von `packages.typst.org` herunterlädt und lokal cached
- Neue Dependencies: `flate2` und `tar` zum Entpacken der Package-Archive
- Konfigurierbare Cache-Location über Environment-Variable `TYPST_PACKAGE_CACHE`

## Capabilities

### New Capabilities
- `typst-package-resolution`: Automatisches Herunterladen und Caching von Typst-Packages aus dem offiziellen Package-Registry bei der Template-Kompilierung

### Modified Capabilities
- `pdf-generation`: Import-Auflösung wird erweitert um Package-Imports (`@namespace/name:version`)

## Impact

- **Code**: `genossi_service_impl/src/pdf_generation.rs` (TemplateWorld), `genossi_bin/src/main.rs` (Cache-Konfiguration)
- **Dependencies**: `flate2`, `tar` neu; `reqwest` wird auch im Service-Layer benötigt (bisher nur in Tests)
- **Netzwerk**: Server benötigt Internetzugang für Package-Downloads (nur beim ersten Zugriff auf ein neues Package)
- **Dateisystem**: Neues Cache-Verzeichnis für heruntergeladene Packages
