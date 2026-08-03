package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraTahtoScriptMetadataTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsPortableModuleMetadataSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT,
            ROOT.resolve("lib/test/tahto/core/script_metadata_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(4, result.facts());
    assertEquals(4, result.checks());
    assertEquals(4, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
