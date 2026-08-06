package hara.kernel;

import hara.kernel.base.RT;
import hara.kernel.base.completion.ClasspathScanner;
import hara.kernel.base.completion.PackageTree;
import hara.lang.base.G;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.kernel.base.Var;
import org.jline.reader.Candidate;
import org.jline.reader.Completer;
import org.jline.reader.LineReader;
import org.jline.reader.ParsedLine;

import java.util.List;

/**
 * JLine completer for the Hara REPL that provides: 1. Symbol completion from the runtime
 * environment 2. Java package and class completion
 *
 * <p>Handles Lisp-style syntax including completion inside parentheses.
 */
public class ReplCompleter implements Completer {
  private final RT.Instance<?> rt;
  private final ClasspathScanner scanner;
  private final PackageTree packageTree;

  public ReplCompleter(RT.Instance<?> rt, ClasspathScanner scanner, PackageTree packageTree) {
    this.rt = rt;
    this.scanner = scanner;
    this.packageTree = packageTree;
  }

  @Override
  public void complete(LineReader reader, ParsedLine line, List<Candidate> candidates) {
    // Update classpath from RT before completion
    scanner.scan(rt.classLoader().getURLs());

    // Extract word using Lisp-aware logic
    String word = extractWord(line.line(), line.cursor());

    if (word.startsWith("/")) {
      addCommandCandidates(word, candidates);
      return;
    }

    // Symbol completion is fuzzy and carries the var's docstring/arglists in the menu.
    for (String name : rt.currentSymbolNames()) {
      int score = fuzzyScore(word, name);
      if (score == Integer.MAX_VALUE) continue;
      candidates.add(new Candidate(name, name, "Hara symbols", describe(name), null, null, true));
    }

    // Java package/class completion remains available as a fallback.
    List<String> suggestions = packageTree.suggest(word);
    for (String suggestion : suggestions) {
      if (fuzzyScore(word, suggestion) != Integer.MAX_VALUE) {
        candidates.add(
            new Candidate(suggestion, suggestion, "Java classes", null, null, null, true));
      }
    }
  }

  private void addCommandCandidates(String prefix, List<Candidate> candidates) {
    String[][] commands = {
      {"/help", "show REPL commands"},
      {"/history", "show persistent input history"},
      {"/clear", "clear the terminal"},
      {"/splash", "show the Hara banner"},
      {"/ns", "show the current namespace"},
      {"/doc", "show documentation for a symbol"},
      {"/apropos", "search documented symbols"},
      {"/time", "show the last evaluation time"},
      {"/quit", "exit the REPL"},
      {"/exit", "exit the REPL"}
    };
    for (String[] command : commands) {
      if (fuzzyScore(prefix, command[0]) != Integer.MAX_VALUE) {
        candidates.add(
            new Candidate(command[0], command[0], "REPL commands", command[1], null, null, true));
      }
    }
  }

  private String describe(String name) {
    try {
      Var variable = rt.getObj(Symbol.create(name));
      if (variable == null || variable.meta() == null) return "";
      Object doc = Keyword.create("doc").invoke(variable.meta());
      Object arglists = Keyword.create("arglists").invoke(variable.meta());
      String description = arglists == null ? "" : G.display(arglists);
      if (doc != null) description += (description.isEmpty() ? "" : " — ") + doc;
      return description;
    } catch (Throwable ignored) {
      return "";
    }
  }

  private static int fuzzyScore(String query, String value) {
    if (query == null || query.isEmpty()) return 0;
    if (value.equals(query)) return 0;
    String lowerQuery = query.toLowerCase(java.util.Locale.ROOT);
    String lowerValue = value.toLowerCase(java.util.Locale.ROOT);
    if (lowerValue.startsWith(lowerQuery)) return 10 + value.length() - query.length();
    if (lowerValue.contains(lowerQuery)) return 100 + lowerValue.indexOf(lowerQuery);
    int index = 0;
    int gaps = 0;
    for (int i = 0; i < lowerValue.length() && index < lowerQuery.length(); i++) {
      if (lowerValue.charAt(i) == lowerQuery.charAt(index)) index++;
      else if (index > 0) gaps++;
    }
    return index == lowerQuery.length() ? 200 + gaps : Integer.MAX_VALUE;
  }

  /**
   * Extract word for Lisp-style completion (handles parens, brackets, etc.) Scans backwards from
   * cursor to find the start of the current word.
   */
  public static String extractWord(String buffer, int cursor) {
    if (buffer == null || cursor <= 0) return "";

    // Find the start of the current word by looking backwards
    int start = cursor - 1;
    while (start >= 0) {
      char c = buffer.charAt(start);
      // Break on Lisp delimiters
      if (c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' || c == ' '
          || c == '\t' || c == '\n' || c == '\r' || c == '"' || c == '\'') {
        start++;
        break;
      }
      start--;
    }
    if (start < 0) start = 0;

    return buffer.substring(start, cursor);
  }
}
