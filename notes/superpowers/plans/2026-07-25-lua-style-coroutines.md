# Lua-Style Coroutines Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add full Lua-style coroutines to hara as a `std.lib.coroutine` library on the Truffle runtime.

**Architecture:** Each coroutine body runs on a parked Java 21 virtual thread (platform-thread fallback under native image). `resume`/`yield` exchange values through two `SynchronousQueue`s; status is a keyword field (`:suspended`/`:running`/`:dead`). Registered as a lazy library provider exactly like `std.lib.task`. No analyzer, AST, or L0 changes.

**Tech Stack:** Java 21, Truffle/GraalVM, Maven (`mvn -q -Ptruffle ...`), JUnit 4, hara guest code in eval strings.

**Spec:** `docs/superpowers/specs/2026-07-25-lua-style-coroutines-design.md`

## Global Constraints

- JDK 21; virtual threads MUST be guarded by `org.graalvm.nativeimage.ImageInfo.inImageRuntimeCode()` with a platform-thread fallback (pattern: `HaraWasmExtension.java:115-123`).
- Off-thread guest code MUST run inside `context.invokeInContext(() -> context.invokeCallable(fn, args))` (pattern: `HaraContext.promiseRun`, `HaraContext.java:2112-2124`).
- Library exports are `public static Object name(HaraContext context, Object[] values)` annotated `@HaraExport(name=..., doc=..., arglists=...)` (pattern: `StdLibTask.java:19-34`).
- The git working tree contains UNRELATED uncommitted work. Every commit step lists exact files with `git add <files>` — NEVER `git add -A` or `git add .`.
- Tests: JUnit 4 (`org.junit.Test`), polyglot `Context.newBuilder(HaraLanguage.ID).build()`, `context.eval(HaraLanguage.ID, "...")`, `assertThrows(PolyglotException.class, () -> ...)` (pattern: `StdLibraryProviderTest.java:30-41`).
- No changes to `HaraAnalyzer.java`, `HaraNodes.java`, `spec/hara/l0-language.md`, or `spec/hara/l0-conformance.edn`.
- `spec/hara/hara-core-symbols.json` and `spec/hara/wasm-truffle-parity.edn` are NOT modified (see Task 6 — this corrects the design spec, which listed them; `std.lib.task` precedent shows lazy providers are not in the core-symbols inventory, and the parity `.edn` is an executable corpus that must not gain uncovered cases).

---

### Task 1: Library scaffold — create, coroutine?, status

**Files:**
- Create: `src/main/java/hara/truffle/StdLibCoroutine.java`
- Create: `src/main/java/hara/truffle/StdLibCoroutineLibraryProvider.java`
- Modify: `src/main/resources/META-INF/services/hara.truffle.HaraLibraryProvider` (append one line)
- Test: `src/test/java/hara/truffle/StdLibCoroutineTest.java`

**Interfaces:**
- Consumes: `@HaraExport` convention from `StdLibTask.java`; `HaraStaticLibrary.install`; `Keyword.create(String)`.
- Produces: `StdLibCoroutine.HaraCoroutine` (package-private static nested class) with `Keyword status()`; exports `std.lib.coroutine/create`, `std.lib.coroutine/coroutine?`, `std.lib.coroutine/status`. Later tasks add `resume`, `yield`, `close`, `await`.

- [ ] **Step 1: Write the failing test**

Create `src/test/java/hara/truffle/StdLibCoroutineTest.java`:

