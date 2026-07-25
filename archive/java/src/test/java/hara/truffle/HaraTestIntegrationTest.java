package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Value;
import org.junit.Test;

public class HaraTestIntegrationTest {
  @Test
  public void requiresJavaBackedNamespaceWithQuotedSymbol() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      Value result = context.eval(HaraLanguage.ID, "(code.test/assert! 1 1)");
      assertTrue(result.asBoolean());
    }
  }

  @Test
  public void registersAndRunsAJavaBackedFactMacro() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(fact \"addition\" (+ 1 2) => 3)");
      Value results = context.eval(HaraLanguage.ID, "(code.test/run)");
      assertTrue(results.hasArrayElements());
      assertEquals(1, results.getArraySize());
      Value result = results.getArrayElement(0);
      assertTrue(result.hasHashEntries());
      assertEquals("PASS", result.getHashValue("status").asString());
    }
  }

  @Test
  public void registersDocumentationOnlyFacts() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(fact \"documented behavior\")");
      Value results =
          context.eval(
              HaraLanguage.ID,
              "(code.test/run {:namespace \"user\" :name \"documented behavior\"})");
      assertEquals(1, results.getArraySize());
      assertEquals("PASS", results.getArrayElement(0).getHashValue("status").asString());
    }
  }

  @Test
  public void supportsMatchersFixturesAndFilteredReports() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test {:refer [anything contains throws use-fixtures]})");
      context.eval(HaraLanguage.ID, "(use-fixtures :before (fn [] true))");
      context.eval(HaraLanguage.ID, "(fact \"map subset\" {:a 1 :b 2} => (contains {:a 1}))");
      context.eval(HaraLanguage.ID, "(fact \"any value\" (+ 1 1) => anything)");
      context.eval(HaraLanguage.ID, "(fact \"throws\" (/ 1 0) => (throws))");
      context.eval(HaraLanguage.ID, "(fact \"bare throws\" (/ 1 0) => throws)");
      Value results = context.eval(HaraLanguage.ID, "(code.test/run {:filter \"user/throws\"})");
      assertEquals(1, results.getArraySize());
      assertEquals("PASS", results.getArrayElement(0).getHashValue("status").asString());
      Value bareResults =
          context.eval(HaraLanguage.ID, "(code.test/run {:filter \"user/bare throws\"})");
      assertEquals(1, bareResults.getArraySize());
      assertEquals("PASS", bareResults.getArrayElement(0).getHashValue("status").asString());
    }
  }

  @Test
  public void supportsTheBroaderCodeTestCheckerFamily() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/assert! [1 2] (code.test/just [1 2]))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/assert! 1.0005 (code.test/approx 1.0))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/assert! 2 (code.test/satisfies 2))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/assert! 2 (code.test/any 1 2))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/assert! 2 (code.test/all (code.test/satisfies 2) (code.test/approx 2)))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/assert! 2 (code.test/is-not 3))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/succeeded? (code.test/verify (code.test/exactly 2) 2))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/checker? (code.test/just [1]))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID,
          "(code.test/assert! {:a {:b 2}} (code.test/just-in {:a {:b 2}}))").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID,
          "(code.test/succeeded? (code.test/verify (code.test/just-in {:a {:b 2}}) {:a {:b 2 :c 3}}))").asBoolean());
    }
  }

  @Test
  public void supportsReferAllAndExcludedSymbols() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID,
          "(require 'code.test {:refer :all :exclude [contains]})");
      assertTrue(context.eval(HaraLanguage.ID, "(assert! 2 2)").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(assert! 1 anything)").asBoolean());
    }
  }

  @Test
  public void exposesFactThroughCodeTestReferCompatibility() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test {:refer [fact]})");
      context.eval(HaraLanguage.ID, "(fact \"referred-fact\" 3 => 3)");
      assertEquals(1, context.eval(HaraLanguage.ID, "(code.test/run {:name \"referred-fact\"})").getArraySize());
    }
  }

  @Test
  public void exposesOnlyTheCanonicalStaticLibraryNamespaces() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      assertEquals(
          "Runs registered facts.",
          context.eval(HaraLanguage.ID, "(get (meta (var code.test/run)) :doc)").asString());
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(require 'code.test.checker.common)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(require 'code.test.base.runtime)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(var code.test/run!)"));

      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(require 'std.task)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(require 'std.lib.task.bulk)"));
    }
  }

  @Test
  public void filtersFactsByInvocationMetadata() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      context.eval(HaraLanguage.ID, "^{:tag \"fast\"} (fact \"tagged\" 1 => 1)");
      context.eval(HaraLanguage.ID, "(fact \"other\" 1 => 1)");
      Value results = context.eval(HaraLanguage.ID,
          "(code.test/run {:metadata {:tag \"fast\"}})");
      assertEquals(1, results.getArraySize());
      assertEquals("tagged", results.getArrayElement(0).getHashValue("name").asString());
    }
  }

  @Test
  public void exposesFactRegistryLifecycleAndGlobalApis() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      context.eval(HaraLanguage.ID, "(fact \"registry-entry\" 1 => 1)");
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/get-fact \"registry-entry\")").hasHashEntries());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/get-flag \"user\" :setup)").asBoolean() == false);
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/set-flag \"user\" :setup true)").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/get-flag \"user\" :setup)").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/set-global {:answer 42})").hasHashEntries());
      assertEquals(42, context.eval(HaraLanguage.ID, "(code.test/get-global :answer)").asInt());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/remove-fact \"registry-entry\")").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/get-fact \"registry-entry\")").isNull());
    }
  }

  @Test
  public void supportsFactScopedSetupFlags() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      context.eval(HaraLanguage.ID, "(fact \"flagged\" 1 => 1)");
      assertTrue(!context.eval(HaraLanguage.ID, "(code.test/get-flag \"flagged\" :setup)").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/set-flag \"flagged\" :setup true)").asBoolean());
      assertTrue(context.eval(HaraLanguage.ID, "(code.test/get-flag \"flagged\" :setup)").asBoolean());
      context.eval(HaraLanguage.ID,
          "(code.test/set-flag \"user\" \"flagged\" :setup false)");
      assertTrue(!context.eval(HaraLanguage.ID,
          "(code.test/get-flag \"user\" \"flagged\" :setup)").asBoolean());
    }
  }

  @Test
  public void createsAndInvokesAStdTask() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      context.eval(HaraLanguage.ID, "(def add-task (std.lib.task/task :default \"add\" (fn [a b] (+ a b))))");
      assertTrue(context.eval(HaraLanguage.ID, "(std.lib.task/task? add-task)").asBoolean());
      assertEquals(5, context.eval(HaraLanguage.ID, "(std.lib.task/invoke add-task 2 3)").asInt());
    }
  }

  @Test
  public void definesTasksWithDeftaskAndRunsBulkInputs() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      context.eval(HaraLanguage.ID,
          "(deftask double-task {:template :default :doc \"doubles\" :main {:fn (fn [x] (* 2 x))}})");
      assertTrue(context.eval(HaraLanguage.ID, "(std.lib.task/task? double-task)").asBoolean());
      assertEquals("doubles", context.eval(HaraLanguage.ID, "(get (meta (var double-task)) :doc)").asString());
      Value values = context.eval(HaraLanguage.ID, "(std.lib.task/invoke double-task [1 2 3])");
      assertEquals(3, values.getArraySize());
      assertEquals(4, values.getArrayElement(1).asInt());
    }
  }

  @Test
  public void exposesTaskProcessAndBulkApis() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      context.eval(HaraLanguage.ID, "(def bulk-task (std.lib.task/task :default \"double\" (fn [x] (* 2 x))))");
      Value items = context.eval(HaraLanguage.ID, "(std.lib.task/bulk-items bulk-task [1 2 3])");
      assertEquals(3, items.getArraySize());
      Value summary = context.eval(HaraLanguage.ID, "(std.lib.task/bulk-summary (std.lib.task/bulk-items bulk-task [1 2 3]))");
      assertEquals(3, summary.getHashValue("results").asInt());
      assertTrue(context.eval(HaraLanguage.ID, "(std.lib.task/select-filter \"code\" \"code.test\")").asBoolean());
    }
  }

  @Test
  public void processesStructuredTasksAndVectorInputs() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      context.eval(HaraLanguage.ID,
          "(def process-task (std.lib.task/task :default \"process\" {:main {:fn (fn [input params lookup env] (* 2 input))}}))");
      Value result = context.eval(HaraLanguage.ID, "(std.lib.task/invoke process-task [2 3])");
      assertEquals(2, result.getArraySize());
      assertEquals(6, result.getArrayElement(1).asInt());
    }
  }

  @Test
  public void exposesSourceStyleBulkFunctionSignatures() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      Value items = context.eval(HaraLanguage.ID,
          "(std.lib.task/bulk-items (fn [input params lookup env] (* 2 input)) "
              + "[1 2] {} {} {})");
      assertEquals(2, items.getArraySize());
      assertTrue(items.getArrayElement(0).hasArrayElements());
      assertEquals(1, items.getArrayElement(0).getArrayElement(0).asInt());
    }
  }

  @Test
  public void processesSourceStyleBulkItemsWithExtraArgumentsAndStatusHelpers() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      context.eval(HaraLanguage.ID,
          "(def bulk-item (std.lib.task/bulk-process-item "
              + "(fn [input params lookup env extra] [input {:status :return :data (+ input extra)}]) "
              + "{:idx 0 :total 1 :input 2} {} {} {} 5))");
      Value item = context.eval(HaraLanguage.ID, "bulk-item");
      assertEquals(2, item.getArrayElement(0).asInt());
      assertEquals(":return", context.eval(HaraLanguage.ID, "(get (nth bulk-item 1) :status)").toString());
      assertEquals(7, context.eval(HaraLanguage.ID, "(get (nth bulk-item 1) :data)").asInt());
      assertTrue(context.eval(HaraLanguage.ID, "(get (nth bulk-item 1) :time)").fitsInLong());

      Value errors = context.eval(HaraLanguage.ID,
          "(std.lib.task/bulk-errors {} [[1 {:status :error}] [2 {:status :return}]])");
      assertEquals(1, errors.getArraySize());
      assertEquals(1, errors.getArrayElement(0).getArrayElement(0).asInt());
    }
  }

  @Test
  public void packagesBulkResultsInReferenceShape() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.task)");
      context.eval(HaraLanguage.ID, "(def bulk-task (std.lib.task/task :default \"bulk\" (fn [x] x)))");
      Value packaged = context.eval(HaraLanguage.ID,
          "(std.lib.task/bulk-package "
              + "{:items [[1 {:status :return :data 1}]] "
              + ":results [{:key 1 :data 1}] :warnings [] :errors [] :summary {}} "
              + ":all :map)");
      assertTrue(packaged.hasHashEntry("items"));
      assertEquals(1, packaged.getHashValue("items").getHashValue(1L).asInt());
      assertEquals(1, packaged.getHashValue("results").getHashValue(1L).asInt());

      Value sourcePackaged = context.eval(HaraLanguage.ID,
          "(std.lib.task/bulk-package (std.lib.task/task :default \"bulk\" (fn [x] x)) "
              + "{:items [[1 {:status :return :data 1}]] "
              + ":results [{:key 1 :data 1}] :warnings [] :errors [] :summary {}} :all :map)");
      assertTrue(sourcePackaged.hasHashEntry("summary"));
      Value summary = context.eval(HaraLanguage.ID,
          "(std.lib.task/bulk-summary (std.lib.task/task :default \"bulk\" (fn [x] x)) {} "
              + "[[1 {:status :return :time 2 :data 1}]] "
              + "[{:key 1 :data 1}] [] [] 10)");
      assertEquals(1, summary.getHashValue("items").asInt());
      assertEquals(2, summary.getHashValue("cumulative").asInt());
    }
  }
}
