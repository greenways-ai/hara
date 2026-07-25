package hara.truffle;

import java.io.IOException;
import java.io.InputStream;

/** Resolves the optional packaged foundation HIR without owning namespace transactions. */
final class FoundationHirLoader {
  private static volatile HirArtifact.Module cachedModule;

  private FoundationHirLoader() {}

  static Attempt load(String resourceName) {
    HirMode mode = HirMode.current();
    if (mode == HirMode.OFF || !HirArtifact.FOUNDATION_RESOURCE.equals(resourceName)) {
      return Attempt.missing();
    }
    HirArtifact.Module module;
    try (InputStream input =
        FoundationHirLoader.class
            .getClassLoader()
            .getResourceAsStream(HirArtifact.FOUNDATION_HIR_RESOURCE)) {
      if (input == null) {
        if (mode == HirMode.STRICT) {
          throw new HaraException(
              "Strict HIR mode could not find " + HirArtifact.FOUNDATION_HIR_RESOURCE);
        }
        return Attempt.missing();
      }
      module = cachedModule(input);
      if (!"std.foundation".equals(module.namespace)
          || !resourceName.equals(module.resource)) {
        throw new HaraException(
            "HIR module identity mismatch: "
                + module.namespace
                + " from "
                + module.resource);
      }
    } catch (IOException | RuntimeException error) {
      if (mode == HirMode.STRICT) {
        if (error instanceof HaraException) throw (HaraException) error;
        throw new HaraException("Unable to load foundation HIR: " + error.getMessage());
      }
      return Attempt.missing();
    }
    // Execution errors must escape even in auto mode so HaraContext can roll back its snapshot.
    return Attempt.loaded(
        HaraLanguage.compileHir(
                module.forms, "classpath:" + HirArtifact.FOUNDATION_HIR_RESOURCE)
            .call());
  }

  private static HirArtifact.Module cachedModule(InputStream input) throws IOException {
    HirArtifact.Module module = cachedModule;
    if (module != null) return module;
    synchronized (FoundationHirLoader.class) {
      module = cachedModule;
      if (module == null) {
        module = HirArtifact.decode(input.readAllBytes());
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
