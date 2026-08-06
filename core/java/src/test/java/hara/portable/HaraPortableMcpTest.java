package hara.portable;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import hara.truffle.HaraNativeTestRunner;
import java.nio.file.Path;
import org.junit.Test;

public final class HaraPortableMcpTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsPortableFoundationMcpBaseSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-lang/xt/mcp/base_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(5, result.facts());
    assertEquals(15, result.checks());
    assertEquals(15, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
