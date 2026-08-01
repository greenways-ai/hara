package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.ByteArrayInputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import java.util.Base64;
import org.junit.Test;

public class MainTest {
  @Test
  public void projectCommandsRunAndReportStandardTestResults() throws Exception {
    Path root = Files.createTempDirectory("hara-cli-project-");
    try {
      Files.createDirectories(root.resolve("src/demo_app"));
      Files.createDirectories(root.resolve("test/demo_app"));
      Files.createDirectories(root.resolve("src-java/demo_app"));
      Files.writeString(
          root.resolve("project.edn"),
          "{:hara/type :project :hara/version \"1.0.0\" :project/id demo-app "
              + ":project/version \"0.1.0\" :project/source-paths [\"src\"] "
              + ":project/test-paths [\"test\"] :project/extension-paths [\"extensions\"] "
              + ":project/main demo_app.main :project/capabilities #{:jvm/reflection} "
              + ":project/dependencies {} "
              + ":jvm/dependencies [[org.apache.commons/commons-lang3 \"3.12.0\"]] "
              + ":jvm/source-paths [\"src-java\"] :jvm/target-path \"target/classes\"}");
      Files.writeString(
          root.resolve("src-java/demo_app/Bridge.java"),
          "package demo_app;\n"
              + "import org.apache.commons.lang3.StringUtils;\n"
              + "public final class Bridge {\n"
              + "  private Bridge() {}\n"
              + "  public static String greeting() {\n"
              + "    return StringUtils.reverse(\"ppa-omed morf olleH\");\n"
              + "  }\n"
              + "}\n");
      Files.writeString(
          root.resolve("src/demo_app/main.hal"),
          "(ns demo_app.main (:flavor :jvm) (:import [demo_app Bridge])) "
              + "(defn main [] (Bridge/greeting)) (main)");
      Files.writeString(
          root.resolve("test/demo_app/main_test.hal"),
          "(ns demo_app.main-test (:require [std.lib.test :as test])) "
              + "(test/print-results [(test/check \"starter project runs\" true true)])");
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      ByteArrayOutputStream error = new ByteArrayOutputStream();
      PrintStream stdout = new PrintStream(output, true, StandardCharsets.UTF_8);
      PrintStream stderr = new PrintStream(error, true, StandardCharsets.UTF_8);
      assertEquals(0, Main.run(new String[] {"--project", root.toString(), "check"}, stdout, stderr));
      assertEquals(0, Main.run(new String[] {"--project", root.toString(), "sync"}, stdout, stderr));
      assertEquals(0, Main.run(new String[] {"--project", root.toString(), "sync", "--frozen"}, stdout, stderr));
      assertEquals(0, Main.run(new String[] {"--project", root.toString(), "run"}, stdout, stderr));
      assertEquals(
          0,
          Main.run(
              new String[] {"--project", root.toString(), "--offline", "repl"},
              new ByteArrayInputStream(
                  ("(ns demo_app.repl (:flavor :jvm) (:import [demo_app Bridge]))\n"
                          + "(Bridge/greeting)\n:quit\n")
                      .getBytes(StandardCharsets.UTF_8)),
              stdout,
              stderr));
      assertEquals(0, Main.run(new String[] {"--project", root.toString(), "test"}, stdout, stderr));
      assertEquals("", error.toString(StandardCharsets.UTF_8));
      assertTrue(output.toString(StandardCharsets.UTF_8).contains("project check: demo-app 0.1.0"));
      assertTrue(output.toString(StandardCharsets.UTF_8).contains("jvm dependencies: 1 direct"));
      assertTrue(output.toString(StandardCharsets.UTF_8).contains("Hello from demo-app"));
      assertTrue(output.toString(StandardCharsets.UTF_8).contains("test result: 1 passed, 0 failed"));
      assertTrue(Files.isRegularFile(root.resolve("target/classes/demo_app/Bridge.class")));
    } finally {
      Files.walk(root).sorted(Comparator.reverseOrder()).forEach(path -> { try { Files.deleteIfExists(path); } catch (Exception ignored) {} });
    }
  }

