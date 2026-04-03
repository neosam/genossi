import { build } from "esbuild";
import { wasmLoader } from "esbuild-plugin-wasm";

await build({
  entryPoints: ["entry.js"],
  bundle: true,
  format: "esm",
  minify: true,
  outfile: "../assets/codemirror-bundle.js",
  plugins: [wasmLoader({ mode: "embedded" })],
});
