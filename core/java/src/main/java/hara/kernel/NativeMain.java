package hara.kernel;

import hara.kernel.base.Parser;
import hara.kernel.base.RT;
import hara.lang.base.G;

import java.io.IOException;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

public class NativeMain {

  public static void main(String[] args) throws IOException {
    NativeMode.enable();
    if (args.length == 0) {
      printUsage();
      return;
    }

    String command = args[0];
    String[] restArgs = java.util.Arrays.copyOfRange(args, 1, args.length);

    if ("help".equals(command)) {
      printUsage();
      return;
    }

    if ("eval".equals(command)) {
      requireArg(command, restArgs, "<code>");
      printResult(evalString(restArgs[0]));
      return;
    }

    if ("stdin".equals(command)) {
      printResult(evalAll(new InputStreamReader(System.in, StandardCharsets.UTF_8)));
      return;
    }

    if ("run".equals(command)) {
      requireArg(command, restArgs, "<file>");
      printResult(evalFile(Path.of(restArgs[0])));
      return;
    }

    if ("repl".equals(command)) {
      System.err.println(NativeMode.unsupported("interactive REPL").getMessage());
      System.exit(1);
    }

    Path implicitFile = Path.of(command);
    if (Files.exists(implicitFile)) {
      printResult(evalFile(implicitFile));
      return;
    }

    System.err.println("Unknown native command: " + command);
    printUsage();
  }

  private static void requireArg(String command, String[] args, String usage) {
    if (args.length == 0) {
      System.err.println("Usage: hara native " + command + " " + usage);
      System.exit(1);
    }
  }

  private static RT.Instance<Object> createRuntime() {
    Foundation foundation = new Foundation();
    RT.Instance<Object> rt = new RT.Instance<>(foundation, "ROOT");
    foundation.RTS.put(rt._key, rt);
    return rt;
  }

  private static Object evalString(String code) {
    RT.Instance<Object> rt = createRuntime();
    return rt.eval(rt.readString(code));
  }

  private static Object evalFile(Path path) throws IOException {
    try (Reader reader = Files.newBufferedReader(path, StandardCharsets.UTF_8)) {
      return evalAll(reader);
    }
  }

  private static Object evalAll(Reader input) throws IOException {
    RT.Instance<Object> rt = createRuntime();
    hara.kernel.base.Reader reader = new hara.kernel.base.Reader(input);
    Object eof = new Object();
    Object result = null;
    while (true) {
      Object form = Parser.LispReader.read(reader, false, eof, false, null);
      if (form == eof) {
        return result;
      }
      result = rt.eval(form);
    }
  }

  private static void printResult(Object result) {
    if (result != null) {
      System.out.println(G.display(result));
    }
  }

  private static void printUsage() {
    System.out.println("hara native eval <code>");
    System.out.println("hara native run <file>");
    System.out.println("hara native stdin");
    System.out.println("hara native help");
  }
}
