// Home editor panel — CodeMirror 6 Clojure editor with paredit and inline eval.
// CodeMirror and the Clojure language are loaded from an ESM CDN.

import { EditorView, basicSetup, keymap } from 'https://esm.sh/codemirror';
import { EditorState, Prec } from 'https://esm.sh/@codemirror/state';
import { syntaxTree } from 'https://esm.sh/@codemirror/language';
import { clojure } from 'https://esm.sh/@nextjournal/lang-clojure';
import { oneDark } from 'https://esm.sh/@codemirror/theme-one-dark';

const MOUNT_ID = 'hara-editor-mount';
const PANEL_ID = 'hara-editor-panel';
const RUN_ID = 'hara-editor-run';
const DOWNLOAD_ID = 'hara-editor-download';
const FORK_ID = 'hara-editor-fork';

const COLLECTION_TYPES = new Set(['List', 'Vector', 'Map', 'Set', 'AnonymousFunction']);

function isCollection(node) {
  return node && COLLECTION_TYPES.has(node.type.name);
}

function enclosingCollection(state, pos) {
  let node = syntaxTree(state).resolveInner(pos, -1);
  while (node && node.type.name !== 'Program') {
    if (isCollection(node)) return node;
    node = node.parent;
  }
  return null;
}

function topLevelFormAt(state, pos) {
  let node = syntaxTree(state).resolveInner(pos, -1);
  while (node && node.type.name !== 'Program') {
    if (node.parent && node.parent.type.name === 'Program') {
      return state.doc.sliceString(node.from, node.to);
    }
    node = node.parent;
  }
  return '';
}

function slurpForward(view) {
  const { state } = view;
  const coll = enclosingCollection(state, state.selection.main.head);
  if (!coll) return false;
  const next = coll.nextSibling;
  if (!next) return false;
  const closePos = coll.to - 1;
  const text = state.doc.sliceString(next.from, next.to);
  view.dispatch({
    changes: [
      { from: coll.to, to: next.to },
      { from: closePos, insert: text },
    ],
  });
  return true;
}

function barfForward(view) {
  const { state } = view;
  const coll = enclosingCollection(state, state.selection.main.head);
  if (!coll) return false;
  const closePos = coll.to - 1;
  const last = coll.childBefore(closePos);
  if (!last || last.to >= closePos) return false;
  const text = state.doc.sliceString(last.from, last.to);
  view.dispatch({
    changes: [
      { from: last.from, to: last.to },
      { from: coll.to, insert: ' ' + text },
    ],
  });
  return true;
}

function spliceCollection(view) {
  const { state } = view;
  const coll = enclosingCollection(state, state.selection.main.head);
  if (!coll) return false;
  view.dispatch({
    changes: [
      { from: coll.from, to: coll.from + 1 },
      { from: coll.to - 1, to: coll.to },
    ],
  });
  return true;
}

function raiseForm(view) {
  const { state } = view;
  const pos = state.selection.main.head;
  const coll = enclosingCollection(state, pos);
  if (!coll) return false;
  let node = syntaxTree(state).resolveInner(pos, -1);
  // Skip delimiter tokens up to the containing collection.
  while (node && node !== coll && (node.from < coll.from || node.to > coll.to)) {
    node = node.parent;
  }
  if (!node || node === coll) return false;
  const text = state.doc.sliceString(node.from, node.to);
  view.dispatch({ changes: { from: coll.from, to: coll.to, insert: text } });
  return true;
}

function wrapParens(view) {
  const { state } = view;
  const range = state.selection.main;
  let from, to;
  if (range.empty) {
    const node = syntaxTree(state).resolveInner(range.head, -1);
    from = node.from;
    to = node.to;
  } else {
    from = range.from;
    to = range.to;
  }
  view.dispatch({
    changes: [
      { from: to, insert: ')' },
      { from, insert: '(' },
    ],
    selection: { anchor: from + 1 },
  });
  return true;
}

function moveSexpForward(view) {
  const { state } = view;
  const pos = state.selection.main.head;
  const node = syntaxTree(state).resolveInner(pos, -1);
  let target;
  if (node.nextSibling) {
    target = node.nextSibling.to;
  } else {
    const coll = enclosingCollection(state, pos);
    target = coll ? coll.to : state.doc.length;
  }
  view.dispatch({ selection: { anchor: target }, scrollIntoView: true });
  return true;
}

function moveSexpBackward(view) {
  const { state } = view;
  const pos = state.selection.main.head;
  const node = syntaxTree(state).resolveInner(pos, -1);
  let target;
  if (node.prevSibling) {
    target = node.prevSibling.from;
  } else {
    const coll = enclosingCollection(state, pos);
    target = coll ? coll.from : 0;
  }
  view.dispatch({ selection: { anchor: target }, scrollIntoView: true });
  return true;
}

function evalForm(form) {
  if (!window.haraRepl) {
    console.error('[hara editor] REPL not ready');
    return;
  }
  try {
    const result = window.haraRepl.eval(form);
    window.haraRepl.log(result);
  } catch (err) {
    window.haraRepl.log(String(err));
  }
}

