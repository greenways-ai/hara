package hara.portable;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import hara.truffle.HaraNativeTestRunner;
import java.nio.file.Path;
import org.junit.Test;

public final class HaraXtalkSelfHostedTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsHaraTargetGrammarSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/tahto/model/v1/spec_hara_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(12, result.facts());
    assertEquals(12, result.checks());
    assertEquals(12, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsRegisteredDependencyEntrySuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/tahto/model/v1/spec_hara_dependency_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(1, result.facts());
    assertEquals(1, result.checks());
    assertEquals(1, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsSelfHostedRuntimeSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/tahto/runtime/basic/type_hara_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(6, result.facts());
    assertEquals(6, result.checks());
    assertEquals(6, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsCanonicalXtalkMathOnHara() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/xt/lang/hara_target_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(4, result.facts());
    assertEquals(4, result.checks());
    assertEquals(4, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
