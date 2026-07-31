package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assume.assumeTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;
import org.junit.Test;

public class HaraSha256ExtensionTest {
  private static final Path ARTIFACT =
      Path.of(
          "rust/extensions/crypto-hash-sha256/target/wasm32-unknown-unknown/release/"
              + "hara_crypto_hash_sha256.wasm");

  @Test
  public void descriptorAndWasmLoadAsCryptoHashSha256() throws Exception {
    assumeTrue("build SHA-256 WASM before this test: " + ARTIFACT, Files.isRegularFile(ARTIFACT));
    Path root = Files.createTempDirectory("hara-sha256-extension-");
    Path extension = root.resolve("crypto/hash/sha256");
    Files.createDirectories(extension);
    Files.copy(
        Path.of("lib/examples/extensions/crypto/hash/sha256/hara.extension.edn"),
        extension.resolve("hara.extension.edn"));
    Files.copy(ARTIFACT, extension.resolve("sha256.wasm"));
    String previous = System.getProperty("hara.extensions.path");
    System.setProperty("hara.extensions.path", root.toString());
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value digest =
          context.eval(
              HaraLanguage.ID,
              "(ns app (:require [crypto.hash.sha256 :as sha])) "
                  + "(deref (sha/digest (bytes 97 98 99)))");
      assertTrue(digest.hasArrayElements());
      assertEquals(32, digest.getArraySize());
      assertEquals((byte) 0xba, digest.getArrayElement(0).asByte());
      assertEquals((byte) 0xad, digest.getArrayElement(31).asByte());
    } finally {
      if (previous == null) System.clearProperty("hara.extensions.path");
      else System.setProperty("hara.extensions.path", previous);
      Files.deleteIfExists(extension.resolve("sha256.wasm"));
      Files.deleteIfExists(extension.resolve("hara.extension.edn"));
      Files.deleteIfExists(extension);
      Files.deleteIfExists(extension.getParent());
      Files.deleteIfExists(extension.getParent().getParent());
      Files.deleteIfExists(root);
    }
  }
}