```java
package hara.truffle;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdLibCoroutineTest {
  @Test
  public void createMakesSuspendedCoroutine() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(std.lib.coroutine/coroutine? (std.lib.coroutine/create (fn [x] x)))")
              .asBoolean());
      assertFalse(
          context.eval(HaraLanguage.ID, "(std.lib.coroutine/coroutine? 42)").asBoolean());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :suspended (std.lib.coroutine/status"
                      + " (std.lib.coroutine/create (fn [x] x))))")
              .asBoolean());
    }
  }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: FAIL — eval of `(require 'std.lib.coroutine)` throws (unknown namespace).

- [ ] **Step 3: Write the scaffold implementation**

Create `src/main/java/hara/truffle/StdLibCoroutine.java`:

```java
package hara.truffle;

import hara.lang.data.Keyword;

/** Static Java implementation exported exclusively as std.lib.coroutine/*. */
public final class StdLibCoroutine {
  private StdLibCoroutine() {}

  static final Keyword STATUS_SUSPENDED = Keyword.create("suspended");
  static final Keyword STATUS_RUNNING = Keyword.create("running");
  static final Keyword STATUS_DEAD = Keyword.create("dead");

  /** A coroutine backed by a parked (virtual) thread. Thread machinery arrives with resume. */
  static final class HaraCoroutine {
    private final HaraContext context;
    private final Object function;
    private volatile Keyword status = STATUS_SUSPENDED;

    HaraCoroutine(HaraContext context, Object function) {
      this.context = context;
      this.function = function;
    }

    Keyword status() {
      return status;
    }

    @Override
    public String toString() {
      return "#<coroutine " + status + ">";
    }
  }

  private static void requireArity(String name, Object[] values, int expected) {
    if (values.length != expected) {
      throw new HaraException(
          name + " expects " + expected + " argument(s), got " + values.length);
    }
  }

  private static HaraCoroutine requireCoroutine(String name, Object value) {
    Object unwrapped = HaraBox.unwrap(value);
    if (!(unwrapped instanceof HaraCoroutine)) {
      throw new HaraException(name + " expects a coroutine");
    }
    return (HaraCoroutine) unwrapped;
  }

  @HaraExport(
      name = "create",
      doc = "Creates a coroutine wrapping f. The body does not start until the first resume.",
      arglists = {"[f]"})
  public static Object create(HaraContext context, Object[] values) {
    requireArity("coroutine/create", values, 1);
    Object f = HaraBox.unwrap(values[0]);
    if (!(f instanceof HaraFunction)
        && !(f instanceof HaraMultiFunction)
        && !(f instanceof hara.lang.protocol.IFn)) {
      throw new HaraException("coroutine/create expects a function");
    }
    return new HaraCoroutine(context, f);
  }

  @HaraExport(
      name = "coroutine?",
      doc = "Returns true when value is a coroutine.",
      arglists = {"[value]"})
  public static Object coroutinePredicate(HaraContext context, Object[] values) {
    requireArity("coroutine/coroutine?", values, 1);
    return HaraBox.unwrap(values[0]) instanceof HaraCoroutine;
  }

  @HaraExport(
      name = "status",
      doc = "Returns the coroutine status: :suspended, :running, or :dead.",
      arglists = {"[co]"})
  public static Object status(HaraContext context, Object[] values) {
    requireArity("coroutine/status", values, 1);
    return requireCoroutine("coroutine/status", values[0]).status();
  }
}
```

Create `src/main/java/hara/truffle/StdLibCoroutineLibraryProvider.java`:

```java
package hara.truffle;

/** Optional Java implementation of std.lib.coroutine. */
public final class StdLibCoroutineLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.coroutine";
  }

  @Override
  public int order() {
    return 30;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdLibCoroutine.class);
  }
}
```

Append this line to `src/main/resources/META-INF/services/hara.truffle.HaraLibraryProvider`:

```
hara.truffle.StdLibCoroutineLibraryProvider
```

- [ ] **Step 4: Run test to verify it passes**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: PASS (1 test).

- [ ] **Step 5: Commit**

```bash
git add src/main/java/hara/truffle/StdLibCoroutine.java \
        src/main/java/hara/truffle/StdLibCoroutineLibraryProvider.java \
        src/main/resources/META-INF/services/hara.truffle.HaraLibraryProvider \
        src/test/java/hara/truffle/StdLibCoroutineTest.java
git commit -m "Add std.lib.coroutine scaffold with create/coroutine?/status"
```

---

### Task 2: First resume — thread start, completion, error propagation

**Files:**
- Modify: `src/main/java/hara/truffle/StdLibCoroutine.java`
- Modify: `src/main/java/hara/truffle/HaraContext.java:2254` (visibility of `invokeInContext`)
- Test: `src/test/java/hara/truffle/StdLibCoroutineTest.java`

**Interfaces:**
- Consumes: `HaraCoroutine` from Task 1; `HaraContext.invokeCallable(Object, Object[])` (package-private, `HaraContext.java:3688`).
- Produces: `Object HaraCoroutine.resume(Object[] args)`; package-private `<T> T HaraContext.invokeInContext(Supplier<T>)`; export `std.lib.coroutine/resume`. A resume that runs the body to completion returns the final value and leaves the coroutine `:dead`; a body exception rethrows at the resume site.

- [ ] **Step 1: Write the failing tests**

Append to `StdLibCoroutineTest.java` (add imports `static org.junit.Assert.assertEquals`, `static org.junit.Assert.assertThrows`, `org.graalvm.polyglot.PolyglotException`):

```java
  @Test
  public void resumeRunsBodyToCompletion() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID, "(def c-resume (std.lib.coroutine/create (fn [x] (* x 2))))");
      assertEquals(
          42, context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-resume 21)").asLong());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-resume))")
              .asBoolean());
    }
  }

  @Test
  public void resumeOnDeadThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(HaraLanguage.ID, "(def c-dead (std.lib.coroutine/create (fn [] 1)))");
      context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-dead)");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-dead)"));
      assertTrue(error.getMessage().contains("dead"));
    }
  }

  @Test
  public void bodyErrorRethrowsAtResumeAndKillsCoroutine() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(HaraLanguage.ID, "(def c-err (std.lib.coroutine/create (fn [] (/ 1 0))))");
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-err)"));
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-err))")
              .asBoolean());
    }
  }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: FAIL — `std.lib.coroutine/resume` is not defined (and earlier scaffold test still passes).

- [ ] **Step 3: Make `invokeInContext` package-private**

In `src/main/java/hara/truffle/HaraContext.java:2254`, change:

```java
  private <T> T invokeInContext(Supplier<T> operation) {
```

to:

```java
  <T> T invokeInContext(Supplier<T> operation) {
```

(Drop the `private` modifier only. This is the same wrapper `promiseRun` uses for off-thread guest calls.)

- [ ] **Step 4: Implement resume machinery**

In `StdLibCoroutine.java`, add imports:

```java
import java.util.concurrent.SynchronousQueue;
import org.graalvm.nativeimage.ImageInfo;
```

Add these nested types inside `StdLibCoroutine` (before `HaraCoroutine`):

```java
  /** Thrown on the coroutine thread when a parked coroutine is closed. Unwinds the body. */
  static final class CoroutineClosed extends RuntimeException {
    CoroutineClosed() {
      super(null, null, false, false);
    }
  }

  /** One handoff across the resume/yield boundary. */
  static final class Transfer {
    final Object value;
    final Throwable error;

    Transfer(Object value, Throwable error) {
      this.value = value;
      this.error = error;
    }
  }
```

Replace the whole `HaraCoroutine` class body from Task 1 with:

```java
  /** A coroutine backed by a parked (virtual) thread. */
  static final class HaraCoroutine {
    private final HaraContext context;
    private final Object function;
    private final SynchronousQueue<Object> input = new SynchronousQueue<>();
    private final SynchronousQueue<Transfer> output = new SynchronousQueue<>();
    private volatile Keyword status = STATUS_SUSPENDED;
    private volatile Thread thread;

    HaraCoroutine(HaraContext context, Object function) {
      this.context = context;
      this.function = function;
    }

    Keyword status() {
      return status;
    }

    Object resume(Object[] args) {
      synchronized (this) {
        if (status == STATUS_DEAD) {
          throw new HaraException("coroutine/resume: cannot resume a dead coroutine");
        }
        if (status == STATUS_RUNNING) {
          throw new HaraException("coroutine/resume: cannot resume a running coroutine");
        }
        status = STATUS_RUNNING;
      }
      if (thread == null) {
        start(args);
      } else {
        putInput(pack(args));
      }
      Transfer transfer = takeOutput();
      if (transfer.error != null) {
        if (transfer.error instanceof RuntimeException) throw (RuntimeException) transfer.error;
        throw new HaraException("coroutine failed: " + transfer.error);
      }
      return transfer.value;
    }

    private void start(Object[] args) {
      Runnable body = () -> runBody(args);
      Thread started =
          ImageInfo.inImageRuntimeCode()
              ? new Thread(body, "hara-coroutine")
              : Thread.ofVirtual().name("hara-coroutine").unstarted(body);
      thread = started;
      started.start();
    }

    private void runBody(Object[] args) {
      try {
        Object result = context.invokeInContext(() -> context.invokeCallable(function, args));
        status = STATUS_DEAD;
        putOutput(new Transfer(result, null));
      } catch (CoroutineClosed closed) {
        // Closed while parked: the unwinding already ran the body's finally clauses.
        Thread.currentThread().interrupt();
      } catch (Throwable error) {
        status = STATUS_DEAD;
        putOutput(new Transfer(null, error));
      }
    }

    private void putInput(Object value) {
      try {
        input.put(value);
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        throw new HaraException("coroutine/resume: interrupted while resuming");
      }
    }

    private void putOutput(Transfer transfer) {
      try {
        output.put(transfer);
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        throw new CoroutineClosed();
      }
    }

    private Transfer takeOutput() {
      try {
        return output.take();
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        throw new HaraException("coroutine/resume: interrupted while waiting for the coroutine");
      }
    }

    @Override
    public String toString() {
      return "#<coroutine " + status + ">";
    }
  }
```

Add the `pack` helper and `takeInput` stub used by resume (takeInput is fully wired to yield in Task 3; resume needs `pack` for post-first-resume args):

```java
  private static Object pack(Object[] values) {
    if (values.length == 0) return null;
    if (values.length == 1) return values[0];
    return hara.lang.data.Vector.Standard.from(null, values.clone());
  }
```

Add to `HaraCoroutine` (referenced by `resume` for subsequent resumes; yield consumes it in Task 3):

```java
    Object takeInput() {
      try {
        return input.take();
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        throw new CoroutineClosed();
      }
    }
```

Add the export to `StdLibCoroutine`:

```java
  @HaraExport(
      name = "resume",
      doc =
          "Starts or continues a coroutine. First resume passes args to the body; later resumes"
              + " deliver args as the yield return. Returns the yielded or final value.",
      arglists = {"[co & args]"})
  public static Object resume(HaraContext context, Object[] values) {
    if (values.length == 0) {
      throw new HaraException("coroutine/resume expects a coroutine");
    }
    HaraCoroutine coroutine = requireCoroutine("coroutine/resume", values[0]);
    Object[] args = new Object[values.length - 1];
    System.arraycopy(values, 1, args, 0, args.length);
    return coroutine.resume(args);
  }
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add src/main/java/hara/truffle/StdLibCoroutine.java \
        src/main/java/hara/truffle/HaraContext.java \
        src/test/java/hara/truffle/StdLibCoroutineTest.java
git commit -m "Add coroutine resume with virtual-thread rendezvous"
```

---

### Task 3: yield — bidirectional values, packing, error cases

**Files:**
- Modify: `src/main/java/hara/truffle/StdLibCoroutine.java`
- Test: `src/test/java/hara/truffle/StdLibCoroutineTest.java`

**Interfaces:**
- Consumes: `HaraCoroutine.resume`, `takeInput`, `putOutput`, `pack` from Task 2.
- Produces: `Object HaraCoroutine.doYield(Object value)`; export `std.lib.coroutine/yield`; `ThreadLocal<HaraCoroutine> CURRENT`. Yield with 0 args yields `nil`, 1 arg yields it as-is, N args yields a vector; resume args pack the same way for yield's return value (already implemented via `pack` in `resume`).

- [ ] **Step 1: Write the failing tests**

Append to `StdLibCoroutineTest.java`:

```java
  @Test
  public void yieldExchangesValuesBothWays() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-y (std.lib.coroutine/create"
              + " (fn [start]"
              + "   (let [a (std.lib.coroutine/yield (* start start))]"
              + "     (let [b (std.lib.coroutine/yield :second)]"
              + "       [a b])))))");
      assertEquals(
          100, context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-y 10)").asLong());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :second (std.lib.coroutine/resume c-y :got-a))")
              .asBoolean());
      assertEquals(
          "[:got-a :got-b]",
          context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-y :got-b)").toString());
      assertTrue(
          context.eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-y))").asBoolean());
    }
  }

  @Test
  public void multiYieldPacksVectorAndZeroYieldsNil() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-m (std.lib.coroutine/create"
              + " (fn [] (std.lib.coroutine/yield 1 2 3) (std.lib.coroutine/yield))))");
      assertEquals(
          "[1 2 3]",
          context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-m)").toString());
      assertTrue(context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-m 9 8)").isNull());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :suspended (std.lib.coroutine/status c-m))")
              .asBoolean());
      context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-m)");
      assertTrue(
          context.eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-m))").asBoolean());
    }
  }

  @Test
  public void yieldWorksFromNestedHelper() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID, "(defn helper-n [x] (std.lib.coroutine/yield (* x 10)))");
      context.eval(
          HaraLanguage.ID,
          "(def c-n (std.lib.coroutine/create (fn [] (helper-n 3) :end)))");
      assertEquals(30, context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-n)").asLong());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :end (std.lib.coroutine/resume c-n))")
              .asBoolean());
    }
  }

  @Test
  public void yieldOutsideCoroutineThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/yield 1)"));
      assertTrue(error.getMessage().contains("outside"));
    }
  }

  @Test
  public void reentrantResumeThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-r (std.lib.coroutine/create (fn [] (std.lib.coroutine/resume c-r))))");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-r)"));
      assertTrue(error.getMessage().contains("running"));
    }
  }

  @Test
  public void nestedCoroutinesResumeEachOther() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-inner (std.lib.coroutine/create"
              + " (fn [] (std.lib.coroutine/yield :inner-yield) :inner-end)))");
      context.eval(
          HaraLanguage.ID,
          "(def c-outer (std.lib.coroutine/create"
              + " (fn []"
              + "   (std.lib.coroutine/yield (std.lib.coroutine/resume c-inner))"
              + "   (std.lib.coroutine/yield (std.lib.coroutine/resume c-inner :x))"
              + "   :outer-end)))");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :inner-yield (std.lib.coroutine/resume c-outer))")
              .asBoolean());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :inner-end (std.lib.coroutine/resume c-outer))")
              .asBoolean());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :outer-end (std.lib.coroutine/resume c-outer))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-outer))")
              .asBoolean());
    }
  }

  @Test
  public void generatorPipelineProducesLazily() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-gen (std.lib.coroutine/create"
              + " (fn [n] (loop [i 0]"
              + "   (if (< i n)"
              + "     (do (std.lib.coroutine/yield (* i i)) (recur (inc i)))"
              + "     :done)))))");
      assertEquals(
          "[0 1 4]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(std.lib.coroutine/resume c-gen 3)"
                      + " (std.lib.coroutine/resume c-gen)"
                      + " (std.lib.coroutine/resume c-gen)]")
              .toString());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :done (std.lib.coroutine/resume c-gen))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-gen))")
              .asBoolean());
    }
  }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: FAIL — `std.lib.coroutine/yield` is not defined.

