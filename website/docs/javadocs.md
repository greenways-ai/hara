# Java API and Javadocs

The Java API is an embedding and tooling surface, not the definition of Hara semantics. The
Truffle language and the specifications are authoritative.

## Public entry points

| API | Purpose |
|---|---|
| `hara.truffle.HaraLanguage` | Truffle language registration and context lifecycle |
| `hara.truffle.HaraContext` | Context-local namespaces, evaluation, capabilities, and providers |
| `hara.truffle.Main` | Truffle CLI (`eval`, `run`, `stdin`, `repl`, `help`) |
| `hara.kernel.Repl` | JVM interactive REPL and JLine integration |
| `hara.kernel.flavor.NativeFlavorProvider` | Runtime-neutral host capability provider contract |
| `hara.kernel.flavor.NativeFlavorAccess` | Capability-gated host access contract |

## Generate API docs

After the public classes have complete lifecycle and capability Javadocs, generate the site with:

```shell
mvn -f java/pom.xml -Pjavadoc javadoc:javadoc
```

The generated output is written under `target/site/apidocs/`. In the published documentation site, the generated API is available at [`/api/java/`](https://hoebat.github.io/hara.lang/api/java/index.html) alongside the Material pages.

## Required Javadoc topics

Every public runtime or provider type should document:

```text
ownership       who creates and closes it
thread-safety   whether calls may cross guest threads
capabilities    which grants are consulted
values          Hara values, opaque handles, and promise behavior
failures        unsupported, denied, timeout, cancellation, and crash errors
```

Do not document the old Foundation/TCP implementation as the current language API. Historical
classes may be marked deprecated or linked to `README.legacy.md` while migration continues.
