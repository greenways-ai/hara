package hara.portable;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import hara.truffle.HaraNativeTestRunner;
import java.nio.file.Path;
import org.junit.Test;

public final class HaraNativeXtalkCollectionTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsNativeCollectionFacadeSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(ROOT, ROOT.resolve("lib/test/tahto/runtime/hara_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(6, result.facts());
    assertEquals(6, result.checks());
    assertEquals(6, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsNativeCollectionEmitterSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/tahto/model/v1/spec_hara_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(8, result.facts());
    assertEquals(8, result.checks());
    assertEquals(8, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsNativeCollectionRuntimeSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT, ROOT.resolve("lib/test/tahto/runtime/basic/impl/process_xtalk_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(6, result.facts());
    assertEquals(6, result.checks());
    assertEquals(6, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
