package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

public class HaraGeneratedLibrariesTest {
  @Test
  public void generatedLibrariesHaveDefaultAliasesWithoutLoadingFiles() {
    try (Context context = context()) {
      assertEquals("hara", context.eval(HaraLanguage.ID, "(str/trim \"  hara  \")").asString());
      assertEquals(255, context.eval(HaraLanguage.ID, "(bytes/get (bytes -1) 0)").asLong());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(promise/native? (promise/new (fn [resolve reject] (resolve 1))))")
              .asBoolean());
    }
  }

  @Test
  public void intrinsicsAllExplicitlyKeepsEveryDefaultAlias() {
    try (Context context = context()) {
      assertEquals(
          1,
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns app (:config {:intrinsics :all})) (bytes/count (str/encode \"x\"))")
              .asLong());
    }
  }

  @Test
  public void promisesRunAndAdoptCallbackPromises() {
    try (Context context = context()) {
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(deref (promise/then (promise/run (fn [] 40)) "
                      + "(fn [x] (promise/run (fn [] (+ x 2))))))")
              .asLong());
      assertEquals(
          7,
          context
              .eval(
                  HaraLanguage.ID,
                  "(deref (promise/catch "
                      + "(promise/run (fn [] (throw \"bad\"))) (fn [error] 7)))")
              .asLong());
      assertEquals(
          3,
          context
              .eval(
                  HaraLanguage.ID,
                  "(nth (deref (promise/all "
                      + "[(promise/run (fn [] 1)) (promise/run (fn [] 3))])) 1)")
              .asLong());
      assertEquals(
          9,
          context
              .eval(HaraLanguage.ID, "(deref (promise/run (fn [] (promise/run (fn [] 9)))))")
              .asLong());
      assertEquals(
          4,
          context
              .eval(
                  HaraLanguage.ID,
                  "(deref (promise/finally (promise/run (fn [] 4)) "
                      + "(fn [] (promise/run (fn [] 99)))))")
              .asLong());
      assertEquals(
          8,
          context
              .eval(
                  HaraLanguage.ID,
                  "(deref (promise/catch "
                      + "(promise/new (fn [resolve reject] (reject \"bad\"))) "
                      + "(fn [error] 8)))")
              .asLong());
      assertEquals(
          11,
          context
              .eval(
                  HaraLanguage.ID,
                  "(deref (promise/catch "
                      + "(promise/finally (promise/run (fn [] (throw \"original\"))) (fn [] nil)) "
                      + "(fn [error] 11)))")
              .asLong());
    }
  }

  @Test
  public void promisesExposePortableStateValueAndCancellation() {
    try (Context context = context()) {
      assertEquals(
          ":fulfilled",
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [p (promise/new (fn [resolve reject] (resolve 42)))] "
                      + "(str (promise/state p)))")
              .asString());
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(promise/value (promise/new (fn [resolve reject] (resolve 42))))")
              .asLong());
      assertEquals(
          ":cancelled",
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [p (promise/delay 10000 (fn [] 1))] "
                      + "(promise/cancel p) (str (promise/state p)))")
              .asString());
    }
  }

  @Test
  public void stringLibraryMatchesTheXtalkSurface() {
    try (Context context = context()) {
      assertEquals(4, context.eval(HaraLanguage.ID, "(str/len \"hara\")").asLong());
      assertEquals(-1, context.eval(HaraLanguage.ID, "(str/comp \"a\" \"b\")").asLong());
      assertTrue(context.eval(HaraLanguage.ID, "(str/lt? \"a\" \"b\")").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(str/gt? \"b\" \"a\")").asBoolean());
      assertEquals("00x", context.eval(HaraLanguage.ID, "(str/pad-left \"x\" 3 \"0\")").asString());
      assertEquals(
          "x00", context.eval(HaraLanguage.ID, "(str/pad-right \"x\" 3 \"0\")").asString());
      assertTrue(context.eval(HaraLanguage.ID, "(str/starts-with? \"hara\" \"ha\")").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(str/ends-with? \"hara\" \"ra\")").asBoolean());
      assertEquals("a", context.eval(HaraLanguage.ID, "(str/char \"hara\" 1)").asString());
      assertEquals(
          "b", context.eval(HaraLanguage.ID, "(nth (str/split \"a,b\" \",\") 1)").asString());
      assertEquals(
          "a,b", context.eval(HaraLanguage.ID, "(str/join \",\" [\"a\" \"b\"])").asString());
      assertEquals(3, context.eval(HaraLanguage.ID, "(str/index-of \"ababa\" \"ba\" 2)").asLong());
      assertEquals("ar", context.eval(HaraLanguage.ID, "(str/substring \"hara\" 1 3)").asString());
      assertEquals("HARA", context.eval(HaraLanguage.ID, "(str/to-upper \"hara\")").asString());
      assertEquals("hara", context.eval(HaraLanguage.ID, "(str/to-lower \"HARA\")").asString());
      assertEquals("1.25", context.eval(HaraLanguage.ID, "(str/to-fixed 1.25 2)").asString());
      assertEquals(
          "hxrx", context.eval(HaraLanguage.ID, "(str/replace \"hara\" \"a\" \"x\")").asString());
      assertEquals("x", context.eval(HaraLanguage.ID, "(str/trim \" x \")").asString());
      assertEquals("x ", context.eval(HaraLanguage.ID, "(str/trim-left \" x \")").asString());
      assertEquals(" x", context.eval(HaraLanguage.ID, "(str/trim-right \" x \")").asString());
      assertEquals(
          "hé", context.eval(HaraLanguage.ID, "(str/decode (str/encode \"hé\"))").asString());
    }
  }

  @Test
  public void strIsVariadicAndMatchesJvmConcatenation() {
    try (Context context = context()) {
      assertEquals("", context.eval(HaraLanguage.ID, "(str)").asString());
      assertEquals("1", context.eval(HaraLanguage.ID, "(str 1)").asString());
      assertEquals("123", context.eval(HaraLanguage.ID, "(str 1 2 3)").asString());
      assertEquals("ab", context.eval(HaraLanguage.ID, "(str \"a\" nil \"b\")").asString());
      assertEquals(":ok", context.eval(HaraLanguage.ID, "(str :ok)").asString());
    }
  }

  @Test
  public void prStrUsesReadableHaraNotation() {
    try (Context context = context()) {
      assertEquals("\"hara\"", context.eval(HaraLanguage.ID, "(pr-str \"hara\")").asString());
      assertEquals(
          "{:a 1}", context.eval(HaraLanguage.ID, "(pr-str {:a 1})").asString());
    }
  }

  @Test
  public void nestedLookupDoesNotConsumeItsPath() {
    try (Context context = context()) {
      assertEquals(
          ",",
          context
              .eval(HaraLanguage.ID, "(get-in {:default {:common {:sep \",\"}}} [:default :common :sep])")
              .asString());
    }
  }

  @Test
  public void emitterTypePredicatesAreAvailable() {
    try (Context context = context()) {
      assertTrue(context.eval(HaraLanguage.ID, "(char? \\a)").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(list? '(a b))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(not (list? '[a b]))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(map-entry? (first {:a 1}))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(not (uuid? :a))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(not (regexp? :a))").asBoolean());
    }
  }

  @Test
  public void intrinsicsCanExcludeAndRenameGeneratedAliases() {
    try (Context context = context()) {
      assertEquals(
          "HARA",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns app (:config {:intrinsics {:exclude [bytes] :alias {string text}}})) "
                      + "(text/to-upper \"hara\")")
              .asString());
      PolyglotException missing =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(bytes/count (bytes 1))"));
      assertTrue(missing.getMessage().contains("Unbound symbol: bytes/count"));
      assertEquals("x", context.eval(HaraLanguage.ID, "(str \"x\")").asString());
    }
  }

  @Test
  public void generatedLibrariesAlsoSupportRequireAsAndRefer() {
    try (Context context = context()) {
      assertEquals(
          "x",
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns app (:config {:intrinsics {:exclude [string]}}) "
                      + "(:require [std.foundation.string :as text :refer [trim]])) "
                      + "(trim (text/trim \" x \"))")
              .asString());
    }
  }

  @Test
  public void foundationNamespaceCombinesJavaAndHalSymbols() {
    try (Context context = context()) {
      assertEquals(
          -1,
          context
              .eval(
                  HaraLanguage.ID, "(ns app (:require [std.foundation :as core])) (core/bit-not 0)")
              .asLong());
      assertEquals(
          1,
          context.eval(HaraLanguage.ID, "(std.foundation/count [1])").asLong());
      assertEquals(
          42,
          context.eval(HaraLanguage.ID, "((std.foundation/comp2 inc inc) 40)").asLong());
    }
  }

  @Test
  public void intrinsicsRejectUnknownConflictingAndDuplicateConfiguration() {
    try (Context context = context()) {
      assertErrorContains(
          context,
          "(ns a (:config {:intrinsics {:exclude [unknown]}}))",
          "Unknown intrinsic library");
      assertErrorContains(
          context,
          "(ns b (:config {:intrinsics {:exclude [bytes] :alias {bytes data}}}))",
          "both excluded and aliased");
      assertErrorContains(
          context,
          "(ns c (:config {:intrinsics {:alias {string data bytes data}}}))",
          "Duplicate intrinsic alias target");
      assertErrorContains(
          context, "(ns d (:config {}) (:config {}))", "only one :config clause");
      assertErrorContains(
          context,
          "(ns e (:config {:intrinsics {:unexpected true}}))",
          "Unsupported :config :intrinsics option");
    }
  }

  @Test
  public void completionIncludesGeneratedAliasesAndMarkerMethods() {
    try (Context context = context()) {
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(iter-any? (fn [x] (= x \"str/trim\")) (current-symbols))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(iter-any? (fn [x] (= x \"push-last\")) (current-symbols))")
              .asBoolean());
    }
  }

  @Test
  public void dotCallsAreRestrictedToMarkedArraysAndObjects() {
    try (Context context = context()) {
      assertEquals(
          6,
          context
              .eval(HaraLanguage.ID, "(. (array 1 2 3) (fold-left (fn [out x] (+ out x)) 0))")
              .asLong());
      assertEquals(
          3,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [a (array 1 2 3 4)] (. (. a (filter (fn [x] (> x 2)))) (get 0)))")
              .asLong());
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [o (object \"answer\" 41)] (. o (set \"answer\" 42)) "
                      + "(. o (get \"answer\")))")
              .asLong());
      PolyglotException denied =
          assertThrows(
              PolyglotException.class, () -> context.eval(HaraLanguage.ID, "(. [1 2] (get 0))"));
      assertTrue(
          denied.getMessage().contains("only supported on values created by array or object"));
    }
  }

  @Test
  public void bitOperationsUseSignedThirtyTwoBitSemantics() {
    try (Context context = context()) {
      assertEquals(2, context.eval(HaraLanguage.ID, "(bit-and 6 3)").asLong());
      assertErrorContains(context, "(bit-and 7 3 1)", "expects two integers");
      assertErrorContains(context, "(bit-or 1 2 4)", "expects two integers");
      assertErrorContains(context, "(bit-xor 1 2 4)", "expects two integers");
      assertEquals(-1, context.eval(HaraLanguage.ID, "(bit-not 0)").asLong());
      assertEquals(-2, context.eval(HaraLanguage.ID, "(bit-shift-right -4 1)").asLong());
      assertEquals(-2147483648L, context.eval(HaraLanguage.ID, "(bit-shift-left 1 31)").asLong());
      assertEquals(1, context.eval(HaraLanguage.ID, "(bit-shift-left 1 0)").asLong());
      assertErrorContains(context, "(bit-shift-left 1 -1)", "distance must be in the range 0..31");
      assertErrorContains(context, "(bit-shift-right 1 32)", "distance must be in the range 0..31");
    }
  }

  private static Context context() {
    return Context.newBuilder(HaraLanguage.ID).build();
  }

  private static void assertErrorContains(Context context, String source, String message) {
    PolyglotException error =
        assertThrows(PolyglotException.class, () -> context.eval(HaraLanguage.ID, source));
    assertTrue(error.getMessage().contains(message));
  }
}
