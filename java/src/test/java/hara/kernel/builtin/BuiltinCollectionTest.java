package hara.kernel.builtin;

import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.HashMap;

public class BuiltinCollectionTest {

  @Test
  public void testConcat() {
    List<Integer> l1 = Arrays.asList(1, 2);
    List<Integer> l2 = Arrays.asList(3, 4);
    // concat takes iterable of iterables
    java.util.Iterator res = BuiltinCollection.concat(Arrays.asList(l1, l2));
    assertTrue(res.hasNext());
    assertEquals(Integer.valueOf(1), res.next());
  }

  @Test
  public void testCons() {
    // cons expects ICons or List (via option)
    // But java.util.Arrays.asList returns fixed-size list, so add(0, e) throws
    // UnsupportedOperationException
    // We need a mutable list
    List<Integer> l = new java.util.ArrayList<>(Arrays.asList(2, 3));
    List<Integer> res = BuiltinCollection.cons(l, 1);
    assertEquals(Integer.valueOf(1), res.get(0));
    assertEquals(Integer.valueOf(2), res.get(1));
  }

  @Test
  public void testCount() {
    assertEquals(3, BuiltinCollection.count(Arrays.asList(1, 2, 3)));
    assertEquals(0, BuiltinCollection.count(Arrays.asList()));
  }

  @Test
  public void testEmpty() {
    // empty clears the collection
    List<Integer> l = new java.util.ArrayList<>(Arrays.asList(1, 2));
    BuiltinCollection.empty(l);
    assertTrue(l.isEmpty());
  }

  @Test
  public void testGet() {
    Map<String, Integer> m = new HashMap<>();
    m.put("a", 1);
    assertEquals(Integer.valueOf(1), BuiltinCollection.get(m, "a"));
    assertNull(BuiltinCollection.get(m, "b"));
    assertEquals(Integer.valueOf(2), BuiltinCollection.get(m, "b", 2));
  }

  @Test
  public void testHas() {
    Map<String, Integer> m = new HashMap<>();
    m.put("a", 1);
    assertTrue(BuiltinCollection.has(m, "a"));
    assertFalse(BuiltinCollection.has(m, "b"));
  }

  @Test
  public void testNth() {
    List<Integer> l = Arrays.asList(1, 2, 3);
    assertEquals(Integer.valueOf(2), BuiltinCollection.nth(l, 1));
  }

  @Test
  public void testToSeq() {
    hara.lang.data.Seq<Integer> singleton = BuiltinCollection.toSeq(Arrays.asList(1));
    assertNotNull(singleton);
    assertEquals(Integer.valueOf(1), singleton.peekFirst());
    assertEquals(1, singleton.count());
    java.util.ArrayList<Integer> values = new java.util.ArrayList<>();
    singleton.iterator().forEachRemaining(values::add);
    assertEquals(Arrays.asList(1), values);
    assertNull(BuiltinCollection.toSeq(java.util.Collections.emptyList()));
  }

  @Test
  public void testZip() {
    // Zip expects iterables
    List<Integer> l1 = Arrays.asList(1, 2);
    List<String> l2 = Arrays.asList("a", "b");
    // zip returns a sequence of vectors
    // Pass iterators to zip
    java.util.Iterator res = BuiltinCollection.zip(Arrays.asList(l1.iterator(), l2.iterator()));
    assertNotNull(res);
    assertTrue(res.hasNext());
  }
}