- [ ] **Step 3: Implement yield**

In `StdLibCoroutine.java`, add the field:

```java
  private static final ThreadLocal<HaraCoroutine> CURRENT = new ThreadLocal<>();
```

Add to `HaraCoroutine`:

```java
    Object doYield(Object value) {
      if (status == STATUS_DEAD) throw new CoroutineClosed();
      status = STATUS_SUSPENDED;
      putOutput(new Transfer(value, null));
      return takeInput();
    }
```

In `HaraCoroutine.runBody`, wrap with the thread-local — change the method to:

```java
    private void runBody(Object[] args) {
      CURRENT.set(this);
      try {
        Object result = context.invokeInContext(() -> context.invokeCallable(function, args));
        status = STATUS_DEAD;
        putOutput(new Transfer(result, null));
      } catch (CoroutineClosed closed) {
        // Closed while parked: the unwinding already ran the body's finally clauses.
        Thread.currentThread().interrupt();
      } catch (Throwable error) {
        status = STATUS_DEAD;
        putOutput(new Transfer(null, error));
      } finally {
        CURRENT.remove();
      }
    }
```

Add the export to `StdLibCoroutine`:

```java
  @HaraExport(
      name = "yield",
      doc =
          "Suspends the current coroutine, handing vals to the resumer. The next resume's args"
              + " become this expression's return. Throws outside a coroutine.",
      arglists = {"[& vals]"})
  public static Object yield(HaraContext context, Object[] values) {
    HaraCoroutine coroutine = CURRENT.get();
    if (coroutine == null) {
      throw new HaraException("coroutine/yield: cannot yield outside a coroutine");
    }
    return coroutine.doYield(pack(values));
  }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: PASS (11 tests).

- [ ] **Step 5: Commit**

```bash
git add src/main/java/hara/truffle/StdLibCoroutine.java \
        src/test/java/hara/truffle/StdLibCoroutineTest.java
