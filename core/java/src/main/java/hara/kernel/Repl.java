package hara.kernel;

import hara.kernel.base.*;
import hara.kernel.builtin.BuiltinStruct;
import hara.kernel.base.completion.ClasspathScanner;
import hara.kernel.base.completion.PackageTree;
import hara.lang.base.Ex;
import hara.lang.base.G;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.protocol.IMetadata;
import org.jline.reader.History;
import org.jline.reader.LineReader;
import org.jline.reader.LineReaderBuilder;
import org.jline.reader.Reference;
import org.jline.reader.Widget;
import org.jline.reader.impl.DefaultParser;
import org.jline.terminal.Terminal;
import org.jline.terminal.TerminalBuilder;

import java.io.IOException;

/**
 * Interactive Read-Eval-Print Loop (REPL) for the Hara runtime.
 *
 * <p>Features: - JLine-based line editing with history - Autocomplete for symbols and Java classes
 * - Persistent command history
 */
@SuppressWarnings("rawtypes")
public class Repl {
  private final RT.Instance rt;
  private final Terminal terminal;
  private final LineReader lineReader;
  private final JLineInputReader inputReader;
  private final Reader reader;
  private final ReplConfig config;
  private long lastElapsedNanos = -1L;

  public Repl(RT.Instance rt) throws IOException {
    this(rt, ReplConfig.defaults(".hara_history"));
  }

  public Repl(RT.Instance rt, ReplConfig config) throws IOException {
    NativeMode.requireDisabled("interactive REPL");
    this.rt = rt;

    // Initialize Terminal
    this.terminal = TerminalBuilder.builder().system(true).build();
    this.config = config.withColor(config.color() && !"dumb".equalsIgnoreCase(terminal.getType()));

    // Initialize Completion System
    PackageTree packageTree = new PackageTree();
    ClasspathScanner scanner = new ClasspathScanner(packageTree);
    scanner.scanBackground();

    ReplCompleter completer = new ReplCompleter(rt, scanner, packageTree);

    // Configure LineReader with history and completion
    this.lineReader =
        LineReaderBuilder.builder()
            .terminal(terminal)
            .completer(completer)
            .parser(new LispParser())
            .variable(LineReader.HISTORY_FILE, this.config.historyFile())
            .option(LineReader.Option.HISTORY_INCREMENTAL, true)
            .build();

    // Register "Show Doc" widget
    this.lineReader.getWidgets().put("show-doc", new DocWidget());
    // Bind to Shift+Enter (commonly \033[13;2u in xterm/vscode, or \033OM)
    this.lineReader.getKeyMaps().get(LineReader.MAIN).bind(new Reference("show-doc"), "\033[13;2u");
    this.lineReader.getKeyMaps().get(LineReader.MAIN).bind(new Reference("show-doc"), "\033OM");
    // Bind to Alt+q (Meta-q)
    this.lineReader.getKeyMaps().get(LineReader.MAIN).bind(new Reference("show-doc"), "\033q");
    // Bind to F1
    this.lineReader.getKeyMaps().get(LineReader.MAIN).bind(new Reference("show-doc"), "\033OP");

    this.inputReader =
        new JLineInputReader(
            lineReader,
            () -> this.config.prompt(currentNamespace()),
            this.config::continuationPrompt,
            this::handleCommand);
    this.reader = new Reader(inputReader);
  }

  /** Print the REPL banner with session information. */
  public void printBanner() {
    terminal.writer().println(config.banner("JVM interpreter", rt._key));
    terminal.writer().flush();
  }

  private String currentNamespace() {
    return rt.getCurrentNs() == null ? "user" : rt.getCurrentNs().name.display();
  }

  private void printHelp() {
    var writer = terminal.writer();
    writer.println("Commands:");
    writer.println("  /help        show REPL commands");
    writer.println("  /history     show persistent input history");
    writer.println("  /clear       clear the terminal");
    writer.println("  /splash      show the Hara banner");
    writer.println("  /ns          show the current namespace");
    writer.println("  /quit        exit the REPL");
    writer.println(
        "Tab completes slash commands and symbols; candidates include docs and arglists.");
    writer.flush();
  }

  private JLineInputReader.CommandResult handleCommand(String line) {
    String command = line.strip();
    var writer = terminal.writer();
    if ("/quit".equals(command) || "/exit".equals(command)) {
      return JLineInputReader.CommandResult.EOF;
    } else if ("/help".equals(command)) {
      printHelp();
    } else if (command.startsWith("/history")) {
      String query = command.length() > 8 ? command.substring(8).strip() : "";
      for (History.Entry entry : lineReader.getHistory()) {
        if (query.isEmpty() || entry.line().toLowerCase().contains(query.toLowerCase())) {
          writer.println((entry.index() + 1) + ": " + entry.line());
        }
      }
    } else if ("/clear".equals(command)) {
      lineReader.callWidget(LineReader.CLEAR_SCREEN);
    } else if ("/splash".equals(command)) {
      writer.println(config.banner("JVM interpreter", rt._key));
    } else if ("/ns".equals(command)) {
      writer.println(currentNamespace() + " (" + rt.currentSymbolNames().size() + " symbols)");
    } else if (command.startsWith("/doc ")) {
      printDocumentation(command.substring(5).strip());
    } else if (command.startsWith("/apropos ")) {
      String query = command.substring(9).strip().toLowerCase();
      for (Object value : rt.currentSymbolNames()) {
        String name = String.valueOf(value);
        if (!name.toLowerCase().contains(query)) continue;
        Var variable = rt.getObj(Symbol.create(name));
        if (variable != null && variable.meta() != null) {
          Object doc = Keyword.create("doc").invoke(variable.meta());
          if (doc != null) writer.println(name + " — " + doc);
        }
      }
    } else if ("/time".equals(command)) {
      writer.println(lastElapsedNanos < 0 ? "No evaluation yet." : formatElapsed(lastElapsedNanos));
    } else {
      writer.println("Unknown command: " + command + ". Try /help.");
    }
    writer.flush();
    return JLineInputReader.CommandResult.CONSUMED;
  }

