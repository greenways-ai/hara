package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertThrows;

import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.InvalidArrayIndexException;
import com.oracle.truffle.api.interop.UnknownKeyException;
import com.oracle.truffle.api.interop.UnsupportedMessageException;
import hara.lang.data.Map;
import hara.lang.data.Vector;
import java.util.ArrayList;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;
import org.junit.Test;

public class HaraInteropTest {
  @Test
  public void exposesJavaBackedValuesAsArraysHashesExecutablesAndIterators() throws Exception {
    InteropLibrary interop = InteropLibrary.getUncached();
    Object vector = HaraBox.export(Vector.Standard.from(null, "zero", "one"));
    assertTrue(interop.hasArrayElements(vector));
    assertEquals(2, interop.getArraySize(vector));
    assertEquals("one", interop.readArrayElement(vector, 1));

    Map.Standard<String, String> map = Map.Standard.from(null, "key", "value");
    Object exportedMap = HaraBox.export(map);
    assertTrue(interop.hasHashEntries(exportedMap));
    assertEquals(1, interop.getHashSize(exportedMap));
    assertEquals("value", interop.readHashValue(exportedMap, "key"));
    assertTrue(interop.isExecutable(exportedMap));
    assertEquals("value", interop.execute(exportedMap, "key"));

    Object entryIterator = interop.getHashEntriesIterator(exportedMap);
    Object entry = interop.getIteratorNextElement(entryIterator);
    assertEquals("key", interop.readArrayElement(entry, 0));
    assertEquals("value", interop.readArrayElement(entry, 1));

    Object javaList = HaraBox.export(new ArrayList<>(java.util.List.of(1L, 2L)));
    assertTrue(interop.hasArrayElements(javaList));
    assertEquals(2, interop.getArraySize(javaList));
    assertEquals(2L, interop.readArrayElement(javaList, 1));

    Object iterator = HaraBox.export(Vector.Standard.from(null, "first", "second").iterator());
    assertTrue(interop.hasIterator(iterator));
    Object iteratorValue = interop.getIterator(iterator);
    assertTrue(interop.isIterator(iteratorValue));
    assertTrue(interop.hasIteratorNextElement(iteratorValue));
    assertEquals("first", interop.getIteratorNextElement(iteratorValue));

    HaraStruct struct =
        new HaraStruct(new HaraType("Person", new String[] {"name"}), new Object[] {"Ada"});
    assertTrue(interop.hasMembers(struct));
    assertEquals("Ada", interop.readMember(struct, "name"));
  }

  @Test
  public void interopFailuresUseCanonicalTruffleMessages() throws Exception {
    InteropLibrary interop = InteropLibrary.getUncached();
    Object vector = HaraBox.export(Vector.Standard.from(null, "zero"));
    assertThrows(InvalidArrayIndexException.class, () -> interop.readArrayElement(vector, 1));
    assertThrows(InvalidArrayIndexException.class, () -> interop.readArrayElement(vector, -1));

    Object map = HaraBox.export(Map.Standard.from(null, "key", "value"));
    assertThrows(UnknownKeyException.class, () -> interop.readHashValue(map, "missing"));
    assertThrows(UnsupportedMessageException.class, () -> interop.getIterator(42L));
  }

  @Test
  public void evaluatesCollectionLiteralContentsBeforeExport() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value vector = context.eval(HaraLanguage.ID, "[1 (+ 1 1)]");
      assertTrue(vector.hasArrayElements());
      assertEquals(2, vector.getArraySize());
      assertEquals(2, vector.getArrayElement(1).asInt());

      Value set = context.eval(HaraLanguage.ID, "#{1 (+ 1 1)}");
      assertTrue(set.hasHashEntries());
      assertEquals(2, set.getHashSize());

      Value map = context.eval(HaraLanguage.ID, "{:value (+ 1 1)}");
      assertTrue(map.hasHashEntries());
      assertEquals(1, map.getHashSize());
    }
  }
}
