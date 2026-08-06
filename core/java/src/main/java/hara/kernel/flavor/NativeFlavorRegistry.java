package hara.kernel.flavor;

import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Provider registry installed explicitly by an embedding runtime. */
public final class NativeFlavorRegistry {
  private final Map<String, NativeFlavorProvider> providers = new ConcurrentHashMap<>();

  public NativeFlavorRegistry register(NativeFlavorProvider provider) {
    NativeFlavorProvider previous = providers.putIfAbsent(provider.name(), provider);
    if (previous != null && previous != provider) {
      throw new IllegalArgumentException("Native flavor already registered: " + provider.name());
    }
    return this;
  }

  public NativeFlavorProvider find(String name) {
    return providers.get(name);
  }

  public NativeFlavorProvider require(String name) {
    NativeFlavorProvider provider = find(name);
    if (provider == null) {
      throw new NativeFlavorException(
          NativeFlavorException.Kind.UNRESOLVED, "Unknown native flavor: " + name);
    }
    return provider;
  }
}
