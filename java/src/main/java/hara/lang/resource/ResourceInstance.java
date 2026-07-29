package hara.lang.resource;

import hara.lang.protocol.IComponent;
import hara.lang.protocol.IMetadata;
import java.util.Map;

public final class ResourceInstance implements IComponent {
  private final String type;
  private final String variant;
  private final Object key;
  private final Object value;
  private final Map<String, Object> config;

  ResourceInstance(String type, String variant, Object key, Object value, Map<String, Object> config) {
    this.type = type; this.variant = variant; this.key = key; this.value = value;
    this.config = config == null ? Map.of() : Map.copyOf(config);
  }

  public String type() { return type; }
  public String variant() { return variant; }
  public Object key() { return key; }
  public Object value() { return value; }
  public Map<String, Object> config() { return config; }

  @Override public IMetadata getProps() {
    return value instanceof IComponent ? ((IComponent) value).getProps() : null;
  }
  @Override public IMetadata getStatus() {
    return value instanceof IComponent ? ((IComponent) value).getStatus() : null;
  }
  @Override public boolean isStarted() {
    return !(value instanceof IComponent) || ((IComponent) value).isStarted();
  }
  @Override public boolean isStopped() {
    return value instanceof IComponent && ((IComponent) value).isStopped();
  }
  @Override public boolean isRemote() {
    return value instanceof IComponent && ((IComponent) value).isRemote();
  }
  @Override public IComponent start() {
    if (value instanceof IComponent) ((IComponent) value).start();
    return this;
  }
  @Override public IComponent stop() {
    if (value instanceof IComponent) ((IComponent) value).stop();
    return this;
  }
  @Override public IComponent kill() {
    if (value instanceof IComponent) ((IComponent) value).kill();
    return this;
  }
  @Override public String toString() { return "#resource[" + type + " " + variant + " " + key + "]"; }
}
