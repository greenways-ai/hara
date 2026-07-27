package hara.truffle;

import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

public class StdFoundationCoroutineTest {
  @Test
  public void resumeOnDeadThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(HaraLanguage.ID, "(def c-dead (std.foundation.coroutine/create (fn [] 1)))");
      context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-dead)");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-dead)"));
      assertTrue(error.getMessage().contains("dead"));
    }
  }

  @Test
  public void bodyErrorRethrowsAtResumeAndKillsCoroutine() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(HaraLanguage.ID, "(def c-err (std.foundation.coroutine/create (fn [] (/ 1 0))))");
      assertThrows(
          PolyglotException.class,
          () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-err)"));
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.foundation.coroutine/status c-err))")
              .asBoolean());
    }
  }

  @Test
  public void closeOnNeverResumedCoroutine() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-unstarted (std.foundation.coroutine/create (fn [] :never-runs)))");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(std.foundation.coroutine/coroutine? (std.foundation.coroutine/close c-unstarted))")
              .asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.foundation.coroutine/status c-unstarted))")
              .asBoolean());
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-unstarted)"));
      assertTrue(error.getMessage().contains("dead"));
    }
  }

  @Test
  public void yieldOutsideCoroutineThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/yield 1)"));
      assertTrue(error.getMessage().contains("outside"));
    }
  }

  @Test
  public void reentrantResumeThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-r (std.foundation.coroutine/create (fn [] (std.foundation.coroutine/resume c-r))))");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-r)"));
      assertTrue(error.getMessage().contains("running"));
    }
  }

  @Test
  public void closeRunsFinallyAndKillsCoroutine() throws InterruptedException {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(HaraLanguage.ID, "(def close-log (atom :init))");
      context.eval(
          HaraLanguage.ID,
          "(def c-close (std.foundation.coroutine/create"
              + " (fn [] (try (std.foundation.coroutine/yield :parked)"
              + "             (finally (reset! close-log :ran))))))");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= :parked (std.foundation.coroutine/resume c-close))")
              .asBoolean());
      context.eval(HaraLanguage.ID, "(std.foundation.coroutine/close c-close)");
      // Wait for the coroutine thread to unwind (close is asynchronous with the interrupt).
      long deadline = System.currentTimeMillis() + 5000;
      while (System.currentTimeMillis() < deadline) {
        if (context
            .eval(HaraLanguage.ID, "(= :ran (deref close-log))")
            .asBoolean()) {
          break;
        }
        Thread.sleep(20);
      }
      assertTrue(
          context.eval(HaraLanguage.ID, "(= :ran (deref close-log))").asBoolean());
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.foundation.coroutine/status c-close))")
              .asBoolean());
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-close)"));
      assertTrue(error.getMessage().contains("dead"));
    }
  }

  @Test
  public void closeOnDeadIsNoOpAndCloseOnRunningThrows() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(HaraLanguage.ID, "(def c-done (std.foundation.coroutine/create (fn [] 1)))");
      context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-done)");
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(std.foundation.coroutine/coroutine? (std.foundation.coroutine/close c-done))")
              .asBoolean());
      context.eval(
          HaraLanguage.ID,
          "(def c-self (std.foundation.coroutine/create (fn [] (std.foundation.coroutine/close c-self))))");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-self)"));
      assertTrue(error.getMessage().contains("running"));
    }
  }

  @Test
  public void awaitRethrowsPromiseRejection() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      context.eval(
          HaraLanguage.ID,
          "(def c-reject (std.foundation.coroutine/create"
              + " (fn [] (std.foundation.coroutine/await (promise (fn [] (/ 1 0)))))))");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/resume c-reject)"));
      assertTrue(error.getMessage().contains("Promise rejected"));
      assertTrue(
          context
              .eval(HaraLanguage.ID, "(= :dead (std.foundation.coroutine/status c-reject))")
              .asBoolean());
    }
  }

  @Test
  public void awaitRejectsNonDerefable() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.foundation.coroutine)");
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(std.foundation.coroutine/await 42)"));
      assertTrue(error.getMessage().contains("derefable"));
    }
  }
}
