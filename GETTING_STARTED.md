# Getting started

## Install the hara CLI

Linux (x86_64) and macOS (arm64, x86_64):

```shell
brew install hara-lang/tap/hara
# or: brew install hara-lang/tap/hara-truffle
```

The Rust CLI and Truffle native image are separate Homebrew formulas. Both
provide a native executable and neither needs a JVM at runtime.

The verified release installer remains available without Homebrew:

```shell
curl -fsSL https://www.hara-lang.org/install.sh | sh -- --rust --truffle
```

This installs `hara` and the native-image `hara-truffle` to `~/.local/bin`; neither needs a
JVM at runtime. GitHub Releases is the publishing authority for the downloaded packages and
checksums. Install only one runtime with `--rust` or `--truffle`.
Override the location with `HARA_INSTALL_DIR`, or pin a release with `HARA_VERSION=v0.1.1`.

The sections below build the Java/Truffle runtime from source instead.

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

This produces `java/target/hara-truffle.jar`.

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
documentation. See [`docs/user-guide.md`](website/docs/user-guide.md) and the
[archived REPL planning document](specs/99-archive/planning/tooling/repl.md).

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
