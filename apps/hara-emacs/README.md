# Hara for Emacs

`hara-mode.el` provides Hara editing, protocol-4 evaluation, inline results, ElDoc, Company/CAPF
completion, Xref navigation, Imenu, sessions, project-aware server startup, and a REPL. Its core
uses built-in Emacs APIs; the optional documentation popup uses `eldoc-box`.

Install from GitHub with `package-vc` (Emacs 29.1+):

```elisp
(package-vc-install
 '(hara-mode :url "https://github.com/hoebat/hara.lang"
             :lisp-dir "apps/hara-emacs"))
(require 'hara-mode)
```

Or use a plain checkout:

```elisp
(add-to-list 'load-path "/path/to/hara.lang/apps/hara-emacs")
(require 'hara-mode)
```

Open a `.hal` file and run `M-x hara-jack-in` or press `C-c C-j`. The client first reuses a
validated project endpoint, then checks `hara-host`/`hara-port`, and finally starts
`hara --port 0 headless`. Emacs-owned servers stop on `M-x hara-disconnect`.

By default, opening a local `.hal` file beneath a directory containing `project.hal` schedules
`hara-jack-in` automatically. Standalone and remote files remain disconnected. Customize
`hara-auto-jack-in-projects` to disable this behavior.

The `hara` launcher executes the prebuilt `target/hara-truffle.jar`; it never invokes Maven during
jack-in. Build or refresh that executable fat JAR explicitly with
`mvn -Ptruffle -DskipTests package`. Override its location with `HARA_RUNTIME_JAR` when using an
installed artifact. New-server endpoint publication may wait up to `hara-server-start-timeout`
(15 seconds by default), while normal endpoint negotiation retains the shorter
`hara-connect-timeout`.

Common commands:

- `C-c C-e`: evaluate the preceding form
- `C-c C-c`: evaluate the top-level form
- `C-c C-r`: evaluate the region
- `C-c C-k`: evaluate the buffer
- `C-c C-z`: open the Hara REPL
- `C-c C-d`: show documentation
- `C-c C-p`: show documentation near point with `eldoc-box`
- `M-.`: jump to a source-backed definition with Xref
- `M-,`: return through Xref history

Evaluation results use CIDER-style boxed, syntax-highlighted overlays at the end of the line, with
fringe feedback and adaptive wrapping for long values. They clear after the next command. The
timeout configured by `hara-inline-result-duration` remains a fallback; customize
`hara-inline-result-max-length` to control truncation.
ElDoc stays silent until the current buffer has explicitly connected to Hara.

Run tests with:

```sh
emacs -Q --batch -L apps/hara-emacs -L apps/hara-emacs/test \
  -l hara-mode-test.el -f ert-run-tests-batch-and-exit
```
