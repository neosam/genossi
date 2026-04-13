## Why

Der Template-Editor verwendet aktuell ein einfaches HTML-`<textarea>` zum Bearbeiten von Typst-Dateien. Das bietet weder Syntax-Highlighting, Zeilennummern, noch sonstige Editor-Features. Typst hat eine eigene Syntax, die sich von Markdown unterscheidet — ohne Highlighting ist das Bearbeiten fehleranfällig und unkomfortabel.

## What Changes

- CodeMirror 6 als JavaScript-Bundle in das Frontend einbinden (pre-built, eingecheckt)
- `codemirror-lang-typst` Paket für Typst-spezifisches Syntax-Highlighting einbinden (inkl. separatem WASM-Parser)
- Separates `codemirror/`-Verzeichnis im Frontend für den einmaligen JS-Build-Prozess (npm/esbuild, nicht Teil des regulären cargo-Builds)
- Das bestehende `<textarea>` durch eine CodeMirror-Editor-Instanz ersetzen
- JS-Interop über `wasm-bindgen` zur Kommunikation zwischen Dioxus (Rust/WASM) und CodeMirror (JS)

## Capabilities

### New Capabilities
- `codemirror-integration`: Einbindung von CodeMirror 6 als pre-built JS-Bundle mit Typst-Syntax-Highlighting. Stellt globale JS-Funktionen bereit (Editor erstellen, Inhalt lesen/setzen, onChange-Callback) und die zugehörige wasm-bindgen-Interop auf Rust-Seite.

### Modified Capabilities
- `template-editor`: Das textarea wird durch eine CodeMirror-Instanz ersetzt. Alle bestehenden Features (Laden, Speichern, Unsaved-Changes-Tracking, Dateiwechsel) bleiben erhalten, nutzen aber die CodeMirror-API statt direkter textarea-Manipulation.

## Impact

- **Frontend Assets**: Zwei neue statische Dateien (`codemirror-bundle.js`, `typst-parser.wasm`) werden ins `assets/`-Verzeichnis eingecheckt
- **Frontend Build-Config**: `Dioxus.toml` bekommt einen `script`-Eintrag für das CodeMirror-Bundle
- **Frontend JS-Interop**: Neues oder erweitertes Modul in `src/js.rs` mit `wasm-bindgen`-Bindings für CodeMirror-Funktionen
- **Frontend Templates-Page**: `src/page/templates.rs` wird modifiziert — textarea durch CodeMirror-Container-Element ersetzt, State-Synchronisation angepasst
- **Neues Verzeichnis**: `genossi-frontend/codemirror/` mit `entry.js`, `package.json`, `build.sh` (isoliert vom Rust-Build)
- **Keine Backend-Änderungen**
