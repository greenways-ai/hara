package hara.lang.context;

import hara.lang.protocol.ISpace;
import hara.lang.resource.ResourceInstance;
import hara.lang.resource.ResourceRegistry;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Namespace-local collection of context configurations and active runtimes. */
public final class Space implements ISpace {
  private final Object namespace;
  private final ContextRegistry contexts;
  private final ResourceRegistry resources;
  private final Map<Object, Map<String, Object>> configurations = new LinkedHashMap<>();
  private final Map<Object, Object> runtimes = new LinkedHashMap<>();

  public Space() {
    this("user", new ContextRegistry(), new ResourceRegistry());
  }

  public Space(Object namespace, ContextRegistry contexts, ResourceRegistry resources) {
    this.namespace = namespace == null ? "user" : namespace;
    this.contexts = contexts == null ? new ContextRegistry() : contexts;
    this.resources = resources == null ? new ResourceRegistry() : resources;
  }

  public Object namespace() {
    return namespace;
  }

  @Override
  public synchronized void contextSet(Object context, Object key, Object options) {
    Object normalized = ContextRegistry.key(context);
    Map<String, Object> descriptor = contexts.runtime(normalized, key);
    descriptor.put(
        "config",
        ContextRegistry.merge(
            map(descriptor.get("config")),
            options instanceof Map<?, ?> ? stringMap((Map<?, ?>) options) : Map.of()));
    contextUnset(normalized);
    configurations.put(normalized, descriptor);
  }

  @Override
  public synchronized void contextUnset(Object context) {
    Object normalized = ContextRegistry.key(context);
    runtimeStop(normalized);
    configurations.remove(normalized);
  }

  @Override
  public synchronized List<?> contextList() {
    return new ArrayList<>(configurations.keySet());
  }

  @Override
  public synchronized Object contextGet(Object context) {
    Object normalized = ContextRegistry.key(context);
    Map<String, Object> descriptor = configurations.get(normalized);
    if (descriptor == null) {
      throw new IllegalArgumentException(
          "Context not found: " + normalized + " (options: " + configurations.keySet() + ")");
    }
    Map<String, Object> result = new LinkedHashMap<>(descriptor);
    result.putIfAbsent("variant", "default");
    return result;
  }

  @Override
  public synchronized List<?> activeRuntimes() {
    return new ArrayList<>(runtimes.keySet());
  }

  @Override
  public synchronized Object runtimeGet(Object context) {
    return runtimes.get(ContextRegistry.key(context));
  }

  @Override
  public synchronized Object runtimeStart(Object context) {
    Object normalized = ContextRegistry.key(context);
    Object current = runtimes.get(normalized);
    if (current != null) return current;
    Map<String, Object> descriptor = cast(contextGet(normalized));
    Object instance = descriptor.get("instance");
    if (instance == null) {
      String resource = ContextRegistry.key(descriptor.get("resource"));
      String variant = ContextRegistry.key(descriptor.getOrDefault("variant", "default"));
      ResourceInstance started =
          resources.resolve(resource, variant, scope(normalized), map(descriptor.get("config")));
      instance = started.value();
    }
    runtimes.put(normalized, instance);
    return instance;
  }

  @Override
  public synchronized boolean runtimeStarted(Object context) {
    return runtimes.containsKey(ContextRegistry.key(context));
  }

  @Override
  public synchronized boolean runtimeStopped(Object context) {
    return !runtimeStarted(context);
  }

  @Override
  public synchronized void runtimeStop(Object context) {
    Object normalized = ContextRegistry.key(context);
    Object current = runtimes.remove(normalized);
    if (current == null) return;
    Map<String, Object> descriptor = configurations.get(normalized);
    if (descriptor == null || descriptor.get("instance") != null) return;
    String resource = ContextRegistry.key(descriptor.get("resource"));
    String variant = ContextRegistry.key(descriptor.getOrDefault("variant", "default"));
    resources.stop(resource, variant, scope(normalized));
  }

  public synchronized Object runtimeCurrent(Object context) {
    Object normalized = ContextRegistry.key(context);
    Object runtime = runtimes.get(normalized);
    return runtime == null ? contexts.scratch(normalized) : runtime;
  }

  public synchronized Space stop() {
    for (Object context : new ArrayList<>(runtimes.keySet())) runtimeStop(context);
    return this;
  }

  private String scope(Object context) {
    return namespace + "/" + context;
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> map(Object value) {
    return value instanceof Map<?, ?> ? (Map<String, Object>) value : Map.of();
  }

  @SuppressWarnings("unchecked")
  private static Map<String, Object> cast(Object value) {
    return (Map<String, Object>) value;
  }

  private static Map<String, Object> stringMap(Map<?, ?> source) {
    Map<String, Object> result = new LinkedHashMap<>();
    source.forEach((key, value) -> result.put(ContextRegistry.key(key), value));
    return result;
  }

  @Override
  public String toString() {
    return "#space-" + namespace + activeRuntimes();
  }
}
