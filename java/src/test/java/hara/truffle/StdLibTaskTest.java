package hara.truffle;

import static org.junit.Assert.assertEquals;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdLibTaskTest {
  @Test
  public void providerSelectsAggregatesAndPackagesResults() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[{2 2 3 3} 5 [7 true :ready :ready]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns std-lib-task-truffle-probe "
                      + "(:require [std.lib.task :as task])) "
                      + "(task/deftask aggregate "
                      + "  {:template :default "
                      + "   :main {:fn (fn [value] {:score value})} "
                      + "   :result {:ignore (fn [data] (= 0 (get data :score))) "
                      + "            :output (fn [data] (get data :score))} "
                      + "   :summary {:aggregate "
                      + "             {:total [(fn [data] (get data :score)) "
                      + "                      (fn [a b] (+ a b)) 0]}}}) "
                      + "(task/deftask constructed "
                      + "  {:template :default "
                      + "   :main {:argcount 4 "
                      + "          :fn (fn [input params lookup env] "
                      + "                [input (get params :flag) "
                      + "                 (get lookup :input) "
                      + "                 (get env :environment)])} "
                      + "   :construct {:input (fn [task] 7) "
                      + "               :env (fn [options] {:environment :ready}) "
                      + "               :lookup (fn [task options] "
                      + "                         {:input (get options :environment)})} "
                      + "   :params {:flag true}}) "
                      + "(let [output (task/invoke aggregate [0 2 3] {:return :all})] "
                      + "  (pr-str [(get output :results) "
                      + "           (get (get output :summary) :total) "
                      + "           (task/invoke constructed)]))")
              .asString());
    }
  }
}
