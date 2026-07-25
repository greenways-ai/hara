package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class PolisSourceTest {
  @Test
  public void loadsPackagedProvenanceNamespace() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.provenance)");
      assertEquals(
          "demo.core",
          context
              .eval(
                  HaraLanguage.ID,
                  "(polis.common.provenance/module-id {:id 'demo.core})")
              .toString());
    }
  }

  @Test
  public void runsTranslatedProvenanceFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.provenance-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.provenance-test\"})");
      assertEquals(12, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedUtilityFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.util-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.util-test\"})");
      assertEquals(17, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedGrammarSpecFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.grammar-spec-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.grammar-spec-test\"})");
      assertEquals(6, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedGrammarMacroFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.grammar-macro-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.grammar-macro-test\"})");
      assertEquals(13, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedPreprocessBaseFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.preprocess-base-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.preprocess-base-test\"})");
      assertEquals(4, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedGrammarXtalkFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.grammar-xtalk-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.grammar-xtalk-test\"})");
      assertEquals(52, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedGrammarXtalkSystemFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.grammar-xtalk-system-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.grammar-xtalk-system-test\"})");
      assertEquals(24, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedGrammarFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.grammar-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.grammar-test\"})");
      assertEquals(19, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedEmitHelperFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.emit-helper-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.emit-helper-test\"})");
      assertEquals(12, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
      var metadata =
          context.eval(
              HaraLanguage.ID,
              "(let [metadata (meta (var polis.common.emit-helper/emit-typed-args))]"
                  + " [(:doc metadata) (:added metadata) (count (:arglists metadata))])");
      assertEquals(
          "create types args from declarationns",
          metadata.getArrayElement(0).asString());
      assertEquals("3.0", metadata.getArrayElement(1).asString());
      assertEquals(2, metadata.getArrayElement(2).asLong());
    }
  }

  @Test
  public void runsJavaBackedStdLibWalkFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.walk-test)");
      var equality =
          context.eval(
              HaraLanguage.ID,
              "(let [actual (std.lib.walk/keyword-spearify-keys"
                  + " {\"a_b_c\" [{\"e_f_g\" 1}]})"
                  + " expected {:a-b-c [{:e-f-g 1}]}]"
                  + " [(= actual expected)"
                  + " (= (:a-b-c actual) (:a-b-c expected))"
                  + " (= (first (:a-b-c actual)) (first (:a-b-c expected)))"
                  + " (= (:e-f-g (first (:a-b-c actual))) 1)])");
      assertTrue(equality.toString(), equality.getArrayElement(3).asBoolean());
      assertTrue(equality.toString(), equality.getArrayElement(2).asBoolean());
      assertTrue(equality.toString(), equality.getArrayElement(1).asBoolean());
      assertTrue(equality.toString(), equality.getArrayElement(0).asBoolean());
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"std.lib.walk-test\"})");
      assertEquals(13, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedPreprocessInputFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.preprocess-input-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.preprocess-input-test\"})");
      assertEquals(3, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedBookEntryFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.book-entry-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.book-entry-test\"})");
      assertEquals(2, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }

  @Test
  public void runsTranslatedBookMetaFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'polis.common.book-meta-test)");
      var results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"polis.common.book-meta-test\"})");
      assertEquals(2, results.getArraySize());
      for (long i = 0; i < results.getArraySize(); i++) {
        assertTrue(
            results.getArrayElement(i).toString(),
            "PASS".equals(results.getArrayElement(i).getHashValue("status").asString()));
      }
    }
  }
}
