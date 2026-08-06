package hara.truffle;

import java.io.IOException;
import java.io.InputStream;

/** Resolves the optional packaged foundation HALC without owning namespace transactions. */
final class FoundationHalcLoader {
  private static volatile HalcArtifact.Module cachedModule;

  private FoundationHalcLoader() {}

  static Attempt load(String resourceName) {
    HalcMode mode = HalcMode.current();
    if (mode == HalcMode.OFF || !HalcArtifact.FOUNDATION_RESOURCE.equals(resourceName)) {
      return Attempt.missing();
    }
    HalcArtifact.Module module;
    try (InputStream input =
        FoundationHalcLoader.class
            .getClassLoader()
            .getResourceAsStream(HalcArtifact.FOUNDATION_HALC_RESOURCE)) {
      if (input == null) {
        if (mode == HalcMode.STRICT) {
          throw new HaraException(
              "Strict HALC mode could not find " + HalcArtifact.FOUNDATION_HALC_RESOURCE);
        }
        return Attempt.missing();
      }
      module = cachedModule(input);
      if (!"std.foundation".equals(module.namespace)
          || !resourceName.equals(module.resource)) {
        throw new HaraException(
            "HALC module identity mismatch: "
                + module.namespace
                + " from "
                + module.resource);
      }
    } catch (IOException | RuntimeException error) {
      if (mode == HalcMode.STRICT) {
        if (error instanceof HaraException) throw (HaraException) error;
        throw new HaraException("Unable to load foundation HALC: " + error.getMessage());
      }
      return Attempt.missing();
    }
    // Execution errors must escape even in auto mode so HaraContext can roll back its snapshot.
    return Attempt.loaded(
        HaraLanguage.compileHalc(
                module, "classpath:" + HalcArtifact.FOUNDATION_HALC_RESOURCE)
            .call());
  }

  private static HalcArtifact.Module cachedModule(InputStream input) throws IOException {
    HalcArtifact.Module module = cachedModule;
    if (module != null) return module;
    synchronized (FoundationHalcLoader.class) {
      module = cachedModule;
      if (module == null) {
        module = HalcArtifact.decode(input.readAllBytes());
        cachedModule = module;
      }
      return module;
    }
  }

  static final class Attempt {
    final boolean loaded;
    final Object value;

    private Attempt(boolean loaded, Object value) {
      this.loaded = loaded;
      this.value = value;
    }

    static Attempt missing() {
      return new Attempt(false, null);
    }

    static Attempt loaded(Object value) {
      return new Attempt(true, value);
    }
  }
}
