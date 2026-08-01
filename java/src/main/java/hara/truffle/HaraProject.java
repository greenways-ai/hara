package hara.truffle;

import hara.kernel.base.Parser;
import hara.lang.data.Keyword;
import hara.lang.data.List;
import hara.lang.data.Symbol;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashSet;
import java.util.Set;

/** Discovers project.edn (or legacy project.hal) and resolves namespace paths. */
final class HaraProject {
  private static final String PROJECT_FILE = "project.edn";
  private static final String LEGACY_PROJECT_FILE = "project.hal";

  private final Path root;
  private final Path descriptor;
  private final Symbol name;
  private final String version;
  private final Symbol main;
  private final java.util.List<Path> sourcePaths;
  private final java.util.List<Path> testPaths;
  private final java.util.List<JvmDependency> jvmDependencies;
  private final java.util.List<Path> jvmSourcePaths;
  private final Path jvmTargetPath;
  private final Set<String> capabilities;

  record JvmDependency(String id, String version) {
    String coordinate() {
      return id.replace('/', ':') + ":" + version;
    }
  }

  private HaraProject(
      Path root,
      Path descriptor,
      Symbol name,
      String version,
      Symbol main,
      java.util.List<Path> sourcePaths,
      java.util.List<Path> testPaths,
      java.util.List<JvmDependency> jvmDependencies,
      java.util.List<Path> jvmSourcePaths,
      Path jvmTargetPath,
      Set<String> capabilities) {
    this.root = root;
    this.descriptor = descriptor;
    this.name = name;
    this.version = version;
    this.main = main;
    this.sourcePaths = java.util.List.copyOf(sourcePaths);
    this.testPaths = java.util.List.copyOf(testPaths);
    this.jvmDependencies = java.util.List.copyOf(jvmDependencies);
    this.jvmSourcePaths = java.util.List.copyOf(jvmSourcePaths);
    this.jvmTargetPath = jvmTargetPath;
    this.capabilities = Set.copyOf(capabilities);
  }

  static HaraProject discover(Path start) {
    Path current = start.toAbsolutePath().normalize();
    while (current != null) {
      Path descriptor = current.resolve(PROJECT_FILE);
      if (Files.isRegularFile(descriptor)) return read(descriptor);
      descriptor = current.resolve(LEGACY_PROJECT_FILE);
      if (Files.isRegularFile(descriptor)) return read(descriptor);
      current = current.getParent();
    }
    return null;
  }

  static HaraProject read(Path descriptor) {
    try {
      Object form =
          Parser.LispReader.readString(
              Files.readString(descriptor, StandardCharsets.UTF_8), null);
      if (PROJECT_FILE.equals(descriptor.getFileName().toString())) {
        if (!(form instanceof IMapType<?, ?> options)
            || !(lookup(options, "project/id") instanceof Symbol projectName)) {
          throw new HaraException("project.edn expects a map with :project/id");
        }
        Path root = descriptor.toAbsolutePath().normalize().getParent();
        return new HaraProject(
            root,
            descriptor,
            projectName,
            lookup(options, "project/version") instanceof String value ? value : null,
            lookup(options, "project/main") instanceof Symbol value ? value : null,
            paths(
                root,
                lookup(options, "project/source-paths"),
                "project/source-paths",
                java.util.List.of("src"),
                PROJECT_FILE),
            paths(
                root,
                lookup(options, "project/test-paths"),
                "project/test-paths",
                java.util.List.of("test"),
                PROJECT_FILE),
            jvmDependencies(lookup(options, "jvm/dependencies"), PROJECT_FILE),
            paths(
                root,
                lookup(options, "jvm/source-paths"),
                "jvm/source-paths",
                java.util.List.of("src-java"),
                PROJECT_FILE),
            path(
                root,
                lookup(options, "jvm/target-path"),
                "jvm/target-path",
                "target/classes",
                PROJECT_FILE),
            capabilities(lookup(options, "project/capabilities"), PROJECT_FILE));
      }
      if (!(form instanceof List<?> list)
          || list.count() != 3
          || !Symbol.create("defproject").equals(list.nth(0))
          || !(list.nth(1) instanceof Symbol projectName)
          || projectName.getNamespace() != null
          || !(list.nth(2) instanceof IMapType<?, ?> options)) {
        throw new HaraException(
            "project.hal expects (defproject unqualified-name options-map)");
      }
      Path root = descriptor.toAbsolutePath().normalize().getParent();
      return new HaraProject(
          root,
          descriptor,
          projectName,
          null,
          null,
          paths(
              root,
              lookup(options, "source-paths"),
              "source-paths",
              java.util.List.of("src"),
              LEGACY_PROJECT_FILE),
          paths(
              root,
              lookup(options, "test-paths"),
              "test-paths",
              java.util.List.of("test"),
              LEGACY_PROJECT_FILE),
          java.util.List.of(),
          java.util.List.of(),
          root.resolve("target/classes"),
          Set.of());
    } catch (IOException error) {
      throw new HaraException(
          "Unable to read project descriptor " + descriptor + ": " + error.getMessage());
    }
  }

