package hara.portable;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import hara.truffle.HaraNativeTestRunner;
import java.nio.file.Path;
import org.junit.Test;

public final class HaraPortableMcpNodeTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsPortableFoundationMemoryTransportSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-lang/xt/substrate/transport_memory_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(13, result.facts());
    assertEquals(19, result.checks());
    assertEquals(19, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsPortableFoundationMcpNodeSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-lang/xt/mcp/node/kernel_base_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(5, result.facts());
    assertEquals(5, result.checks());
    assertEquals(5, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
