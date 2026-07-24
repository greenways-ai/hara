# Walkthrough: a namespace project

This walkthrough uses the executable project in `lib/examples/services`. It separates an API boundary
from a worker implementation, loads them by namespace, and returns persistent Hara data.

## 1. Describe the project

`lib/examples/services/project.hal` defines the roots searched by `require`:

```clojure
(defproject services
  {:source-paths ["src"]
   :test-paths ["test"]})
```

Run project commands from this directory so Hara discovers the descriptor by walking upward.

## 2. Define the worker

`src/services/worker.hal` matches the namespace `services.worker`:

```clojure
(ns services.worker)

(def worker-name "background")

(defn process-job [job]
  {:worker worker-name
   :job job
   :status :processed})
```

The function accepts and returns persistent values. Nothing mutable or JVM-specific crosses its
public boundary.

## 3. Require it from the API

`src/services/api.hal` uses one project namespace and one generated library namespace:

```clojure
(ns services.api
  (:require [services.worker :as worker]
            [std.lib.string :as string]))

(defn normalize-route [route]
  (string/to-lower (string/trim route)))

(defn dispatch [route job]
  {:route (normalize-route route)
   :result (worker/process-job job)})
```

The alias is local to `services.api`; requiring this namespace elsewhere does not copy its aliases
into the caller.

## 4. Run the entry point

From the repository root:

```shell
cd lib/examples/services
../../hara --allow-file run run.hal
```

The result is the readable response map:

```clojure
{:route "/status"
 :result {:worker "background"
          :job {:id 42 :action :health-check}
          :status :processed}}
```

You can also load the namespace and call one Var directly:

```shell
../../hara --allow-file eval \
  "(require 'services.api) (services.api/normalize-route \"  /STATUS  \")"
```

## 5. Grow the project

For each new namespace:

1. Choose a stable qualified name under the project owner.
2. Create its convention-mapped `.hal` file under a declared source root.
3. Declare the same name with `ns`.
4. Require dependencies explicitly and keep aliases local.
5. Add a matching `-test` namespace under a test root.

The complete example is maintained as executable source in
[`lib/examples/services`](https://github.com/hoebat/hara.lang/tree/main/lib/examples/services).
