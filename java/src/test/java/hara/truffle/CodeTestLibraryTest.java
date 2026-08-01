package hara.truffle;

import static org.junit.Assert.assertEquals;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class CodeTestLibraryTest {
  @Test
  public void classpathDiscoveryRunsFactsWithStructuredLifecycleEvents() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[:passed 1 1 1 [:run/start :fact/start :fact/end :run/end]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns code-test-truffle-probe (:use code.test)) "
                      + "(fact \"portable\" (promise/from 42) => 42) "
                      + "(let [reporter (event-reporter) "
                      + "      summary (run {:namespace \"code-test-truffle-probe\" "
                      + "                    :reporter reporter}) "
                      + "      positional (run '[code-test])]"
                      + "  (pr-str [(get summary :status) "
                      + "           (get summary :facts) "
                      + "           (get summary :checks) "
                      + "           (get positional :facts) "
                      + "           (vec (map (fn [event] (get event :event)) "
                      + "                     (reporter-events reporter)))]))")
              .asString());
    }
  }

  @Test
  public void foundationCompatibilityNamespacesLoadAndCompose() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[true true true 42 {:ns [std code]}]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns code-test-compat-truffle-probe "
                      + "(:require [code.test :as test] "
                      + "[code.test.checker.common :as common] "
                      + "[code.test.checker.collection :as collection] "
                      + "[code.test.compile.types :as types] "
                      + "[code.test.task :as task])) "
                      + "(pr-str (let [fact (types/Fact :core 'id 'probe nil nil "
                      + "\"portable\" 1 1 nil nil (fn [] 42) {})] "
                      + "[(common/succeeded? "
                      + "  (common/verify (common/exactly 1) 1)) "
                      + " (:pass (test/check "
                      + "         (fn [] {:a 1 :b 2}) "
                      + "         (collection/contains-map {:a 1}))) "
                      + " (types/fact? fact) "
                      + " (fact) "
                      + " (task/process-test-args "
                      + "  [\":only\" \"std\" \"code\"])]))")
              .asString());
    }
  }

  @Test
  public void dynamicBindingsResolveInTheirDefiningNamespace() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[2 3]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns binding-source) "
                      + "(def ^:dynamic *value* 1) "
                      + "(defn locally [] (binding [*value* 2] *value*)) "
                      + "(ns binding-caller) "
                      + "[(binding-source/locally) "
                      + " (binding [binding-source/*value* 3] binding-source/*value*)]")
              .toString());
    }
  }

}
