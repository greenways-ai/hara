package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.junit.Test;

public class HaraWasmExtensionTest {
  private static final byte[] ADD_WASM = {
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01,
    0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00, 0x0a, 0x09,
    0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b
  };

  @Test
  public void descriptorAndWasmGenerateTheDeclaredAnswer42Namespace() throws Exception {
    Path root = Files.createTempDirectory("hara-wasm-extension-");
    Path extension = root.resolve("demo/000-answer-42");
    Files.createDirectories(extension);
    Files.writeString(
        extension.resolve("hara.extension.edn"),
        "{:namespace \"demo.000-answer-42\" :version \"1.0.0\" :provider :wasm "
            + ":module \"answer-42.wasm\" :abi :core.v1 "
            + ":exports {\"add\" {:args [:i32 :i32] :returns :i32 :async true}} "
            + ":capabilities []}");
    Files.write(extension.resolve("answer-42.wasm"), ADD_WASM);
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", root.toString());
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          45,
          context
              .eval(
                  HaraLanguage.ID,
                  "(ns app (:require [demo.000-answer-42 :as answer :refer [add]])) "
                      + "(+ (deref (answer/add 20 22)) (deref (add 1 2)))")
              .asLong());
      PolyglotException arity =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(deref (answer/add 1))"));
      assertTrue(arity.getMessage().contains("expects 2 arguments"));
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
      Files.deleteIfExists(extension.resolve("answer-42.wasm"));
      Files.deleteIfExists(extension.resolve("hara.extension.edn"));
      Files.deleteIfExists(extension);
      Files.deleteIfExists(extension.getParent());
      Files.deleteIfExists(extension.getParent().getParent());
      Files.deleteIfExists(root);
    }
  }

  @Test
  public void answer42IsNotInstalledUntilItsDescriptorAndWasmAreBothPresent() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () ->
                  context.eval(
                      HaraLanguage.ID, "(ns app (:require [demo.000-answer-42 :as answer]))"));
      assertTrue(error.getMessage().contains("Cannot require missing namespace"));
    }
  }
}
