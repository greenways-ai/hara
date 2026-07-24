# Hara RESP Protocol

The Truffle runtime listens on `127.0.0.1:1311` by default. RESP is the external control-plane
protocol; HTA is the internal Truffle/WASM ABI.

## Sessions

The Hara process owns sessions. Each session contains one isolated Hara kernel, while the RESP
listener is only one way to reach them. The local JLine REPL and RESP clients share `ROOT`. Clients attach to a
session independently, and multiple clients may attach to the same session. Requests within one
session are serialized.

Protocol 4 is negotiated with `["HELLO", "4", "CLIENT", CLIENT-NAME]`. Every subsequent request
uses `[OPERATION, REQUEST-ID, ...ARGUMENTS]`, including session operations:

```text
["SESSION", "REQ-1", "NEW", "APP"]
["SESSION", "REQ-2", "LIST"]
["SESSION", "REQ-3", "ATTACH", "APP"]
["SESSION", "REQ-4", "DETACH"]
["SESSION", "REQ-5", "INFO"]
["SESSION", "REQ-6", "CLOSE", "APP"]
```

`ROOT` is created automatically and remains alive when the listener is stopped or restarted. Other
sessions remain alive until explicitly closed or the Hara process exits.

## Evaluation

After `SESSION ATTACH`, evaluation requests use the attached session:

```text
["EVAL", "REQ-7", "(+ 1 2)"]
["DOC", "REQ-8", "str/join"]
["COMPLETE", "REQ-9", "ma"]
```

Editors may append source context to `EVAL` or `LOAD`. This preserves definition and error
locations when evaluating a form or region instead of a complete file:

```text
["EVAL", "REQ-10", "(defn add [x y] (+ x y))",
 "FILE", "/project/src/sample.hal", "LINE", "12", "COLUMN", "3"]
```

`DOC` returns a flat, keyed array so RESP clients do not need to parse printed Hara values:

```text
["SYMBOL", "sample/add",
 "DOC", "Adds two values.",
 "ARGLISTS", [["x", "y"]],
 "FILE", "/project/src/sample.hal",
 "LINE", 12,
 "COLUMN", 3]
```

Documentation and arglists may be null. Runtime and Java-backed symbols may also omit source
location fields.

Responses are streamed as RESP arrays:

```text
["RESULT", "REQ-7", "3"]
["DONE", "REQ-7", "OK"]
```

Failures emit `["ERROR", ID, CODE, MESSAGE]` followed by `["DONE", ID, "ERROR"]`. Stable codes
include `BAD_REQUEST`, `UNKNOWN_OP`, `NO_SESSION`, `EVAL_ERROR`, `UNSUPPORTED`, and
`INTERNAL_ERROR`. `HELLO` and `INFO` include an instance ID and canonical project root for endpoint
validation. `COMMANDS` is derived from the installed operation-handler registry.

Protocol 3 and clients that omit `HELLO` remain supported. Protocol-3 clients retain the previous
mixed request shapes; legacy clients may use `EVAL SESSION SOURCE`.

## Client libraries

The JVM RESP2 transport is public under `std.lib.resp`. Hara programs with network capability can
use `std.resp.client/connect`, `call`, `write`, `read`, `pipeline`, `open?`, and `close`. The client
is blocking; `connect` accepts `:connect-timeout-ms`, `:read-timeout-ms`, and
`:decode-bulk :string|:bytes` options.

## Runtime modes

```text
hara                         # ROOT JLine REPL + RESP on 127.0.0.1:1311
hara --offline               # ROOT JLine REPL, listener initially disabled
hara headless                # ROOT RESP listener without terminal UI
hara server                  # compatibility alias for headless
hara standalone              # compatibility alias for --offline
hara remote HOST:PORT        # remote client mode
```
