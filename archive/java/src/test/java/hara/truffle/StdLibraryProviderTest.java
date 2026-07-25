package hara.truffle;

import java.lang.reflect.Method;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

public class StdLibraryProviderTest {
  @Test
  public void loadsBlockThroughTheLibraryProvider() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.block)");
      assertEquals("abc", context.eval(HaraLanguage.ID, "(std.lib.block/parse \"abc\")").toString());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(std.lib.block/parse-root \"a b\")")
              .toString()
              .contains("a b"));
      context.eval(HaraLanguage.ID, "(std.lib.block/parse-first \"  a b\")");
      assertEquals(
          "Parses the first source-preserving block.",
          context
              .eval(HaraLanguage.ID, "(get (meta (var std.lib.block/parse)) :doc)")
              .asString());
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.lib.block/parse)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.lib.block/parse \"a\" \"b\")"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(var std.lib.block/grid)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(require 'std.block)"));
    }
  }

  @Test
  public void loadsZipThroughTheLibraryProvider() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.zip)");
      context.eval(
          HaraLanguage.ID,
          "(std.lib.zip/step-inside (std.lib.zip/zipper [1 2]))");
      context.eval(
          HaraLanguage.ID,
          "(-> [1 [2]] std.lib.zip/zipper std.lib.zip/step-inside "
              + "std.lib.zip/step-right std.lib.zip/step-inside "
              + "std.lib.zip/step-outside std.lib.zip/step-left)");
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.lib.zip/step-left 1)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.lib.zip/zipper)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.lib.zip/step-right)"));
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(var std.lib.zip/remove)"));
    }
  }

  @Test
  public void requireAliasesResolveToProviderBackedNamespaces() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns provider.aliases "
              + "(:require [std.lib.block :as block] [std.lib.zip :as zip]))");
      assertEquals("abc", context.eval(HaraLanguage.ID, "(block/parse \"abc\")").toString());
      assertTrue(context.eval(HaraLanguage.ID, "(zip/zipper [1])").toString().startsWith("#zip"));
    }
  }

  @Test
  public void loadsContextThroughTheLibraryProvider() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.lib.context)");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(std.lib.context/rt-null? "
                      + "(std.lib.context/registry-scratch :null))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(std.lib.context/space? (std.lib.context/space))")
              .asBoolean());
      context.eval(
          HaraLanguage.ID,
          "(std.lib.context/space:context-set :null :default {})");
      assertFalse(
          context
              .eval(HaraLanguage.ID, "(std.lib.context/space:rt-started? :null)")
              .asBoolean());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(std.lib.context/rt-null? "
                      + "(std.lib.context/space:rt-start :null))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(std.lib.context/space:rt-started? :null)")
              .asBoolean());
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [ptr (std.lib.context/pointer {:context :null :value 42})]"
                      + " (get (std.lib.context/pointer-deref ptr) :value))")
              .asInt());
      context.eval(HaraLanguage.ID, "(std.lib.context/space:rt-stop :null)");
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(std.lib.context/space:rt-stopped? :null)")
              .asBoolean());
      assertEquals(
          "Returns the context space for the current or supplied namespace.",
          context
              .eval(HaraLanguage.ID, "(get (meta (var std.lib.context/space)) :doc)")
              .asString());
      assertTrue(
          context
                  .eval(
                      HaraLanguage.ID,
                      "(count (get (meta (var std.lib.context/space)) :arglists))")
                  .asInt()
              > 0);
    }
  }

  @Test
  public void contextProviderKeepsStateIsolatedAndHidesImplementationNamespaces() {
    try (Context first = Context.newBuilder(HaraLanguage.ID).build();
        Context second = Context.newBuilder(HaraLanguage.ID).build()) {
      first.eval(HaraLanguage.ID, "(require 'std.lib.context)");
      second.eval(HaraLanguage.ID, "(require 'std.lib.context)");
      first.eval(
          HaraLanguage.ID,
          "(std.lib.context/registry-install :isolated "
              + "{:scratch (std.lib.context/registry-scratch :null)})");
      assertTrue(
          first
              .eval(
                  HaraLanguage.ID,
                  "(iter-any? (fn [x] (= x \"isolated\")) "
                      + "(std.lib.context/registry-list))")
              .asBoolean());
      assertFalse(
          second
              .eval(
                  HaraLanguage.ID,
                  "(iter-any? (fn [x] (= x \"isolated\")) "
                      + "(std.lib.context/registry-list))")
              .asBoolean());

      assertThrows(
          PolyglotException.class,
          () -> first.eval(HaraLanguage.ID, "(var std.lib.context/+rt-null+)"));
      assertThrows(
          PolyglotException.class,
          () -> first.eval(HaraLanguage.ID, "(var std.lib.context/protocol-tmpl)"));
      assertThrows(
          PolyglotException.class,
          () -> first.eval(HaraLanguage.ID, "(var std.lib.context/p:registry-list)"));
      assertThrows(
          PolyglotException.class,
          () -> first.eval(HaraLanguage.ID, "(require 'std.lib.context.space)"));
      assertThrows(
          PolyglotException.class,
          () -> first.eval(HaraLanguage.ID, "(require 'std.lib.context.registry)"));
    }
  }

  @Test
  public void providersDeclareCanonicalNamespacesAndStableOrder() {
    HaraLibraryProvider foundation = new StdFoundationLibraryProvider();
    HaraLibraryProvider string = new StdFoundationStringLibraryProvider();
    HaraLibraryProvider bytes = new StdFoundationBytesLibraryProvider();
    HaraLibraryProvider promise = new StdFoundationPromiseLibraryProvider();
    HaraLibraryProvider handle = new StdLibHandleLibraryProvider();
    HaraLibraryProvider file = new StdFoundationFileLibraryProvider();
    HaraLibraryProvider socket = new StdLibSocketLibraryProvider();
    HaraLibraryProvider block = new StdBlockLibraryProvider();
    HaraLibraryProvider zip = new StdZipLibraryProvider();
    HaraLibraryProvider context = new StdLibContextLibraryProvider();
    HaraLibraryProvider task = new StdLibTaskLibraryProvider();
    HaraLibraryProvider resp = new StdRespClientLibraryProvider();
    assertEquals("std.foundation", foundation.namespace());
    assertEquals("std/foundation.hal", foundation.fallbackResource());
    assertTrue(foundation.eager());
    assertEquals("std.foundation.string", string.namespace());
    assertEquals("std.foundation.bytes", bytes.namespace());
    assertEquals("std.foundation.promise", promise.namespace());
    assertEquals("std.lib.handle", handle.namespace());
    assertEquals("std.foundation.file", file.namespace());
    assertEquals("std.lib.socket", socket.namespace());
    assertEquals("std.lib.block", block.namespace());
    assertEquals("std.lib.zip", zip.namespace());
    assertEquals("std.lib.context", context.namespace());
    assertEquals("std.lib.task", task.namespace());
    assertEquals("std.resp.client", resp.namespace());
    assertEquals(5, foundation.order());
    assertEquals(20, string.order());
    assertEquals(20, bytes.order());
    assertEquals(20, promise.order());
    assertEquals(20, handle.order());
    assertEquals(20, file.order());
    assertEquals(20, socket.order());
    assertEquals(20, block.order());
    assertEquals(20, zip.order());
    assertEquals(20, context.order());
    assertEquals(20, task.order());
    assertEquals(20, resp.order());
  }

  @Test
  public void everyContextExportProvidesDocumentationAndArglists() {
    int exports = 0;
    for (Method method : StdLibContext.class.getDeclaredMethods()) {
      HaraExport export = method.getAnnotation(HaraExport.class);
      if (export == null) continue;
      exports++;
      assertFalse(export.name(), export.doc().isEmpty());
      assertTrue(export.name(), export.arglists().length > 0);
    }
    assertEquals(43, exports);
  }
}
