package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraLangPostgresConstTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsPortablePostgresConstSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT,
            ROOT.resolve("lib/test/lang/model/v1/spec_postgres_const_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(14, result.facts());
    assertEquals(14, result.checks());
    assertEquals(14, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
