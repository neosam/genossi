## Why

The CodeMirror template editor currently loads without Typst syntax highlighting. The `codemirror-lang-typst` npm package is declared as a dependency but marked as `--external` in the esbuild configuration, so it is never bundled. The package uses a WASM-based Typst parser (`typst_syntax_bg.wasm`) imported via ESM syntax, which esbuild cannot handle natively. Board members editing Typst templates have no visual feedback for syntax errors, keywords, or structure.

## What Changes

- Replace the esbuild CLI invocation with a JS-based build script using `esbuild-plugin-wasm` to handle WASM imports
- Remove `--external:codemirror-lang-typst` so the package (including its WASM parser) is bundled
- Embed the WASM binary (~313 KB) inline in the bundle to avoid separate file serving
- Simplify `entry.js` to use a static import instead of the async dynamic import fallback

## Capabilities

### New Capabilities

### Modified Capabilities
- `template-editor`: The code editor gains working Typst syntax highlighting via the bundled WASM parser

## Impact

- `genossi-frontend/codemirror/build.sh`: Replaced with call to new `build.mjs`
- `genossi-frontend/codemirror/build.mjs`: New esbuild build script with WASM plugin
- `genossi-frontend/codemirror/entry.js`: Simplified imports, removed async loading logic
- `genossi-frontend/codemirror/package.json`: New devDependency `esbuild-plugin-wasm`
- `genossi-frontend/assets/codemirror-bundle.js`: Larger bundle (~800 KB vs ~375 KB) due to embedded WASM
- No backend changes required
