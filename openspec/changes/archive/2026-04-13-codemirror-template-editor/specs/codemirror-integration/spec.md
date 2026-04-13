## ADDED Requirements

### Requirement: CodeMirror JS-Bundle bereitstellen
Das Frontend SHALL ein pre-built CodeMirror 6 JS-Bundle als statisches Asset unter `assets/codemirror-bundle.js` ausliefern. Das Bundle SHALL als IIFE-Format vorliegen und globale Funktionen auf `window` exportieren.

#### Scenario: Bundle wird beim Seitenaufruf geladen
- **WHEN** ein Nutzer die Anwendung öffnet
- **THEN** SHALL das CodeMirror-Bundle als Script-Tag geladen werden

### Requirement: Typst WASM-Parser bereitstellen
Das Frontend SHALL den Typst-Syntax-Parser als separates WASM-File unter `assets/typst-parser.wasm` ausliefern. Das JS-Bundle SHALL das WASM-File per `fetch()` laden.

#### Scenario: WASM-Parser wird beim ersten Editor-Öffnen geladen
- **WHEN** ein Nutzer erstmals eine Template-Datei zum Bearbeiten öffnet
- **THEN** SHALL das JS-Bundle den WASM-Parser laden und initialisieren

### Requirement: Globale JS-Interop-API
Das CodeMirror-Bundle SHALL folgende globale Funktionen auf `window` bereitstellen:
- `createTypstEditor(elementId, content, onChangeCallback)` → editorId
- `setEditorContent(editorId, content)`
- `getEditorContent(editorId)` → string
- `destroyEditor(editorId)`

#### Scenario: Editor erstellen
- **WHEN** `createTypstEditor("editor-container", "Hello #strong[world]", callback)` aufgerufen wird
- **THEN** SHALL ein CodeMirror-Editor im Element mit der gegebenen ID erstellt werden
- **AND** der Editor SHALL den übergebenen Inhalt anzeigen
- **AND** der Editor SHALL bei Textänderungen den Callback mit dem neuen Inhalt aufrufen

#### Scenario: Editor-Inhalt programmatisch setzen
- **WHEN** `setEditorContent(editorId, "Neuer Inhalt")` aufgerufen wird
- **THEN** SHALL der Editor-Inhalt ersetzt werden ohne einen onChange-Callback auszulösen

#### Scenario: Editor-Inhalt auslesen
- **WHEN** `getEditorContent(editorId)` aufgerufen wird
- **THEN** SHALL der aktuelle Editor-Inhalt als String zurückgegeben werden

#### Scenario: Editor zerstören
- **WHEN** `destroyEditor(editorId)` aufgerufen wird
- **THEN** SHALL die CodeMirror-Instanz entfernt und Ressourcen freigegeben werden

### Requirement: Rust wasm-bindgen Bindings
Das Frontend SHALL in `js.rs` extern-Deklarationen für die globalen CodeMirror-Funktionen bereitstellen, sodass Rust-Code die JS-API typsicher aufrufen kann.

#### Scenario: Editor aus Rust erstellen
- **WHEN** Rust-Code `create_typst_editor(element_id, content, callback)` aufruft
- **THEN** SHALL die JS-Funktion `window.createTypstEditor` aufgerufen werden

### Requirement: onChange-Debouncing
Das CodeMirror-Bundle SHALL onChange-Callbacks mit einem Debounce von ca. 300ms versehen, um die WASM-Boundary nicht bei jedem Tastendruck zu kreuzen.

#### Scenario: Schnelles Tippen löst nur einen Callback aus
- **WHEN** ein Nutzer 5 Zeichen in schneller Folge tippt
- **THEN** SHALL nur ein onChange-Callback nach der Tipp-Pause ausgelöst werden

### Requirement: Build-Skript für CodeMirror-Bundle
Ein `build.sh` im Verzeichnis `genossi-frontend/codemirror/` SHALL das JS-Bundle und die WASM-Datei erstellen und nach `assets/` kopieren. Dieses Skript ist NICHT Teil des regulären cargo-Builds.

#### Scenario: Bundle neu bauen
- **WHEN** ein Entwickler `./build.sh` im `codemirror/`-Verzeichnis ausführt
- **THEN** SHALL `assets/codemirror-bundle.js` und `assets/typst-parser.wasm` aktualisiert werden
