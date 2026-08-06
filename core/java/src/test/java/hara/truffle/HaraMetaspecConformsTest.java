package hara.truffle;

import static org.junit.Assert.assertTrue;

import hara.kernel.base.Parser;
import hara.lang.base.G;
import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.junit.Test;

public class HaraMetaspecConformsTest {
  private static final Path ROOT =
      Path.of("../hara-specs/01-lang/000-metaspec/draft/metaspec-metaspec.edn");
  private static final Path LANGUAGE =
      Path.of("../hara-specs/01-lang/001-language/metaspec/language-metaspec.edn");
  private static final Path LANGUAGE_SPEC =
      Path.of("../hara-specs/01-lang/001-language/draft/hal-langspec.edn");
  private static final Path ARTIFACT =
      Path.of("../hara-specs/00-unsorted/artifact/metaspec/artifact-metaspec.edn");

  @Test
  public void rootChecksItselfAndSpecializedMetaspecs() throws Exception {
    Object root = read(ROOT);
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns hara.metaspec.conforms-test "
              + "(:require [hara.metaspec.core :as metaspec]))");
      assertPass(context, root, root);
      Object language = read(LANGUAGE);
      assertPass(context, language, root);
      assertPass(context, read(LANGUAGE_SPEC), language);
      assertPass(context, read(ARTIFACT), root);
    }
  }

  @Test
  public void unresolvedMetaspecAndCheckerPackagesAreBlocked() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(
          HaraLanguage.ID,
          "(ns hara.metaspec.blocked-test "
              + "(:require [hara.metaspec.core :as metaspec]))");
      String unresolved =
          Main.display(
              context.eval(
                  HaraLanguage.ID,
                  "(metaspec/conforms "
                      + "{:document/id :demo/spec :document/version \"1.0.0\" "
                      + ":spec/conforms-to {:spec/id :missing/meta "
                      + ":spec/version \"1.0.0\"}})"));
      assertTrue(unresolved, unresolved.contains(":report/status :blocked"));
    }
  }

  private static Object read(Path path) throws Exception {
    return Parser.LispReader.readString(Files.readString(path), null);
  }

  private static void assertPass(Context context, Object document, Object metaspec) {
    String report =
        Main.display(
            context.eval(
                HaraLanguage.ID,
                "(metaspec/conforms "
                    + G.display(document)
                    + " {:metaspec "
                    + G.display(metaspec)
                    + "})"));
    assertTrue(report, report.contains(":report/status :pass"));
  }
}
