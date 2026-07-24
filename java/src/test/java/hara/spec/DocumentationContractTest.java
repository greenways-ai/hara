package hara.spec;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.ServiceLoader;
import hara.truffle.HaraLibraryProvider;
import org.junit.Test;

/** Verifies that published examples and mirrored normative documents describe the current slice. */
public class DocumentationContractTest {
  private static final List<String> MIRRORED_FILES =
      List.of(
          "l0-language.md",
          "runtime-libraries.md",
          "xtalk-equivalence.md",
          "extensions-contract.md",
          "extensions.md",
          "native-flavors.md",
          "jvm-flavor.md",
          "repl.md",
          "l0-conformance.edn",
          "rust-runtime.md");

  @Test
  public void normativeSpecificationsAreMirroredIntoDocumentationReference() throws Exception {
    for (String file : MIRRORED_FILES) {
      Path canonical =
          file.endsWith(".edn")
              ? Path.of("spec/hara/corpora", file)
              : Path.of("spec/hara", file);
      assertTrue("Missing canonical spec: " + file, Files.exists(canonical));
      assertTrue(
          "Missing documentation mirror: " + file, Files.exists(Path.of("website/docs/reference", file)));
      assertTrue(
          "Spec and documentation mirror differ: " + file,
          Files.readString(canonical, StandardCharsets.UTF_8)
              .equals(Files.readString(Path.of("website/docs/reference", file), StandardCharsets.UTF_8)));
    }
  }

  @Test
  public void publishedExamplesUseSupportedMarkerSyntaxAndExistingFiles() throws Exception {
    String userGuide = Files.readString(Path.of("website/docs/user-guide.md"), StandardCharsets.UTF_8);
    assertTrue(userGuide.contains("(. a (push-last 4))"));
    assertTrue(userGuide.contains("(. a (get 3))"));
    assertFalse(userGuide.contains("(array:push-last"));
    assertFalse(userGuide.contains("(array:get"));
    assertTrue(Files.exists(Path.of("lib/examples/hello.hal")));
    assertFalse(Files.exists(Path.of("lib/examples/hello.hara")));
  }

  @Test
  public void namespaceCatalogTracksEveryRegisteredProvider() throws Exception {
    String catalog =
        Files.readString(Path.of("website/docs/reference/namespaces.md"), StandardCharsets.UTF_8);
    int providers = 0;
    for (HaraLibraryProvider provider : ServiceLoader.load(HaraLibraryProvider.class)) {
      providers++;
      assertTrue(
          "Missing provider namespace from catalog: " + provider.namespace(),
          catalog.contains("`" + provider.namespace() + "`"));
    }
    assertTrue("No Hara library providers were discovered", providers > 0);
  }

  @Test
  public void currentGuidesUseCurrentNamespaceAndLauncherConventions() throws Exception {
    List<Path> currentGuides =
        List.of(
            Path.of("README.md"),
            Path.of("GETTING_STARTED.md"),
            Path.of("website/docs/namespaces.md"),
            Path.of("website/docs/user-guide.md"),
            Path.of("website/docs/reference/namespaces.md"),
            Path.of("lib/examples/code-test/README.md"));
    for (Path guide : currentGuides) {
      String content = Files.readString(guide, StandardCharsets.UTF_8);
      assertFalse("Stale launcher in " + guide, content.contains("truffle-hara"));
      assertFalse("Stale project descriptor in " + guide, content.contains("project.hara"));
    }
    assertTrue(Files.readString(Path.of("hara")).contains("\"$@\""));
  }
}
