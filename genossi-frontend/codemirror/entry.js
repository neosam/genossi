import { basicSetup } from "codemirror";
import { EditorView } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { typst } from "codemirror-lang-typst";

const editors = new Map();
let nextId = 1;

window.createTypstEditor = function (elementId, content, onChangeCallback) {
  const container = document.getElementById(elementId);
  if (!container) {
    console.error("CodeMirror: container not found:", elementId);
    return null;
  }

  const editorId = nextId++;
  const entry = { view: null, debounceTimer: null, suppressChange: false };

  const updateListener = EditorView.updateListener.of((update) => {
    if (update.docChanged && !entry.suppressChange && onChangeCallback) {
      clearTimeout(entry.debounceTimer);
      entry.debounceTimer = setTimeout(() => {
        const newContent = update.state.doc.toString();
        onChangeCallback(newContent);
      }, 300);
    }
  });

  const extensions = [basicSetup, updateListener, typst()];

  const state = EditorState.create({
    doc: content || "",
    extensions,
  });

  entry.view = new EditorView({
    state,
    parent: container,
  });

  editors.set(editorId, entry);
  return editorId;
};

window.setEditorContent = function (editorId, content) {
  const entry = editors.get(editorId);
  if (!entry) return;

  entry.suppressChange = true;
  entry.view.dispatch({
    changes: {
      from: 0,
      to: entry.view.state.doc.length,
      insert: content || "",
    },
  });
  setTimeout(() => {
    entry.suppressChange = false;
  }, 0);
};

window.getEditorContent = function (editorId) {
  const entry = editors.get(editorId);
  if (!entry) return "";
  return entry.view.state.doc.toString();
};

window.destroyEditor = function (editorId) {
  const entry = editors.get(editorId);
  if (!entry) return;
  clearTimeout(entry.debounceTimer);
  entry.view.destroy();
  editors.delete(editorId);
};
