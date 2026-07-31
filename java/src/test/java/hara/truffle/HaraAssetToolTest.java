package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Comparator;
import org.junit.Test;

public class HaraAssetToolTest {
  @Test
  public void buildsStableDigestManifest() throws Exception {
    Path root = Files.createTempDirectory("hara-asset-tool-");
    try {
      Files.createDirectories(root.resolve("images"));
      Files.write(root.resolve("images/hero.png"), "png".getBytes(StandardCharsets.UTF_8));
      Files.writeString(
          root.resolve("asset.edn"),
          "{:asset/format 1 :asset/coordinate \"alice/gallery\" "
              + ":asset/version \"1.0.0\" "
              + ":asset/entries [{:entry/path \"images/hero.png\" "
              + ":entry/media-type \"image/png\"}]}\n");
      Path first = root.resolve("first.edn");
      Path second = root.resolve("second.edn");
      ByteArrayOutputStream output = new ByteArrayOutputStream();
      ByteArrayOutputStream error = new ByteArrayOutputStream();
      PrintStream stdout = new PrintStream(output, true, StandardCharsets.UTF_8);
      PrintStream stderr = new PrintStream(error, true, StandardCharsets.UTF_8);
      assertEquals(
          0,
          HaraAssetTool.run(
              new String[] {"build", root.toString(), "--output", first.toString()},
              stdout,
              stderr));
      assertEquals(
          0,
          HaraAssetTool.run(
              new String[] {"build", root.toString(), "--output", second.toString()},
              stdout,
              stderr));
      assertEquals(Files.readString(first), Files.readString(second));
      assertTrue(Files.readString(first).contains(":asset/coordinate \"hara:alice/gallery\""));
      assertTrue(Files.readString(first).contains(":entry/sha256 \"sha256:"));
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
