package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;
import org.junit.Test;

public class HaraHtaExtensionTest {
  private static final Path ARTIFACT =
      Path.of("rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm");

  @Test
  public void htaActorEvaluatesAndSettlesTasks() throws Exception {
    withExtension(
        "",
        context -> {
          assertEquals(
              42,
              context
                  .eval(
                      HaraLanguage.ID,
                      "(ns app (:require [hara.runtime.wasm :as runtime])) "
                          + "(deref (runtime/eval \"(+ 20 22)\"))")
                  .asLong());
        });
  }

  @Test
  public void allowlistedHostCallSettlesWithSha256Bytes() throws Exception {
    withExtension(
        ":host-calls {\"crypto.hash.sha256\" [\"digest\"]}",
        context -> {
          Value digest =
              context.eval(
                  HaraLanguage.ID,
                  "(ns app (:require [hara.runtime.wasm :as runtime])) "
                      + "(deref (runtime/eval "
                      + "\"(ns user (:require [std.foundation.host :as host])) "
                      + "(host/call \\\"crypto.hash.sha256\\\" \\\"digest\\\" (bytes 97 98 99))\"))");
          assertTrue(digest.hasArrayElements());
          assertEquals(32, digest.getArraySize());
          assertEquals((byte) 0xba, digest.getArrayElement(0).asByte());
          assertEquals((byte) 0xad, digest.getArrayElement(31).asByte());
        });
  }

  @Test
  public void pendingDerefResumesInsideNestedEvaluation() throws Exception {
    withExtension(
        ":host-calls {\"crypto.hash.sha256\" [\"digest\"]}",
        context ->
            assertEquals(
                42,
                context
                    .eval(
                        HaraLanguage.ID,
                        "(ns app (:require [hara.runtime.wasm :as runtime])) "
                            + "(deref (runtime/eval \"(ns user (:require [std.foundation.host :as host])) "
                            + "(+ 10 (count (deref (host/call \\\"crypto.hash.sha256\\\" \\\"digest\\\" (bytes 97 98 99)))))\"))")
                    .asLong()));
  }

  @Test
  public void rejectedPendingDerefFlowsThroughCatch() throws Exception {
    withExtension(
        "",
        context ->
            assertEquals(
                42,
                context
                    .eval(
                        HaraLanguage.ID,
                        "(ns app (:require [hara.runtime.wasm :as runtime])) "
                            + "(deref (runtime/eval \"(ns user (:require [std.foundation.host :as host])) "
                            + "(try (deref (host/call \\\"denied\\\" \\\"call\\\")) (catch error 42))\"))")
                    .asLong()));
  }

  private static void withExtension(String hostCalls, CheckedConsumer operation) throws Exception {
    assertTrue("build rust/raw before HTA tests: " + ARTIFACT, Files.isRegularFile(ARTIFACT));
    Path root = Files.createTempDirectory("hara-hta-extension-");
    Path extension = root.resolve("hara/runtime/wasm");
    Files.createDirectories(extension);
    Files.writeString(
        extension.resolve("hara.extension.edn"),
        "{:namespace \"hara.runtime.wasm\" :version \"0.1.0\" :provider :wasm "
            + ":module \"hara.wasm\" :abi :hta.v1 "
            + ":exports {\"eval\" {:args [:value] :returns :value :async true}} "
            + ":capabilities [] "
            + hostCalls
            + "}");
    Files.copy(ARTIFACT, extension.resolve("hara.wasm"));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", root.toString());
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      operation.accept(context);
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
      Files.deleteIfExists(extension.resolve("hara.wasm"));
      Files.deleteIfExists(extension.resolve("hara.extension.edn"));
      Files.deleteIfExists(extension);
      Files.deleteIfExists(extension.getParent());
      Files.deleteIfExists(extension.getParent().getParent());
      Files.deleteIfExists(root);
    }
  }

  private interface CheckedConsumer {
    void accept(Context context) throws Exception;
  }
}
