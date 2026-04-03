## 1. Build Tooling

- [x] 1.1 Add `esbuild-plugin-wasm` as devDependency in `genossi-frontend/codemirror/package.json`
- [x] 1.2 Create `genossi-frontend/codemirror/build.mjs` using esbuild JS API with wasmLoader plugin (mode: "embedded")
- [x] 1.3 Update `genossi-frontend/codemirror/build.sh` to call `node build.mjs` instead of `npx esbuild` CLI, remove `--external:codemirror-lang-typst`

## 2. CodeMirror Entry Point

- [x] 2.1 Replace dynamic `import("codemirror-lang-typst")` with static `import { typst } from "codemirror-lang-typst"` in `entry.js`
- [x] 2.2 Remove `typstLang`, `typstLoading`, and `loadTypst()` async loading logic
- [x] 2.3 Add `typst()` directly to the extensions array in `createTypstEditor`

## 3. Build and Verify

- [x] 3.1 Run `npm install` and `build.sh` to produce the new bundle
- [x] 3.2 Verify the bundle includes the WASM (check file size is ~800 KB)
- [x] 3.3 Commit updated `codemirror-bundle.js` to assets
