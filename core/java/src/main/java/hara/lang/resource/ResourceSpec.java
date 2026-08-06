package hara.lang.resource;

import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;
import java.util.function.Function;

public final class ResourceSpec {
  private final String type;
  private final ResourceMode mode;
  private final Map<String, Object> config;
  private final Map<String, ResourceVariant> variants = new LinkedHashMap<>();
  private final Function<Map<String, Object>, Object> factory;

  public ResourceSpec(
      String type,
      ResourceMode mode,
      Map<String, Object> config,
      Function<Map<String, Object>, Object> factory) {
    this.type = Objects.requireNonNull(type, "type");
    this.mode = mode == null ? ResourceMode.GLOBAL : mode;
    this.config = config == null ? Collections.emptyMap() : Map.copyOf(config);
    this.factory = Objects.requireNonNull(factory, "factory");
    variants.put("default", new ResourceVariant("default", Collections.emptyMap()));
  }

  public String type() {
    return type;
  }

  public ResourceMode mode() {
    return mode;
  }

  public Map<String, Object> config() {
    return config;
  }

  public Function<Map<String, Object>, Object> factory() {
    return factory;
  }

  public synchronized void variant(ResourceVariant value) {
    variants.put(value.id(), value);
  }

  public synchronized ResourceVariant variant(String id) {
    ResourceVariant value = variants.get(id == null ? "default" : id);
    if (value == null) throw new IllegalArgumentException("Unknown resource variant: " + id);
    return value;
  }

  public synchronized Map<String, ResourceVariant> variants() {
    return Map.copyOf(variants);
  }
}
