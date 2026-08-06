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
                      + "(:require [std.logic.kanren :as logic])) "
                      + "(pr-str "
                      + " (logic/run* "
                      + "  (fn [query] "
                      + "   (logic/conde [(logic/== query 1)] "
                      + "                [(logic/== query 2)]))))")
              .asString());
    }
  }

  @Test
  public void classpathDiscoveryLoadsTypedDatalogAndRelationalAdapters() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[true [[:demo/missing]] [:sky]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns metaspec-logic-truffle-probe "
                      + "(:require [std.logic.kanren :as logic] "
                      + "          [std.logic.datalog :as datalog] "
                      + "          [std.logic.relational :as relational] "
                      + "          [std.typed.schema :as schema])) "
                      + "(def db "
                      + " (datalog/database {} "
                      + "  [[:requirement :demo/missing :must []]])) "
                      + "(pr-str "
                      + " [(schema/valid? [:tuple :keyword :int] [:age 42]) "
                      + "  (datalog/query db "
                      + "   '{:find [?id] "
                      + "     :where [[:requirement ?id :must ?path]]}) "
                      + "  (relational/query* "
                      + "   (fn [query] "
                      + "    (relational/relationo "
                      + "     [[:color :sky :blue]] "
                      + "     [:color query :blue])))])")
              .asString());
    }
  }
}
