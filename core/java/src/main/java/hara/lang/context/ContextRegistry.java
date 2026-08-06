package hara.lang.context;

import hara.lang.protocol.IContext;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Registry of context definitions, runtime variants, and scratch runtimes. */
public final class ContextRegistry {
  private final Map<Object, Definition> contexts = new ConcurrentHashMap<>();

  public ContextRegistry() {
    Map<String, Object> nullRuntime = new LinkedHashMap<>();
    nullRuntime.put("key", "default");
    nullRuntime.put("resource", "hara/context.rt.null");
    nullRuntime.put("config", Map.of());
    Map<String, Object> config = new LinkedHashMap<>();
    config.put("context", "null");
    config.put("rt", Map.of("default", nullRuntime));
    config.put("scratch", NullContext.INSTANCE);
    install("null", config);
  }

  public void install(Object key, IContext context) {
    if (key == null || context == null)
      throw new IllegalArgumentException("Context and key required");
    Map<String, Object> runtime = new LinkedHashMap<>();
    runtime.put("key", "default");
    runtime.put("instance", context);
    runtime.put("config", Map.of());
    install(
        key,
        Map.of(
            "context", key,
            "scratch", context,
            "rt", Map.of("default", runtime)));
  }

  public void install(Object key, Map<String, Object> config) {
    if (key == null) throw new IllegalArgumentException("Context key required");
    contexts.put(key, Definition.from(key, config));
  }

  public Map<String, Object> uninstall(Object key) {
    Definition removed = contexts.remove(key);
    return removed == null ? null : removed.asMap();
  }

  public IContext get(Object key) {
    return scratch(key);
  }

  public boolean contains(Object key) {
    return contexts.containsKey(key);
  }

  public java.util.List<Object> list() {
    return new ArrayList<>(contexts.keySet());
  }

  public Map<String, Object> definition(Object key) {
    Definition definition = require(key);
    return definition.asMap();
  }

  public Map<String, Object> runtime(Object context, Object runtime) {
    return require(context).runtime(runtime == null ? "default" : key(runtime));
  }

  public java.util.List<Object> runtimeList(Object context) {
    return new ArrayList<>(require(context).runtimes.keySet());
  }

  public Map<Object, java.util.List<Object>> runtimeList() {
    Map<Object, java.util.List<Object>> result = new LinkedHashMap<>();
    contexts.forEach((key, value) -> result.put(key, new ArrayList<>(value.runtimes.keySet())));
    return result;
  }

  public Map<String, Object> addRuntime(Object context, Map<String, Object> runtime) {
    Definition definition = require(context);
    Object key = runtime.get("key");
    if (key == null) throw new IllegalArgumentException("Runtime key required");
    return definition.addRuntime(key(key), runtime);
  }

  public Map<String, Object> removeRuntime(Object context, Object runtime) {
    return require(context).runtimes.remove(key(runtime));
  }

  public IContext scratch(Object context) {
    Definition definition = contexts.get(context);
    return definition == null || definition.scratch == null
        ? NullContext.INSTANCE
        : definition.scratch;
  }

  private Definition require(Object key) {
    Definition definition = contexts.get(key);
    if (definition == null) {
      throw new IllegalArgumentException(
          "No context available: " + key + " (options: " + contexts.keySet() + ")");
    }
    return definition;
  }

  public static String key(Object value) {
    if (value instanceof hara.lang.data.Keyword keyword) return keyword.getName();
    if (value instanceof hara.lang.data.Symbol symbol) return symbol.getName();
    String text = String.valueOf(value);
    return text.startsWith(":") ? text.substring(1) : text;
  }

  private static final class Definition {
    private final Object context;
    private final Map<String, Object> config;
    private final Map<String, Map<String, Object>> runtimes;
    private final IContext scratch;

    private Definition(
        Object context,
        Map<String, Object> config,
        Map<String, Map<String, Object>> runtimes,
        IContext scratch) {
      this.context = context;
      this.config = config;
      this.runtimes = runtimes;
      this.scratch = scratch;
    }

    @SuppressWarnings("unchecked")
    static Definition from(Object key, Map<String, Object> source) {
      Map<String, Object> input = source == null ? Map.of() : source;
      Object context = input.getOrDefault("context", key);
      Map<String, Object> config =
          input.get("config") instanceof Map<?, ?>
              ? new LinkedHashMap<>((Map<String, Object>) input.get("config"))
              : new LinkedHashMap<>();
      Map<String, Map<String, Object>> runtimes = new ConcurrentHashMap<>();
      if (input.get("rt") instanceof Map<?, ?> values) {
        values.forEach(
            (runtimeKey, value) -> {
              if (value instanceof Map<?, ?> map) {
                Map<String, Object> runtime = stringMap(map);
                runtime.putIfAbsent("key", key(runtimeKey));
                runtime.putIfAbsent("config", Map.of());
                runtimes.put(key(runtimeKey), runtime);
              }
            });
      }
      Object scratch = input.get("scratch");
      return new Definition(
          context,
          config,
          runtimes,
          scratch instanceof IContext ? (IContext) scratch : NullContext.INSTANCE);
    }

    Map<String, Object> addRuntime(String key, Map<String, Object> source) {
      Map<String, Object> runtime = new LinkedHashMap<>(source);
      runtime.put("key", key);
      runtime.putIfAbsent("config", Map.of());
      runtimes.put(key, runtime);
      return runtime;
    }

    Map<String, Object> runtime(String key) {
      Map<String, Object> runtime = runtimes.get(key);
      if (runtime == null) {
        throw new IllegalArgumentException(
            "No runtime installed: " + key + " (options: " + runtimes.keySet() + ")");
      }
      Map<String, Object> result = new LinkedHashMap<>(config);
      result.put("context", context);
      result.putAll(runtime);
      result.put("config", merge(config, map(runtime.get("config"))));
      return result;
    }

    Map<String, Object> asMap() {
      Map<String, Object> result = new LinkedHashMap<>(config);
      result.put("context", context);
      result.put("rt", Collections.unmodifiableMap(runtimes));
      result.put("scratch", scratch);
      return result;
    }
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> map(Object value) {
    return value instanceof Map<?, ?> ? (Map<String, Object>) value : Map.of();
  }

  private static Map<String, Object> stringMap(Map<?, ?> source) {
    Map<String, Object> result = new LinkedHashMap<>();
    source.forEach((key, value) -> result.put(ContextRegistry.key(key), value));
    return result;
  }

  @SuppressWarnings("unchecked")
  public static Map<String, Object> merge(
      Map<String, Object> first, Map<String, Object> second) {
    Map<String, Object> result = new LinkedHashMap<>(first);
    second.forEach(
        (key, value) -> {
          Object current = result.get(key);
          if (current instanceof Map<?, ?> && value instanceof Map<?, ?>) {
            result.put(
                key,
                merge(
                    (Map<String, Object>) current,
                    (Map<String, Object>) value));
          } else {
            result.put(key, value);
          }
        });
    return result;
  }
}
