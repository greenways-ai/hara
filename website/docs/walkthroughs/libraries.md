# Walkthrough: use library namespaces

Hara keeps the automatically referred core small. Other operations live in named libraries so code
shows which portable feature or host capability it uses.

## Start with generated aliases

The default aliases are available in every namespace:

```clojure
(ns tutorial.data)

(def encoded (str/encode "Hara"))

{:size (bytes/count encoded)
 :text (str/to-lower (str/decode encoded))}
```

This evaluates to `{:size 4 :text "hara"}`. Strings and bytes are portable values; no ambient JVM
interop is involved.

The other intrinsic aliases are `promise/`, `file/`, `socket/`, `block/`, and `zip/`. The last two
support source-preserving blocks and tree navigation:

```clojure
(block/parse-first "(+ 1 2)")
(zip/zipper [1 [2 3]])
```

## Choose an explicit alias

An explicit alias can make a dependency clearer or replace a default name:

```clojure
(ns tutorial.labels
  (:require [std.lib.string :as text]))

(text/join ", " [(text/to-upper "hara") "L0"])
```

The service walkthrough uses this form so its namespace declaration is a complete dependency
summary. Both explicit aliases and intrinsic aliases point to the same live library Vars.

## Require opt-in providers

Some provider-backed namespaces are intentionally not injected:

```clojure
(ns tutorial.tasks
  (:require [std.lib.task :as task]))
```

The same rule applies to `std.lib.context`, `std.lib.handle`, and `code.test`. Requiring them asks the
runtime's library-provider registry to install their canonical public Vars. Implementation helper
namespaces are not part of the public surface.

## Treat capabilities separately

The `file/` and `socket/` aliases always exist, but availability is not authority. Grant only the
capability the program needs:

```shell
./hara --allow-file run program.hal
./hara --allow-net run client.hal
```

File operations exchange bytes and promises. Socket v1 is callback-based. A denied operation and an
unsupported operation remain distinct failures, allowing the application to respond deliberately.

## Discover a Var

Use metadata rather than guessing arities:

```clojure
[(get (meta #'str/join) :doc)
 (get (meta #'str/join) :arglists)]
```

In the REPL, put the cursor on a symbol and use `Alt-q` or `F1` for the same information. The
[namespace catalog](../reference/namespaces.md) summarizes every shipped family; the
[runtime-library contract](../reference/runtime-libraries.md) is normative.
