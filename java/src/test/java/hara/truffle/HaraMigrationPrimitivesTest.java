package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Value;
import org.graalvm.polyglot.io.IOAccess;
import org.junit.Test;

public class HaraMigrationPrimitivesTest {
  @Test
  public void defstructDefinesPositionalAndMapConstructors() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(
          3,
          context
              .eval(
                  HaraLanguage.ID,
                  "(defstruct Point [x y]) "
                      + "(+ (field (->Point 1 2) :x) "
                      + "   (field (map->Point {:x 2 :y 4}) :x))")
              .asInt());
    }
  }

  @Test
  public void instancePredicateIsRestrictedToHaraStructTypes() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value result =
          context.eval(
              HaraLanguage.ID,
              "(defstruct Point [x y]) "
                  + "(defstruct Other [x y]) "
                  + "[(instance? Point (->Point 1 2)) "
                  + " (instance? Other (->Point 1 2)) "
                  + " (instance? Point {:x 1 :y 2})]");
      assertTrue(result.getArrayElement(0).asBoolean());
      assertFalse(result.getArrayElement(1).asBoolean());
      assertFalse(result.getArrayElement(2).asBoolean());
    }
  }

  @Test
  public void namespaceIntrospectionIsNarrowAndDeterministic() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value result =
          context.eval(
              HaraLanguage.ID,
              "(ns sample.alpha) "
                  + "(def zed 1) "
                  + "(def alpha 2) "
                  + "(let [entries (iter (ns-publics 'sample.alpha))] "
                  + " (let [first-entry (iter-next entries)] "
                  + "  (let [second-entry (iter-next entries)] "
                  + "   [(ns-name 'sample.alpha) "
                  + "    (nth first-entry 0) "
                  + "    (nth second-entry 0) "
                  + "    (the-ns 'missing.namespace)])))");
      assertEquals("sample.alpha", result.getArrayElement(0).toString());
      assertEquals("alpha", result.getArrayElement(1).toString());
      assertEquals("zed", result.getArrayElement(2).toString());
      assertTrue(result.getArrayElement(3).isNull());
    }
  }

  @Test
  public void readFormsRequiresIoAndPreservesSourceSpans() throws Exception {
    Path source = Files.createTempFile("hara-read-forms", ".hal");
    Files.writeString(source, "(def first-value 1)\n\n(def second-value 2)\n");
    String expression =
        "(let [forms (read-forms \""
            + source.toString().replace("\\", "\\\\")
            + "\")] "
            + "[(count forms) "
            + " (get (meta (nth forms 1)) :file) "
            + " (get (meta (nth forms 1)) :line)])";

    try (Context denied = Context.newBuilder(HaraLanguage.ID).build()) {
      PolyglotException error =
          assertThrows(PolyglotException.class, () -> denied.eval(HaraLanguage.ID, expression));
      assertFalse(error.getMessage().isEmpty());
    }

    try (Context allowed =
        Context.newBuilder(HaraLanguage.ID).allowIO(IOAccess.ALL).build()) {
      Value result = allowed.eval(HaraLanguage.ID, expression);
      assertEquals(2, result.getArrayElement(0).asInt());
      assertEquals(source.toAbsolutePath().normalize().toString(), result.getArrayElement(1).asString());
      assertEquals(3, result.getArrayElement(2).asInt());
    }
  }
}