function evalSelectionOrTopFormCmd(view) {
  const { state } = view;
  const range = state.selection.main;
  const form = range.empty
    ? topLevelFormAt(state, range.head)
    : state.sliceDoc(range.from, range.to).trim();
  if (form) evalForm(form);
  return true;
}

function evalTopFormCmd(view) {
  const form = topLevelFormAt(view.state, view.state.selection.main.head);
  if (form) evalForm(form);
  return true;
}

function createEditor() {
  const parent = document.getElementById(MOUNT_ID);
  if (!parent) return null;

  const customKeymap = Prec.highest(
    keymap.of([
      { key: 'Mod-Enter', run: evalSelectionOrTopFormCmd },
      { key: 'Mod-Shift-Enter', run: evalTopFormCmd },
      { key: 'Mod-ArrowRight', run: slurpForward },
      { key: 'Alt-ArrowRight', run: slurpForward },
      { key: 'Mod-ArrowLeft', run: barfForward },
      { key: 'Alt-ArrowLeft', run: barfForward },
      { key: 'Alt-s', run: spliceCollection },
      { key: 'Alt-r', run: raiseForm },
      { key: 'Mod-w', run: wrapParens },
      { key: 'Mod-ArrowDown', run: moveSexpForward },
      { key: 'Alt-ArrowDown', run: moveSexpForward },
      { key: 'Mod-ArrowUp', run: moveSexpBackward },
      { key: 'Alt-ArrowUp', run: moveSexpBackward },
    ]),
  );

  const state = EditorState.create({
    doc: '',
    extensions: [basicSetup, oneDark, clojure(), customKeymap],
  });

  return new EditorView({ state, parent });
}

function waitForGlobal(name, timeout = 10000) {
  return new Promise((resolve, reject) => {
    if (window[name]) {
      resolve(window[name]);
      return;
    }
    const start = performance.now();
    const check = () => {
      if (window[name]) {
        resolve(window[name]);
        return;
      }
      if (performance.now() - start > timeout) {
        reject(new Error(`${name} not available`));
        return;
      }
      requestAnimationFrame(check);
    };
    check();
  });
}

async function loadSceneSource(id) {
  const response = await fetch(new URL(`scenes/${encodeURIComponent(id)}.hal`, document.baseURI));
  if (!response.ok) {
    throw new Error(`fetch scenes/${id}.hal: HTTP ${response.status}`);
  }
  return response.text();
}

function downloadSource(id, source) {
  const blob = new Blob([source], { type: 'text/plain' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `${id}.hal`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

function forkOnGitHub(id, source) {
  const filename = encodeURIComponent(`${id}.hal`);
  const value = encodeURIComponent(source);
  const url = `https://github.com/hoebat/hara.lang/new/main/scenes?filename=${filename}&value=${value}`;
  window.open(url, '_blank');
}

function getActiveIframe() {
  return document.querySelector('#hara-scene-mount iframe');
}

async function init() {
  const manager = await waitForGlobal('haraSceneManager');
  const panel = document.getElementById(PANEL_ID);
  const runBtn = document.getElementById(RUN_ID);
  const downloadBtn = document.getElementById(DOWNLOAD_ID);
  const forkBtn = document.getElementById(FORK_ID);

  const view = createEditor();
  if (!view) {
    console.error('[hara editor] mount point not found');
    return;
  }

  let loadedSceneId = null;

  const loadScene = async (id) => {
    if (!id || id === loadedSceneId) return;
    try {
      const source = await loadSceneSource(id);
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: source },
        selection: { anchor: 0 },
      });
      loadedSceneId = id;
    } catch (err) {
      console.error('[hara editor]', err);
      if (window.haraRepl) window.haraRepl.log(String(err));
    }
  };

  const getValue = () => view.state.doc.toString();
  const setValue = (text) => {
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: String(text) },
      selection: { anchor: 0 },
    });
  };

  runBtn?.addEventListener('click', () => {
    const iframe = getActiveIframe();
    if (!iframe?.contentWindow) return;
    iframe.contentWindow.postMessage(
      { type: 'hara:scene-source', source: getValue() },
      '*',
    );
  });

  downloadBtn?.addEventListener('click', () => {
    const id = manager.currentSceneId || loadedSceneId || 'scene';
    downloadSource(id, getValue());
  });

  forkBtn?.addEventListener('click', () => {
    const id = manager.currentSceneId || loadedSceneId || 'scene';
    forkOnGitHub(id, getValue());
  });

  // Load source when the editor panel is open; poll for scene id changes.
  setInterval(() => {
    if (!panel?.classList.contains('open')) return;
    const id = manager.currentSceneId;
    if (id && id !== loadedSceneId) {
      loadScene(id);
    }
  }, 500);

  window.haraEditor = {
    loadScene,
    getValue,
    setValue,
    evalSelectionOrTopForm: evalSelectionOrTopFormCmd.bind(null, view),
  };
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init().catch((err) => console.error('[hara editor] init failed:', err));
}
