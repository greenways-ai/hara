package hara.truffle;

import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.junit.Test;

public class HaraDbRuntimeTest {
  private static void assertHalFixturePasses(String path) throws Exception {
    String source = Files.readString(Path.of(path));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      String result = context.eval(HaraLanguage.ID, source).asString();
      assertTrue(result, !result.contains(":pass false"));
    }
  }

  @Test
  public void databaseKernelClientAndProxyRuntimeFixturePasses() throws Exception {
    assertHalFixturePasses("lib/test/std/db/node/runtime_test.hal");
  }

  @Test
  public void databaseDynamicServiceLifecycleFixturePasses() throws Exception {
    assertHalFixturePasses("lib/test/std/db/node/service_test.hal");
  }

  @Test
  public void databaseRuntimeStatusFixturePasses() throws Exception {
    assertHalFixturePasses("lib/test/std/db/node/status_test.hal");
  }

  @Test
  public void databaseWorkerMessageTransportFixturePasses() throws Exception {
    assertHalFixturePasses("lib/test/std/db/node/transport_test.hal");
  }
}
