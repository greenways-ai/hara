package hara.truffle;

import java.util.ArrayList;
import java.util.List;
import java.util.ServiceLoader;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Discovers optional Java-backed Hara libraries through the application class loader. */
final class HaraLibraryLoader {
  private final Map<String, HaraLibraryProvider> providers = new ConcurrentHashMap<>();
  private final Map<String, Boolean> installed = new ConcurrentHashMap<>();

  HaraLibraryLoader() {
    this(discover());
  }

  HaraLibraryLoader(Iterable<HaraLibraryProvider> discovered) {
    List<HaraLibraryProvider> providers = new ArrayList<>();
    discovered.forEach(providers::add);
    providers.sort(java.util.Comparator.comparingInt(HaraLibraryProvider::order));
    for (HaraLibraryProvider provider : providers) this.providers.put(provider.namespace(), provider);
  }

  private static Iterable<HaraLibraryProvider> discover() {
    return ServiceLoader.load(HaraLibraryProvider.class, HaraContext.class.getClassLoader());
  }

  void ensure(HaraContext context, String namespace) {
    HaraLibraryProvider provider = providers.get(namespace);
    if (provider == null) return;
    installProvider(context, provider);
  }

  private void installProvider(HaraContext context, HaraLibraryProvider provider) {
    String namespace = provider.namespace();
    if (installed.putIfAbsent(namespace, Boolean.TRUE) != null) return;
    try {
      provider.install(context);
    } catch (RuntimeException error) {
      installed.remove(namespace);
      throw error;
    }
  }

  void installEagerJava(HaraContext context) {
    providers.values().stream()
        .filter(HaraLibraryProvider::eager)
        .sorted(java.util.Comparator.comparingInt(HaraLibraryProvider::order))
        .forEach(provider -> installProvider(context, provider));
  }

  void installEagerResources(HaraContext context) {
    providers.values().stream()
        .filter(HaraLibraryProvider::eager)
        .sorted(java.util.Comparator.comparingInt(HaraLibraryProvider::order))
        .forEach(provider -> context.loadLibraryResource(provider.namespace(), false));
  }

  boolean provides(String namespace) {
    return providers.containsKey(namespace);
  }
}
