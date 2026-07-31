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

  private HaraProject(
      Path root,
      Path descriptor,
      Symbol name,
      String version,
      Symbol main,
      java.util.List<Path> sourcePaths,
      java.util.List<Path> testPaths) {
    this.root = root;
    this.descriptor = descriptor;
    this.name = name;
    this.version = version;
    this.main = main;
    this.sourcePaths = java.util.List.copyOf(sourcePaths);
    this.testPaths = java.util.List.copyOf(testPaths);
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
                PROJECT_FILE));
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
              LEGACY_PROJECT_FILE));
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
}
