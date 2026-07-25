package hara.truffle;

import hara.lang.context.ContextRegistry;
import hara.lang.context.NullContext;
import hara.lang.context.Pointer;
import hara.lang.context.Space;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.types.IMapType;
import hara.lang.protocol.IContext;
import hara.lang.protocol.IContextLifeCycle;
import hara.lang.protocol.IPointer;
import hara.lang.resource.ResourceMode;
import hara.lang.resource.ResourceRegistry;
import hara.lang.resource.ResourceSpec;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Static Java implementation exported exclusively as {@code std.lib.context/*}. */
public final class StdLibContext {
  private static final String STATE_KEY = "std.lib.context";

  private StdLibContext() {}

  /** Java-only access for extensions; this method is deliberately not a Hara export. */
  public static ContextRegistry registry(HaraContext context) {
    return state(context).contexts;
  }

  /** Java-only access for extensions; this method is deliberately not a Hara export. */
  public static ResourceRegistry resources(HaraContext context) {
    return state(context).resources;
  }

  /** Java-only access for extensions; this method is deliberately not a Hara export. */
  public static Space namespaceSpace(HaraContext context, Object namespace) {
    return state(context).space(namespace);
  }

  @HaraExport(
      name = "space",
      doc = "Returns the context space for the current or supplied namespace.",
      arglists = {"[]", "[namespace]"})
  public static Object space(HaraContext context, Object[] values) {
    atMost("space", values, 1);
    return state(context).space(values.length == 0 ? context.currentNamespaceName() : values[0]);
  }

  @HaraExport(
      name = "space?",
      doc = "Returns true when value is a context space.",
      arglists = {"[value]"})
  public static Object spacePredicate(HaraContext context, Object[] values) {
    exactly("space?", values, 1);
    return values[0] instanceof Space;
  }

  @HaraExport(
      name = "space-create",
      doc = "Creates an independent context space.",
      arglists = {"[options]"})
  public static Object spaceCreate(HaraContext context, Object[] values) {
    exactly("space-create", values, 1);
    Map<String, Object> options = stringMap(values[0], "space-create");
    return new Space(
        options.getOrDefault("namespace", context.currentNamespaceName()),
        state(context).contexts,
        state(context).resources);
  }

  @HaraExport(
      name = "space-resolve",
      doc = "Resolves a space object or namespace identifier.",
      arglists = {"[value]"})
  public static Object spaceResolve(HaraContext context, Object[] values) {
    exactly("space-resolve", values, 1);
    return resolveSpace(context, values[0]);
  }

  @HaraExport(
      name = "space:context-set",
      doc = "Configures a context runtime in a space.",
      arglists = {"[context key options]", "[space context key options]"})
  public static Object spaceContextSet(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:context-set", values, 3);
    call.space.contextSet(call.args[0], call.args[1], mapOrEmpty(call.args[2]));
    return guest(call.space.contextGet(call.args[0]));
  }

  @HaraExport(
      name = "space:context-unset",
      doc = "Removes a configured context and stops its runtime.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceContextUnset(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:context-unset", values, 1);
    Object previous = call.space.contextGet(call.args[0]);
    call.space.contextUnset(call.args[0]);
    return guest(previous);
  }

  @HaraExport(
      name = "space:context-list",
      doc = "Lists configured contexts in a space.",
      arglists = {"[]", "[space]"})
  public static Object spaceContextList(HaraContext context, Object[] values) {
    return guest(zeroOrSpace(context, "space:context-list", values).contextList());
  }

  @HaraExport(
      name = "space:context-get",
      doc = "Returns a configured context descriptor.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceContextGet(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:context-get", values, 1);
    return guest(call.space.contextGet(call.args[0]));
  }

  @HaraExport(
      name = "space:rt-active",
      doc = "Lists active context runtimes in a space.",
      arglists = {"[]", "[space]"})
  public static Object spaceRuntimeActive(HaraContext context, Object[] values) {
    return guest(zeroOrSpace(context, "space:rt-active", values).activeRuntimes());
  }

  @HaraExport(
      name = "space:rt-get",
      doc = "Returns an active runtime or nil.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceRuntimeGet(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:rt-get", values, 1);
    return call.space.runtimeGet(call.args[0]);
  }

  @HaraExport(
      name = "space:rt-start",
      doc = "Starts and returns a configured context runtime.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceRuntimeStart(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:rt-start", values, 1);
    return call.space.runtimeStart(call.args[0]);
  }

  @HaraExport(
      name = "space:rt-started?",
      doc = "Returns true when a context runtime is active.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceRuntimeStarted(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:rt-started?", values, 1);
    return call.space.runtimeStarted(call.args[0]);
  }

  @HaraExport(
      name = "space:rt-stopped?",
      doc = "Returns true when a context runtime is inactive.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceRuntimeStopped(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:rt-stopped?", values, 1);
    return call.space.runtimeStopped(call.args[0]);
  }

  @HaraExport(
      name = "space:rt-stop",
      doc = "Stops a configured context runtime.",
      arglists = {"[context]", "[space context]"})
  public static Object spaceRuntimeStop(HaraContext context, Object[] values) {
    SpaceCall call = spaceCall(context, "space:rt-stop", values, 1);
    call.space.runtimeStop(call.args[0]);
    return null;
  }

  @HaraExport(
      name = "space:rt-current",
      doc = "Returns the active, scratch, or null runtime for a context.",
      arglists = {"[context]", "[namespace context]"})
  public static Object spaceRuntimeCurrent(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw error("space:rt-current", "expects context or namespace and context");
    }
    Object namespace = values.length == 1 ? context.currentNamespaceName() : values[0];
    Object key = values.length == 1 ? values[0] : values[1];
    return state(context).space(namespace).runtimeCurrent(key);
  }

  @HaraExport(
      name = "registry-list",
      doc = "Lists registered context identifiers.",
      arglists = {"[]"})
  public static Object registryList(HaraContext context, Object[] values) {
    exactly("registry-list", values, 0);
    return guest(state(context).contexts.list());
  }

  @HaraExport(
      name = "registry-install",
      doc = "Installs or replaces a context definition.",
      arglists = {"[context]", "[context config]"})
  public static Object registryInstall(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw error("registry-install", "expects context and optional configuration");
    }
    Object key = key(values[0]);
    Map<String, Object> config =
        values.length == 1 ? new LinkedHashMap<>() : stringMap(values[1], "registry-install");
    config.put("context", key);
    state(context).contexts.install(key, config);
    return guest(state(context).contexts.definition(key));
  }

  @HaraExport(
      name = "registry-uninstall",
      doc = "Uninstalls a context definition.",
      arglists = {"[context]"})
  public static Object registryUninstall(HaraContext context, Object[] values) {
    exactly("registry-uninstall", values, 1);
    return guest(state(context).contexts.uninstall(key(values[0])));
  }

  @HaraExport(
      name = "registry-get",
      doc = "Returns a registered context definition.",
      arglists = {"[context]"})
  public static Object registryGet(HaraContext context, Object[] values) {
    exactly("registry-get", values, 1);
    return guest(state(context).contexts.definition(key(values[0])));
  }

  @HaraExport(
      name = "registry-rt-list",
      doc = "Lists runtime variants for all contexts or one context.",
      arglists = {"[]", "[context]"})
  public static Object registryRuntimeList(HaraContext context, Object[] values) {
    atMost("registry-rt-list", values, 1);
    return guest(
        values.length == 0
            ? state(context).contexts.runtimeList()
            : state(context).contexts.runtimeList(key(values[0])));
  }

  @HaraExport(
      name = "registry-rt-add",
      doc = "Adds or replaces a runtime variant for a context.",
      arglists = {"[context config]"})
  public static Object registryRuntimeAdd(HaraContext context, Object[] values) {
    exactly("registry-rt-add", values, 2);
    return guest(
        state(context)
            .contexts
            .addRuntime(key(values[0]), stringMap(values[1], "registry-rt-add")));
  }

  @HaraExport(
      name = "registry-rt-remove",
      doc = "Removes a runtime variant from a context.",
      arglists = {"[context key]"})
  public static Object registryRuntimeRemove(HaraContext context, Object[] values) {
    exactly("registry-rt-remove", values, 2);
    return guest(state(context).contexts.removeRuntime(key(values[0]), key(values[1])));
  }

  @HaraExport(
      name = "registry-rt",
      doc = "Returns a merged runtime descriptor.",
      arglists = {"[context]", "[context key]"})
  public static Object registryRuntime(HaraContext context, Object[] values) {
    if (values.length < 1 || values.length > 2) {
      throw error("registry-rt", "expects context and optional runtime key");
    }
    return guest(
        state(context)
            .contexts
            .runtime(key(values[0]), values.length == 1 ? "default" : key(values[1])));
  }

  @HaraExport(
      name = "registry-scratch",
      doc = "Returns the scratch runtime for a context.",
      arglists = {"[context]"})
  public static Object registryScratch(HaraContext context, Object[] values) {
    exactly("registry-scratch", values, 1);
    return state(context).contexts.scratch(key(values[0]));
  }

  @HaraExport(
      name = "rt-null?",
      doc = "Returns true when value is the null context runtime.",
      arglists = {"[value]"})
  public static Object runtimeNullPredicate(HaraContext context, Object[] values) {
    exactly("rt-null?", values, 1);
    return values[0] instanceof NullContext;
  }

  @HaraExport(
      name = "rt:raw-eval",
      doc = "Evaluates raw source through a context runtime.",
      arglists = {"[runtime source]"})
  public static Object runtimeRawEval(HaraContext context, Object[] values) {
    exactly("rt:raw-eval", values, 2);
    return runtime(context, values[0]).rawEval(String.valueOf(HaraBox.unwrap(values[1])));
  }

  @HaraExport(
      name = "rt:init-ptr",
      doc = "Initializes a pointer through a context runtime.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeInitPointer(HaraContext context, Object[] values) {
    exactly("rt:init-ptr", values, 2);
    return runtime(context, values[0]).initPtr(pointer(values[1], "rt:init-ptr"));
  }

  @HaraExport(
      name = "rt:tags-ptr",
      doc = "Returns runtime-specific tags for a pointer.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeTagsPointer(HaraContext context, Object[] values) {
    exactly("rt:tags-ptr", values, 2);
    return runtime(context, values[0]).tagsPtr(pointer(values[1], "rt:tags-ptr"));
  }

  @HaraExport(
      name = "rt:deref-ptr",
      doc = "Dereferences a pointer through a context runtime.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeDerefPointer(HaraContext context, Object[] values) {
    exactly("rt:deref-ptr", values, 2);
    return guest(runtime(context, values[0]).derefPtr(pointer(values[1], "rt:deref-ptr")));
  }

  @HaraExport(
      name = "rt:display-ptr",
      doc = "Returns a runtime-specific pointer display value.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeDisplayPointer(HaraContext context, Object[] values) {
    exactly("rt:display-ptr", values, 2);
    return runtime(context, values[0]).displayPtr(pointer(values[1], "rt:display-ptr"));
  }

  @HaraExport(
      name = "rt:invoke-ptr",
      doc = "Invokes a pointer through a context runtime.",
      arglists = {"[runtime pointer args]"})
  public static Object runtimeInvokePointer(HaraContext context, Object[] values) {
    exactly("rt:invoke-ptr", values, 3);
    return runtime(context, values[0])
        .invokePtr(pointer(values[1], "rt:invoke-ptr"), objects(values[2]));
  }

  @HaraExport(
      name = "rt:transform-in-ptr",
      doc = "Transforms pointer arguments before runtime invocation.",
      arglists = {"[runtime pointer args]"})
  public static Object runtimeTransformInPointer(HaraContext context, Object[] values) {
    exactly("rt:transform-in-ptr", values, 3);
    return runtime(context, values[0])
        .transformInPtr(pointer(values[1], "rt:transform-in-ptr"), objects(values[2]));
  }

  @HaraExport(
      name = "rt:transform-out-ptr",
      doc = "Transforms a pointer return value after runtime invocation.",
      arglists = {"[runtime pointer return]"})
  public static Object runtimeTransformOutPointer(HaraContext context, Object[] values) {
    exactly("rt:transform-out-ptr", values, 3);
    return runtime(context, values[0])
        .transformOutPtr(pointer(values[1], "rt:transform-out-ptr"), values[2]);
  }

  @HaraExport(
      name = "rt:has-module?",
      doc = "Returns true when a runtime has a module.",
      arglists = {"[runtime module]"})
  public static Object runtimeHasModule(HaraContext context, Object[] values) {
    exactly("rt:has-module?", values, 2);
    return lifecycle(context, values[0], "rt:has-module?").hasModule(values[1]);
  }

  @HaraExport(
      name = "rt:setup-module",
      doc = "Sets up a module in a runtime.",
      arglists = {"[runtime module]"})
  public static Object runtimeSetupModule(HaraContext context, Object[] values) {
    exactly("rt:setup-module", values, 2);
    IContextLifeCycle lifecycle = lifecycle(context, values[0], "rt:setup-module");
    lifecycle.setupModule(values[1]);
    return values[0];
  }

  @HaraExport(
      name = "rt:teardown-module",
      doc = "Tears down a module in a runtime.",
      arglists = {"[runtime module]"})
  public static Object runtimeTeardownModule(HaraContext context, Object[] values) {
    exactly("rt:teardown-module", values, 2);
    IContextLifeCycle lifecycle = lifecycle(context, values[0], "rt:teardown-module");
    lifecycle.teardownModule(values[1]);
    return values[0];
  }

  @HaraExport(
      name = "rt:has-ptr?",
      doc = "Returns true when a pointer is initialized in a runtime.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeHasPointer(HaraContext context, Object[] values) {
    exactly("rt:has-ptr?", values, 2);
    return lifecycle(context, values[0], "rt:has-ptr?")
        .hasPointer(pointer(values[1], "rt:has-ptr?"));
  }

  @HaraExport(
      name = "rt:setup-ptr",
      doc = "Sets up a pointer in a runtime.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeSetupPointer(HaraContext context, Object[] values) {
    exactly("rt:setup-ptr", values, 2);
    IPointer pointer = pointer(values[1], "rt:setup-ptr");
    lifecycle(context, values[0], "rt:setup-ptr").setupPointer(pointer);
    return pointer;
  }

  @HaraExport(
      name = "rt:teardown-ptr",
      doc = "Tears down a pointer in a runtime.",
      arglists = {"[runtime pointer]"})
  public static Object runtimeTeardownPointer(HaraContext context, Object[] values) {
    exactly("rt:teardown-ptr", values, 2);
    IPointer pointer = pointer(values[1], "rt:teardown-ptr");
    lifecycle(context, values[0], "rt:teardown-ptr").teardownPointer(pointer);
    return pointer;
  }

  @HaraExport(
      name = "pointer",
      doc = "Creates a context-qualified pointer.",
      arglists = {"[options]"})
  public static Object pointerCreate(HaraContext context, Object[] values) {
    exactly("pointer", values, 1);
    Map<String, Object> options = stringMap(values[0], "pointer");
    Object pointerContext = options.get("context");
    if (pointerContext == null) throw error("pointer", "requires :context");
    State state = state(context);
    return new Pointer(
        key(pointerContext),
        options,
        requested -> {
          Object explicit = options.get("context/rt");
          if (explicit instanceof IContext) return (IContext) explicit;
          Object factory = options.get("context/fn");
          if (factory != null) {
            Object produced =
                context.invokeCallable(factory, new Object[] {options});
            if (produced instanceof IContext) return (IContext) produced;
            throw error("pointer-default", ":context/fn must return an IContext");
          }
          return state.current(context.currentNamespaceName(), requested);
        });
  }

  @HaraExport(
      name = "pointer?",
      doc = "Returns true when value is a context pointer.",
      arglists = {"[value]"})
  public static Object pointerPredicate(HaraContext context, Object[] values) {
    exactly("pointer?", values, 1);
    return values[0] instanceof Pointer;
  }

  @HaraExport(
      name = "pointer-deref",
      doc = "Dereferences a context pointer.",
      arglists = {"[pointer]"})
  public static Object pointerDeref(HaraContext context, Object[] values) {
    exactly("pointer-deref", values, 1);
    return guest(((Pointer) pointer(values[0], "pointer-deref")).deref());
  }

  @HaraExport(
      name = "pointer-default",
      doc = "Returns the runtime selected for a context pointer.",
      arglists = {"[pointer]"})
  public static Object pointerDefault(HaraContext context, Object[] values) {
    exactly("pointer-default", values, 1);
    return ((Pointer) pointer(values[0], "pointer-default")).runtime();
  }

  private static State state(HaraContext context) {
    return context.libraryState(STATE_KEY, State::new);
  }

  private static Space resolveSpace(HaraContext context, Object value) {
    if (value instanceof Space) return (Space) value;
    return state(context).space(value == null ? context.currentNamespaceName() : value);
  }

  private static Space zeroOrSpace(HaraContext context, String name, Object[] values) {
    atMost(name, values, 1);
    return values.length == 0
        ? state(context).space(context.currentNamespaceName())
        : resolveSpace(context, values[0]);
  }

  private static SpaceCall spaceCall(
      HaraContext context, String name, Object[] values, int implicitArity) {
    if (values.length == implicitArity) {
      return new SpaceCall(
          state(context).space(context.currentNamespaceName()), values);
    }
    if (values.length == implicitArity + 1) {
      Object[] args = new Object[implicitArity];
      System.arraycopy(values, 1, args, 0, implicitArity);
      return new SpaceCall(resolveSpace(context, values[0]), args);
    }
    throw error(name, "received invalid arguments");
  }

  private static IContext runtime(HaraContext context, Object value) {
    if (value instanceof IContext) return (IContext) value;
    return state(context).current(context.currentNamespaceName(), value);
  }

  private static IContextLifeCycle lifecycle(
      HaraContext context, Object value, String function) {
    IContext runtime = runtime(context, value);
    if (!(runtime instanceof IContextLifeCycle)) {
      throw error(function, "requires an IContextLifeCycle runtime");
    }
    return (IContextLifeCycle) runtime;
  }

  private static IPointer pointer(Object value, String function) {
    if (!(value instanceof IPointer)) throw error(function, "expects a pointer");
    return (IPointer) value;
  }

  private static Object[] objects(Object value) {
    if (value instanceof Iterable<?> iterable) {
      ArrayList<Object> result = new ArrayList<>();
      iterable.forEach(result::add);
      return result.toArray();
    }
    return new Object[] {value};
  }

  private static Map<String, Object> mapOrEmpty(Object value) {
    if (value == null) return Map.of();
    return stringMap(value, "space:context-set");
  }

  private static Map<String, Object> stringMap(Object value, String function) {
    Map<String, Object> result = new LinkedHashMap<>();
    if (value instanceof IMapType<?, ?> map) {
      for (Object entryValue : map) {
        Map.Entry<?, ?> entry = (Map.Entry<?, ?>) entryValue;
        result.put(key(entry.getKey()), normalize(entry.getValue()));
      }
      return result;
    }
    if (value instanceof Map<?, ?> map) {
      map.forEach((entryKey, entryValue) -> result.put(key(entryKey), normalize(entryValue)));
      return result;
    }
    throw error(function, "expects a map");
  }

  private static Object normalize(Object value) {
    if (value instanceof IMapType<?, ?> || value instanceof Map<?, ?>) {
      return stringMap(value, "context configuration");
    }
    return value;
  }

  private static Object guest(Object value) {
    if (value instanceof Map<?, ?> map) {
      ArrayList<Object> entries = new ArrayList<>();
      map.forEach(
          (entryKey, entryValue) -> {
            entries.add(
                entryKey instanceof String ? Keyword.create((String) entryKey) : entryKey);
            entries.add(guest(entryValue));
          });
      return hara.lang.data.Map.Standard.from(null, entries.toArray());
    }
    if (value instanceof Iterable<?> iterable) {
      ArrayList<Object> entries = new ArrayList<>();
      iterable.forEach(entry -> entries.add(guest(entry)));
      return hara.lang.data.Vector.Standard.from(null, entries.toArray());
    }
    return value;
  }

  private static String key(Object value) {
    if (value instanceof Keyword keyword) {
      return keyword.getNamespace() == null
          ? keyword.getName()
          : keyword.getNamespace() + "/" + keyword.getName();
    }
    if (value instanceof Symbol symbol) {
      return symbol.getNamespace() == null
          ? symbol.getName()
          : symbol.getNamespace() + "/" + symbol.getName();
    }
    String text = String.valueOf(value);
    return text.startsWith(":") ? text.substring(1) : text;
  }

  private static void exactly(String name, Object[] values, int count) {
    if (values.length != count) {
      throw error(name, "expects " + count + (count == 1 ? " argument" : " arguments"));
    }
  }

  private static void atMost(String name, Object[] values, int count) {
    if (values.length > count) throw error(name, "accepts at most " + count + " argument");
  }

  private static HaraException error(String function, String message) {
    return new HaraException("std.lib.context/" + function + " " + message);
  }

  private static final class SpaceCall {
    private final Space space;
    private final Object[] args;

    private SpaceCall(Space space, Object[] args) {
      this.space = space;
      this.args = args;
    }
  }

  private static final class State {
    private final ContextRegistry contexts = new ContextRegistry();
    private final ResourceRegistry resources = new ResourceRegistry();
    private final Map<String, Space> spaces = new ConcurrentHashMap<>();

    private State() {
      resources.add(
          new ResourceSpec(
              "hara/context.rt.null",
              ResourceMode.NAMESPACE,
              Map.of(),
              options -> NullContext.INSTANCE));
    }

    private Space space(Object namespace) {
      String name = key(namespace);
      return spaces.computeIfAbsent(name, ignored -> new Space(name, contexts, resources));
    }

    private IContext current(Object namespace, Object context) {
      Object runtime = space(namespace).runtimeCurrent(key(context));
      return runtime instanceof IContext ? (IContext) runtime : NullContext.INSTANCE;
    }
  }
}