git commit -m "Add coroutine yield with bidirectional value passing"
```

---

### Task 4: close — interrupt parked coroutine, finally semantics

**Files:**
- Modify: `src/main/java/hara/truffle/StdLibCoroutine.java`
- Test: `src/test/java/hara/truffle/StdLibCoroutineTest.java`

**Interfaces:**
- Consumes: `HaraCoroutine` fields `status`, `thread`; `CoroutineClosed`; `takeInput` (interrupted by close).
- Produces: `Object HaraCoroutine.close()`; export `std.lib.coroutine/close`. Contract: `:suspended` → mark `:dead`, interrupt parked thread, body's `finally` runs on the coroutine thread; `:running` → throws; `:dead` → no-op returning the coroutine.

- [ ] **Step 1: Write the failing tests**

Append to `StdLibCoroutineTest.java`:

```java
  @Test
  public void closeRunsFinallyAndKillsCoroutine() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(HaraLanguage.ID, "(def close-log (atom :init))");
      context.eval(
          HaraLanguage.ID,
          "(def c-close (std.lib.coroutine/create"
              + " (fn [] (try (std.lib.coroutine/yield :parked)"
              + "             (finally (reset! close-log :ran))))))");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :parked (std.lib.coroutine/resume c-close))")
              .asBoolean());
      context.eval(HaraLanguage.ID, "(std.lib.coroutine/close c-close)");
      // Wait for the coroutine thread to unwind (close is asynchronous with the interrupt).
      long deadline = System.currentTimeMillis() + 5000;
      while (System.currentTimeMillis() < deadline) {
        if (context
            .eval(HaraLanguage.ID, "(= :ran (deref close-log))")
            .asBoolean()) {
          break;
        }
        Thread.sleep(20);
      }
      assertTrue(
          context.eval(HaraLanguage.ID, "(= :ran (deref close-log))").asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-close))")
              .asBoolean());
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-close)"));
      assertTrue(error.getMessage().contains("dead"));
    }
  }

  @Test
  public void closeOnDeadIsNoOpAndCloseOnRunningThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(HaraLanguage.ID, "(def c-done (std.lib.coroutine/create (fn [] 1)))");
      context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-done)");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(std.lib.coroutine/coroutine? (std.lib.coroutine/close c-done))")
              .asBoolean());
      context.eval(
          HaraLanguage.ID,
          "(def c-self (std.lib.coroutine/create (fn [] (std.lib.coroutine/close c-self))))");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-self)"));
      assertTrue(error.getMessage().contains("running"));
    }
  }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: FAIL — `std.lib.coroutine/close` is not defined.

