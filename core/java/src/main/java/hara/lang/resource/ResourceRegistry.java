package hara.lang.resource;

import hara.lang.protocol.IComponent;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;

/** Thread-safe resource specification and active-instance registry. */
public final class ResourceRegistry {
  private final Map<String, ResourceSpec> specs = new LinkedHashMap<>();
  private final Map<ResourceKey, ResourceInstance> active = new LinkedHashMap<>();

  public synchronized void add(ResourceSpec spec) { specs.put(spec.type(), Objects.requireNonNull(spec)); }
  public synchronized void remove(String type) { specs.remove(type); }
  public synchronized ResourceSpec spec(String type) {
    ResourceSpec spec = specs.get(type);
    if (spec == null) throw new IllegalArgumentException("No resource spec: " + type);
    return spec;
  }
  public synchronized Map<String, ResourceSpec> specs() { return Map.copyOf(specs); }
  public synchronized ResourceInstance get(String type, String variant, Object scope) {
    ResourceSpec spec = spec(type);
    return active.get(new ResourceKey(spec.mode(), type, variantName(variant), key(spec.mode(), scope)));
  }
  public synchronized ResourceInstance start(String type, String variant, Object scope, Map<String, Object> options) {
    ResourceSpec spec = spec(type);
    String variantName = variantName(variant);
    Object key = key(spec.mode(), scope);
    ResourceKey resourceKey = new ResourceKey(spec.mode(), type, variantName, key);
    if (active.containsKey(resourceKey)) throw new IllegalStateException("Resource already started: " + resourceKey);
    ResourceVariant resourceVariant = spec.variant(variantName);
    Map<String, Object> config = new LinkedHashMap<>(spec.config());
    config.putAll(resourceVariant.config());
    if (options != null) config.putAll(options);
    Object value = spec.factory().apply(Collections.unmodifiableMap(config));
    if (value instanceof IComponent) ((IComponent) value).start();
    ResourceInstance instance = new ResourceInstance(type, variantName, key, value, config);
    active.put(resourceKey, instance);
    return instance;
  }
  public synchronized ResourceInstance resolve(String type, String variant, Object scope, Map<String, Object> options) {
    ResourceInstance current = get(type, variant, scope);
    return current == null ? start(type, variant, scope, options) : current;
  }
  public synchronized void stop(String type, String variant, Object scope) {
    ResourceSpec spec = spec(type);
    ResourceKey resourceKey = new ResourceKey(spec.mode(), type, variantName(variant), key(spec.mode(), scope));
    ResourceInstance instance = active.remove(resourceKey);
    if (instance != null && instance.value() instanceof IComponent) ((IComponent) instance.value()).stop();
  }
  public synchronized void restart(String type, String variant, Object scope) {
    ResourceInstance current = get(type, variant, scope);
    Map<String, Object> config = current == null ? Map.of() : current.config();
    stop(type, variant, scope);
    start(type, variant, scope, config);
  }
  public synchronized void stopAll() {
    for (ResourceInstance instance : new ArrayList<>(active.values())) {
      if (instance.value() instanceof IComponent) ((IComponent) instance.value()).stop();
    }
    active.clear();
  }
  public synchronized Map<String, ResourceInstance> active() {
    Map<String, ResourceInstance> result = new LinkedHashMap<>();
    active.forEach((key, value) -> result.put(key.toString(), value));
    return result;
  }
  private static String variantName(String value) { return value == null ? "default" : value; }
  private static Object key(ResourceMode mode, Object scope) { return mode == ResourceMode.GLOBAL ? null : scope; }
  private static final class ResourceKey {
    private final ResourceMode mode;
    private final String type;
    private final String variant;
    private final Object key;

    private ResourceKey(ResourceMode mode, String type, String variant, Object key) {
      this.mode = mode;
      this.type = type;
      this.variant = variant;
      this.key = key;
    }

    @Override
    public boolean equals(Object other) {
      if (!(other instanceof ResourceKey)) return false;
      ResourceKey value = (ResourceKey) other;
      return java.util.Objects.equals(mode, value.mode)
          && java.util.Objects.equals(type, value.type)
          && java.util.Objects.equals(variant, value.variant)
          && java.util.Objects.equals(key, value.key);
    }

    @Override
    public int hashCode() {
      return java.util.Objects.hash(mode, type, variant, key);
    }
  }
}
