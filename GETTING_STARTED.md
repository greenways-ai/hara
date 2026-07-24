# Getting started

## 1. Install prerequisites

Install JDK 21 and Maven, then verify:

```shell
java -version
mvn -version
```

## 2. Build the Truffle runtime

```shell
mvn -f java/pom.xml -Ptruffle package
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

The REPL supports multiline forms, persistent history, symbol and Java completion, and inline
documentation. See [`docs/user-guide.md`](website/docs/user-guide.md) and [`spec/hara/repl.md`](spec/hara/repl.md).

## 5. Run a file or stdin

```shell
./hara run lib/examples/hello.hal
./hara stdin < lib/examples/hello.hal
```

## 6. Run tests

```shell
mvn -q -f java/pom.xml test
mvn -q -f java/pom.xml -Ptruffle -Dtest=hara.truffle.HaraL0ConformanceTest test
```

For contributor workflows, test slices, native-image builds, and troubleshooting, read the
[developer guide](website/docs/development.md). To build a multi-file project, continue with
[Namespaces and modules](website/docs/namespaces.md).