- [ ] **Step 3: Implement close**

Add to `HaraCoroutine`:

```java
    Object close() {
      Thread toInterrupt;
      synchronized (this) {
        if (status == STATUS_DEAD) return this;
        if (status == STATUS_RUNNING) {
          throw new HaraException("coroutine/close: cannot close a running coroutine");
        }
        status = STATUS_DEAD;
        toInterrupt = thread;
      }
      if (toInterrupt != null) toInterrupt.interrupt();
      return this;
    }
```

(The parked coroutine thread is inside `takeInput()`; the interrupt surfaces as `InterruptedException`, which `takeInput` converts to `CoroutineClosed`, unwinding the body through its `finally` clauses. `runBody` catches `CoroutineClosed` and exits silently. A `yield` attempted after close — e.g. from a `finally` — hits the `STATUS_DEAD` guard in `doYield` and throws `CoroutineClosed` instead of parking forever.)

Add the export to `StdLibCoroutine`:

```java
  @HaraExport(
      name = "close",
      doc =
          "Closes a suspended coroutine: marks it dead and unwinds it, running finally clauses."
              + " No-op on a dead coroutine. Throws on a running coroutine.",
      arglists = {"[co]"})
  public static Object close(HaraContext context, Object[] values) {
    requireArity("coroutine/close", values, 1);
    return requireCoroutine("coroutine/close", values[0]).close();
  }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: PASS (13 tests).

- [ ] **Step 5: Commit**

```bash
git add src/main/java/hara/truffle/StdLibCoroutine.java \
        src/test/java/hara/truffle/StdLibCoroutineTest.java
