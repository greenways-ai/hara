package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

public class HaraGeneratedLibrariesTest {
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
      assertTrue(context.eval(HaraLanguage.ID, "(fn? (fn [value] value))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(not (fn? :a))").asBoolean());
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
  public void namespaceUseLoadsAndRefersVarsAndMacros() {
    try (Context context = context()) {
      assertEquals(
          84,
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns demo.use-lib) "
                      + "(def answer 42) "
                      + "(defmacro twice [form] `(+ ~form ~form)) "
                      + "(ns demo.use-app (:use demo.use-lib)) "
                      + "(twice answer)")
              .asLong());
      assertErrorContains(
          context,
          "(ns demo.bad-use (:use [demo.use-lib]))",
          ":use expects unqualified namespace symbols");
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
          context.eval(HaraLanguage.ID, "((std.foundation/comp inc inc) 40)").asLong());
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
