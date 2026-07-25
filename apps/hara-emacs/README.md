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
- `C-c C-i`: evaluate the preceding form and insert the result at point
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
cd apps/hara-emacs
make test        # run the ERT suite
make compile     # byte-compile hara-mode.el
make upgrade     # update the package-vc checkout in ~/.emacs.d/elpa
```

Build and install a Hara runtime (defaults to the Truffle jar):

```sh
cd apps/hara-emacs
make bin-build            # compile the Truffle runtime jar
make bin-install          # install Truffle launcher + jar to ~/.local
make bin-build-native     # build GraalVM native image
make bin-install-native   # install native binary as ~/.local/bin/hara-native
make bin-build-rust       # build Rust release binary
make bin-install-rust     # install Rust binary as ~/.local/bin/hara-rust
make bin-clean            # remove installed binaries and jar
```

The Emacs package ships with `apps/hara-emacs/bin/hara`, which `hara-mode`
auto-detects and uses as `hara-command`. It runs the Truffle jar by default;
set `HARA_BACKEND=rust` (or `native`) to switch backends, or customize
`hara-command` directly.

Or directly:

```sh
emacs -Q --batch -L apps/hara-emacs -L apps/hara-emacs/test \
  -l hara-mode-test.el -f ert-run-tests-batch-and-exit
```

## Troubleshooting

### `EVAL_ERROR: No host builtins are registered for namespace: std.foundation`

This almost always means hara-mode is using a stale runtime jar/binary. The
launcher prefers freshly-built artifacts in the repo root, then falls back to
`~/.local`. To fix:

```sh
cd apps/hara-emacs
make bin-build-truffle   # or make bin-build for the default backend
make bin-install-truffle # update the fallback jar in ~/.local
make upgrade             # update the package-vc checkout in ~/.emacs.d/elpa
```

Then restart Emacs (or run `M-x package-vc-upgrade RET hara-mode RET`).

To see exactly which binary/jar the launcher picked, run it with diagnostics:

```sh
HARA_DIAGNOSTICS=1 ~/.emacs.d/elpa/hara-mode/apps/hara-emacs/bin/hara eval '(+ 1 2)'
```

In Emacs, `M-x hara-jack-in` logs the resolved command to the message area.
Check `M-x describe-variable RET hara-command RET` to see what is configured.
