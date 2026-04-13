## Why

The CodeMirror-based Typst editor works locally with `dx serve` but fails in the Nix-deployed version with `TypeError: window.createTypstEditor is not a function`. The Nix build (`genossi-frontend/flake.nix`) neither builds the CodeMirror bundle nor includes it in the output, and generates its own `index.html` that lacks the `<script>` tag for the bundle.

## What Changes

- Build the CodeMirror bundle (`npm install && node build.mjs`) during the Nix build phase
- Copy `codemirror-bundle.js` into the `dist/` output
- Add the `<script type="module" src="/codemirror-bundle.js"></script>` tag to the Nix-generated `index.html`

## Capabilities

### New Capabilities

_(none — this is a build/deployment fix, not a new feature)_

### Modified Capabilities

- `template-editor`: The Typst editor requires the CodeMirror bundle to be present at runtime. The Nix build must produce this asset.

## Impact

- **File**: `genossi-frontend/flake.nix` — build phase and generated `index.html`
- **Dependencies**: Node.js and npm are already available in `nativeBuildInputs`
- **Risk**: Low — only affects build output, no runtime code changes