  private String formatElapsed(long nanos) {
    if (nanos < 1_000_000L) return nanos + " ns";
    if (nanos < 1_000_000_000L)
      return String.format(java.util.Locale.ROOT, "%.2f ms", nanos / 1_000_000.0);
    return String.format(java.util.Locale.ROOT, "%.2f s", nanos / 1_000_000_000.0);
  }

  private void printDocumentation(String name) {
    Var variable = rt.getObj(Symbol.create(name));
    var writer = terminal.writer();
    if (variable == null || variable.meta() == null) {
      writer.println("No documentation for " + name);
      return;
    }
    Object arglists = Keyword.create("arglists").invoke(variable.meta());
    Object doc = Keyword.create("doc").invoke(variable.meta());
    writer.println("Documentation: " + name);
    if (arglists != null) writer.println("  Arglists: " + G.display(arglists));
    if (doc != null) writer.println("  " + doc);
  }

  /**
   * Start the REPL loop. Reads forms, evaluates them, and prints results until EOF is encountered.
   */
  public void run() {
    lineReader.callWidget(LineReader.CLEAR_SCREEN);
    printBanner();
    printHelp();

    // Use hara.lang.data.Map from BuiltinStruct.hashMap
    hara.lang.data.Map opts = BuiltinStruct.hashMap(new Object[] {});
    Object EOF_SENTINEL = new Object();

    while (true) {
      try {
        inputReader.resetPrompt();
        Object form = Parser.LispReader.read(reader, false, EOF_SENTINEL, false, opts);

        if (form == EOF_SENTINEL) break;

        long started = System.nanoTime();
        Object res = rt.eval(form);
        lastElapsedNanos = System.nanoTime() - started;
        terminal.writer().println(G.display(res));
        terminal.writer().println("[" + formatElapsed(lastElapsedNanos) + "]");
        terminal.writer().flush();
      } catch (Throwable t) {
        Throwable cause = t;
        while (cause instanceof java.lang.reflect.InvocationTargetException
            || cause instanceof java.util.concurrent.ExecutionException
            || (cause instanceof RuntimeException
                && cause.getCause() != null
                && cause.getMessage() == null)) {
          if (cause.getCause() != null) {
            cause = cause.getCause();
          } else {
            break;
          }
        }

        if (cause instanceof Ex.Runtime
            || cause instanceof Ex.Unsupported
            || cause instanceof Ex.Arity) {
          terminal.writer().println("Error: " + cause.getMessage());
          terminal.writer().flush();
        } else {
          t.printStackTrace();
        }
      }
    }
  }

  public static class LispParser extends DefaultParser {
    @Override
    public boolean isDelimiterChar(CharSequence buffer, int pos) {
      char c = buffer.charAt(pos);
      return c == '('
          || c == ')'
          || c == '['
          || c == ']'
          || c == '{'
          || c == '}'
          || Character.isWhitespace(c);
    }
  }

  /** Widget to display documentation and arguments for the current symbol. */
  public class DocWidget implements Widget {
    @Override
    public boolean apply() {
      var buffer = lineReader.getBuffer().toString();
      var cursor = lineReader.getBuffer().cursor();
      var word = ReplCompleter.extractWord(buffer, cursor);

      if (word == null || word.isEmpty()) return true;

      try {
        Symbol sym = Symbol.create(word);
        // Look up Var in the environment
        Var v = rt.getObj(sym);
        if (v != null) {
          IMetadata meta = v.meta();
          if (meta != null) {
            Object doc = Keyword.create("doc").invoke(meta);
            Object arglists = Keyword.create("arglists").invoke(meta);

            if (doc != null || arglists != null) {
              // Print info below the prompt
              lineReader.callWidget(LineReader.CLEAR);

              // We use the terminal writer directly to print above the prompt
              var writer = lineReader.getTerminal().writer();
              writer.println();

              if (arglists != null) {
                writer.println("Arglists: " + G.display(arglists));
              }
              if (doc != null) {
                writer.println("Doc: " + doc);
              }
              // Force redraw to restore prompt and buffer
              lineReader.callWidget(LineReader.REDRAW_LINE);
              lineReader.callWidget(LineReader.REDISPLAY);
            }
          }
        }
      } catch (Throwable t) {
        // ignore
      }
      return true;
    }
  }
}
