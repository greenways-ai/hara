package hara.truffle;

import hara.kernel.base.Parser;
import hara.lang.base.G;
import hara.lang.data.Keyword;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import java.io.BufferedInputStream;
import java.io.BufferedOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipInputStream;
import java.util.zip.ZipOutputStream;

/** Deterministic local package commands for the Truffle CLI. */
final class HaraPackageTool {
  private static final Pattern NAMESPACE =
      Pattern.compile("\\(ns\\+?\\s+([a-zA-Z0-9_.-]+)");

  private HaraPackageTool() {}

  static int run(String[] arguments, PrintStream output, PrintStream error) {
    if (arguments.length == 0 || "--help".equals(arguments[0]) || "-h".equals(arguments[0])) {
      usage(output);
      return 0;
    }
    try {
      return switch (arguments[0]) {
        case "check" -> check(path(arguments, 1, Path.of(".")), output);
        case "build" -> build(arguments, output);
        case "inspect" -> inspect(requiredPath(arguments, "inspect"), output);
        case "install" -> install(path(arguments, 1, Path.of(".")), output);
        case "sync", "add", "remove", "update", "publish", "tap", "registry", "search", "info" -> {
          error.println(
              "unavailable: hara package "
                  + arguments[0]
                  + " requires the reviewed registry and identity client");
          yield 2;
        }
        default -> {
          error.println("unknown package command: " + arguments[0]);
          yield 2;
        }
      };
    } catch (HaraException | IOException exception) {
      error.println(exception.getMessage());
      return exception instanceof HaraException ? 1 : 2;
    }
  }

  private static int check(Path input, PrintStream output) {
    HaraProject project = project(input);
    output.println("package check: " + project.name().display() + " " + project.version());
    return 0;
  }

  private static int build(String[] arguments, PrintStream output) throws IOException {
    Path input = path(arguments, 1, Path.of("."));
    HaraProject project = project(input);
    Path destination = option(arguments, "--output");
    if (destination == null)
      destination =
          project
              .root()
              .resolve("target")
              .resolve(
                  project.name().display().replace('/', '-')
                      + "-"
                      + project.version()
                      + ".harp");
    buildArchive(project, destination);
    output.println("package build: " + destination);
    return 0;
  }

  private static int inspect(Path archive, PrintStream output) throws IOException {
    try (ZipFile zip = new ZipFile(archive.toFile(), StandardCharsets.UTF_8)) {
      ZipEntry entry = zip.getEntry("package.edn");
      if (entry == null) throw new HaraException("archive is missing package.edn");
      try (InputStream input = zip.getInputStream(entry)) {
        output.print(new String(input.readAllBytes(), StandardCharsets.UTF_8));
      }
    }
    return 0;
  }

