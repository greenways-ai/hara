package hara.truffle;

import static org.junit.Assert.assertEquals;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class CodeTestLibraryTest {
  @Test
  public void classpathDiscoveryRunsFactsWithStructuredLifecycleEvents() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[:passed 1 1 [:run/start :fact/start :fact/end :run/end]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns code-test-truffle-probe (:use code.test)) "
                      + "(fact \"portable\" (promise/from 42) => 42) "
                      + "(let [reporter (event-reporter) "
                      + "      summary (run {:namespace \"code-test-truffle-probe\" "
                      + "                    :reporter reporter})] "
                      + "  (pr-str [(get summary :status) "
                      + "           (get summary :facts) "
                      + "           (get (get summary :checks) :total) "
                      + "           (vec (map (fn [event] (get event :event)) "
                      + "                     (reporter-events reporter)))]))")
              .asString());
    }
  }
}
