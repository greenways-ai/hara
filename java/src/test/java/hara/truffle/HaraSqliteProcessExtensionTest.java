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
  public void sqliteWasmExecutesARecursiveSchemaGraphQuery() {
    Assume.assumeTrue(
        Files.isRegularFile(ROOT.resolve("std/db/provider/sqlite/hara.extension.edn")));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", ROOT.toString());
    try (Context context =
        Context.newBuilder(HaraLanguage.ID).allowCreateProcess(true).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns graph-app (:require [std.db :as db] "
              + "[std.db.sqlite :as sqlite] "
              + "[std.db.text.sql-graph :as graph] "
              + "[std.db.text.sql-util :as sql] "
              + "[std.foundation.json :as json] "
              + "[std.foundation.string :as string])) "
              + "(def graph-schema "
              + "{\"User\" {\"id\" {\"ident\" \"id\" \"order\" 0 \"type\" \"uuid\" \"scope\" \"id\"} "
              + "\"name\" {\"ident\" \"name\" \"order\" 1 \"type\" \"text\" \"scope\" \"data\"} "
              + "\"team\" {\"ident\" \"team\" \"order\" 2 \"type\" \"ref\" \"scope\" \"ref\" "
              + "\"ref\" {\"ns\" \"Team\" \"type\" \"forward\" \"key\" \"team\"}} "
              + "\"profile\" {\"ident\" \"profile\" \"type\" \"ref\" \"scope\" \"ref\" "
              + "\"ref\" {\"ns\" \"Profile\" \"type\" \"reverse\" \"rkey\" \"user\"}}} "
              + "\"Team\" {\"id\" {\"ident\" \"id\" \"order\" 0 \"type\" \"uuid\" \"scope\" \"id\"} "
              + "\"title\" {\"ident\" \"title\" \"order\" 1 \"type\" \"text\" \"scope\" \"data\"}} "
              + "\"Profile\" {\"id\" {\"ident\" \"id\" \"order\" 0 \"type\" \"uuid\" \"scope\" \"id\"} "
              + "\"bio\" {\"ident\" \"bio\" \"order\" 1 \"type\" \"text\" \"scope\" \"data\"} "
              + "\"user\" {\"ident\" \"user\" \"order\" 2 \"type\" \"ref\" \"scope\" \"ref\" "
              + "\"ref\" {\"ns\" \"User\" \"type\" \"forward\" \"key\" \"user\"}}}}) "
              + "(def graph-connection (deref (sqlite/open))) "
              + "(deref (db/exec graph-connection \"create table Team (id text primary key, title text not null)\")) "
              + "(deref (db/exec graph-connection \"create table User (id text primary key, name text not null, team_id text)\")) "
              + "(deref (db/exec graph-connection \"create table Profile (id text primary key, bio text, user_id text)\")) "
              + "(deref (db/exec graph-connection \"insert into Team (id, title) values (?, ?)\" [\"t1\" \"Core\"])) "
              + "(deref (db/exec graph-connection \"insert into User (id, name, team_id) values (?, ?, ?)\" [\"u1\" \"Ada\" \"t1\"])) "
              + "(deref (db/exec graph-connection \"insert into Profile (id, bio, user_id) values (?, ?, ?)\" [\"p1\" \"Compiler\" \"u1\"])) "
              + "(def graph-statement "
              + "(graph/select graph-schema "
              + "[\"User\" {\"name\" \"Ada\"} "
              + "[\"id\" \"name\" [\"team\" [\"id\" \"title\"]] [\"profile\" [\"id\" \"bio\"]]]] "
              + "(sql/sqlite-opts {}))) "
              + "(def graph-result (deref (db/query graph-connection graph-statement))) "
              + "(def graph-json (get (get (get graph-result :rows) 0) 0)) "
              + "(def graph-data (json/read graph-json))");
      assertTrue(
          context.eval(HaraLanguage.ID, "(string/includes? graph-statement \"FROM \\\"Team\\\"\")").asBoolean());
      assertTrue(
          context.eval(HaraLanguage.ID, "(string/includes? graph-statement \"FROM \\\"Profile\\\"\")").asBoolean());
      assertTrue(
          context.eval(HaraLanguage.ID, "(string/includes? graph-json \"Core\")").asBoolean());
      assertTrue(
          context.eval(HaraLanguage.ID, "(string/includes? graph-json \"Compiler\")").asBoolean());
      assertEquals(
          "Ada",
          context.eval(HaraLanguage.ID, "(get (get graph-data 0) \"name\")").asString());
      assertTrue(context.eval(HaraLanguage.ID, "(deref (db/close graph-connection))").asBoolean());
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
          "(ns runtime-app (:require [std.db :as db] "
              + "[std.lib.substrate :as substrate] "
              + "[std.db.node.runtime :as runtime] "
              + "[std.db.node.driver.sqlite :as sqlite-driver])) "
              + "(def runtime-config {:primary {:type :sqlite :options {}}}) "
              + "(def server (substrate/node-create \"sqlite-runtime-server\")) "
              + "(def client-node (substrate/node-create \"sqlite-runtime-client\")) "
              + "(sqlite-driver/install server) "
              + "(def connected (deref (runtime/local-connect "
              + "client-node server runtime-config {} {}))) "
              + "(def runtime-connection "
              + "(deref (runtime/open-service (get connected :runtime) \"db/primary\"))) "
              + "(deref (db/exec runtime-connection "
              + "\"create table items (id integer primary key, name text not null)\")) "
              + "(deref (db/exec runtime-connection "
              + "\"insert into items (name) values (?)\" [\"runtime-wombat\"])) "
              + "(def runtime-result (deref (db/query runtime-connection "
              + "\"select id, name from items\"))) "
              + "(def runtime-info (db/info runtime-connection))");
      assertTrue(
          context.eval(HaraLanguage.ID, "(get connected :transport-attached)").asBoolean());
      assertEquals(
          "setup",
          context.eval(HaraLanguage.ID, "(name (get (get connected :init) :status))").asString());
      assertEquals(
          "sqlite",
          context.eval(HaraLanguage.ID, "(name (get runtime-info :provider))").asString());
      assertTrue(context.eval(HaraLanguage.ID, "(get runtime-info :remote)").asBoolean());
      assertEquals(
          "runtime-wombat",
          context
              .eval(HaraLanguage.ID, "(get (get (get runtime-result :rows) 0) 1)")
              .asString());
      assertTrue(context.eval(HaraLanguage.ID, "(deref (db/close runtime-connection))").asBoolean());
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