  @Test
  public void fileLibraryIsDefaultDeniedAndExplicitlyGranted() throws Exception {
    ByteArrayOutputStream deniedOutput = new ByteArrayOutputStream();
    ByteArrayOutputStream deniedError = new ByteArrayOutputStream();
    int denied =
        Main.run(
            new String[] {"eval", "(file/read \"denied.bin\")"},
            new PrintStream(deniedOutput, true, StandardCharsets.UTF_8),
            new PrintStream(deniedError, true, StandardCharsets.UTF_8));
    assertEquals(1, denied);
    assertTrue(deniedError.toString(StandardCharsets.UTF_8).contains("file access is denied"));

    Path path = Files.createTempFile("hara-runtime-library-", ".bin");
    Files.delete(path);
    try {
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      ByteArrayOutputStream error = new ByteArrayOutputStream();
      String escaped = path.toString().replace("\\", "\\\\").replace("\"", "\\\"");
      String form =
          "(deref (file/write \""
              + escaped
              + "\" (bytes 1 -1))) "
              + "(bytes/get (deref (file/read \""
              + escaped
              + "\")) 1)";
      int status =
          Main.run(
              new String[] {"--allow-file", "eval", form},
              new PrintStream(output, true, StandardCharsets.UTF_8),
              new PrintStream(error, true, StandardCharsets.UTF_8));
      assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
      assertEquals("255\n", output.toString(StandardCharsets.UTF_8));
    } finally {
      Files.deleteIfExists(path);
    }
  }

  @Test
  public void fileLibrarySupportsExistsListMkdirAndDelete() throws Exception {
    Path directory = Files.createTempDirectory("hara-file-library-");
    try {
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      ByteArrayOutputStream error = new ByteArrayOutputStream();
      String escaped = directory.toString().replace("\\", "\\\\").replace("\"", "\\\"");
      String child = escaped + "/child.bin";
      String form =
          "[(deref (file/write \""
              + child
              + "\" (bytes 1 2 3))) "
              + "(deref (file/exists? \""
              + child
              + "\")) "
              + "(count (deref (file/list \""
              + escaped
              + "\"))) "
              + "(deref (file/delete \""
              + child
              + "\")) "
              + "(deref (file/exists? \""
              + child
              + "\"))]";
      int status =
          Main.run(
              new String[] {"--allow-file", "eval", form},
              new PrintStream(output, true, StandardCharsets.UTF_8),
              new PrintStream(error, true, StandardCharsets.UTF_8));
      assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
      assertEquals("[nil true 1 nil false]\n", output.toString(StandardCharsets.UTF_8));
    } finally {
      Files.walk(directory)
          .sorted(java.util.Comparator.reverseOrder())
          .forEach(
              p -> {
                try {
                  Files.deleteIfExists(p);
                } catch (Exception ignored) {
                }
              });
    }
  }

  @Test
  public void stringLibraryUsesSpecNamesAndUnicodeCodePointIndexes() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    String form =
        "[(str/length \"a😀b\") "
            + "(str/char-at \"a😀b\" 1) "
            + "(str/slice \"a😀b\" 1 2) "
            + "(str/index-of \"a😀b\" \"b\") "
            + "(str/last-index-of \"😀a😀\" \"😀\") "
            + "(str/pad-left \"x\" 3 \"😀\") "
            + "(str/replace-first \"a.a\" \".\" \"-\")]";

