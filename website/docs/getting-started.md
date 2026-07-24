# Getting started

## 1. Install prerequisites

Install JDK 21 and Maven, then verify:

```shell
java -version
mvn -version
```

## 2. Build the Truffle runtime

```shell
mvn -Ptruffle package
```

This produces `target/hara-truffle.jar`.

## 3. Evaluate a form

```shell
./hara eval '(let [x 19] (+ x 23))'
```

Expected result:

```text
42
```

## 4. Start the REPL

```shell
./hara

# ROOT REPL without a RESP listener
./hara --offline
```

The REPL opens with large alien-abduction Hara art, a clear-to-blue-to-black gradient, the
`Journey Within` tagline, and a spaced command menu.
It supports multiline forms, persistent history, cursor-level slash and symbol completion, inline documentation,
and RESP listener control through `/resp`. The left prompt shows only the current namespace; the
header identifies session `ROOT`, and the right prompt shows live listener status. See [`User guide`](user-guide.md) and [`REPL specification`](reference/repl.md).

## 5. Run a file or stdin

```shell
./hara run lib/examples/hello.hal
cat lib/examples/hello.hal | ./hara stdin
```

## 6. Run tests

```shell
mvn -q test
mvn -q -Ptruffle -Dtest=hara.truffle.HaraL0ConformanceTest test
```

For contributor workflows, test slices, native-image builds, and troubleshooting, read the
[developer guide](development.md). To build a multi-file project, continue with
[Namespaces and modules](namespaces.md).
