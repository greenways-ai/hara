package hara.truffle;

import java.io.IOException;
import java.io.InputStream;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Enumeration;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/** Discovers extension manifests from the classpath and configured project roots. */
final class HaraExtensionRegistry {
  private static final int MAX_MANIFEST_BYTES = 1024 * 1024;
  private final ClassLoader classLoader;
  private final List<Path> roots;
  private final Map<String, HaraExtensionPackage> packages = new ConcurrentHashMap<>();

  HaraExtensionRegistry(ClassLoader classLoader) {
    this(classLoader, configuredRoots());
  }

  HaraExtensionRegistry(ClassLoader classLoader, List<Path> roots) {
    this.classLoader = classLoader;
    this.roots = Collections.unmodifiableList(new ArrayList<>(roots));
  }

  HaraExtensionPackage discover(String namespace) {
    return discover(namespace, null);
  }

  HaraExtensionPackage discover(String namespace, Path projectRoot) {
    HaraExtensionPackage cached = packages.get(namespace);
    if (cached != null) return cached;
    String resource = resourceName(namespace);
    try {
      ArrayList<URL> candidates = new ArrayList<>();
      Enumeration<URL> urls = classLoader.getResources(resource);
      while (urls.hasMoreElements()) candidates.add(urls.nextElement());
      Path relative = Path.of(namespace.replace('.', '/'), "hara.extension.edn");
      if (projectRoot != null) addCandidate(candidates, projectRoot, relative, namespace);
      for (Path root : roots) addCandidate(candidates, root, relative, namespace);
      if (candidates.isEmpty()) return null;
      if (candidates.size() > 1) {
        throw new HaraException(
            "extension/ambiguous: multiple packages export " + namespace + ": " + candidates);
      }
      URL location = candidates.get(0);
      byte[] bytes;
      try (InputStream input = location.openStream()) {
        bytes = input.readNBytes(MAX_MANIFEST_BYTES + 1);
      }
      if (bytes.length > MAX_MANIFEST_BYTES) {
        throw new HaraException("extension/malformed " + location + ": manifest is too large");
      }
      HaraExtensionManifest manifest =
          HaraExtensionManifest.parse(
              new String(bytes, StandardCharsets.UTF_8), location.toString());
      if (!manifest.namespace().equals(namespace)) {
        throw new HaraException(
            "extension/malformed " + location + ": expected namespace " + namespace);
      }
      HaraExtensionPackage extensionPackage = new HaraExtensionPackage(manifest, location);
      extensionPackage.validateDeclaredFiles();
      packages.put(namespace, extensionPackage);
      return extensionPackage;
    } catch (IOException | IllegalArgumentException error) {
      if (error instanceof HaraException) throw (HaraException) error;
      throw new HaraException(error.getMessage());
    }
  }

  private static void addCandidate(
      List<URL> candidates, Path root, Path relative, String namespace) throws IOException {
    Path normalizedRoot = root.toAbsolutePath().normalize();
    Path descriptor = normalizedRoot.resolve(relative).normalize();
    if (!descriptor.startsWith(normalizedRoot)) {
      throw new HaraException("extension/path-denied: " + namespace);
    }
    if (Files.isRegularFile(descriptor)) {
      URL candidate = descriptor.toUri().toURL();
      if (!candidates.contains(candidate)) candidates.add(candidate);
    }
  }

  static String resourceName(String namespace) {
    return "META-INF/hara/extensions/"
        + namespace.replace('.', '/')
        + "/hara.extension.edn";
  }

  private static List<Path> configuredRoots() {
    ArrayList<Path> roots = new ArrayList<>();
    HaraProject project = HaraProject.discover(Path.of("."));
    if (project != null) roots.add(project.extensionRoot().toAbsolutePath().normalize());
    String configured = System.getProperty("hara.extensions.path", "");
    if (configured.isBlank()) configured = System.getenv().getOrDefault("HARA_EXTENSION_PATH", "");
    if (configured.isBlank()) return roots;
    for (String value : configured.split(java.io.File.pathSeparator)) {
      if (!value.isBlank()) roots.add(Path.of(value).toAbsolutePath().normalize());
    }
    return roots;
  }
}
