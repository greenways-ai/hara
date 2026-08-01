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
              + "[std.db.text.sql-raw :as raw] "
              + "[std.db.text.sql-util :as sql])) "
              + "(def connection (deref (sqlite/open))) "
              + "(deref (db/exec connection "
              + "\"create table items (id integer primary key, name text not null)\")) "
              + "(deref (db/exec connection "
              + "\"insert into items (name) values (?)\" [\"wombat\"])) "
              + "(def statement "
              + "(raw/raw-select \"items\" {\"name\" \"wombat\"} "
              + "[\"id\" \"name\"] (sql/sqlite-opts {}))) "
              + "(def result (deref (db/query connection statement)))");
      assertEquals("sqlite", context.eval(HaraLanguage.ID, "(name (db/engine connection))").asString());
      assertEquals("sqlite", context.eval(HaraLanguage.ID, "(name (db/provider connection))").asString());
      assertEquals(
          "SELECT \"id\", \"name\"\n  FROM \"items\"\n WHERE \"name\" = 'wombat';",
          context.eval(HaraLanguage.ID, "statement").asString());
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
  public void sqliteWasmRunsThroughTheDatabaseKernelRuntime() {
    Assume.assumeTrue(
        Files.isRegularFile(ROOT.resolve("std/db/provider/sqlite/hara.extension.edn")));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", ROOT.toString());
    try (Context context =
        Context.newBuilder(HaraLanguage.ID).allowCreateProcess(true).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns runtime-app (:require [std.lib.substrate :as substrate] "
              + "[std.db.node.runtime :as runtime] "
              + "[std.db.node.client-base :as client] "
              + "[std.db.node.driver.sqlite :as sqlite-driver])) "
              + "(def runtime-config {:primary {:type :sqlite :options {}}}) "
              + "(def server (substrate/node-create \"sqlite-runtime-server\")) "
              + "(def client-node (substrate/node-create \"sqlite-runtime-client\")) "
              + "(sqlite-driver/install server) "
              + "(def connected (deref (runtime/local-connect "
              + "client-node server runtime-config {} {}))) "
              + "(deref (client/exec client-node \"db/primary\" "
              + "\"create table items (id integer primary key, name text not null)\" [] {})) "
              + "(deref (client/exec client-node \"db/primary\" "
              + "\"insert into items (name) values (?)\" [\"runtime-wombat\"] {})) "
              + "(def runtime-result (deref (client/query client-node \"db/primary\" "
              + "\"select id, name from items\" [] {}))) "
              + "(def runtime-info (deref (client/service-info client-node \"db/primary\" {})))");
      assertTrue(
          context.eval(HaraLanguage.ID, "(get connected :transport-attached)").asBoolean());
      assertEquals(
          "setup",
          context.eval(HaraLanguage.ID, "(name (get (get connected :init) :status))").asString());
      assertEquals(
          "sqlite",
          context.eval(HaraLanguage.ID, "(name (get runtime-info :provider))").asString());
      assertEquals(
          "runtime-wombat",
          context
              .eval(HaraLanguage.ID, "(get (get (get runtime-result :rows) 0) 1)")
              .asString());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (deref (runtime/close-runtime (get connected :runtime) runtime-config)) true)")
              .asBoolean());
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
