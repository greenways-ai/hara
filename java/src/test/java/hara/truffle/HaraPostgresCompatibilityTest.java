package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.junit.Test;

public final class HaraPostgresCompatibilityTest {
  private static final Path ROOT = Path.of(".").toAbsolutePath().normalize();

  private static void assertSuite(String path, int facts, int checks) throws Exception {
    HaraNativeTestRunner.Result result =
        HaraNativeTestRunner.runFile(ROOT, ROOT.resolve(path));

    assertTrue(result.failureMessage(), result.passed());
    assertEquals(facts, result.facts());
    assertEquals(checks, result.checks());
    assertEquals(checks, result.passedChecks());
    assertEquals(0, result.failedChecks());
    assertEquals(0, result.errors());
    assertEquals(0, result.timeouts());
  }

  @Test
  public void runsFoundationPostgresConnectionFacade() throws Exception {
    assertSuite("lib/test/lib/postgres/connection_test.hal", 5, 5);
  }

  @Test
  public void runsFoundationPostgresLifecycleFacade() throws Exception {
    assertSuite("lib/test/lib/postgres_test.hal", 4, 4);
  }
}
