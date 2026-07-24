# Hara developer guide

## Repository map

```text
src/main/java/hara/truffle/  current Truffle language and context
src/main/java/hara/kernel/   CLI, REPL, legacy interpreter, host services
src/main/java/hara/lang/     values and protocols
spec/hara/                   normative language and runtime contracts
src/test/java/               JVM and Truffle tests
site/                        static project website
```

## Build and test

```shell
mvn -q -f java/pom.xml test
mvn -q -f java/pom.xml -Ptruffle package
mvn -q -f java/pom.xml -Ptruffle -Dtest=hara.truffle.HaraL0ConformanceTest test
```

When changing a boundary, run the narrow focused suite first and then the full suite. Graal fallback
warnings are expected on ordinary JVMs without JVMCI; they are not, by themselves, test failures.

## Adding a core operation

```text
language contract -> HaraContext / AST -> focused test -> L0 corpus -> docs
```

Keep the core runtime-neutral. Host-dependent behavior belongs behind a capability or provider
interface. Do not add guest-visible JVM interop to make a library convenient.

## Adding a generated library

1. Define the public names and argument/result semantics in `spec/hara/runtime-libraries.md`.
2. Add the implementation to the runtime-generated namespace.
3. Add valid, invalid, and unsupported capability cases.
4. Add a conformance test and update the user guide.

## Adding an extension provider

```text
manifest -> validate -> provider handshake -> Hara namespace -> promise/error boundary
```

Providers must not change Hara call sites. Keep transport-specific details inside the provider and
preserve stable errors for malformed manifests, denied capabilities, timeouts, cancellation, and
crashes. See [the extension contract](reference/extensions-contract.md).

## Java API documentation

Public Java entry points should have Javadoc describing lifecycle, ownership, thread-safety,
capabilities, and failure behavior. Add API documentation in the same change as a public surface.
The [Javadocs guide](javadocs.md) lists the current targets and generation command.

## Pull requests

Describe the runtime layer changed, the compatibility boundary, the focused tests run, and any
unsupported behavior. Keep unrelated generated files and `.orig` artifacts out of commits.
