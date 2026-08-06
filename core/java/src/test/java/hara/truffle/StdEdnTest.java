package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;

import org.graalvm.polyglot.Context;
import org.junit.Test;

public class StdEdnTest {
  @Test
  public void readsAndWritesRestrictedEdnThroughHalWrappers() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "{:a [1 2] :b #{:x}}",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.edn)"
                      + " (std.foundation.edn/read \"{:a [1 2] :b #{:x}}\"))")
              .toString());
      assertEquals(
          "{:a [1 2]}",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.edn)"
                      + " (std.foundation.edn/write {:a [1 2]}))")
              .asString());
      assertEquals(
          "[:a 1]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.edn)"
                      + " (std.foundation.edn/pretty [:a 1] {}))")
              .asString());
      assertThrows(
          RuntimeException.class,
          () ->
              context.eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.edn)"
                      + " (std.foundation.edn/pretty [:a 1] nil))"));
      assertEquals(
          "(+ 1 2)",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.edn)"
                      + " (std.foundation.edn/read \"(+ 1 2)\"))")
              .toString());
      assertEquals(
          "[\"bad input\" {:kind :invalid}]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(try"
                      + " (throw (ex-info \"bad input\" {:kind :invalid}))"
                      + " (catch Throwable error"
                      + "   [(ex-message error) (ex-data error)]))")
              .toString());
      assertEquals(
          "{:kind :invalid}",
          context
              .eval(
                  HaraLanguage.ID,
                  "(IExInfo/data"
                      + " (ex-info \"bad input\" {:kind :invalid}))")
              .toString());
    }
  }

  @Test
  public void rejectsUnsupportedNumbersAndMultipleValues() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      for (String source : new String[] {"1/2", "1 2"}) {
        assertThrows(
            source,
            RuntimeException.class,
            () ->
                context.eval(
                    HaraLanguage.ID,
                    "(do (require 'std.foundation.edn)"
                        + " (std.foundation.edn/read \""
                        + source
                        + "\"))"));
      }
      assertThrows(
          RuntimeException.class,
          () ->
              context.eval(
                  HaraLanguage.ID,
                  "(do (require 'std.foundation.edn) (std.foundation.edn/read \"1N\"))"));
    }
  }
}
