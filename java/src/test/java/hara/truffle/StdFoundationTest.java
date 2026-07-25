package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdFoundationTest {
  @Test
  public void halFoundationOwnsMapAndFillsPortableSymbols() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          42,
          context.eval(HaraLanguage.ID, "((std.foundation/comp2 inc inc) 40)").asLong());
      assertEquals(
          "[2 3 4]",
          context.eval(HaraLanguage.ID, "(map inc [1 2 3])").toString());
      assertEquals(
          "[true false nil -1 1 3]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(T 1) (F 1) (NIL 1) (compare 2 3) (min 3 1 2) (max 3 1 2)]")
              .toString());
    }
  }

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
  public void mapIsEagerDirectAndLazyWhenCurried() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[true true true true]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(vector? (map inc [1 2 3])) "
                      + "(iter? ((map inc) [1 2 3])) "
                      + "(array? (map inc (array 1 2 3))) "
                      + "(iter? ((comp (map inc) (map inc)) [1 2 3]))]")
              .toString());
      assertEquals(
          "[[2 3 4] [3 4 5]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(vec ((map inc) [1 2 3])) "
                      + "(vec ((comp (map inc) (map inc)) [1 2 3]))]")
              .toString());
      assertEquals(
          "[2, 3, 4]", context.eval(HaraLanguage.ID, "(map inc (array 1 2 3))").toString());
    }
  }

  @Test
  public void collectionAndOrderingAdditionsArePortableHaraValues() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[[1 2] [2 3] [[1 2] [3]] [[1 3] [2 4] [5]] [0 2 4]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(butlast [1 2 3]) (take-last 2 [1 2 3]) "
                      + "(split-at 2 [1 2 3]) (partition-by odd? [1 3 2 4 5]) "
                      + "(take-nth 2 [0 1 2 3 4])]")
              .toString());
      assertEquals(
          "[[1 2 3] [[:a] [:a :b] [:a :b :c]] [1 2 3] true false "
              + "{:a 1 :b 2} {:a 3 :b 3} {true [1 3] false [2 4]} {:a 2 :b 1}]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(sort [3 1 2]) (sort-by count [[:a :b] [:a] [:a :b :c]]) "
                      + "(distinct [1 2 1 3]) (distinct? 1 2 3) (distinct? 1 2 1) "
                      + "(zipmap [:a :b] [1 2]) (merge-with + {:a 1} {:a 2 :b 3}) "
                      + "(group-by odd? [1 2 3 4]) (frequencies [:a :b :a])]")
              .toString());
    }
  }

  @Test
  public void atomsValidateMutateAndNotifyFourArgumentWatches() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[9 [[:log 1 3] [:log 3 9]] 1 true 10]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [a (atom 1) seen (atom [])] "
                      + "(add-watch a :log "
                      + "  (fn [key ref old new] (swap! seen conj [key old new]))) "
                      + "(swap! a + 2) (reset! a 9) "
                      + "[(deref a) (deref seen) (count (get-watches a)) "
                      + " (compare-and-set! a 9 10) (deref a)])")
              .toString());
    }
  }

  @Test
  public void foundationMacrosReceivePortableFormAndEnvironment() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[:yes 5 6 6 [2 3]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(if-not false :yes :no) (if-let [x 4] (+ x 1) 0) "
                      + "(when-let [x 4] (+ x 2)) "
                      + "(cond-> 1 true inc false (+ 10) true (* 3)) "
                      + "(cond->> [1 2] true (map inc))]")
              .toString());
      assertEquals(
          "[true true true]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (defmacro env-line [] (:line &env)) "
                      + "[(number? (code-line)) (number? (code-column)) "
                      + " (number? (env-line))])")
              .toString());
      assertEquals(
          "[[4 4] [5 5]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (defmacro hygienic [value] "
                      + "      `(let [x# ~value] [x# x#])) "
                      + "    [(hygienic 4) (hygienic 5)])")
              .toString());
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
