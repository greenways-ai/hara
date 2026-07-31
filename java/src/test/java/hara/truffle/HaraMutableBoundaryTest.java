package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Value;
import org.junit.Test;

public class HaraMutableBoundaryTest {
  @Test
  public void bytesHaveReadableContentEqualityHashCopyAndSliceSemantics() {
    try (Context context = context()) {
      Value bytes = context.eval(HaraLanguage.ID, "(bytes 1 2 -3)");
      assertEquals("(bytes 1 2 -3)", bytes.toString());
      assertTrue(context.eval(HaraLanguage.ID, "(= (bytes 1 2 -3) (bytes 1 2 -3))").asBoolean());
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(= (IHash/hash (bytes 1 2 -3)) "
                      + "(IHash/hash (bytes 1 2 -3)))")
              .asBoolean());
      assertEquals(
          2,
          context.eval(HaraLanguage.ID, "(bytes/get (bytes/slice (bytes 1 2 -3) 1 3) 0)").asLong());
      assertEquals(
          1,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [source (bytes 1 2)] "
                      + "(let [copy (bytes/copy source)] (bytes/set copy 0 9) (bytes/get source 0)))")
              .asLong());
    }
  }

  @Test
  public void byteValuesConvertBetweenSignedAndUnsignedRepresentations() {
    try (Context context = context()) {
      assertEquals(255, context.eval(HaraLanguage.ID, "(bytes/u8 -1)").asLong());
      assertEquals(-1, context.eval(HaraLanguage.ID, "(bytes/s8 255)").asLong());
      assertEquals(127, context.eval(HaraLanguage.ID, "(bytes/s8 127)").asLong());
      assertTrue(
          assertThrows(
                  PolyglotException.class, () -> context.eval(HaraLanguage.ID, "(bytes/u8 256)"))
              .getMessage()
              .contains("range -128..255"));
    }
  }

  @Test
  public void ordinaryByteOperationsHaveExplicitBoundsAndMutationSemantics() {
    try (Context context = context()) {
      assertEquals(3, context.eval(HaraLanguage.ID, "(bytes/count (bytes 1 2 -3))").asLong());
      assertEquals(2, context.eval(HaraLanguage.ID, "(bytes/get (bytes 1 2 -3) 1)").asLong());
      assertEquals(
          9,
          context
              .eval(HaraLanguage.ID, "(let [b (bytes 1 2)] (bytes/set b 0 9) (bytes/get b 0))")
              .asLong());
      assertEquals(7, context.eval(HaraLanguage.ID, "(bytes/get (bytes 1) 4 7)").asLong());
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () -> context.eval(HaraLanguage.ID, "(bytes/get (bytes 1) 4)"))
              .getMessage()
              .contains("bytes/get index out of bounds"));
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () -> context.eval(HaraLanguage.ID, "(bytes/set (bytes 1) 0 256)"))
              .getMessage()
              .contains("bytes/set expects a value in the range -128..255"));
    }
  }

  @Test
  public void libraryByteLookupIsUnsignedWhileProtocolNthIsSigned() {
    try (Context context = context()) {
      assertEquals(255, context.eval(HaraLanguage.ID, "(bytes/get (bytes -1) 0)").asLong());
      assertEquals(
          -1, context.eval(HaraLanguage.ID, "(INth/nth (bytes -1) 0)").asLong());
      assertEquals(
          2,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [b (bytes 1 2)] (bytes/set b 0 3) (count (bytes/set b 1 4)))")
              .asLong());
    }
  }

  @Test
  public void mutableObjectsUseKeysWhileSequentialTargetsRequireNumericIndexes() {
    try (Context context = context()) {
      assertEquals(
          42,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [object (object)] "
                      + "(. object (set \"answer\" 42)) (. object (get \"answer\")))")
              .asLong());
      assertEquals(7, context.eval(HaraLanguage.ID, "(. (object) (get \"missing\" 7))").asLong());

      PolyglotException invalidIndex =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(. (array 1) (get :bad))"));
      assertTrue(invalidIndex.getMessage().contains("expects a numeric index"));
    }
  }

  @Test
  public void byteOperationsRejectWrongTypesAndInvalidRanges() {
    try (Context context = context()) {
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () -> context.eval(HaraLanguage.ID, "(bytes/copy [1 2])"))
              .getMessage()
              .contains("bytes/copy expects bytes"));
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () -> context.eval(HaraLanguage.ID, "(bytes/slice (bytes 1 2) 0 3)"))
              .getMessage()
              .contains("range is out of bounds"));
    }
  }

  @Test
  public void mutableMutationBoundsHaveStableDiagnostics() {
    try (Context context = context()) {
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () -> context.eval(HaraLanguage.ID, "(. (array 1) (set 4 9))"))
              .getMessage()
              .contains("set index out of bounds"));
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () -> context.eval(HaraLanguage.ID, "(. (array 1) (remove 4))"))
              .getMessage()
              .contains("remove index out of bounds"));
    }
  }

  @Test
  public void iteratorFormsAreLazyAndExplicitlyClosable() {
    try (Context context = context()) {
      assertEquals(
          1, context.eval(HaraLanguage.ID, "(let [it (iter [1 2])] (iter-next it))").asLong());
      assertEquals(
          2,
          context
              .eval(HaraLanguage.ID, "(let [it (iter [1 2])] (iter-next it) (iter-next it))")
              .asLong());
      assertTrue(
          !context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter [1 2])] (iter-next it) (iter-next it) (iter-next? it))")
              .asBoolean());
      assertTrue(
          assertThrows(
                  PolyglotException.class,
                  () ->
                      context.eval(HaraLanguage.ID, "(iter-next (iter [1])) (iter-next (iter []))"))
              .getMessage()
              .contains("reached the end"));
      context.eval(HaraLanguage.ID, "(iter-close (iter \"abc\"))");
      assertEquals(
          4,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [ia (iter (array 1 2)) ib (iter (bytes 3 4))] "
                      + "(+ (iter-next ia) (iter-next ib)))")
              .asLong());
    }
  }

  @Test
  public void concatIsLazyAndIteratorBacked() {
    try (Context context = context()) {
      assertEquals(
          2,
          context
              .eval(
                  HaraLanguage.ID, "(let [it (concat [1 2] [3 4])] (iter-next it) (iter-next it))")
              .asLong());
      assertEquals(
          1, context.eval(HaraLanguage.ID, "(let [it (concat [1] 1)] (iter-next it))").asLong());
      assertThrows(
          PolyglotException.class,
          () ->
              context.eval(
                  HaraLanguage.ID, "(let [it (concat [1] 1)] (iter-next it) (iter-next it))"));
    }
  }

  @Test
  public void iteratorCombinatorsRemainLazyAndUseHaraFunctions() {
    try (Context context = context()) {
      assertEquals(
          4,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-map (fn [x] (* x 2)) [1 2])] (iter-next it) (iter-next it))")
              .asLong());
      assertEquals(
          2,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-filter (fn [x] (= x 2)) [1 2 3])] (iter-next it))")
              .asLong());
      assertEquals(
          2,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-drop 1 (iter-take 3 [1 2 3 4]))] (iter-next it))")
              .asLong());
      assertEquals(
          3, context.eval(HaraLanguage.ID, "(nth (iter-next (iter-zip [1 2] [3 4])) 1)").asLong());
      assertTrue(
          !context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-map (fn [x] x) [1 2])] (iter-close it) (iter-next? it))")
              .asBoolean());
      assertTrue(
          !context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-zip [1 2] [3 4])] (iter-close it) (iter-next? it))")
              .asBoolean());
    }
  }

  @Test
  public void cycleAndPartitionPairRemainIteratorBacked() {
    try (Context context = context()) {
      assertEquals(
          1,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-cycle [1 2])] (iter-next it) (iter-next it) (iter-next it))")
              .asLong());
      assertEquals(
          2,
          context
              .eval(HaraLanguage.ID, "(nth (iter-next (iter-partition-pair [1 2 3 4])) 1)")
              .asLong());
    }
  }

  @Test
  public void mapcatAndKeepRemainLazyIteratorCombinators() {
    try (Context context = context()) {
      assertEquals(
          2,
          context
              .eval(
                  HaraLanguage.ID,
                  "(let [it (iter-mapcat (fn [x] [x (+ x 10)]) [1 2])] "
                      + "(iter-next it) (iter-next it) (iter-next it))")
              .asLong());
      assertEquals(
          2,
          context
              .eval(HaraLanguage.ID, "(iter-next (iter-keep (fn [x] (if (= x 2) x nil)) [1 2 3]))")
              .asLong());
    }
  }

  private static Context context() {
    return Context.newBuilder(HaraLanguage.ID).build();
  }
}
