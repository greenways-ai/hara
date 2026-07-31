package hara.truffle;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import org.junit.Test;

public class HaraPackageToolTest {
  @Test
  public void localBuildAndInspectAreDeterministic() throws Exception {
    Path root = Files.createTempDirectory("hara-package-tool-");
    try {
      Files.createDirectories(root.resolve("src/demo"));
      Files.writeString(
          root.resolve("project.edn"),
          "{:hara/type :project :hara/version \"1.0.0\" "
              + ":project/id demo/app :project/version \"1.2.3\" "
              + ":project/source-paths [\"src\"] :project/test-paths [] "
              + ":project/extension-paths [\"extensions\"] "
              + ":project/capabilities #{}}\n");
      Files.writeString(root.resolve("src/demo/main.hal"), "(ns demo.main)\n(def answer 42)\n");
      Path first = root.resolve("first.harp");
      Path second = root.resolve("second.harp");
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      ByteArrayOutputStream error = new ByteArrayOutputStream();
      PrintStream stdout = new PrintStream(output, true, StandardCharsets.UTF_8);
      PrintStream stderr = new PrintStream(error, true, StandardCharsets.UTF_8);
      assertEquals(
          0,
          HaraPackageTool.run(
              new String[] {"build", root.toString(), "--output", first.toString()},
              stdout,
              stderr));
      assertEquals(
          0,
          HaraPackageTool.run(
              new String[] {"build", root.toString(), "--output", second.toString()},
              stdout,
              stderr));
      assertArrayEquals(Files.readAllBytes(first), Files.readAllBytes(second));
      output.reset();
      assertEquals(
          0,
          HaraPackageTool.run(
              new String[] {"inspect", first.toString()}, stdout, stderr));
      String manifest = output.toString(StandardCharsets.UTF_8);
      assertTrue(manifest.contains(":identity \"demo/app\""));
      assertTrue(manifest.contains("\"demo.main\" \"src/demo/main.hal\""));
      assertEquals("", error.toString(StandardCharsets.UTF_8));
    } finally {
      Files.walk(root)
          .sorted(Comparator.reverseOrder())
          .forEach(
              path -> {
                try {
                  Files.deleteIfExists(path);
                } catch (Exception ignored) {
                }
              });
    }
  }
}