git commit -m "Add coroutine close with finally unwinding"
```

---

### Task 5: await — promise integration

**Files:**
- Modify: `src/main/java/hara/truffle/StdLibCoroutine.java`
- Test: `src/test/java/hara/truffle/StdLibCoroutineTest.java`

**Interfaces:**
- Consumes: `hara.lang.protocol.IDeref` (`Object deref()`), implemented by hara promises (`HaraContext.HaraPromise`); guest-side `promise/delay` (`(promise/delay ms thunk)`) and `promise/run` (`(promise/run thunk)`) — both already available without require.
- Produces: export `std.lib.coroutine/await` — blocking deref on the calling (coroutine) thread; returns settlement value, rethrows rejection (`"Promise rejected: ..."` from `HaraPromise.deref`); throws on non-derefable input. Works identically inside and outside a coroutine.

- [ ] **Step 1: Write the failing tests**

Append to `StdLibCoroutineTest.java`:

```java
  @Test
  public void awaitReturnsSettledPromiseValue() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-await (std.lib.coroutine/create"
              + " (fn [] (std.lib.coroutine/await (promise/delay 50 (fn [] :delayed-value))))))");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :delayed-value (std.lib.coroutine/resume c-await))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-await))")
              .asBoolean());
      // Already-settled promise resolves without parking.
      context.eval(
          HaraLanguage.ID,
          "(def c-quick (std.lib.coroutine/create"
              + " (fn [] (std.lib.coroutine/await (promise/run (fn [] 7))))))");
      assertEquals(
          7, context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-quick)").asLong());
    }
  }

  @Test
  public void awaitRethrowsPromiseRejection() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-reject (std.lib.coroutine/create"
              + " (fn [] (std.lib.coroutine/await (promise/run (fn [] (/ 1 0)))))))");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/resume c-reject)"));
      assertTrue(error.getMessage().contains("Promise rejected"));
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.lib.coroutine/status c-reject))")
              .asBoolean());
    }
  }

  @Test
  public void awaitRejectsNonDerefable() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.coroutine)");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.lib.coroutine/await 42)"));
      assertTrue(error.getMessage().contains("derefable"));
    }
  }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: FAIL — `std.lib.coroutine/await` is not defined.

