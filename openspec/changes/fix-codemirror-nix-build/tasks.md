## 1. CodeMirror Bundle in Nix Build integrieren

- [x] 1.1 Add npm dependency fetching for the `codemirror/` directory in `genossi-frontend/flake.nix` (e.g., using `pkgs.buildNpmPackage`, `fetchNpmDeps`, or `importNpmLock` to handle `node_modules` in the Nix sandbox)
- [x] 1.2 Add CodeMirror bundle build step to `buildPhase`: run `node build.mjs` in `codemirror/` directory after npm dependencies are available
- [x] 1.3 Copy `assets/codemirror-bundle.js` to `dist/` in the `buildPhase`

## 2. Generated index.html aktualisieren

- [x] 2.1 Add `<script type="module" src="/codemirror-bundle.js"></script>` to the Nix-generated `index.html` heredoc in `flake.nix`, before the WASM init script

## 3. Verifizierung

- [x] 3.1 Run `nix build .#frontend` and verify `codemirror-bundle.js` is present in the output
- [x] 3.2 Verify the generated `index.html` contains the codemirror script tag
