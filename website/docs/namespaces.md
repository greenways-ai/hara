# Namespaces and modules

A Hara namespace gives Vars a stable name and gives source files a predictable place in a
project. The syntax is Clojure-inspired, but the loader and available libraries are Hara's own.

## Declare a namespace

Start a portable source file with `ns`:

```clojure
(ns services.worker)

(def worker-name "background")
```

Definitions are Vars. Their fully qualified names are `services.worker/worker-name` and, for a
function named `process-job`, `services.worker/process-job`.

## Map names to files

The nearest `project.hal` declares source and test roots:

```clojure
(defproject services
  {:source-paths ["src"]
   :test-paths ["test"]})
```

Within those roots, dots become directories, hyphens become underscores, and the module extension
is `.hal`:

| Namespace | Source path |
| --- | --- |
| `services.worker` | `src/services/worker.hal` |
| `services.api-test` | `test/services/api_test.hal` |

Project loading uses file access, so the CLI needs `--allow-file` when a form requires project
source.

## Require and qualify

Use `:as` when the dependency has several public Vars:

```clojure
(ns services.api
  (:require [services.worker :as worker]
            [std.lib.string :as string]))

(worker/process-job {:id 42})
(string/trim "  /status  ")
```

Use `:refer` for a small set of names that reads naturally without a qualifier:

```clojure
(ns services.api-test
  (:require [services.worker :refer [worker-name]]))
```

Referred names retain the identity of the source Var; Hara does not copy their current values.
Prefer explicit lists over `:refer :all` in application code. Test namespaces commonly use
`[code.test :refer :all]` because its facts and matchers form a small testing language.

At runtime, require a project, packaged, provider-backed, or extension namespace with a quoted
symbol:

```clojure
(require 'services.api)
(services.api/normalize-route "  /STATUS  ")
```

Use `{:reload true}` only during development when the source must be evaluated again:

```clojure
(require 'services.api {:reload true})
```

Failed loads roll back their namespace changes. Already compiled call targets remain immutable;
newly compiled source observes the reloaded definitions.

## Built-in library aliases

Every namespace receives aliases for the generated string, bytes, promise, file, socket, block,
and zip libraries. For example, `str/trim`, `bytes/count`, and `zip/zipper` work without an explicit
`:require`. You can still require these namespaces under descriptive aliases, as the service
example does.

Use `:intrinsics` to remove or rename generated aliases:

```clojure
(ns compact.app
  (:intrinsics
    {:exclude [socket]
     :aliases {string text
               promise async}}))
```

Excluding an alias does not remove core constructors such as `bytes`, and it does not revoke a host
capability. See the [namespace catalog](reference/namespaces.md) for load modes and capabilities.

## Inspect what loaded

Vars retain documentation and argument metadata:

```clojure
(get (meta #'services.api/dispatch) :doc)
(get (meta #'services.api/dispatch) :arglists)
```

The REPL's symbol completion and inline documentation use the same metadata. `current-symbols`
lists the symbols visible from the active namespace, including aliases and referred Vars.

## Common loading errors

- **Cannot require missing namespace**: check `project.hal`, the source root, and the dot/hyphen path mapping.
- **Namespace source did not declare requested namespace**: make the file's `ns` name match the required symbol.
- **Namespace alias already refers to ...**: choose a distinct alias; aliases cannot silently replace one another.
- **Cannot refer missing var**: use a public Var that actually exists in the required namespace.
- **Capability denied or unsupported**: loading `std.lib.file` or `std.lib.socket` does not grant file or network authority.

Continue with the [service project walkthrough](walkthroughs/service-project.md).
