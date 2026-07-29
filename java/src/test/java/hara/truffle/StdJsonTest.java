package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdJsonTest {
  @Test
  public void strictJsonReadsWritesAndPrettyPrints() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "[nil true -2 \"x\" [3] {\"a\" 4}]",
          context
              .eval(HaraLanguage.ID, "(std.foundation.json/read \"[null,true,-2,\\\"x\\\",[3],{\\\"a\\\":4}]\")")
              .toString());
      assertEquals(
          "{\"a\":1,\"b\":[true,null]}",
          context.eval(HaraLanguage.ID, "(std.foundation.json/write {\"a\" 1 \"b\" [true nil]})").asString());
      assertEquals(
          "{\"a\":1}",
          context.eval(HaraLanguage.ID, "(std.native.Json/write {\"a\" 1})").asString());
      assertEquals(
          "{\n  \"a\": 1\n}",
          context.eval(HaraLanguage.ID, "(std.foundation.json/pretty {\"a\" 1} {})").asString());
      assertThrows(
          RuntimeException.class,
          () -> context.eval(HaraLanguage.ID, "(std.foundation.json/pretty {\"a\" 1} nil)"));
    }
  }

  @Test
  public void strictJsonRejectsUnsupportedFormsAndPrettyUsesReadablePrinter() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertThrows(
          RuntimeException.class,
          () -> context.eval(HaraLanguage.ID, "(std.foundation.json/read \"1.5\")"));
      assertThrows(
          RuntimeException.class,
          () -> context.eval(HaraLanguage.ID, "(std.foundation.json/write {:a 1})"));
      assertEquals(
          "{:a [1 2]}",
          context.eval(HaraLanguage.ID, "(do (require 'std.pretty) (std.pretty/pprint-str {:a [1 2]}))").asString());
    }
  }
}