    int status =
        Main.run(
            new String[] {"eval", form},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
    assertEquals(
        "[3 \"\\uD83D\\uDE00\" \"\\uD83D\\uDE00\" 2 2 \"\\uD83D\\uDE00\\uD83D\\uDE00x\" \"a-a\"]\n",
        output.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void evaluatesAnExpression() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();

    int status =
        Main.run(
            new String[] {"eval", "(+ 19 23)"},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
    assertEquals("42\n", output.toString(StandardCharsets.UTF_8));
    assertEquals("", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void noArgumentsEnterTheRepl() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    int status =
        Main.run(
            new String[] {"--port=0"},
            new ByteArrayInputStream(new byte[0]),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));
    assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
    assertEquals("", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void replLoadsCoreAndRendersLazyIterators() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    int status =
        Main.run(
            new String[] {"--offline", "repl"},
            new ByteArrayInputStream(
                "(inc 1)\n((map inc) [1 2 3])\n".getBytes(StandardCharsets.UTF_8)),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(0, status);
    assertTrue(output.toString(StandardCharsets.UTF_8).contains("2\n"));
    assertTrue(output.toString(StandardCharsets.UTF_8).contains("#<lazy-iterator>\n"));
    assertEquals("", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void runsThePackagedL0ConformanceCorpus() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();

    int status =
        Main.run(
            new String[] {"conformance"},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
    assertTrue(output.toString(StandardCharsets.UTF_8).contains("L0 conformance passed:"));
    assertEquals("", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void reportsGuestErrorsWithoutAJavaStackTrace() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();

    int status =
        Main.run(
            new String[] {"eval", "missing"},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(1, status);
    assertEquals("", output.toString(StandardCharsets.UTF_8));
    assertEquals("Unbound symbol: missing\n", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void replRetainsContextAcrossInputsAndSupportsMultilineForms() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    byte[] input =
        "(def answer 40)\n(+ answer 2)\n(let [x 1]\n  (+ x\n     2))\n:quit\n"
            .getBytes(StandardCharsets.UTF_8);

    int status =
        Main.run(
            new String[] {"--offline", "repl"},
            new ByteArrayInputStream(input),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(0, status);
    assertTrue(output.toString(StandardCharsets.UTF_8).contains("=> #'user/answer\n=> 42\n=> 3\n"));
    assertEquals("", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void replContinuesAfterGuestErrors() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    byte[] input =
        "(def answer 40)\nmissing\n(+ answer 2)\n:quit\n".getBytes(StandardCharsets.UTF_8);

    int status =
        Main.run(
            new String[] {"--offline", "repl"},
            new ByteArrayInputStream(input),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(0, status);
    assertTrue(output.toString(StandardCharsets.UTF_8).contains("=> 42\n"));
    assertEquals("Unbound symbol: missing\n", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void replRetainsCompletedFormHistory() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    byte[] input = "(def answer 40)\nmissing\n:history\n:quit\n".getBytes(StandardCharsets.UTF_8);

    int status =
        Main.run(
            new String[] {"--offline", "repl"},
            new ByteArrayInputStream(input),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(0, status);
    String history = output.toString(StandardCharsets.UTF_8);
    assertTrue(history.contains("1: (def answer 40)"));
    assertTrue(history.contains("2: missing"));
    assertEquals("Unbound symbol: missing\n", error.toString(StandardCharsets.UTF_8));
  }

  @Test
  public void helpUsesTheCanonicalHaraCommandName() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();

    int status =
        Main.run(
            new String[] {"help"},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));

    assertEquals(0, status);
    assertTrue(output.toString(StandardCharsets.UTF_8).contains("hara [OPTIONS]"));
    assertTrue(!output.toString(StandardCharsets.UTF_8).contains("hara-truffle"));
  }

  @Test
  public void parsesSeparateOptionValuesAndRejectsInvalidPorts() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    int valid =
        Main.run(
            new String[] {"--offline", "--host", "127.0.0.1", "--port", "0"},
            new ByteArrayInputStream(new byte[0]),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));
    assertEquals(error.toString(StandardCharsets.UTF_8), 0, valid);

    error.reset();
    int invalid =
        Main.run(
            new String[] {"--port", "70000"},
            new ByteArrayInputStream(new byte[0]),
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));
    assertEquals(2, invalid);
    assertTrue(error.toString(StandardCharsets.UTF_8).contains("between 0 and 65535"));
  }

  @Test
  public void benchmarkCommandEmitsValidatedMachineReadableSamples() {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    String source =
        Base64.getUrlEncoder()
            .withoutPadding()
            .encodeToString("(+ 19 23)".getBytes(StandardCharsets.UTF_8));
    int status =
        Main.run(
            new String[] {"benchmark", "test", "arithmetic", source, "42", "2", "1"},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));
    String json = output.toString(StandardCharsets.UTF_8);
    assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
    assertTrue(json.contains("\"runtime\":\"test\""));
    assertTrue(json.contains("\"workload\":\"arithmetic\""));
    assertTrue(json.contains("\"first_ns\":"));
    assertTrue(json.contains("\"samples_ns\":["));
    assertEquals(2, json.substring(json.indexOf('[') + 1, json.indexOf(']')).split(",").length);
  }

}
