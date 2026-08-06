package hara.truffle;

import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

/** Verifies that a poisoned namespace load retains the initial failure detail. */
public class NamespaceLoadFailureDiagnosticsTest {
  @Test
  public void poisonedNamespaceLoadRetainsInitialFailureDetail() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      PolyglotException first =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(require 'acme.missing.namespace)"));
      assertTrue(first.getMessage().contains("Cannot require missing namespace"));

      PolyglotException retried =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(require 'acme.missing.namespace)"));
      assertTrue(retried.getMessage().contains("Namespace load previously failed"));
      assertTrue(
          "retry should retain the initial failure detail: " + retried.getMessage(),
          retried.getMessage().contains("initial failure"));

      PolyglotException reloaded =
          assertThrows(
              PolyglotException.class,
              () ->
                  context.eval(
                      HaraLanguage.ID, "(require 'acme.missing.namespace {:reload true})"));
      assertTrue(reloaded.getMessage().contains("Cannot require missing namespace"));
    }
  }
}
