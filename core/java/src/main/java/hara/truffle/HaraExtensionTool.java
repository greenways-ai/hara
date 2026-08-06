package hara.truffle;

import hara.kernel.base.Parser;
import hara.lang.data.Keyword;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import java.io.IOException;
import java.io.PrintStream;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HexFormat;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;

/** Packaging commands for validated, built Hara extension artifacts. */
final class HaraExtensionTool {
  private static final String DESCRIPTOR = "hara.extension.edn";
  private static final String BUILD_DESCRIPTOR = "hara.build.edn";

  private HaraExtensionTool() {}

  static int run(
      String[] arguments, boolean allowProcess, PrintStream output, PrintStream error) {
    if (arguments.length == 0 || "help".equals(arguments[0]) || "--help".equals(arguments[0])) {
      usage(output);
      return 0;
    }
    try {
      return switch (arguments[0]) {
        case "check" -> checkCommand(arguments, output);
        case "build" -> buildCommand(arguments, allowProcess, output);
        case "install" -> installCommand(arguments, output);
        case "test" -> testCommand(arguments, allowProcess, output);
        default -> {
          error.println("Unknown extension command: " + arguments[0]);
          usage(error);
          yield 2;
        }
      };
    } catch (HaraException | IOException | InterruptedException exception) {
      if (exception instanceof InterruptedException) Thread.currentThread().interrupt();
      error.println(exception.getMessage());
      return 1;
    }
  }

  private static int checkCommand(String[] arguments, PrintStream output) throws IOException {
    Path root = packageArgument(arguments, "check");
    HaraExtensionPackage extensionPackage = load(root);
    output.println(
        "Extension "
            + extensionPackage.manifest().namespace()
            + (extensionPackage.manifest().identity() == null
                ? ""
                : " [" + extensionPackage.manifest().identity() + "]")
            + " "
            + extensionPackage.manifest().version()
            + " is valid ("
            + declaredFiles(extensionPackage.manifest()).size()
            + " files)");
    return 0;
  }

  private static int buildCommand(String[] arguments, boolean allowProcess, PrintStream output)
      throws IOException, InterruptedException {
    Path source = packageArgument(arguments, "build");
    Path buildFile = source.resolve(BUILD_DESCRIPTOR);
    if (!Files.isRegularFile(buildFile)) {
      throw new HaraException("Missing " + buildFile);
    }
    Build build = Build.parse(buildFile);
    Path result;
    if ("prebuilt".equals(build.adapter)) {
      result = source.resolve(build.output).normalize();
    } else if ("command".equals(build.adapter)) {
      if (!allowProcess) {
        throw new HaraException("extension/capability-denied: build requires --allow-process");
      }
      Path workingDirectory = source.resolve(build.workingDirectory).normalize();
      if (!Files.isDirectory(workingDirectory)) {
        throw new HaraException("Extension build directory does not exist: " + workingDirectory);
      }
      Process process =
          new ProcessBuilder(build.command)
              .directory(workingDirectory.toFile())
              .inheritIO()
              .start();
      int status = process.waitFor();
      if (status != 0) throw new HaraException("Extension build failed with status " + status);
      result = source.resolve(build.output).normalize();
    } else {
      throw new HaraException("Unsupported extension build adapter: :" + build.adapter);
    }
    HaraExtensionPackage extensionPackage = load(result);
    output.println("Built " + extensionPackage.manifest().namespace() + " at " + result);
    return 0;
  }

  private static int installCommand(String[] arguments, PrintStream output) throws IOException {
    Path source = packageArgument(arguments, "install");
    HaraExtensionPackage extensionPackage = load(source);
    HaraProject project = HaraProject.discover(Path.of("."));
    if (project == null) throw new HaraException("Cannot install without a project.hal");
    HaraExtensionManifest manifest = extensionPackage.manifest();
    Path extensionRoot = project.extensionRoot();
    Path destination = extensionRoot.resolve(manifest.namespace().replace('.', '/')).normalize();
    if (!destination.startsWith(extensionRoot.toAbsolutePath().normalize())) {
      throw new HaraException("extension/path-denied: " + manifest.namespace());
    }
    if (Files.exists(destination)) {
      throw new HaraException("Extension is already installed: " + destination);
    }
    Files.createDirectories(extensionRoot);
    Path staging = Files.createTempDirectory(extensionRoot, ".hara-install-");
    boolean moved = false;
    try {
      ArrayList<String> lockEntries = new ArrayList<>();
      for (String relative : declaredFiles(manifest)) {
        Path sourceFile = DESCRIPTOR.equals(relative)
            ? source.resolve(DESCRIPTOR)
            : extensionPackage.file(relative);
        Path targetFile = staging.resolve(relative).normalize();
        if (!targetFile.startsWith(staging)) throw new HaraException("extension/path-denied: " + relative);
        Files.createDirectories(targetFile.getParent());
        Files.copy(sourceFile, targetFile, StandardCopyOption.COPY_ATTRIBUTES);
        lockEntries.add("  \"" + relative + "\" \"sha256:" + sha256(targetFile) + "\"");
      }
      String lock =
          "{:namespace \""
              + manifest.namespace()
              + "\"\n"
              + (manifest.identity() == null
                  ? ""
                  : " :identity \"" + manifest.identity() + "\"\n")
              + " :version \""
              + manifest.version()
              + "\"\n :files\n {"
              + String.join("\n", lockEntries)
              + "}}\n";
      Files.writeString(staging.resolve("hara.install.edn"), lock, StandardCharsets.UTF_8);
      try {
        Files.createDirectories(destination.getParent());
        Files.move(staging, destination, StandardCopyOption.ATOMIC_MOVE);
      } catch (AtomicMoveNotSupportedException unsupported) {
        Files.move(staging, destination);
      }
      moved = true;
      load(destination);
      output.println("Installed " + manifest.namespace() + " at " + destination);
      return 0;
    } finally {
      if (!moved) deleteTree(staging);
    }
  }

