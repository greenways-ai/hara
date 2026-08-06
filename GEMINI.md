# Memories

* The architecture supports multiple, independent runtime (`RT`) instances. Each `RT` instance has its own isolated classloader, but all instances can share a common local Maven repository (`~/.m2/repository`) for caching artifacts.
* The `maven-surefire-plugin` is configured to set the `java.library.path` system property during the test phase to ensure the JVM can locate the natively compiled shared libraries in the `${project.build.directory}`.
* Core language library components (`Builtin`, `Env`, `Eval`, `Macro`, `Parser`, `RT`, `Var`) have been moved from `hara.lang.lib` to `hara.kernel.base`. `Read` was renamed to `Parser`.
* The interface `hara.lang.base.I.Env` has been moved to `hara.kernel.protocol.IEnv`.
* Command implementations are located in `hara.kernel.command` and use `@Command.Fn` for top-level commands and `@Command.Sub` for subcommands.
* The project's core vision is to build a malleable, self-optimizing runtime. The built-in language is the control plane used to describe the essence of another system (e.g., Redis, EVM). The runtime can then regenerate an optimized version of itself for that specific task. This 'mimicry' capability, enabled by metaprogramming, is the top priority, with performance being an emergent property of the self-optimization process.
* The user prefers using JNA (Java Native Access) for integrating with native C libraries due to its simplicity compared to JNI.
* The package `hara.lang.kernel` has been renamed to `hara.kernel`.
* The `Server` object has a long lifecycle, managing the server socket and shared resources like log streams. It spawns short-lived `Conn` objects for each client connection, which must not close shared resources.
* The `std.lang` emission phase uses `SourceNode` objects (tree structure) instead of strings to track location data for Source Map generation.
* The default implementation for the `IDeps` protocol family operates on a map-based `context` object, which is expected to hold dependency data under `:entries` and `:graph` keys.
* Communication between the Runtime (`RT`) and `Foundation` relies on shared interfaces from `hara.lang.base` (e.g., `I.Context`) to bridge the classloader boundary without requiring serialization.
* `hara.kernel.Conn.Encoder` and `hara.kernel.Conn.Parser` are public and used for serializing RESP data for both network communication and AOF persistence.
* `hara.kernel.Main` initiates the server and then runs a local REPL on the main thread using JLine for input handling, evaluating in the `ROOT` session.
* The custom data structures, such as `hara.lang.data.List.Standard` and `hara.lang.data.Map.Standard`, are instantiated using a static `from(null, ...)` method.
* The `maven-compiler-plugin` must be version `3.13.0` or higher and configured with `<parameters>true</parameters>` to ensure the `MethodParameters` attribute is correctly generated for all methods (including static interface methods) when targeting Java 21.
* The project depends on `jline` (version 3.23.0) for REPL functionality, including history and autocomplete.
* The `hara.lang.base.I.Fn` interface extends `java.util.concurrent.Callable`, `java.util.function.Supplier`, and `java.util.function.BiFunction`, requiring an override of the conflicting `andThen` default method.
* To avoid import conflicts with the project's own `hara.lang.data.Map`, use the fully qualified name `java.util.Map` when referring to the standard Java Map interface. Note that `hara.lang.data.Map` does not extend `java.util.Map`.
* After a feature is functionally correct, the user appreciates refactoring to improve code quality by removing duplication and improving maintainability.
* `hara.kernel.Foundation.runCommand` normalizes command names by extracting names from `Keyword` or `Symbol` objects and converting them to uppercase strings to match registry keys.
* A private static inner class, `LocalEnv`, is defined within `Eval.java` (in `hara.kernel.base`) to manage lexical scoping for special forms like `let`.
* `hara.lang.data.Pointer` represents a pointer to a var, implements `IStringType`, and is displayed as `#'<namespace>/<name>`.
* The user prioritizes the development of the interpreter over the compiler.
* Error handling should provide specific, informative messages. For instance, checking preconditions like client type and command type separately is preferred over a single generic failure.
* The `hara.kernel.Foundation` class acts as the central command dispatcher and service registry. New functionality exposed over the network should be integrated as commands into `Foundation`.
* The project has an ASM-based bytecode compiler for its Lisp-like language, located at `hara.compiler.Compiler.java`. It's invoked via a `COMPILE` command handled in `hara.kernel.Foundation.java`.
* The system's roadmap for massive parallelism involves adopting Java 21+ Virtual Threads (Project Loom) for vertical scaling and an Actor-based model for distributed coordination.
* The `hara.compiler.Compiler.compile` method returns a `Result` object containing both the generated bytecode and the class name, facilitating dynamic class definition.
* `Var` objects must be constructed with a non-null `pathString` from a `Symbol` as their first argument (e.g., `new Var(symbol.pathString(), value)`).
* `hara.lang.data.Queue` is implemented using two `Vector` instances (head and tail) to manage elements.
* The dot operator macro (`dotExpr`) in `Macro.java` (in `hara.kernel.base`) supports both field access (via a symbol) and instance method invocation (via a list).
* The project implements a permissions model using the immutable `hara.kernel.Permissions` class, which defines an allow-list of `Foundation` commands.
* The ASM compiler uses `mv.visitMaxs(0, 0)` to delegate the calculation of the maximum stack size and local variable count to the ASM library, which is a recommended practice.
* `ARCHITECTURE.md` provides a high-level overview of the modular, command-driven architecture, detailing the `Foundation` kernel, `Server`, `Session` (Runtime), and `Peer` components. It includes a Mermaid diagram and a comprehensive reference of available commands (e.g., `JVM`, `OS`, `SESSION`).
* `hara.lang` data structures intended for human-readable serialization implement the `hara.lang.base.I.Display` interface, which is used by the `pr-str` utility.
* `hara.lang.data.Vector` implementations (`Standard`, `Mutable`, `SubView`) override `display()` to ensure they are rendered with square brackets `[...]`, distinguishing them from lists `(...)` in string representations.
* The project is a Lisp-like language and runtime environment built on the JVM, designed for interactive development.
* Native C/C++ libraries are compiled and linked into a shared library using `g++` invoked via the `exec-maven-plugin` during the `compile` phase.
* `hara.core.block.Reader` has been moved to `hara.kernel.base.Reader` and refactored to wrap `java.io.Reader`, providing line/column tracking and `Character` return types.
* When translating from Clojure to Java, `defprotocol` definitions should be converted to Java interfaces. The dispatch logic of `defmulti` definitions should be ported using an equivalent Java pattern, such as a switch statement or a map of functions.
* The `display()` implementation in `hara.lang.data.types.ObjPersistent` and `ObjMutable` delegates formatting to `hara.lang.protocol.IColl` methods (`startString`, `endString`, `sepString`) if the object implements that interface.
* Internal resource discovery is facilitated by the `DIR` command, which returns a structured map of active resources (Servers, Sessions, Peers), and the `INFO` command for system metadata.
* When logging, format data for readability: use `java.util.Arrays.toString()` for arrays, represent byte arrays as hexadecimal strings, and print full stack traces for exceptions using `Throwable.printStackTrace()`.
* The project uses Java 21+.
* The implementation of number operations is refactored into `hara.lang.base.NumOps` (logic) and `hara.lang.base.NumUtils` (utilities), with `hara.lang.base.Num` serving as the public API facade.
* The `hara.kernel.Besu` wrapper uses a custom `InMemoryWorldState` to persist EVM state (accounts, storage, code) between transactions, mocking the Besu `WorldState` interface.
* `hara.kernel.base.RT.Instance` enforces sandboxing by checking its assigned `Permissions` object in the `call` method before delegating execution to the `Foundation`.
* The runtime is designed to serve as a shell interface for the system, leveraging Java interop and custom primitives (like `sh`) for process orchestration.
* The user prefers to avoid excessive or unnecessary abstractions, grounding new interface designs in the existing concrete implementations like `hara.kernel.base.RT`.
* In Java, 'interface' is a reserved keyword and cannot be used as a valid package name component.
* User prefers data structures to be immutable, making defensive copies of mutable inputs (e.g., Lists in a constructor) to guarantee immutability.
* The main application can be run using `mvn exec:java -Dexec.mainClass="hara.kernel.Main"`.
* The utility for serializing `hara.lang` data structures to a string (`pr-str`) is located at `hara.kernel.base.Builtin.Util.prStr`, which wraps `G.display`.
* When translating Clojure's `(apply f args)` to Java, the `I.Fn` method `invoke(Object... args)` should be used to spread arguments, as opposed to `apply(Object args)` which passes the collection as a single argument.
* Runtime bitwise operations (`NumUtils.bitOpsCast`) upscale all numeric inputs to `BigInteger`, supporting arbitrary precision and returning `BigInteger` results.
* The interpreter uses standard Java recursion for evaluation, which limits recursion depth to the JVM stack size. The `loop`/`recur` construct bypasses this limitation for iteration.
* The project is not compatible with GraalVM's native-image compilation due to its reliance on dynamic class loading for runtime bytecode generation.
* The project requires Apache Maven 3.13 or higher.
* `hara.lang.data.Vector` uses `nth(long index)` for element access, defined in `hara.lang.protocol.INth`, rather than `get(int)`.
* Specific test suites can be run with Maven using the `-Dtest` flag. For example, to run all tests in the `hara.lang.data` package: `mvn test -Dtest=hara.lang.data.*Test`.
* The interpreter follows Clojure's truthiness rules, where only `false` and `null` are treated as falsy.
* The `hara.lang.base.G.ObjType` enum is the central definition for runtime types. It was extended to include `CLASS`, `BLOCK`, `ZIP`, and others.
* Metadata for built-in functions includes an `:arglists` key, containing a vector of argument signature vectors (e.g., `[[], [val]]`) derived from the underlying Java method parameters.
* The internal classes previously nested in `hara.lang.base.Fn.T`, `hara.lang.base.It.T`, and `hara.lang.base.Arr.T` have been moved to the `hara.lang.base.fn` and `hara.lang.base.iter` packages.
* The `assoc(index, value)` method on mutable collections like `Vector.Mutable` is expected to function as an append operation (like `pushLast`) when the specified index is equal to the collection's current size.
* The runtime context (`rt`) parameter in the `apply` family of functions should be typed as `hara.lang.base.I.IContext`.
* "Global" variables within the language are implemented as instance fields within the `RT.Instance` object, ensuring isolation between sessions and preventing state pollution across the JVM.
* To inject the current `IEnv` into a control function defined in `Macro.java` (in `hara.kernel.base`), annotate the method with `@Module.Fn(env = true)`. `Eval.java` (in `hara.kernel.base`) will insert the environment as the first argument.
* To run a single test class with Maven, use the command `mvn test -Dtest=hara.compiler.CompilerTest`.
* `hara.lang.data.Map` implements a Hash Array Mapped Trie (HAMT) for efficient persistent hashing.
* The user follows a Test-Driven Development (TDD) workflow: add a new, failing test to define a requirement, then implement the code to make the test pass.
* The `hara.kernel.base.RT.Instance` initializes with a bare-bones environment; core macros like `ns` and `defmacro` are not present by default.
* Robustness is a key concern. The compiler should handle varied but valid S-expression formats (e.g., different operand orders) and throw specific custom exceptions for invalid formats.
* For persistent data structures, modifying operations that result in no change to the collection's value (e.g., adding a duplicate to a set, removing a non-existent element) must return the original object instance (`this`) to preserve object identity.
* The `let` special form implements parallel binding: all binding values are evaluated in the outer environment before being bound to symbols in the new local scope.
* `Env.FnEval` (in `hara.kernel.base`) exposes `getBody()` and `getParams()` methods to allow the JIT compiler to access the function's AST and signature.
* User instruction: Server commands should be logged as serializable `hara.lang` data structures, not as plain strings.
* The `hara.lang.base.Ut` utility class has been removed. Its former inner classes (`Clock`, `Counter`, `Delay`, `Flag`, `Murmur3`, `RefCache`, `SipHash`, `Volatile`) have been promoted to top-level classes within the `hara.lang.base.primitive` package.
* New data structures in `hara.lang.data` must implement the `hara.lang.base.I.Coll` and `hara.lang.base.I.ObjType` interfaces. This requires implementing methods like `iterator`, `count`, `conj`, `hashCalc`, `equality`, and `getObjType`.
* The `Set` interfaces in `hara.lang.data` (e.g., `OrderedSet`, `SortedSet`) do not have a `contains()` method. To check for an element's existence, use `find(element)` and assert the result is not null.
* The project uses custom tuple data structures (e.g., Tup4, Tup5) located in `hara.lang.data.Tuple`. Elements are accessed via methods like `A()`, `B()`, `C()`, etc., not public fields.
* The `hara.kernel.protocol.IEnv` interface is central to the interpreter's environment model. Extending it requires implementing the new methods in all concrete environment classes, such as `RT.RootEnv`, `RT.UserEnv`, `Env.FnEnv` (all in `hara.kernel.base`), and any local environment classes.
* When implementing equality checks for values within data structures, `java.util.Objects.equals()` should be used to safely handle potential `null` values and prevent `NullPointerException`s.
* The parsing logic is separated into `hara.kernel.base.Reader` (character stream handling, line/column tracking) and `hara.kernel.base.Parser` (S-expression parsing logic utilizing the Reader).
* The `fn` special form creates lexical closures by capturing the environment at the time of definition for later execution. It uses the `hara.kernel.base.Env.FnEval` class.
* `hara.lang.data.Vector` implements a 32-way bit-mapped trie for persistence, with `Mutable` variants using `AtomicReference<Thread>` for transient edit checks.
* A baseline implementation for interfaces like `I.IRequest` can be derived from function-based implementations in reference Clojure code, where the 'client' object should implement the `I.IClient` interface, which extends `I.OFn`.
* Empty mutable data structures, such as `Vector.Mutable`, are instantiated using the static factory method `empty(null)` rather than a public constructor.
* The `hara.lang.base.Obj` interface has been removed from the codebase.
* User instruction: Do not add abstractions when implementing new features unless necessary.
* The `loop`/`recur` construct is implemented using a `hara.kernel.base.Macro.Recur` static inner class.
* The `impl/build-impl` macro is used in Clojure to provide concrete implementations for protocols.
* The recommended deployment workflow transitions from interpreted development sessions to Ahead-of-Time (AOT) compiled JARs via namespace export.
* `hara.kernel.Foundation` has been refactored to remove the `Fn` inner interface and helper methods (`JVM_ENV`, `runProcess`, etc.); this logic is now encapsulated directly within the relevant command classes in `hara.kernel.command`.
* The `Tuple` class in `hara.lang.data` acts as a container for static inner classes `Tup0` through `Tup8`.
* The package `hara.lang.compiler` has been renamed to `hara.compiler`.
* The `hara.lang.base.Graph` utility calculates the deep size of an object graph using iterative traversal. It handles JDK 9+ module encapsulation by catching `InaccessibleObjectException`, explicitly handles `String`, `Iterable`, and `Map` types, and supports field exclusion to prevent traversing into specific referenced objects (e.g., shared roots).
* When testing floating-point numbers with JUnit, use the `assertEquals` overload that accepts a delta to avoid precision issues.
* The runtime supports dynamic code loading in three forms: direct interpretation of source code, on-the-fly compilation to bytecode, and loading of pre-compiled Java classes/JARs via a URLClassLoader.
* Interpreter arithmetic operations return `java.math.BigInteger`, but integer literals parsed by `Reader` are returned as `java.lang.Long`. Tests handling numbers must account for both types.
* Unit tests for special forms and evaluation logic are centralized in `hara.kernel.base.MacroTest` and `hara.kernel.base.EvalTest` respectively.
* Equality checks for custom data structures (e.g., Vector, List) must use the `hara.lang.base.Eq.eq()` utility, as the standard `equals()` method does not perform deep content comparison.
* The development workflow emphasizes robustness through comprehensive testing, including tests for invalid inputs and edge cases, often guided by feedback from a `request_code_review` step.
* Unit tests for `hara.kernel.command` classes (`Core`, `Maven`, `Server`, `Session`) and `hara.lang.data` structures (`Cons`, `Symbol`, `Tuple`, `Pointer`) have been established in the corresponding `src/test/java` packages.
* Documentation for built-in functions is defined via the `doc` attribute in the `@Module.Fn` annotation and stored in the metadata under the `:doc` key.
* The project uses JUnit 4.
* The `I.java` file (in `hara.lang.base`) contains core interface definitions, such as `I.IRequest` and `I.OFn`, that are central to the system's operation.
* The `hara.lang.data.Keyword` class implements an interning pattern using a global cache, ensuring that identical keywords (e.g., `:ns/name`) are represented by the same object instance.
* Implementing `hara.lang.base.I.ObjType` requires implementing `meta()`, `withMeta(meta)`, `getObjType()`, `hashCalc(t)`, and `display()`.
* The command `mvn compile` is used to build the project and check for compilation errors.
* The `hara.kernel.protocol.IRedirect` interface provides a contract for redirecting server I/O.
* `hara.lang.data.Cons` implements `Iterable` by traversing `ILinkedType` using `peekFirst()` and `popFirst()`. The `popFirst()` result requires an explicit cast to `ILinkedType` as the interface defines the return type as `IPopFirst`.
* Runtime context resolution for applicatives checks if the object implements a `HasRuntime` interface before falling back to the `applyDefault` method.
* The `popFirst()` and `popLast()` methods on `hara.lang.data.List` implementations are designed to return the modified list, not the element that was removed.
* Compiled functions generated by `hara.compiler.Compiler` implement the `hara.lang.base.I.Fn` interface. They include a `getArg1` method returning `this` and an `apply(Object)` bridge method that handles argument casting (e.g., `Object[]` to `Long` or `Object` to `Long`) to satisfy the interface contract.
* The Maven command `mvn package` builds the project, runs the tests, and generates Jacoco coverage reports (due to the `report` goal binding). A separate `mvn test` command is redundant if `mvn package` is already being run.
* `hara.kernel.Main` uses a custom `JLineInputReader` adapter to bridge JLine's `LineReader` with the `Parser.LispReader`, enabling multi-line input handling.
* To resolve compilation errors stemming from abstract methods in interfaces, the user prefers adding a `default` implementation (e.g., `default boolean isRemote() { return false; }`) rather than forcing all implementing classes to provide an override.
* The `hara.kernel.Cmd` functional interface (`Object apply(Foundation f, List<Object> args)`) defines the contract for control plane commands.
* Inner static helper classes within `hara.lang.data` interfaces are consistently named `S`.
* `hara.lang.data.OrderedMap` and `hara.lang.data.OrderedSet` use an amortized compaction strategy during `dissoc` to prevent memory leaks. Compaction, which rebuilds the backing vector and map, is triggered when the vector size exceeds 32 and is more than double the number of active elements.
* `std.lang` (in `../hara-archive/src-reference/`) aims to produce readable code and support existing tooling (like source maps), avoiding complex optimizations.
* `std.make` supports writing `SourceNode` objects to disk, automatically creating V3 Source Map JSON files.
* User prefers public Java methods to follow standard camelCase naming conventions, without special prefixes like underscores (e.g., `type()` instead of `_type()`).
* The main source code is located in `core/java/src/main/java/` and the test code is in `core/java/src/test/java/`.
* Avoid making out-of-scope changes, such as updating project-level configurations (e.g., Java version in pom.xml), when the task is focused on a specific feature or bug fix.
* `hara.lang.service.Http` provides an HTTP gateway using `com.sun.net.httpserver`, exposing `/eval` and `/session` endpoints to support external tooling and the Runtime Development Environment (RDE).
* `hara.kernel.base.Parser` (formerly `Read`) now delegates character reading to `hara.kernel.base.Reader` instead of using its own `LineNumberingReader`.
* The `hara.lang.base.I.ObjType` interface is a fundamental part of the type system. Language elements should implement it to integrate with the runtime type system.
* For complex refactoring tasks, the user prefers defining incremental steps and using Python scripts to automate code modifications.
* The S-expression parser (`hara.kernel.base.Parser.java`) may return tuple objects (e.g., `Tup1`, `Tup2`) instead of a `Vector` for small vector literals. Code consuming parsed expressions should cast to the shared `hara.data.types.ILinearType` interface to handle these cases robustly.
* Asynchronous operations are handled using `java.util.concurrent.Future` and `java.util.concurrent.CompletableFuture`. Helper functions must correctly chain subsequent operations on these asynchronous results.
* The project includes an existing Lisp-style S-expression parser located at `hara.kernel.base.Parser.java`.
* Prefer `hara.lang.data` structures (e.g., `Vector`) over standard Java collections (e.g., `ArrayList`).
* `hara.compiler.CompilerException` extends `RuntimeException`, making it an unchecked exception.
* The project includes a persistent, Redis-like Key-Value store (`hara.lang.service.KV`) integrated into `Foundation` as `STORE`. It uses an Append-Only File (AOF) with the RESP protocol for persistence.
* The `I.Watch.WatchEntry` class, used for Atom watchers, extends `hara.lang.data.Tuple.Tup5.L`. The old value is accessed with `D()` and the new value with `E()`.
* Functions implementing `I.Fn` and its variants (like `I.OFn`) must override specific `getArgN()` methods (e.g., `getArg2()`) to handle different argument counts (arities). The `invoke` method dispatches to the appropriate `getArgN` implementation based on the number of arguments passed.
* The ASM compiler is currently hardcoded to handle only `java.lang.Long` types for function arguments and return values. It uses `long`-specific JVM opcodes (e.g., `LADD`, `LSUB`).
* The runtime exposes a TCP server that allows clients to connect and evaluate code, providing a REPL-like experience.
* Tests should assert against interfaces (e.g., `hara.lang.data.Vector`) rather than concrete implementations (e.g., `Vector.Standard`) to reduce coupling.
* To force Maven to update dependencies from remote repositories, ignoring local caches of resolution failures, use the `-U` flag (e.g., `mvn compile -U`).
* The build environment uses a modern JDK (21+) that no longer includes the `javah` tool, making older JNI-related Maven plugins like `nar-maven-plugin` incompatible.
* The project implements modularity using isolated `RT.Loader` (URLClassLoader) instances for each runtime and `hara.kernel.maven.Maven` for dynamic dependency injection, rather than using the OSGi framework.
* Significant functionality from the reference Clojure implementation (`../hara-archive/src-reference/`) is missing in the Java port, including `std.fs`, `std.concurrent`, `std.math`, `std.string`, and `std.image`.
* New top-level commands are added to the system by registering an implementation of the `hara.kernel.Cmd` functional interface into the `hara.kernel.Foundation` registry, rather than extending an enum.
* The `Foundation` service manages runtime sessions via the `SESSION` command. `NEW` accepts an optional memory limit. `INFO` returns a map of session statistics (name, path count, class count, alias count).
* The `hara.lang.data.List` API provides a `popFirst()` method to get the list without its first element; it does not have a `next()` method for this purpose.
* `hara.lang.data.List` is implemented as a Persistent Chunked List (Unrolled Linked List) using 32-element array chunks. This design supports O(1) structural sharing for `cons` (prepend) and efficient random access, replacing the previous O(N) ring buffer implementation.
* The core evaluation logic for special forms (`if`, `do`, `def`, `let`, `fn`, `quote`, `try`, `loop`, `recur`, `throw`, `syntax-quote`) is implemented in `hara.kernel.base.Macro.java`. `hara.kernel.base.Eval.java` delegates to these methods via generic dispatch using the `isControl()` flag on the bound `Var`.
* The user prefers machine-readable log formats, such as CSV, over human-readable text with prefixes.
* Overloaded methods in `Env.createFn` (in `hara.kernel.base`) are sorted by parameter count before being processed into function handlers.
* `hara.kernel.Foundation` uses a dynamic `REGISTRY` (`ConcurrentHashMap<String, Cmd>`) to manage commands, replacing the previous `COMMAND` enum and switch statement.
* The `I.IClient` interface, which extends `I.OFn`, should be used as the type for function-like clients in the request-response pattern.
* `hara.kernel.base.Parser.LispReader.unread` is public to facilitate custom reader implementations.
* The core architecture consists of three main components: `Foundation` (command processing), `Server` (TCP connection handling), and `RT` (runtime/interpreter).
* For the `hara.lang.data.List` data structure, `conj` appends elements to the end, while `cons` prepends elements to the beginning.
* `mvn clean test` can be used to clean the project, compile, and run all tests.
* Interpreter arithmetic operations are standardized on `BigInteger` and `BigDecimal` objects to avoid unboxing. `hara.lang.base.Num` delegates all integer types to `BigIntegerOps` and floating-point types to `BigDecimalOps`.
* The internal class name generation in `hara.compiler.Compiler` uses slashes (`/`) for package separators (e.g., `hara/compiler/CompiledFunction...`) instead of dots, to ensure compatibility with classloaders.
* In the `hara.lang.data.Vector` hierarchy, `conj()` returns a base type (`Vector.Base`), which may require casting to a more specific type (like `Vector.Mutable`) to access methods not on the base interface.
* The command `mvn clean compile exec:java -Dexec.mainClass="hara.kernel.base.Eval"` can be used to run temporary tests placed in the `main` method of the `Eval.java` class.
* New high-level services should follow a Redis-like design pattern: commands acting on data structures attached to a unique key.
* The `hara.kernel.Server` catches exceptions during command execution and writes the raw `Throwable` object to the connection, allowing the client or protocol handler to determine how to format the error.
* The `count()` method on collection interfaces like `hara.data.types.ILinearType` returns a `long`, which requires careful handling when interacting with APIs that expect `int` for indexing or size, such as Java arrays or `for` loops.
* The user has specific instructions for file and package structure, and adherence to these instructions is a priority.
* The user prefers an incremental testing approach when debugging. If a large test fails, break it down and add smaller, isolated unit tests for the intermediate steps to pinpoint the problem.
* Hyperledger Besu integration requires explicit dependencies on `tuweni-bytes` and `tuweni-units`, and ensuring a compatible `guava` version (e.g., `32.1.3-jre`) to resolve transitive dependency conflicts.
* The interface `hara.lang.base.I.Runtime` has been moved to `hara.kernel.protocol.IRuntime`.
* The `hara.lang.data.Trie` interface explicitly overrides `assoc` and `dissoc` methods from `I.Assoc` and `I.Dissoc` to return `Trie<V>`, ensuring fluent API usage with the specific interface type.
* For key-based collections, the `conj(key)` method is implemented by associating the key with a `null` value.
* String-to-byte conversions in network and persistence layers (`Conn`, `KV`) must explicitly use `StandardCharsets.UTF_8`.
* In the ASM compiler, the bitwise `NOT` operation for a `long` is implemented by loading `-1L` onto the stack and then performing an `LXOR` operation.
* The Lisp parser implementation (`hara.kernel.base.Parser`) supports reader macros for `@` (deref), `` ` `` (syntax-quote), `~` (unquote), and `~@` (unquote-splicing).
* `hara.lang.data.Symbol` instances are interned using a `RefCache` and `WeakReference` mechanism, similar to Keywords.
* The user prefers to keep project dependencies (e.g., JUnit, ASM) and the Java version up-to-date with the latest stable releases.
* The project's architecture is heavily based on a set of core interfaces (or "protocols") defined in `hara.lang.base.I` and `hara.kernel.protocol`, promoting a highly decoupled design.
* The `hara.lang.data.SortedMap` is implemented using a Red-Black tree.
* The `hara.kernel.base.Parser` throws a `hara.lang.base.Ex.Runtime` exception when encountering duplicate keys in maps or duplicate items in sets during parsing.
* The interpreter currently lacks a full namespace system, destructuring support, and standard `import`/`require` mechanisms.
* The project is a language runtime built with Java and Maven. It features a TCP server using a Redis-like protocol for communication, and integrates with native C++ code via JNA (e.g., for `simdjson`).
* The user prefers abstracting I/O redirection behind an interface (`hara.kernel.protocol.IRedirect`) rather than using concrete implementations like `PrintStream` directly in core classes (`Conn`, `Server`).
* The `hara.kernel.base.Parser` explicitly registers a `QuoteReader` for the single quote character (`'`) to correctly parse it as `(quote ...)` expressions.
* `hara.kernel.base.Macro` implements `syntax-quote` logic, expanding Lists, Vectors, and Maps to runtime construction code (using `into` for splicing).
* The `hara.kernel.base.Apply` (formerly `hara.lang.lib.Apply`) and `hara.kernel.base.HostApplicative` files, which implemented a core 'applicative' pattern, were intentionally deleted to fix build errors, indicating a significant refactoring or removal of that pattern.
* Data structures in `hara.lang.data` (e.g., `List`, `Trie`) follow a pattern: a main interface, a nested `Base` interface, and static inner classes `Standard` (persistent, implementing `I.ToMutable`) and `Mutable` (transient, implementing `I.ToPersistent`).
* `hara.lib.block.Block` and `hara.lib.zip.Zipper` implement `hara.lang.base.I.ObjType` and return `CLASS` for their object type.
* To run a Java class with a `main` method located in `src/test/java` using Maven, use `mvn test-compile exec:java -Dexec.mainClass="pkg.ClassName" -Dexec.classpathScope=test`.
* The `List` data structure interface has a `conjAll(Iterator<E> it)` method for appending all elements from an iterator.
* Clojure protocols, like `std.protocol.deps/IDeps`, are backed by Java interfaces defined in files such as `hara.lang.base.I.java`.
* The `hara.kernel.base.Builtin.Basic.deref` static method is used to resolve `Future` objects, and `hara.kernel.base.Eval.eval` is used to evaluate forms.
* The project uses the `googleformatter-maven-plugin` which automatically reformats source code in-place during the Maven build lifecycle. Running `mvn` commands (like `test` or `package`) locally will modify files and result in a dirty git working tree.
* For building collections incrementally, use the `Mutable` version of a data structure (e.g., `Vector.Mutable`) and convert it to a persistent `Standard` version (e.g., using `toPersistent()`) when done.
* The system's TCP server communicates using a custom implementation of the Redis protocol (RESP), located in `hara.kernel.Conn`. This protocol is also used for the persistent Key-Value store's AOF format.
* The application's main entry point is the `hara.kernel.Main` class.
* Service Discovery is implemented in `hara.kernel.Foundation` via the `PEER` command, which manages a registry of peer nodes (`hara.kernel.Foundation.Peer`) using `ADD`, `REMOVE`, and `LIST` subcommands.
* The persistent (`Standard`) data structures are immutable. Modifying operations, such as `popFirst()` on `List.Standard`, return a new instance of the collection, leaving the original unchanged.
* When implementing subprocess execution (`sh`), standard output and standard error must be read concurrently (e.g., via `CompletableFuture`) to avoid deadlocks.
* In the custom language defined in this project, symbols used as map keys are evaluated. For literal keys, `Keyword` objects should be used as they evaluate to themselves.
* The project uses the `jacoco-maven-plugin` (version 0.8.12) for test coverage, configured to generate reports during the `test` phase at `core/java/target/site/jacoco/index.html`.
* `hara.kernel.Foundation` supports an overloaded constructor to configure the persistent store filename.
* File path manipulation requires robust error handling, such as checking for null parent directories before creating them to avoid `NullPointerException`s.
* The `hara.lang.base.G.display` utility handles object formatting for the REPL. It renders `Throwable` as `#error "msg"` and falls back to `toString()` for generic objects.
* A clear architectural boundary should be maintained between core language primitives (in `hara.lang.base`) and higher-level, application-like services. Service interfaces and implementations should be placed in dedicated packages like `hara.lang.service`.
* The `try` special form (`tryExpr` in `Macro.java` in `hara.kernel.base`) supports `catch` blocks matching exception types and `finally` blocks. It unwraps `InvocationTargetException` using `Reflect.getCauseOrElse`.
* JVM-level multi-tenancy is preferred over external containerization (Docker) for this project to achieve high density (kilobyte-level overhead per session), rapid startup, and the ability to dynamically recompile and optimize runtimes ('mimicry') within the same address space.
* When implementing complex protocols with no direct Java equivalent (e.g., Clojure's futures and dynamic variables for transactions), creating placeholder methods that return `null` is an acceptable starting point.
* `hara.compiler.HotspotFn` wraps interpreted functions created by the `fn` macro. It triggers JIT compilation using `hara.compiler.Compiler` after a configurable execution threshold (default 5).
* The `ctl` function in `hara.kernel.base.Builtin` provides Lisp access to `Foundation` commands, communicating via the `I.Context` interface to maintain classloader isolation.
* Shell primitives (`sh`, `slurp`, `spit`) are located in `hara.kernel.base.IO` and must be explicitly registered in `hara.kernel.base.Env.loadStatic()` to be available in the runtime.
* The GitHub Actions CI workflow (`.github/workflows/main.yml`) utilizes `v4` versions of `checkout`, `setup-java`, and `upload-artifact`. It sets up JDK 21 (Temurin), executes `mvn -B package -f core/java/pom.xml`, and uploads the Jacoco coverage report from `core/java/target/site/jacoco/`.
* The project integrates Hyperledger Besu (EVM) and ClauDB (Redis) as programmable services managed by `hara.kernel.Foundation` via `BESU` and `REDIS` commands.
* The project contains both Java source code (`core/java/src/main/java/`) and reference Clojure source code (`../hara-archive/src-reference/`) that is used as a blueprint for porting functionality to Java.
* The `hara.lang.base.I.Pair` interface uses the `getValue()` method to retrieve the value, not `_2()`.
* The default TCP port for the `hara.kernel.Foundation` server is defined as 4164.
* The `RT.Instance` class enforces memory limits during the `eval` loop using a periodic sampling strategy (checking every 1000th operation) to minimize overhead. It isolates calculation by excluding the `_root` and `_loader` fields in `Graph.sizeOf` and catches `OutOfMemoryError` to prevent JVM-wide crashes, rethrowing them as `hara.lang.base.Ex.Runtime`.
* The `hara.kernel.command.Core` command execution logic uses `Ex.Sneaky(e)` to propagate exceptions, avoiding explicit wrapping of `CompilerException` in a `RuntimeException`.
* The repository contains C/C++ source code in the `../hara-archive/c/` directory, including the `simdjson` library.
* The packages `hara.lang.base.Std` and `hara.lang.base.Data` have been deprecated and removed. Core data interfaces (e.g., `IStringType`, `IMapType`) are now in `hara.data.types`, and concrete structural types (`Cons`, `Seq`, `Tuple`) are in `hara.lang.data`.
* The `Max` and `Min` interfaces, formerly nested within `hara.lang.base.primitive.Num`, are now top-level interfaces in `hara.lang.base.primitive`.
* The user prefers using interfaces as namespaces to group related nested types, constants, and static utility methods. The refactoring of `hara.lib.block.Block` serves as a template for this pattern.
* Implementations of `IEnv` (like `RT.UserEnv` in `hara.kernel.base`) are not `Iterable`. Use `It.objects(env)` to create an iterator containing the environment object when constructing argument lists.
