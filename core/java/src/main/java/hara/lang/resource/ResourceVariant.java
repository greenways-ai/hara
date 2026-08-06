package hara.lang.resource;

import java.util.Collections;
import java.util.Map;

public final class ResourceVariant {
  private final String id;
  private final Map<String, Object> config;

  public ResourceVariant(String id, Map<String, Object> config) {
    this.id = id == null ? "default" : id;
    this.config = config == null ? Collections.emptyMap() : Map.copyOf(config);
  }

  public String id() {
    return id;
  }

  public Map<String, Object> config() {
    return config;
  }
}
