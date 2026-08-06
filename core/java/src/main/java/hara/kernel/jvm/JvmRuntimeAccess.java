package hara.kernel.jvm;

import hara.compiler.DynamicClassLoader;
import hara.kernel.base.RT;
import hara.kernel.flavor.NativeCapability;
import hara.kernel.flavor.NativeFlavorAccess;
import hara.kernel.flavor.NativeFlavorException;
import java.net.URL;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Set;

/** Mutable JVM runtime services, kept separate from evaluator-neutral flavor syntax. */
public final class JvmRuntimeAccess implements NativeFlavorAccess {
  private final RT.Loader loader;
  private final Set<NativeCapability> capabilities;

  public JvmRuntimeAccess(RT.Loader loader, Set<NativeCapability> capabilities) {
    this.loader = loader;
    this.capabilities = Set.copyOf(capabilities);
  }

  @Override
  public ClassLoader classLoader() {
    return loader;
  }

  @Override
  public boolean allows(NativeCapability capability) {
    return capabilities.contains(capability);
  }

  @Override
  public String[] classPath() {
    return Arrays.stream(loader.getURLs()).map(URL::toExternalForm).sorted().toArray(String[]::new);
  }

  @Override
  public String addClassPath(String location) {
    try {
      URL url = toUrl(location);
      loader.addURL(url);
      return url.toExternalForm();
    } catch (Exception error) {
      throw new NativeFlavorException(
          NativeFlavorException.Kind.UNSUPPORTED,
          "Unable to add JVM classpath entry " + location + ": " + error.getMessage(),
          error);
    }
  }

  @Override
  public Class<?> defineClass(byte[] bytecode) {
    return new DynamicClassLoader(loader).defineClass(null, bytecode);
  }

  private static URL toUrl(String location) throws Exception {
    if (location.matches("^[A-Za-z][A-Za-z0-9+.-]*:.*")) {
      return new URL(location);
    }
    return Path.of(location).toAbsolutePath().normalize().toUri().toURL();
  }
}