  private static int testCommand(
      String[] arguments, boolean allowProcess, PrintStream output) throws IOException {
    Path root = packageArgument(arguments, "test");
    HaraExtensionPackage extensionPackage = load(root);
    HaraExtensionManifest manifest = extensionPackage.manifest();
    if ("hta".equals(manifest.provider()) && manifest.target("node") != null) {
      try (HaraProcessExtension runtime =
          new HaraProcessExtension(extensionPackage, allowProcess)) {
        runtime.check();
      }
      output.println("Extension " + manifest.namespace() + " passed the Node HTA handshake");
    } else {
      output.println("Extension " + manifest.namespace() + " passed package validation");
    }
    return 0;
  }

  private static Path packageArgument(String[] arguments, String operation) {
    if (arguments.length != 2) {
      throw new HaraException("extension " + operation + " requires one package directory");
    }
    Path path = Path.of(arguments[1]).toAbsolutePath().normalize();
    if (!Files.isDirectory(path)) throw new HaraException("Not an extension directory: " + path);
    return path;
  }

  private static HaraExtensionPackage load(Path root) throws IOException {
    Path descriptor = root.resolve(DESCRIPTOR);
    if (!Files.isRegularFile(descriptor)) throw new HaraException("Missing " + descriptor);
    String source = Files.readString(descriptor, StandardCharsets.UTF_8);
    HaraExtensionManifest manifest = HaraExtensionManifest.parse(source, descriptor.toString());
    URL location = descriptor.toUri().toURL();
    HaraExtensionPackage extensionPackage = new HaraExtensionPackage(manifest, location);
    extensionPackage.validateDeclaredFiles();
    return extensionPackage;
  }

  private static List<String> declaredFiles(HaraExtensionManifest manifest) {
    LinkedHashSet<String> result = new LinkedHashSet<>();
    result.add(DESCRIPTOR);
    if (manifest.module() != null) result.add(manifest.module());
    manifest.targets().values().forEach(target -> result.add(target.module()));
    result.addAll(manifest.assets());
    return List.copyOf(result);
  }

  private static String sha256(Path file) throws IOException {
    try {
      MessageDigest digest = MessageDigest.getInstance("SHA-256");
      try (var input = Files.newInputStream(file)) {
        byte[] buffer = new byte[8192];
        int count;
        while ((count = input.read(buffer)) >= 0) digest.update(buffer, 0, count);
      }
      return HexFormat.of().formatHex(digest.digest());
    } catch (NoSuchAlgorithmException impossible) {
      throw new IllegalStateException(impossible);
    }
  }

  private static void deleteTree(Path root) throws IOException {
    if (!Files.exists(root)) return;
    try (var paths = Files.walk(root)) {
      for (Path path : paths.sorted(Comparator.reverseOrder()).toList()) Files.deleteIfExists(path);
    }
  }

  private static void usage(PrintStream output) {
    output.println("hara extension check PACKAGE");
    output.println("hara --allow-process extension build SOURCE");
    output.println("hara extension install PACKAGE");
    output.println("hara --allow-process extension test PACKAGE");
  }

  private static final class Build {
    final String adapter;
    final List<String> command;
    final String workingDirectory;
    final String output;

    private Build(String adapter, List<String> command, String workingDirectory, String output) {
      this.adapter = adapter;
      this.command = command;
      this.workingDirectory = workingDirectory;
      this.output = output;
    }

    @SuppressWarnings("rawtypes")
    static Build parse(Path descriptor) throws IOException {
      Object value =
          Parser.LispReader.readString(Files.readString(descriptor, StandardCharsets.UTF_8), null);
      if (!(value instanceof IMapType<?, ?> map)) {
        throw new HaraException("hara.build.edn must be a map");
      }
      Object adapterValue = ((IMapType) map).lookup(Keyword.create("adapter"));
      if (!(adapterValue instanceof Keyword adapter)) {
        throw new HaraException("hara.build.edn requires :adapter");
      }
      String output = string((IMapType) map, "output", true);
      String workingDirectory = string((IMapType) map, "working-directory", false);
      if (workingDirectory == null) workingDirectory = ".";
      ArrayList<String> command = new ArrayList<>();
      Object commandValue = ((IMapType) map).lookup(Keyword.create("command"));
      if (commandValue instanceof ILinearType<?> values) {
        for (Object item : values) {
          if (!(item instanceof String text) || text.isBlank()) {
            throw new HaraException("hara.build.edn :command expects non-empty strings");
          }
          command.add(text);
        }
      }
      if ("command".equals(adapter.getName()) && command.isEmpty()) {
        throw new HaraException("hara.build.edn :command adapter requires :command");
      }
      return new Build(adapter.getName(), List.copyOf(command), workingDirectory, output);
    }

    @SuppressWarnings("rawtypes")
    private static String string(IMapType map, String key, boolean required) {
      Object value = map.lookup(Keyword.create(key));
      if (value == null && !required) return null;
      if (!(value instanceof String text) || text.isBlank()) {
        throw new HaraException("hara.build.edn :" + key + " expects a non-empty string");
      }
      return text;
    }
  }
}
