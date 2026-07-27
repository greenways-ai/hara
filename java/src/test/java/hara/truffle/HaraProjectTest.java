package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.io.IOAccess;
import org.junit.Test;

public class HaraProjectTest {
  @Test
  public void parsesProjectDescriptorAndResolvesNamespacePaths() throws Exception {
    Path root = Files.createTempDirectory("hara-project");
    Files.writeString(
        root.resolve("project.edn"),
        "{:hara/type :project :project/id sample "
            + ":project/source-paths [\"src\"] :project/test-paths [\"test\"]}");
    Path source = root.resolve("src/sample/core_name.hal");
    Files.createDirectories(source.getParent());
    Files.writeString(source, "(ns sample.core-name)");

    HaraProject project = HaraProject.discover(source.getParent());
    assertEquals("sample", project.name().display());
    assertEquals(root, project.root());
    assertEquals(source, project.resolve("sample.core-name", false));
  }

  @Test
  public void rejectsProjectPathsOutsideTheProjectRoot() throws Exception {
    Path root = Files.createTempDirectory("hara-project-invalid");
    Path descriptor = root.resolve("project.edn");
    Files.writeString(
        descriptor, "{:project/id sample :project/source-paths [\"../outside\"]}");
    HaraException error =
        assertThrows(HaraException.class, () -> HaraProject.read(descriptor));
    assertTrue(error.getMessage().contains("cannot escape"));
  }

  @Test
  public void keepsLegacyProjectHalAsMigrationFallback() throws Exception {
    Path root = Files.createTempDirectory("hara-project-legacy");
    Files.writeString(
        root.resolve("project.hal"),
        "(defproject sample {:source-paths [\"src\"] :test-paths [\"test\"]})");
    HaraProject project = HaraProject.discover(root);
    assertEquals("sample", project.name().display());
  }

  @Test
  public void requiresProjectNamespacesByConvention() {
    Path benchmark =
        Path.of("lib", "bench", "001-simple-test").toAbsolutePath().normalize();
    try (Context project =
        Context.newBuilder(HaraLanguage.ID)
            .currentWorkingDirectory(benchmark)
            .allowIO(IOAccess.ALL)
            .build()) {
      project.eval(HaraLanguage.ID, "(require 'testing.project-fixture)");
      assertEquals(
          42,
          project.eval(HaraLanguage.ID, "testing.project-fixture/answer").asInt());
      project.eval(HaraLanguage.ID, "(require 'testing.project-test-path-test)");
      assertEquals(
          ":test-path",
          project
              .eval(HaraLanguage.ID, "testing.project-test-path-test/location")
              .toString());
      project.eval(
          HaraLanguage.ID,
          "(require 'testing.project-test-path-test {:reload true})");
      assertThrows(
          PolyglotException.class,
          () -> project.eval(HaraLanguage.ID, "(require 'testing.project-mismatch-test)"));
    }
  }

  @Test
  public void defprojectIsAnExecutableProjectForm() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          "sample",
          context
              .eval(
                  HaraLanguage.ID,
                  "(defproject sample {:source-paths [\"src\"]}) "
                      + "(get project :name)")
              .toString());
    }
  }
}
