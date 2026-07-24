/* Hara Studio — UI shell behaviour (mock; wasm runtime pending).
   Wiring points are marked with data-hara-studio attributes in studio.html:
   the file tree, editor buffer, and REPL will be driven by the hara wasm
   module (rust/web/hta.js + hara.wasm) once linked. */
(function () {
  var shell = document.querySelector('[data-hara-studio="shell"]');
  if (!shell) return;

  var FILES = {
    '01-intro.hal':
      ';; 01 / INTRO — everything is a form\n;;\n' +
      ';; A form evaluates to a value. Try one in\n' +
      ';; the console below.\n\n' +
      '(ns lessons.intro)\n\n(+ 1 2 3)',
    '02-values.hal':
      ';; 02 / VALUES — data you can trust\n;;\n' +
      ';; Values are immutable. Keywords name things,\n' +
      ';; vectors and maps hold them.\n\n' +
      '(ns lessons.values)\n\n' +
      '{:name "grid" :lights [3 1 4]}',
    '03-functions.hal':
      ';; 03 / FUNCTIONS — small, composable units\n;;\n' +
      ';; defn binds a form to a name. Functions\n' +
      ';; are values too.\n\n' +
      '(ns lessons.functions)\n\n' +
      '(defn light-cycle [speed]\n  (* speed 3))\n\n(light-cycle 4)',
    '04-promises.hal':
      ';; 04 / PROMISES — time as a value\n;;\n' +
      ';; A promise is a value you do not have yet.\n' +
      ';; Compose them; the system stays alive.\n\n' +
      '(ns lessons.promises\n  (:require [std.lib.promise :as promise]))\n\n' +
      '(promise/then (promise/run discover) render)',
    'scratch.hal':
      ';; scratch — your own corner of the grid\n\n(ns user)\n\n'
  };

  var editor = shell.querySelector('[data-hara-studio="editor"]');
  var editorName = shell.querySelector('[data-hara-studio="editor-name"]');
  var files = shell.querySelectorAll('.hara-studio-file');
  var steps = shell.querySelectorAll('.hara-studio-step');

  function select(name) {
    if (!FILES.hasOwnProperty(name)) return;
    editor.textContent = FILES[name];
    editorName.textContent = name;
    files.forEach(function (f) {
      f.classList.toggle('is-active', f.getAttribute('data-file') === name);
    });
    steps.forEach(function (s) {
      s.classList.toggle('is-active', s.getAttribute('data-file') === name);
    });
  }

  files.forEach(function (f) {
    f.addEventListener('click', function () { select(f.getAttribute('data-file')); });
  });
  steps.forEach(function (s) {
    s.addEventListener('click', function () { select(s.getAttribute('data-file')); });
  });

  /* REPL: echo only until the wasm runtime is linked */
  var log = shell.querySelector('[data-hara-studio="repl-log"]');
  var input = shell.querySelector('[data-hara-studio="repl-input"]');

  input.addEventListener('keydown', function (ev) {
    if (ev.key !== 'Enter' || !input.value.trim()) return;
    var form = document.createElement('div');
    var prompt = document.createElement('span');
    prompt.className = 'hara-tty-p';
    prompt.textContent = 'hara › ';
    form.appendChild(prompt);
    form.appendChild(document.createTextNode(input.value));
    var note = document.createElement('div');
    note.className = 'hara-tty-o';
    note.textContent = ';; queued — wasm runtime not yet linked';
    log.appendChild(form);
    log.appendChild(note);
    log.scrollTop = log.scrollHeight;
    input.value = '';
  });
})();
