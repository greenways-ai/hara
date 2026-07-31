package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.Test;

public class HaraSpecToolTest {
  @Test
  public void portableMetaspecLintReturnsEdnAndStableExitCodes() throws Exception {
    Path valid = Path.of("../specs/01-language-meta/000-metaspec/draft/hal-metaspec-metaspec.edn");
    if (!Files.isRegularFile(valid))
      valid = Path.of("specs/01-language-meta/000-metaspec/draft/hal-metaspec-metaspec.edn");
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    int status =
        HaraSpecTool.run(
            new String[] {"lint", valid.toString(), "--format", "edn"},
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));
    assertEquals(error.toString(StandardCharsets.UTF_8), 0, status);
    assertTrue(output.toString(StandardCharsets.UTF_8).contains(":report/type :hara/metaspec-verification"));
    assertTrue(output.toString(StandardCharsets.UTF_8).contains(":report/status :pass"));
  }

  @Test
  public void readErrorsUseToolExitTwo() {
    int status =
        HaraSpecTool.run(
            new String[] {"lint", "missing-metaspec.edn"},
            new PrintStream(new ByteArrayOutputStream()),
            new PrintStream(new ByteArrayOutputStream()));
    assertEquals(2, status);
  }
}