  private static int install(Path input, PrintStream output) throws IOException {
    Path archive = input;
    if (Files.isDirectory(input)) {
      HaraProject project = project(input);
      archive =
          project
              .root()
              .resolve("target")
              .resolve(
                  project.name().display().replace('/', '-')
                      + "-"
                      + project.version()
                      + ".harp");
      buildArchive(project, archive);
    }
    if (!Files.isRegularFile(archive))
      throw new HaraException("package archive does not exist: " + archive);
    String digest = sha256(Files.readAllBytes(archive));
    Path root =
        System.getenv("HARA_DIST_HOME") == null
            ? Path.of(System.getProperty("user.home"), ".hara", "dist")
            : Path.of(System.getenv("HARA_DIST_HOME"));
    Path archiveTarget = root.resolve("archives/sha256/" + digest + ".harp");
    Path packageRoot = root.resolve("roots/sha256/" + digest);
    Files.createDirectories(archiveTarget.getParent());
    Files.createDirectories(packageRoot.getParent());
    if (!Files.exists(archiveTarget))
      Files.copy(archive, archiveTarget, StandardCopyOption.COPY_ATTRIBUTES);
    if (!Files.exists(packageRoot)) {
      Path staging = root.resolve("roots/sha256/." + digest + ".tmp-" + ProcessHandle.current().pid());
      Files.createDirectories(staging);
      try (ZipInputStream zip =
          new ZipInputStream(
              new BufferedInputStream(Files.newInputStream(archiveTarget)),
              StandardCharsets.UTF_8)) {
        ZipEntry entry;
        while ((entry = zip.getNextEntry()) != null) {
          Path relative = Path.of(entry.getName()).normalize();
          if (relative.isAbsolute() || relative.startsWith(".."))
            throw new HaraException("archive contains an unsafe path");
          Path destination = staging.resolve(relative).normalize();
          if (!destination.startsWith(staging))
            throw new HaraException("archive contains an unsafe path");
          if (entry.isDirectory()) Files.createDirectories(destination);
          else {
            Files.createDirectories(destination.getParent());
            try (OutputStream file =
                new BufferedOutputStream(Files.newOutputStream(destination))) {
              zip.transferTo(file);
            }
          }
        }
      }
      Files.move(staging, packageRoot);
    }
    output.println("package install: " + packageRoot);
    return 0;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static void buildArchive(HaraProject project, Path output) throws IOException {
    Object document =
        Parser.LispReader.readString(
            Files.readString(project.descriptor(), StandardCharsets.UTF_8), null);
    IMapType manifest = (IMapType) document;
    TreeMap<String, byte[]> files = new TreeMap<>();
    for (Path source : project.sourcePaths()) collect(project.root(), source, files);
    Object artifacts = manifest.lookup(Keyword.create("project/artifact-paths"));
    if (artifacts instanceof ILinearType values) {
      for (Object value : values) {
        if (!(value instanceof String relative))
          throw new HaraException(":project/artifact-paths must contain strings");
        collect(project.root(), project.root().resolve(relative).normalize(), files);
      }
    }
    files.put(
        "project.edn",
        Files.readAllBytes(project.descriptor()));
    if (files.isEmpty()) throw new HaraException("package build found no declared files");
    byte[] packageEdn = packageManifest(project, files).getBytes(StandardCharsets.UTF_8);
    if (output.getParent() != null) Files.createDirectories(output.getParent());
    try (ZipOutputStream zip =
        new ZipOutputStream(
            new BufferedOutputStream(Files.newOutputStream(output)),
            StandardCharsets.UTF_8)) {
      writeEntry(zip, "package.edn", packageEdn);
      for (Map.Entry<String, byte[]> entry : files.entrySet())
        writeEntry(zip, entry.getKey(), entry.getValue());
    }
  }

  private static void collect(Path projectRoot, Path root, Map<String, byte[]> output)
      throws IOException {
    if (!Files.exists(root)) return;
    try (var paths = Files.walk(root)) {
      for (Path path : paths.filter(Files::isRegularFile).sorted().toList()) {
        Path relative = projectRoot.relativize(path.normalize());
        if (relative.isAbsolute() || relative.startsWith(".."))
          throw new HaraException("package path escapes project root: " + path);
        String name = relative.toString().replace('\\', '/');
        if (output.put(name, Files.readAllBytes(path)) != null)
          throw new HaraException("duplicate package archive path: " + name);
      }
    }
  }

  private static String packageManifest(HaraProject project, Map<String, byte[]> contents) {
    MessageDigest tree = digest();
    StringBuilder files = new StringBuilder();
    TreeMap<String, String> resources = new TreeMap<>();
    ArrayList<String> extensions = new ArrayList<>();
    for (Map.Entry<String, byte[]> entry : contents.entrySet()) {
      String path = entry.getKey();
      byte[] bytes = entry.getValue();
      tree.update(path.getBytes(StandardCharsets.UTF_8));
      tree.update((byte) 0);
      tree.update(bytes);
      files
          .append("  ")
          .append(edn(path))
          .append(" {:sha256 \"sha256:")
          .append(sha256(bytes))
          .append("\" :size ")
          .append(bytes.length)
          .append("}\n");
      if (path.endsWith(".hal")) {
        Matcher matcher = NAMESPACE.matcher(new String(bytes, StandardCharsets.UTF_8));
        if (matcher.find()) {
          String previous = resources.put(matcher.group(1), path);
          if (previous != null)
            throw new HaraException("duplicate package namespace: " + matcher.group(1));
        }
      }
      if (path.endsWith("hara.extension.edn")) extensions.add(path);
    }
    StringBuilder resourceEdn = new StringBuilder();
    resources.forEach(
        (namespace, path) ->
            resourceEdn
                .append("  ")
                .append(edn(namespace))
                .append(" ")
                .append(edn(path))
                .append("\n"));
    extensions.sort(Comparator.naturalOrder());
    return "{:harp/format 1\n :package {:identity "
        + edn(project.name().display())
        + " :version "
        + edn(project.version())
        + "}\n :files {\n"
        + files
        + "} :resources {\n"
        + resourceEdn
        + "} :extensions ["
        + String.join(" ", extensions.stream().map(HaraPackageTool::edn).toList())
        + "]\n :integrity {:tree-sha256 \"sha256:"
        + HexFormat.of().formatHex(tree.digest())
        + "\"}}\n";
  }

  private static void writeEntry(ZipOutputStream zip, String name, byte[] bytes)
      throws IOException {
    ZipEntry entry = new ZipEntry(name);
    entry.setTimeLocal(LocalDateTime.of(1980, 1, 1, 0, 0));
    zip.putNextEntry(entry);
    zip.write(bytes);
    zip.closeEntry();
  }

  private static HaraProject project(Path input) {
    HaraProject project = HaraProject.discover(input);
    if (project == null) throw new HaraException("no project.edn found above " + input);
    project.validateCliProject();
    return project;
  }

  private static Path requiredPath(String[] arguments, String command) {
    if (arguments.length != 2)
      throw new HaraException("hara package " + command + " requires ARCHIVE.harp");
    return Path.of(arguments[1]);
  }

  private static Path path(String[] arguments, int index, Path fallback) {
    if (arguments.length <= index || arguments[index].startsWith("--")) return fallback;
    return Path.of(arguments[index]);
  }

  private static Path option(String[] arguments, String name) {
    for (int index = 0; index < arguments.length; index++) {
      if (name.equals(arguments[index])) {
        if (index + 1 >= arguments.length)
          throw new HaraException(name + " requires a value");
        return Path.of(arguments[index + 1]);
      }
    }
    return null;
  }

  private static String edn(String value) {
    return G.display(value);
  }

  private static MessageDigest digest() {
    try {
      return MessageDigest.getInstance("SHA-256");
    } catch (NoSuchAlgorithmException impossible) {
      throw new IllegalStateException(impossible);
    }
  }

  private static String sha256(byte[] value) {
    return HexFormat.of().formatHex(digest().digest(value));
  }

  private static void usage(PrintStream output) {
    output.println("hara package check [PATH]");
    output.println("hara package build [PATH] [--output PATH]");
    output.println("hara package inspect ARCHIVE.harp");
    output.println("hara package install [PATH|ARCHIVE.harp]");
  }
}