  Path resolve(String namespace, boolean includeTests) {
    String relative = namespace.replace('.', '/').replace('-', '_') + ".hal";
    for (Path sourcePath : sourcePaths) {
      Path candidate = sourcePath.resolve(relative).normalize();
      if (candidate.startsWith(root) && Files.isRegularFile(candidate)) return candidate;
    }
    if (includeTests) {
      for (Path testPath : testPaths) {
        Path candidate = testPath.resolve(relative).normalize();
        if (candidate.startsWith(root) && Files.isRegularFile(candidate)) return candidate;
      }
    }
    return null;
  }

  Symbol name() {
    return name;
  }

  Path descriptor() {
    return descriptor;
  }

  String version() {
    return version;
  }

  Symbol main() {
    return main;
  }

  void validateCliProject() {
    if (!PROJECT_FILE.equals(descriptor.getFileName().toString()))
      throw new HaraException("project CLI requires project.edn");
    try {
      Object form = Parser.LispReader.readString(Files.readString(descriptor, StandardCharsets.UTF_8), null);
      if (!(form instanceof IMapType<?, ?> options)
          || !(lookup(options, "hara/type") instanceof Keyword type)
          || !"project".equals(type.getName()))
        throw new HaraException("project.edn :hara/type must be :project");
      for (String key :
          java.util.List.of(
              "hara/version",
              "project/version",
              "project/source-paths",
              "project/test-paths",
              "project/extension-paths",
              "project/capabilities")) {
        if (lookup(options, key) == null) throw new HaraException("project.edn missing required key :" + key);
      }
      if (!(lookup(options, "hara/version") instanceof String))
        throw new HaraException("project.edn :hara/version must be a string");
      if (!(lookup(options, "project/version") instanceof String version)
          || !version.matches(
              "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(?:[-+][0-9A-Za-z.-]+)?$"))
        throw new HaraException("project.edn :project/version is not SemVer");
      Object dependencies = lookup(options, "project/dependencies");
      if (dependencies != null && !(dependencies instanceof IMapType<?, ?>))
        throw new HaraException("project.edn :project/dependencies must be a map");
      paths(
          root,
          lookup(options, "project/artifact-paths"),
          "project/artifact-paths",
          java.util.List.of(),
          PROJECT_FILE);
    } catch (IOException error) {
      throw new HaraException("Unable to read project descriptor " + descriptor + ": " + error.getMessage());
    }
  }

  Path mainFile() {
    if (main == null) throw new HaraException("project.edn is missing :project/main");
    Path source = resolve(main.display(), false);
    if (source == null)
      throw new HaraException("cannot find :project/main " + main.display() + " in :project/source-paths");
    return source;
  }

  Path root() {
    return root;
  }

  java.util.List<Path> sourcePaths() {
    return sourcePaths;
  }

  java.util.List<Path> testPaths() {
    return testPaths;
  }

  java.util.List<JvmDependency> jvmDependencies() {
    return jvmDependencies;
  }

  java.util.List<Path> jvmSourcePaths() {
    return jvmSourcePaths;
  }

