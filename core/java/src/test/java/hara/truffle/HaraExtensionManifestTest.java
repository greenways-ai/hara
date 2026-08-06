package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.charset.StandardCharsets;
import org.junit.Test;

public class HaraExtensionManifestTest {
  @Test
  public void packagedAnswer42ManifestMatchesTheProviderContract() throws Exception {
    Path descriptor = Path.of("lib/examples/extensions/demo/000-answer-42/hara.extension.edn");
    HaraExtensionManifest manifest =
        HaraExtensionManifest.parse(
            Files.readString(descriptor, StandardCharsets.UTF_8), descriptor.toString());
    assertEquals("demo.000-answer-42", manifest.namespace());
    assertEquals("0.1.0", manifest.version());
    assertEquals("wasm", manifest.provider());
    assertEquals("answer-42.wasm", manifest.module());
    assertEquals("core.v1", manifest.abi());
    assertEquals(2, manifest.exports().size());
    assertEquals("i32", manifest.exports().get("version").returns());
    assertEquals(2, manifest.exports().get("add").arguments().size());
    assertTrue(manifest.capabilities().isEmpty());
  }

  @Test
  public void parsesCompactPublicHandleTags() {
    String source =
        "{:namespace \"math.tensor\" :identity \"hara/math.tensor\" :version \"1\" :provider :wasm "
            + ":module \"tensor.wasm\" :abi :hta.v1 "
            + ":exports {\"open\" {:args [] :returns :value :async true}} "
            + ":handles {\"tensor\" {:tag math}} :capabilities []}";
    HaraExtensionManifest manifest = HaraExtensionManifest.parse(source, "test");
    assertEquals("hara/math.tensor", manifest.identity());
    assertEquals("math", manifest.handleTag("tensor"));
    assertEquals(null, manifest.handleTag("buffer"));
    assertThrows(
        IllegalArgumentException.class,
        () -> HaraExtensionManifest.parse(source.replace(":tag math", ":tag Math"), "test"));
    assertThrows(
        IllegalArgumentException.class,
        () ->
            HaraExtensionManifest.parse(
                source.replace("hara/math.tensor", "Hara/math.tensor"), "test"));
  }

  @Test
  public void malformedManifestsFailBeforeProviderSelection() {
    String base =
        "{:namespace \"demo.extension\" :version \"1\" :provider :wasm "
            + ":module \"demo.wasm\" :abi :core.v1 "
            + ":exports {\"run\" {:args [] :returns :i32}} "
            + ":capabilities []}";
    assertThrows(
        IllegalArgumentException.class,
        () ->
            HaraExtensionManifest.parse(
                base.replace(":provider :wasm", ":provider \"wasm\""), "test"));
    assertThrows(
        IllegalArgumentException.class,
        () ->
            HaraExtensionManifest.parse(
                base.replace(":capabilities []", ":capabilities [] :extra true"), "test"));
  }
}
