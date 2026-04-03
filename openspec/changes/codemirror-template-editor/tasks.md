## 1. CodeMirror Bundle erstellen

- [x] 1.1 Verzeichnis `genossi-frontend/codemirror/` anlegen mit `package.json` (Abhängigkeiten: `codemirror`, `codemirror-lang-typst`, `esbuild`)
- [x] 1.2 `entry.js` erstellen: CodeMirror mit Typst-Language, basicSetup, und globalen Funktionen (`createTypstEditor`, `setEditorContent`, `getEditorContent`, `destroyEditor`) mit onChange-Debouncing (300ms)
- [x] 1.3 `build.sh` erstellen: npm install + esbuild-Aufruf → `../assets/codemirror-bundle.js` + WASM-Datei nach `../assets/` kopieren
- [x] 1.4 Bundle bauen, Ergebnis-Dateien (`codemirror-bundle.js`, `typst-parser.wasm`) in `assets/` einchecken
- [x] 1.5 `node_modules/` in `.gitignore` eintragen

## 2. Dioxus-Integration vorbereiten

- [x] 2.1 `Dioxus.toml` anpassen: `script = ["/codemirror-bundle.js"]` eintragen
- [x] 2.2 `js.rs` erweitern: `#[wasm_bindgen]` extern-Deklarationen für `createTypstEditor`, `setEditorContent`, `getEditorContent`, `destroyEditor`

## 3. Template-Editor-Seite umbauen

- [x] 3.1 In `templates.rs`: Textarea durch ein Container-`div` mit fester ID ersetzen
- [x] 3.2 `onmounted`-Hook: CodeMirror-Instanz via `createTypstEditor()` erstellen, onChange-Callback an Dioxus-Signal binden
- [x] 3.3 Dateiwechsel-Logik: `setEditorContent()` aufrufen wenn eine neue Datei geladen wird
- [x] 3.4 Speichern-Logik: `getEditorContent()` statt textarea-Value verwenden
- [x] 3.5 Unsaved-Changes-Tracking an CodeMirror-Content anbinden
- [x] 3.6 Cleanup: `destroyEditor()` beim Unmount aufrufen

## 4. Testen

- [x] 4.1 Manuell testen: Editor öffnet, Syntax-Highlighting funktioniert, Laden/Speichern/Dateiwechsel arbeiten korrekt
- [x] 4.2 Bestehende E2E-Tests prüfen und ggf. anpassen (falls Template-Editor-Tests existieren)
