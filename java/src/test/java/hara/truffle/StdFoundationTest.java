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
  public void startupDefaultsExposeEdnNativeTypesAndProtocols() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[\"{:a 1}\" true true true true true 3 true true true]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns startup.defaults)"
                      + " [(edn/write {:a 1})"
                      + "  (= Maths std.native/Maths std.foundation/Maths)"
                      + "  (= Edn std.native/Edn std.foundation/Edn)"
                      + "  (= Json std.native/Json std.foundation/Json)"
                      + "  (= Arr std.native/Arr std.foundation/Arr)"
                      + "  (= Obj std.native/Obj std.foundation/Obj)"
                      + "  (ICount/count [1 2 3])"
                      + "  (iter-any? (fn [x] (= x \"edn/pretty\")) (current-symbols))"
                      + "  (iter-any? (fn [x] (= x \"Maths\")) (current-symbols))"
                      + "  (every? (fn [type] (not (nil? (resolve type))))"
                      + "    '[Maths Numbers Bits String Bytes File Socket Promise Coroutine"
                      + "      Arr Obj Runtime Printer Edn Json Regex UUID Error])]")
              .toString());
    }
  }

  @Test
  public void nativeTypesAreDescriptorsAndFoundationLibrariesAreHalWrappers() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).allowAllAccess(true).build()) {
      assertEquals(
          "[\"#<native-type std.native/Maths>\" \"Maths\" \"std.native\" true 0.0 \"HARA\" \"HARA\" 255 255]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(str std.native/Maths)"
                      + " (INamespaced/name std.native/Maths)"
                      + " (INamespaced/namespace std.native/Maths)"
                      + " (= std.native/Maths (with-meta std.native/Maths {:doc \"math\"}))"
                      + " (std.native.Maths/sin 0)"
                      + " (std.native.String/upper \"hara\")"
                      + " (str/upper \"hara\")"
                      + " (std.native.Bytes/u8 -1)"
                      + " (bytes/u8 -1)]")
              .toString());
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.native/Maths 1)"));
      assertEquals(
          "[255 3]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(std.foundation.bytes/get (bytes -1) 0)"
                      + " (std.foundation.bytes/count (bytes 1 2 3))]")
              .toString());
      PolyglotException unavailable =
          assertThrows(
              PolyglotException.class,
              () ->
                  context.eval(
                      HaraLanguage.ID,
                      "(ns legacy.activation (:config {:builtins [inc]}))"));
      assertTrue(
          unavailable
              .getMessage()
              .contains("No host builtins are registered for namespace: legacy.activation"));
      assertEquals(
          "2",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns std.foundation (:config {:builtins [count]}))"
                      + " (std.foundation/count [41 42])")
              .toString());
    }
  }

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
  public void fallbackSourceDocumentsNativeFoundationVars() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[\"Returns the portable character count of value.\" [[value]]"
              + " [:fn [:str] :int]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.string)"
                      + " (let [m (meta #'std.foundation.string/length)]"
                      + "   [(get m :doc) (get m :arglists) (get m :schema)]))")
              .toString());
      assertEquals(
          "4",
          context.eval(HaraLanguage.ID, "(std.foundation.string/length \"hara\")").toString());
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
  public void basicMathHasThePortableRootSurfaceAndExplicitNumericBoundary() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[true true 0.0 1.0 0.0 0.0 0.0 0.0 0.0 0.0 1.0 0.0 0.0 0.0 0.0 1.0 2.0 8.0 3 1.0 3.0]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(= E 2.718281828459045) (= PI 3.141592653589793) "
                      + "(sin 0) (cos 0) (tan 0) (asin 0) (acos 1) (atan 0) "
                      + "(atan2 0 1) (sinh 0) (cosh 0) (tanh 0) "
                      + "(asinh 0) (acosh 1) (atanh 0) "
                      + "(floor 1.75) (ceil 1.25) (pow 2 3) (abs -3) "
                      + "(exp 0) (sqrt 9)]")
              .toString());
      assertEquals("NaN", context.eval(HaraLanguage.ID, "(sqrt -1)").toString());
      assertEquals(3.0, context.eval(HaraLanguage.ID, "(sqrt (long 9.9))").asDouble(), 0.0);
      assertEquals(3.0, context.eval(HaraLanguage.ID, "(sqrt (double 9))").asDouble(), 0.0);
      assertTrue(Double.isFinite(context.eval(HaraLanguage.ID, "(asinh 1.0e300)").asDouble()));
      assertTrue(Double.isFinite(context.eval(HaraLanguage.ID, "(acosh 1.0e300)").asDouble()));
      for (String source : new String[] {"(sin)", "(pow 2)", "(sqrt \"9\")"}) {
        assertThrows(source, PolyglotException.class, () -> context.eval(HaraLanguage.ID, source));
      }
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
