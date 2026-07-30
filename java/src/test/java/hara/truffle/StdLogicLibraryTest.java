package hara.truffle;

import static org.junit.Assert.assertEquals;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdLogicLibraryTest {
  @Test
  public void classpathDiscoveryLoadsCoroutineBackedSolverWithoutProvider() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[1 2]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns std-logic-truffle-probe "
                      + "(:require [std.logic :as logic])) "
                      + "(pr-str "
                      + " (logic/run* "
                      + "  (fn [query] "
                      + "   (logic/conde [(logic/== query 1)] "
                      + "                [(logic/== query 2)]))))")
              .asString());
    }
  }
}
