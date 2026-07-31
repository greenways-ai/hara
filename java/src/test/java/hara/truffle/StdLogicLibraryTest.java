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

  @Test
  public void classpathDiscoveryLoadsTypedDatalogAndKanrenAdapters() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[true [[:demo/missing]] [:sky]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns metaspec-logic-truffle-probe "
                      + "(:require [std.logic :as logic] "
                      + "          [std.logic.datalog :as datalog] "
                      + "          [std.logic.kanren :as kanren] "
                      + "          [std.typed.schema :as schema])) "
                      + "(def db "
                      + " (datalog/database {} "
                      + "  [[:requirement :demo/missing :must []]])) "
                      + "(pr-str "
                      + " [(schema/valid? [:tuple :keyword :int] [:age 42]) "
                      + "  (datalog/query db "
                      + "   '{:find [?id] "
                      + "     :where [[:requirement ?id :must ?path]]}) "
                      + "  (kanren/query* "
                      + "   (fn [query] "
                      + "    (kanren/relationo "
                      + "     [[:color :sky :blue]] "
                      + "     [:color query :blue])))])")
              .asString());
    }
  }
}
