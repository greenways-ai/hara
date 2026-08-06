package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraLangCompileModuleTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsPortableCompileModuleSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT,
            ROOT.resolve("lib/test/lang/core/compile_module_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(6, result.facts());
    assertEquals(6, result.checks());
    assertEquals(6, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
