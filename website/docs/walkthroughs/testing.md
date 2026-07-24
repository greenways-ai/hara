# Walkthrough: test a namespace

Hara tests are ordinary `.hal` namespaces using `code.test`. Facts execute inside a real Hara
context, so they verify namespace loading, aliases, persistent values, and language semantics.

## Match the test path

The service project declares `"test"` as a test root. Its test file is
`test/services/api_test.hal`, matching `services.api-test`:

```clojure
(ns services.api-test
  (:require [code.test :refer :all]
            [services.api :as api]
            [services.worker :refer [worker-name]]))
```

Application dependencies stay qualified. The single referred value demonstrates that referred Vars
remain connected to their owning namespace.

## Register facts

```clojure
(fact "normalizes routes through a namespace alias"
  (api/normalize-route "  /STATUS  ")
  => "/status")

(fact "dispatches persistent data across namespaces"
  (api/dispatch "/jobs" {:id 42})
  => {:route "/jobs"
      :result {:worker "background"
               :job {:id 42}
               :status :processed}})
```

Facts register when the namespace loads. Keeping expected values explicit makes collection shape,
keywords, and nested values part of the contract.

## Run one namespace

From `lib/examples/services`:

```shell
../../hara --allow-file eval \
  "(require 'services.api-test) (code.test/run {:namespace \"services.api-test\"})"
```

The result contains one entry per fact. Each successful entry has `"PASS"` in its `status` field.
Filtering by namespace avoids accidentally reporting facts registered by unrelated modules in the
same context.

## Use matchers deliberately

Exact values are the simplest expectation. For partial or exceptional results, require the matcher
you need from `code.test`:

```clojure
(ns example.matchers
  (:require [code.test :refer [contains fact throws]]))

(fact "matches part of a response"
  {:status :ready :count 2}
  => (contains {:status :ready}))

(fact "captures a guest error"
  (/ 1 0)
  => (throws))
```

The maintained matcher, fixture, and task examples live under
[`lib/examples/code-test`](https://github.com/hoebat/hara.lang/tree/main/lib/examples/code-test).

## Test from the JVM harness

Repository tests require the `.hal` test namespace in a fresh Truffle context, call
`code.test/run`, and assert that every returned result has `PASS` status. This prevents a test file
from being packaged but never executed and keeps the guest-language facts as the behavioral source
of truth.
