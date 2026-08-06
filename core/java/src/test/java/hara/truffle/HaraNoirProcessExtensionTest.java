package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.junit.Assume;
import org.junit.Test;

public class HaraNoirProcessExtensionTest {
  private static final Path ROOT =
      Path.of("rust/extensions/ledger-noir/target/package").toAbsolutePath().normalize();

  @Test
  public void realNoirCompilerProverAndVerifierRunThroughManagedNode() {
    Assume.assumeTrue(Files.isRegularFile(ROOT.resolve("ledger/noir/hara.extension.edn")));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", ROOT.toString());
    try (Context context =
        Context.newBuilder(HaraLanguage.ID).allowCreateProcess(true).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns app (:require [ledger.noir :as noir])) "
              + "(def artifact (deref (noir/compile "
              + "{:name \"proof_demo\" "
              + ":source \"fn main(secret: Field, expected: pub Field) { "
              + "assert(secret * secret == expected); }\"})))");
      assertEquals(
          "hara/ledger.noir/v1",
          context.eval(HaraLanguage.ID, "(get artifact :format)").asString());
      context.eval(
          HaraLanguage.ID,
          "(def proof (deref (noir/prove artifact {:secret \"7\" :expected \"49\"})))");
      assertEquals(
          "hara.noir.proof/v1",
          context.eval(HaraLanguage.ID, "(get proof :format)").asString());
      assertTrue(context.eval(HaraLanguage.ID, "(deref (noir/verify artifact proof))").asBoolean());
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
    }
  }

  @Test
  public void processCapabilityIsDeniedBeforeWorkerStartup() {
    Assume.assumeTrue(Files.isRegularFile(ROOT.resolve("ledger/noir/hara.extension.edn")));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", ROOT.toString());
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Exception error =
          org.junit.Assert.assertThrows(
              Exception.class,
              () ->
                  context.eval(
                      HaraLanguage.ID,
                      "(ns app (:require [ledger.noir :as noir]))"));
      assertTrue(error.getMessage().contains("capability-denied"));
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
    }
  }
}
