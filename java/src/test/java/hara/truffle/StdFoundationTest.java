package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

public class StdFoundationTest {
  @Test
  public void referenceFunctionsRouteThroughCanonicalProtocols() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[3 9 true true [[:log 1 3] [:log 3 9] [:log 9 10]]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [reference (atom 1) seen (atom [])] "
                      + "  (watch-add reference :log "
                      + "    (fn [key ref old new] "
                      + "      (swap! seen "
                      + "        (fn [values item] (conj values item)) "
                      + "        [key old new]))) "
                      + "  [(swap! reference (fn [value amount] (+ value amount)) 2) "
                      + "   (reset! reference 9) "
                      + "   (cas! reference 9 10) "
                      + "   (std.protocol.iiterator/iter-next? "
                      + "     (std.protocol.iiter/iter (watch-list reference))) "
                      + "   (deref seen)])")
              .toString());
      for (String legacy :
          new String[] {
            "compare:set!", "compare-and-set!", "add-watch", "remove-watch", "get-watches"
          }) {
        PolyglotException error =
            assertThrows(
                legacy,
                PolyglotException.class,
                () -> context.eval(HaraLanguage.ID, legacy));
        assertTrue(legacy, error.getMessage().contains("Unbound symbol"));
      }
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
  public void publicMapDotoAndSetHelpersArePortable() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[{2 :a 3 :b} {:a 2 :b 3} [1 [1 2]]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(map-keys inc {1 :a 2 :b}) "
                      + "(map-vals inc {:a 1 :b 2}) "
                      + "(let [calls (atom 0) "
                      + "      value (doto (do (swap! calls inc) (atom [])) "
                      + "              (swap! (fn [values item] (conj values item)) 1) "
                      + "              (swap! (fn [values item] (conj values item)) 2))] "
                      + "  [(deref calls) (deref value)])]")
              .toString());
      assertEquals(
          "[#{1 2 3} #{3} #{1} true true #{1 3}]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do "
                      + "(ns set-test (:require [std.foundation.set :as set])) "
                      + "[(set/union #{1 2} #{2 3}) "
                      + " (set/intersection #{1 2 3} #{2 3 4} #{3 5}) "
                      + " (set/difference #{1 2 3} #{2} #{3}) "
                      + " (set/subset? #{1 2} #{1 2 3}) "
                      + " (set/superset? #{1 2 3} #{1 2}) "
                      + " (set/select odd? #{1 2 3 4})])")
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
