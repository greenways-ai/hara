package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.junit.Assume;
import org.junit.Test;

public class HaraSqliteProcessExtensionTest {
  private static final Path ROOT =
      Path.of("rust/extensions/std-db-sqlite/target/package").toAbsolutePath().normalize();

  @Test
  public void sqliteWasmRunsGeneratedAndParameterizedSqlThroughTheGenericDbApi() {
    Assume.assumeTrue(
        Files.isRegularFile(ROOT.resolve("std/db/provider/sqlite/hara.extension.edn")));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", ROOT.toString());
    try (Context context =
        Context.newBuilder(HaraLanguage.ID).allowCreateProcess(true).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns app (:require [std.db :as db] "
              + "[std.db.sqlite :as sqlite] "
              + "[std.db.text.sql-util :as sql])) "
              + "(def connection (deref (sqlite/open))) "
              + "(deref (db/exec connection "
              + "\"create table items (id integer primary key, name text not null)\")) "
              + "(deref (db/exec connection "
              + "\"insert into items (name) values (?)\" [\"wombat\"])) "
              + "(def predicate "
              + "(sql/encode-query-string {\"name\" \"wombat\"} \"WHERE\" "
              + "{:column-fn sql/default-quote-fn})) "
              + "(def result (deref (db/query connection "
              + "(str \"select id, name from items \" predicate))))");
      assertEquals("sqlite", context.eval(HaraLanguage.ID, "(name (db/engine connection))").asString());
      assertEquals("sqlite", context.eval(HaraLanguage.ID, "(name (db/provider connection))").asString());
      assertEquals(
          "WHERE \"name\" = 'wombat'",
          context.eval(HaraLanguage.ID, "predicate").asString());
      assertEquals(
          "name",
          context.eval(HaraLanguage.ID, "(get (get result :columns) 1)").asString());
      assertEquals(
          "wombat",
          context.eval(HaraLanguage.ID, "(get (get (get result :rows) 0) 1)").asString());
      assertTrue(context.eval(HaraLanguage.ID, "(deref (db/close connection))").asBoolean());
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
    }
  }

  @Test
  public void sqliteProcessProviderRequiresProcessCapability() {
    Assume.assumeTrue(
        Files.isRegularFile(ROOT.resolve("std/db/provider/sqlite/hara.extension.edn")));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", ROOT.toString());
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Exception error =
          org.junit.Assert.assertThrows(
              Exception.class,
              () ->
                  context.eval(
                      HaraLanguage.ID,
                      "(ns app (:require [std.db.sqlite :as sqlite]))"));
      assertTrue(error.getMessage().contains("capability-denied"));
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
    }
  }
}
