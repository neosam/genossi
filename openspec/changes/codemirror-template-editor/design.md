## Context

Der Template-Editor im Dioxus-Frontend (WASM) nutzt aktuell ein HTML-`<textarea>` für Typst-Dateien. Das Frontend hat bereits JS-Interop-Infrastruktur (`wasm-bindgen`, `web-sys`, `js-sys`) und ein eigenes `js.rs`-Modul mit Clipboard- und Date-Bindings. Dioxus 0.6.3 ist im Einsatz. CodeMirror 6 ist modular aufgebaut und muss gebundelt werden. Für Typst existiert das Community-Paket `codemirror-lang-typst`, das einen WASM-basierten Parser aus dem offiziellen `typst-syntax`-Crate nutzt.

## Goals / Non-Goals

**Goals:**
- CodeMirror 6 als Code-Editor mit Typst-Syntax-Highlighting einbinden
- Zeilennummern, Bracket-Matching, Suchen/Ersetzen, Undo/Redo bereitstellen
- Bidirektionale State-Synchronisation zwischen CodeMirror (JS) und Dioxus (Rust/WASM)
- npm/node NICHT als Teil des regulären Build-Prozesses erforderlich machen
- Alle bestehenden Editor-Features (Laden, Speichern, Unsaved-Changes, Dateiwechsel) beibehalten

**Non-Goals:**
- Kein Autocomplete für Typst-Variablen (spätere Erweiterung)
- Keine Live-Preview (Side-by-side Editor + PDF) — spätere Erweiterung
- Kein LSP oder erweiterte Diagnostik

## Decisions

### 1. Pre-built JS-Bundle, eingecheckt in assets/

CodeMirror wird in einem separaten `genossi-frontend/codemirror/`-Verzeichnis als npm-Projekt verwaltet. Ein `build.sh`-Skript baut mit `esbuild` ein einzelnes IIFE-Bundle (`codemirror-bundle.js`) und kopiert das Typst-WASM (`typst-parser.wasm`) nach `assets/`. Beide Dateien werden eingecheckt.

**Warum**: Der reguläre `cargo build` / `dx serve` bleibt frei von npm/node. Das Bundle wird nur bei CodeMirror-Updates neu gebaut. Alternativen:
- CDN zur Laufzeit: Erfordert Internetzugang, ungeeignet für Self-Hosting
- `eval()`-basiert: Fragil, schlecht wartbar
- Monaco: Overkill, ~2MB Bundle

### 2. Globale JS-Funktionen als Interop-API

Das Bundle exportiert globale Funktionen auf `window`:

```
window.createTypstEditor(elementId, content, onChangeCallback) → editorId
window.setEditorContent(editorId, content)
window.getEditorContent(editorId) → string
window.destroyEditor(editorId)
```

Auf Rust-Seite werden diese via `#[wasm_bindgen]` extern-Deklarationen in `js.rs` angebunden.

**Warum**: Minimale Kopplung. Das JS-Bundle kennt Dioxus nicht, Dioxus kennt CodeMirror nicht — nur die 4 Funktionen bilden die Schnittstelle. Alternative wäre Custom Elements / Web Components, aber das wäre overengineered.

### 3. onChange-Callback via Closure

Der `onChangeCallback` wird als `wasm-bindgen` Closure an JS übergeben. Bei jeder Änderung im Editor ruft CodeMirror den Callback mit dem neuen Inhalt auf. Debouncing (z.B. 300ms) passiert auf JS-Seite, um die WASM-Boundary nicht bei jedem Tastendruck zu kreuzen.

**Warum**: Das Projekt nutzt bereits `wasm-bindgen`-Closures (siehe Clipboard-API in `js.rs`). Debouncing auf JS-Seite reduziert die Interop-Kosten. Alternativ könnte Rust pollen (`getEditorContent` in einem Interval), aber das wäre weniger responsiv.

### 4. WASM-Parser als separates Asset

Das `typst-parser.wasm` wird als separate Datei in `assets/` ausgeliefert. Das JS-Bundle lädt es per `fetch()` beim ersten Editor-Öffnen. Der Pfad wird im Bundle als konfigurierbare Konstante hinterlegt.

**Warum**: Inline (base64) würde das JS-Bundle um ~500KB aufblähen und den Initial-Load verlangsamen. Als separates File wird es lazy und cachebar geladen.

### 5. Editor-Lifecycle an Dioxus-Component gebunden

- `onmounted`: Editor erstellen via `createTypstEditor()`
- Dateiwechsel: `setEditorContent()` mit neuem Inhalt aufrufen
- Component-Unmount: `destroyEditor()` aufrufen (Cleanup)

Der Editor-Container ist ein leeres `div` mit fester ID, das Dioxus rendert. CodeMirror füllt es dann via JS.

**Warum**: Dioxus' Virtual DOM darf den Editor-DOM-Baum nicht anfassen — CodeMirror verwaltet seinen eigenen DOM. Ein leeres Container-div mit stabiler ID ist die saubere Grenze.

## Risks / Trade-offs

- **WASM-in-WASM Kompatibilität**: Das Frontend ist selbst WASM, der Typst-Parser ist auch WASM. → Mitigation: Browser unterstützen mehrere WASM-Module problemlos. Das JS-Bundle lädt den Parser unabhängig vom Dioxus-WASM.

- **Bundle-Größe (~300-350KB gzip)**: Signifikanter Overhead für den Initial-Load. → Mitigation: Das Bundle wird als `defer`-Script geladen und der WASM-Parser lazy. Templates-Seite ist nicht die Startseite.

- **Eingecheckte Binaries**: `.js` und `.wasm` im Git-Repo. → Mitigation: Nur zwei Dateien, Updates sind selten. `.gitattributes` kann binäre Diffs unterdrücken.

- **Typst-Parser-Kompatibilität**: `codemirror-lang-typst` ist experimentell und könnte bei Typst-Updates brechen. → Mitigation: Highlighting ist nice-to-have, bei Problemen kann man auf generisches Highlighting zurückfallen ohne den Editor zu verlieren.

## Open Questions

- Soll das JS-Bundle mit einem Dark-Theme ausgeliefert werden, oder reicht das Standard-Theme?
