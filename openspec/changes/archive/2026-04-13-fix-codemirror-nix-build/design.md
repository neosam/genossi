## Context

The Nix frontend build (`genossi-frontend/flake.nix`) compiles the Dioxus WASM app and generates its own `index.html`. It does not build or include the CodeMirror bundle (`codemirror-bundle.js`), which provides the `window.createTypstEditor` function needed by the template editor.

The CodeMirror bundle is built separately via esbuild (`codemirror/build.mjs`), which bundles `codemirror/entry.js` with its npm dependencies including `codemirror-lang-typst` (which embeds WASM for Typst syntax highlighting).

## Goals / Non-Goals

**Goals:**
- CodeMirror bundle is built and included in the Nix build output
- The generated `index.html` loads the bundle before the WASM app
- No changes to local development workflow (`dx serve`)

**Non-Goals:**
- Refactoring how the CodeMirror integration works
- Switching from esbuild to another bundler
- Changing the `entry.js` or Rust FFI layer

## Decisions

### Build the CodeMirror bundle in the Nix buildPhase

Add steps to `buildPhase` in `flake.nix`:
1. Run `npm install` in the `codemirror/` directory (with `HOME=$TMPDIR` for npm cache)
2. Run `node build.mjs` to produce `assets/codemirror-bundle.js`
3. Copy `assets/codemirror-bundle.js` to `dist/`

**Rationale**: The npm tooling is already in `nativeBuildInputs`. Building in-place follows the same pattern as the Tailwind CSS build that already exists in the `buildPhase`.

### Add script tag to the Nix-generated index.html

Insert `<script type="module" src="/codemirror-bundle.js"></script>` before the WASM init script in the heredoc `index.html`.

**Rationale**: Must load before the WASM app so `window.createTypstEditor` is available. Module scripts execute in document order, so placing it first guarantees this.

## Risks / Trade-offs

- **[npm install in Nix sandbox]** → Nix builds are sandboxed without network. Must use a fixed-output derivation (e.g., `fetchNpmDeps`, `npmDeps`) or pre-fetch `node_modules`. Alternatively, commit the pre-built `codemirror-bundle.js` or use `pkgs.buildNpmPackage`.
  - Mitigation: Use Nix's `pkgs.buildNpmPackage` or `fetchNpmDeps` to handle npm dependencies reproducibly. Or use `npmDeps = pkgs.importNpmLock { npmRoot = ./codemirror; }` if available.
- **[WASM embedded in bundle]** → The `codemirror-lang-typst` package includes embedded WASM (via esbuild-plugin-wasm). This should work fine since it's embedded as base64 data, not fetched at runtime.
