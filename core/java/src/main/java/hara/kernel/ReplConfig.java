package hara.kernel;

import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Objects;

/** Visual and persistence settings shared by Hara's interactive REPLs. */
public final class ReplConfig {
  public static final String DEFAULT_SPLASH =
      """


                               ░░░▒▒▓▒▒░░░
                          ░░░░░▒▒▒▒▒▓▒▒▒▒▒░░░░░
                     ░░░░░▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒░░░░░
                ░░░░░▒▒▒▒▒▒▒▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▒▒▒▒▒▒▒░░░░░

          ██╗       ██╗   ███████╗    ███████╗     ███████╗
          ██║       ██║  ██╔═════██╗  ██╔════██╗   ██╔═════██╗
          ██║  ●────██║  ██║     ██║  ██║     ██║  ██║     ██║
          ████████████║  ███████████║  ████████╔╝   ███████████║
          ██╔═══════██║  ██╔══════██║  ██╔═══██╗    ██╔══════██║
          ██║ ──●── ██║  ██║  ●───██║  ██║    ██╗   ██║  ●───██║
          ██║       ██║  ██║      ██║  ██║     ██╗  ██║      ██║
          ╚═╝       ╚═╝  ╚═╝      ╚═╝  ╚═╝     ╚═╝  ╚═╝      ╚═╝
                ·───────●───────────────●───────────────·
      """;


  private final Path historyFile;
  private final String splash;
  private final boolean color;

  public ReplConfig(Path historyFile, String splash, boolean color) {
    this.historyFile = Objects.requireNonNull(historyFile, "historyFile");
    this.splash = Objects.requireNonNull(splash, "splash");
    this.color = color;
  }

  public static ReplConfig defaults(String historyName) {
    String configuredSplash = System.getProperty("hara.repl.splash");
    String splash =
        configuredSplash == null ? DEFAULT_SPLASH : configuredSplash.replace("\\n", "\n");
    boolean color = !Boolean.getBoolean("hara.repl.no-color") && System.getenv("NO_COLOR") == null;
    return new ReplConfig(Paths.get(System.getProperty("user.home"), historyName), splash, color);
  }

  public Path historyFile() {
    return historyFile;
  }

  public String splash() {
    return splash;
  }

  public String renderedSplash() {
    String value = splash.stripTrailing();
    if (!color || value.isBlank()) return value;
    String[] lines = value.split("\\R", -1);
    StringBuilder rendered = new StringBuilder();
    boolean defaultSplash = DEFAULT_SPLASH.equals(splash);
    int triangleStart = defaultSplash ? 2 : 0;
    int wordStart = defaultSplash ? 7 : Math.min(2, lines.length - 1);
    int wordGradientLength = Math.max(1, lines.length - wordStart - 1);
    for (int index = 0; index < lines.length; index++) {
      if (index > 0) rendered.append('\n');
      if (defaultSplash && (index < triangleStart || index == 6)) {
        rendered.append(lines[index]);
        continue;
      }
      double position;
      int[][] stops;
      if (defaultSplash && index < wordStart) {
        position = (index - triangleStart) / 3.0;
        stops =
            new int[][] {
              {255, 246, 150},
              {235, 246, 185},
              {170, 226, 230},
              {85, 170, 255}
            };
      } else {
        position = (index - wordStart) / (double) wordGradientLength;
        stops =
            new int[][] {
              {105, 245, 255},
              {35, 185, 255},
              {45, 105, 255},
              {105, 65, 235},
              {185, 65, 220},
              {70, 20, 100},
              {5, 8, 20}
            };
      }
      int[] color = gradient(position, stops);
      rendered
          .append("\u001b[38;2;")
          .append(color[0])
          .append(';')
          .append(color[1])
          .append(';')
          .append(color[2])
          .append('m')
          .append(lines[index])
          .append("\u001b[0m");
    }
    return rendered.toString();
  }

  private static int blend(int from, int to, double position) {
    return (int) Math.round(from + (to - from) * position);
  }

  private static int[] gradient(double position, int[][] stops) {
    double scaled = Math.max(0, Math.min(1, position)) * (stops.length - 1);
    int from = Math.min((int) scaled, stops.length - 2);
    double phase = scaled - from;
    return new int[] {
      blend(stops[from][0], stops[from + 1][0], phase),
      blend(stops[from][1], stops[from + 1][1], phase),
      blend(stops[from][2], stops[from + 1][2], phase)
    };
  }

  public boolean color() {
    return color;
  }

  public ReplConfig withHistoryFile(Path value) {
    return new ReplConfig(value, splash, color);
  }

  public ReplConfig withSplash(String value) {
    return new ReplConfig(historyFile, value, color);
  }

  public ReplConfig withColor(boolean value) {
    return new ReplConfig(historyFile, splash, value);
  }

  public String banner(String runtime, String session) {
    StringBuilder out = new StringBuilder();
    if (!splash.isBlank()) {
      out.append(renderedSplash()).append("\n\n\n");
    }
    out.append(paint("35;1", "Hara"))
        .append(" · ")
        .append(runtime)
        .append(" · session ")
        .append(session)
        .append('\n')
        .append(paint("2", "Type /help for commands."));
    return out.toString();
  }

  public String tagline(String text) {
    if (!color || text.isEmpty()) return text;
    int[][] stops = {
      {100, 245, 255}, {45, 145, 255}, {125, 75, 235}, {220, 90, 205}
    };
    StringBuilder out = new StringBuilder();
    for (int index = 0; index < text.length(); index++) {
      char character = text.charAt(index);
      if (Character.isWhitespace(character)) {
        out.append(character);
        continue;
      }
      int[] shade = gradient(index / (double) Math.max(1, text.length() - 1), stops);
      out.append("\u001b[38;2;")
          .append(shade[0])
          .append(';')
          .append(shade[1])
          .append(';')
          .append(shade[2])
          .append('m')
          .append(character);
    }
    return out.append("\u001b[0m").toString();
  }

  public String prompt(String namespace) {
    return paint("36;1", "hara") + paint("2", "[" + namespace + "] ");
  }

  public String sessionPrompt(String namespace) {
    return paint("2", "[") + paint("36;1", namespace) + paint("2", "] ");
  }

  public String continuationPrompt() {
    return paint("2", "       … ");
  }

  private String paint(String code, String text) {
    return color ? "\u001b[" + code + "m" + text + "\u001b[0m" : text;
  }
}
