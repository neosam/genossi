## Context

The CodeMirror template editor is bundled via esbuild as an IIFE. The `codemirror-lang-typst` package provides Typst syntax highlighting using a WASM-based parser (compiled from the Typst Rust codebase via wasm-pack with `--target=bundler`). The WASM import uses ESM syntax (`import * as wasm from "*.wasm"`), which esbuild cannot resolve natively. Currently the package is excluded via `--external`, so no highlighting is available.

## Goals / Non-Goals

**Goals:**
- Working Typst syntax highlighting in the CodeMirror template editor
- Single-file bundle (no separate WASM file to serve)
- No changes to the Dioxus frontend Rust code

**Non-Goals:**
- Typst autocompletion or LSP-like features
- Custom theme for the Typst highlighting (the package provides a default)
- Reducing bundle size

## Decisions

### Decision: Use esbuild-plugin-wasm with embedded mode

**Choice**: Use `esbuild-plugin-wasm` to handle the WASM import, embedding the binary as base64 in the JS bundle.

**Alternatives considered**:
- **Webpack**: Supports WASM ESM imports natively via `experiments.asyncWebAssembly`. Rejected because introducing a second build tool adds complexity for a single integration.
- **Vite**: Supports WASM via plugin. Same rationale as Webpack — unnecessary tool change.
- **Manual patching**: Rewrite the WASM loader to use `fetch()`. Fragile and breaks on package updates.
- **esbuild `--loader:.wasm=file`**: Outputs the file separately but returns a path string, not a WASM module — incompatible with the ESM import pattern used by wasm-pack.

**Rationale**: `esbuild-plugin-wasm` with `mode: "embedded"` inlines the WASM as base64, then instantiates it at runtime. This keeps the single-file deployment model and stays within the existing esbuild toolchain.

### Decision: Switch build script from shell to JS

**Choice**: Replace the esbuild CLI call in `build.sh` with a `build.mjs` script using the esbuild JS API, because esbuild plugins are only available via the JS API, not the CLI.

`build.sh` remains as the entry point but delegates to `node build.mjs`.

### Decision: Static import instead of dynamic

**Choice**: Replace `await import("codemirror-lang-typst")` with a static `import { typst } from "codemirror-lang-typst"` since the package is now bundled.

This removes the async loading logic, retry mechanism, and the fallback path where the editor loads without highlighting.

## Risks / Trade-offs

- **Bundle size increase (~400 KB)**: The WASM is ~313 KB, base64 adds ~33% overhead → ~417 KB additional. Total bundle goes from ~375 KB to ~800 KB. Acceptable for an internal admin tool.
- **esbuild-plugin-wasm compatibility**: The plugin must correctly handle wasm-pack's `--target=bundler` output pattern. If it doesn't work, fallback is Option C (manual WASM patching) or switching to embedded mode with `deferred` and serving the WASM file separately.
- **IIFE + async WASM init**: Embedded WASM is instantiated asynchronously even in an IIFE bundle. The `createTypstEditor` function may be called before WASM is ready. The existing retry loop in `templates.rs` (polling for `window.createTypstEditor`) already handles this.
