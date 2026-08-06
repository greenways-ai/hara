package hara.truffle;

import java.io.IOException;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.regex.Pattern;

/** Local public tap trust-store management for the Truffle CLI. */
final class HaraTapTool {
  private static final Pattern SHA256 = Pattern.compile("^(?:sha256:)?[0-9a-f]{64}$");

  private HaraTapTool() {}

  static int run(String[] arguments, PrintStream output, PrintStream error) {
    if (arguments.length == 0 || "--help".equals(arguments[0]) || "-h".equals(arguments[0])) {
      usage(output);
      return 0;
    }
    try {
      return switch (arguments[0]) {
        case "bootstrap" -> bootstrap(arguments, output);
        case "list" -> list(output);
        case "verify", "init", "add", "remove", "mirror" -> {
          error.println(
              "unavailable: hara tap "
                  + arguments[0]
                  + " requires the complete federated Git trust client");
          yield 2;
        }
        default -> {
          error.println("unknown tap command: " + arguments[0]);
          yield 2;
        }
      };
    } catch (HaraException | IOException exception) {
      error.println(exception.getMessage());
      return exception instanceof HaraException ? 1 : 2;
    }
  }

  private static int bootstrap(String[] arguments, PrintStream output) throws IOException {
    String profile = arguments.length > 1 ? arguments[1] : "hara";
    if ("official".equals(profile)) profile = "hara";
    if (!"hara".equals(profile)) throw new HaraException("unknown built-in tap profile: " + profile);
    String fingerprint = System.getenv("HARA_OFFICIAL_ROOT_SHA256");
    if (fingerprint == null || !SHA256.matcher(fingerprint).matches())
      throw new HaraException(
          "official Hara tap root fingerprint is not configured; set HARA_OFFICIAL_ROOT_SHA256");
    if (!fingerprint.startsWith("sha256:")) fingerprint = "sha256:" + fingerprint;
    Path root = configRoot();
    Files.createDirectories(root);
    String document =
        "{:tap-store/format 1\n :taps {\n"
            + "  \"hara\" {:registry [\"https://packages.hara-lang.org\"] "
            + ":identity [\"https://id.hara-lang.org\"] "
            + ":identity-key \""
            + fingerprint
            + "\" :trust :signed-root}\n }}\n";
    Files.writeString(root.resolve("taps.edn"), document, StandardCharsets.UTF_8);
    output.println("tap bootstrap: hara");
    return 0;
  }

  private static int list(PrintStream output) throws IOException {
    Path path = configRoot().resolve("taps.edn");
    if (!Files.exists(path)) {
      output.println("{:tap-store/format 1 :taps {}}");
      return 0;
    }
    output.print(Files.readString(path, StandardCharsets.UTF_8));
    return 0;
  }

  private static Path configRoot() {
    String configured = System.getenv("HARA_CONFIG_HOME");
    if (configured != null) return Path.of(configured);
    String xdg = System.getenv("XDG_CONFIG_HOME");
    if (xdg != null) return Path.of(xdg, "hara");
    return Path.of(System.getProperty("user.home"), ".config", "hara");
  }

  private static void usage(PrintStream output) {
    output.println("hara tap bootstrap [hara]");
    output.println("hara tap <init|add|remove|list|verify>");
    output.println("hara tap mirror <add|remove|list|sync|status>");
  }
}