  Path jvmTargetPath() {
    return jvmTargetPath;
  }

  boolean hasCapability(String capability) {
    return capabilities.contains(capability);
  }

  Path extensionRoot() {
    return root.resolve("extensions");
  }

  @SuppressWarnings("rawtypes")
  private static Object lookup(IMapType<?, ?> map, String key) {
    return ((IMapType) map).lookup(Keyword.create(key));
  }

  private static java.util.List<Path> paths(
      Path root,
      Object value,
      String option,
      java.util.List<String> defaults,
      String descriptor) {
    Iterable<?> entries;
    if (value == null) {
      entries = defaults;
    } else if (value instanceof ILinearType<?>) {
      entries = (ILinearType<?>) value;
    } else {
      throw new HaraException(descriptor + " :" + option + " expects a sequential collection");
    }
    ArrayList<Path> paths = new ArrayList<>();
    for (Object entry : entries) {
      if (!(entry instanceof String) || ((String) entry).isBlank()) {
        throw new HaraException(descriptor + " :" + option + " expects non-empty path strings");
      }
      Path path = root.resolve((String) entry).normalize();
      if (!path.startsWith(root)) {
        throw new HaraException(descriptor + " :" + option + " cannot escape the project root");
      }
      paths.add(path);
    }
    return Collections.unmodifiableList(paths);
  }

  private static Path path(
      Path root, Object value, String option, String defaultValue, String descriptor) {
    Object selected = value == null ? defaultValue : value;
    if (!(selected instanceof String entry) || entry.isBlank()) {
      throw new HaraException(descriptor + " :" + option + " expects a non-empty path string");
    }
    Path path = root.resolve(entry).normalize();
    if (!path.startsWith(root)) {
      throw new HaraException(descriptor + " :" + option + " cannot escape the project root");
    }
    return path;
  }

  private static java.util.List<JvmDependency> jvmDependencies(Object value, String descriptor) {
    if (value == null) return java.util.List.of();
    if (!(value instanceof ILinearType<?> entries)) {
      throw new HaraException(descriptor + " :jvm/dependencies expects a vector");
    }
    ArrayList<JvmDependency> dependencies = new ArrayList<>();
    LinkedHashSet<String> ids = new LinkedHashSet<>();
    for (Object valueEntry : entries) {
      if (!(valueEntry instanceof ILinearType<?> entry) || entry.count() != 2) {
        throw new HaraException(
            descriptor + " :jvm/dependencies entries must be [group/artifact \"version\"]");
      }
      Object idValue = entry.nth(0);
      String id;
      if (idValue instanceof Symbol symbol) {
        id = symbol.display();
      } else if (idValue instanceof String text) {
        id = text.replace(':', '/');
      } else {
        throw new HaraException(
            descriptor + " :jvm/dependencies coordinates must be symbols or strings");
      }
      if (!id.matches("[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")) {
        throw new HaraException(descriptor + " invalid JVM dependency coordinate " + id);
      }
      if (!(entry.nth(1) instanceof String version)
          || !version.matches("[A-Za-z0-9][A-Za-z0-9._+-]*")) {
        throw new HaraException(
            descriptor + " JVM dependency " + id + " requires an exact Maven version");
      }
      if (!ids.add(id)) {
        throw new HaraException(descriptor + " duplicate JVM dependency " + id);
      }
      dependencies.add(new JvmDependency(id, version));
    }
    return java.util.List.copyOf(dependencies);
  }

  private static Set<String> capabilities(Object value, String descriptor) {
    if (value == null) return Set.of();
    if (!(value instanceof Iterable<?> entries)) {
      throw new HaraException(descriptor + " :project/capabilities expects a collection");
    }
    LinkedHashSet<String> capabilities = new LinkedHashSet<>();
    for (Object entry : entries) {
      if (!(entry instanceof Keyword capability)) {
        throw new HaraException(descriptor + " :project/capabilities expects keywords");
      }
      String name =
          capability.getNamespace() == null
              ? capability.getName()
              : capability.getNamespace() + "/" + capability.getName();
      capabilities.add(name);
    }
    return Set.copyOf(capabilities);
  }
}
