package hara.truffle;

import static org.junit.Assert.assertEquals;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdLibZipTest {
  @Test
  public void providerLoadsPersistentZipImplementation() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[[1 9 8 3] [1 2 3]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns std-lib-zip-truffle-probe "
                      + "(:require [std.lib.zip :as zip])) "
                      + "(let [root [1 2 3] "
                      + "      location (zip/step-right "
                      + "                (zip/step-inside (zip/vector-zip root))) "
                      + "      edited (zip/replace-right "
                      + "              (zip/insert-left location 9) 8)] "
                      + "  (pr-str [(zip/result edited) root]))")
              .asString());
    }
  }
}