- [ ] **Step 3: Implement await**

In `StdLibCoroutine.java`, add import `hara.lang.protocol.IDeref;` and the export:

```java
  @HaraExport(
      name = "await",
      doc =
          "Blocks the current (coroutine) thread until the promise settles and returns its"
              + " value. Rethrows on rejection. Outside a coroutine, behaves as a plain deref.",
      arglists = {"[p]"})
  public static Object await(HaraContext context, Object[] values) {
    requireArity("coroutine/await", values, 1);
    Object value = HaraBox.unwrap(values[0]);
    if (!(value instanceof IDeref)) {
      throw new HaraException("coroutine/await expects a derefable (e.g. a promise)");
    }
    return ((IDeref<?>) value).deref();
  }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `mvn -q -Ptruffle -Dtest=hara.truffle.StdLibCoroutineTest test`
Expected: PASS (16 tests).

- [ ] **Step 5: Commit**

```bash
git add src/main/java/hara/truffle/StdLibCoroutine.java \
        src/test/java/hara/truffle/StdLibCoroutineTest.java
git commit -m "Add coroutine await for promise settlement"
```

---

### Task 6: Spec and docs updates

**Files:**
- Modify: `spec/hara/runtime-libraries.md`
- Modify: `docs/reference/runtime-libraries.md`
- Modify: `spec/hara/xtalk-equivalence.md`
- Modify: `docs/reference/xtalk-equivalence.md`
- Modify: `docs/superpowers/specs/2026-07-25-lua-style-coroutines-design.md` (corrections below)

**Interfaces:**
- Consumes: the provider table at `spec/hara/runtime-libraries.md:73-85`; the equivalence table at `spec/hara/xtalk-equivalence.md:7-20`.
- Produces: documented `std.lib.coroutine` contract; `x:coroutine-*` equivalence rows; corrected design spec.

- [ ] **Step 1: Check whether spec and docs/reference copies are mirrored**

Run: `diff spec/hara/runtime-libraries.md docs/reference/runtime-libraries.md; diff spec/hara/xtalk-equivalence.md docs/reference/xtalk-equivalence.md`
Expected: no output (identical) or minor drift. If identical, make the same edits to both files. If drifted, edit each to match its own surrounding format.

- [ ] **Step 2: Add the std.lib.coroutine entry to runtime-libraries.md (both copies)**

Add a row to the provider-backed namespaces table (after the `std.lib.task` row):

```markdown
| `std.lib.coroutine` | Lua-style coroutines: create, resume, yield, status, close, await |
```

Add a prose section after the table:

```markdown
`std.lib.coroutine` provides Lua-style coroutines on the Truffle runtime. `create` wraps a
function without starting it; `resume` starts or continues a coroutine and returns the yielded
or final value; `yield` suspends the current coroutine from any call depth and returns the next
resume's arguments; `status` reports `:suspended`, `:running`, or `:dead`. With multiple values,
`yield` and resume arguments pack into vectors (single values pass through as-is). Errors inside
a body rethrow at the resume site and leave the coroutine `:dead`. `close` unwinds a suspended
coroutine, running its `finally` clauses. `await` blocks the coroutine until a promise settles.
Var bindings established around a `resume` do not propagate into the coroutine body. Coroutines
are currently Truffle-only; the Rust runtime does not implement them yet.
```

- [ ] **Step 3: Add coroutine rows to xtalk-equivalence.md (both copies)**

Append to the equivalence table:

```markdown
| `coroutine/create`, `coroutine/resume`, `coroutine/yield`, `coroutine/status` | `x:coroutine-create`, `x:coroutine-resume`, `x:coroutine-yield`, `x:coroutine-status` |
```

- [ ] **Step 4: Correct the design spec**

In `docs/superpowers/specs/2026-07-25-lua-style-coroutines-design.md`:

1. In "Modified files", remove the `spec/hara/wasm-truffle-parity.edn` and
   `spec/hara/hara-core-symbols.json` bullets and add one bullet:

```markdown
- No `wasm-truffle-parity.edn` or `hara-core-symbols.json` changes: the parity file is an
  executable corpus that must not gain uncovered cases (the Truffle-only status is recorded
  in `runtime-libraries.md` instead), and the core-symbols inventory covers L0 plus eagerly
  referred `std.lib.foundation` only — lazy providers like `std.lib.task` are not listed.
