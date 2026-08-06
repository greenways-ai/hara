package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraTahtoPostgresDeftypeTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  @Test
  public void runsPortablePostgresDeftypeSuite() throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(
            ROOT,
            ROOT.resolve("lib/test/tahto/model/v1/spec_postgres_deftype_test.hal"));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(10, result.facts());
    assertEquals(10, result.checks());
    assertEquals(10, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }
}
