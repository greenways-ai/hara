package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdFoundationTest {
  @Test
  public void fallbackReloadRefreshesHalFoundation() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      long revision =
          context.eval(HaraLanguage.ID, "(module-revision \"std/foundation.hal\")").asLong();
      context.eval(HaraLanguage.ID, "(require 'std.foundation {:reload true})");
      assertEquals(
          revision + 1,
          context.eval(HaraLanguage.ID, "(module-revision \"std/foundation.hal\")").asLong());
      assertEquals(
          "[2 3 4]", context.eval(HaraLanguage.ID, "(map inc [1 2 3])").toString());
    }
  }

  @Test
  public void optimizedOperationsMatchTheirHalDefinitions() throws Exception {
    String source;
    try (InputStream input =
        StdFoundationTest.class.getClassLoader().getResourceAsStream("std/foundation.hal")) {
      assertTrue("missing foundation fallback resource", input != null);
      source =
          new String(input.readAllBytes(), StandardCharsets.UTF_8)
              .replace("ns std.foundation", "ns testing.foundation-fallback")
              .replaceAll("(?s)\\(:config.*?\\]\\}\\)", "");
    }
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, source);
      assertEquals(
          "[[2 3 4] [2 3 4]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(std.foundation/map inc [1 2 3]) "
                      + " (testing.foundation-fallback/map inc [1 2 3])]")
              .toString());
      assertEquals(
          "[10 10]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(std.foundation/reduce + 0 [1 2 3 4]) "
                      + " (testing.foundation-fallback/reduce + 0 [1 2 3 4])]")
              .toString());
    }
  }
}