```

2. In the `await` table row, remove "interrupt-sensitive (a `close` during await unwinds
   it)" phrasing if present and state: `await` is a plain blocking deref; `close` cannot fire
   during an `await` because the coroutine is `:running` then and `close` throws on `:running`.

- [ ] **Step 5: Commit**

```bash
git add spec/hara/runtime-libraries.md docs/reference/runtime-libraries.md \
        spec/hara/xtalk-equivalence.md docs/reference/xtalk-equivalence.md \
        docs/superpowers/specs/2026-07-25-lua-style-coroutines-design.md
git commit -m "Document std.lib.coroutine contract and xtalk equivalence"
```

---

### Task 7: Full verification

**Files:** none modified (verification only).

- [ ] **Step 1: Run the full test suite**

Run: `mvn -q test`
Expected: BUILD SUCCESS, all tests pass including the 16 `StdLibCoroutineTest` tests.

- [ ] **Step 2: REPL smoke test**

Run:

```bash
./hara eval "(do (require 'std.lib.coroutine) (def g (std.lib.coroutine/create (fn [] (std.lib.coroutine/yield 1) (std.lib.coroutine/yield 2) 3))) [(std.lib.coroutine/resume g) (std.lib.coroutine/resume g) (std.lib.coroutine/resume g)])"
```

Expected output: `[1 2 3]`

- [ ] **Step 3: Generator-pipeline smoke test (async interop)**

Run:

```bash
./hara eval "(do (require 'std.lib.coroutine) (def c (std.lib.coroutine/create (fn [] (std.lib.coroutine/await (promise/delay 10 (fn [] 40))) 2))) (+ (std.lib.coroutine/resume c) 0))"
```

Expected output: `42`
