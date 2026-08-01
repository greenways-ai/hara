package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraNativeTestRunnerTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void classifiesPassingCodeTestSummary() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-fixtures/std/native/test_runner_pass.hal"));

    assertTrue(result.passed());
    assertEquals(1, result.facts());
    assertEquals(1, result.checks());
    assertEquals(1, result.passedChecks());
    assertEquals(0, result.failedChecks());
  }

  @Test
  public void preservesFailingSummaryForHostReporting() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test-fixtures/std/native/test_runner_fail.hal"));

    assertFalse(result.passed());
    assertEquals(1, result.facts());
    assertEquals(1, result.checks());
    assertEquals(0, result.passedChecks());
    assertEquals(1, result.failedChecks());
    assertTrue(result.failureMessage().contains(":failed"));
  }
}
