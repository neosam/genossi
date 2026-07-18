/** @type {import('tailwindcss').Config} */

const colors = require("tailwindcss/colors");
module.exports = {
  mode: "all",
  content: ["./src/**/*.{rs,html,css}", "./dist/**/*.html"],
  theme: {
    extend: {
      screens: {
        print: { raw: 'print' },
        screen: { raw: 'screen' },
      },
      colors: {
        missingColor: colors.amber[200],
        blockedColor: colors.red[300],
      },
    },
  },
  plugins: [],
  safelist: [
    "bg-red-200",
    "print:bg-white",
    "cursor-not-allowed",
    "text-green-800",
    "text-red-800",
    "bg-missingColor",
    "bg-blockedColor",
    // ─── Phase 4 Plan 02 ─── (RESEARCH Pitfall 6 — präventive Erhaltung)
    "qr-card",
    "bg-amber-100",
    "text-amber-900",
    "border-amber-400",
    "border-b-2",
    "animate-spin",
    "animate-pulse",
    "print:hidden",
    // Quick 260718-wysiwyg-editor-preview-css-fix — WYSIWYG-Editor + TemplatePreview
    // custom-scoped semantic-HTML re-styling. Belt-and-suspenders against purge:
    // the class is referenced in wysiwyg_editor.rs + template_preview.rs which
    // `content` already covers, but safelist keeps it alive if content-globs shift.
    "mail-html-render",
  ]
};
