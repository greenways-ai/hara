package hara.lang.data;

import org.junit.Test;

import java.util.Iterator;
import java.util.Map.Entry;

import static org.junit.Assert.*;

public class OrderedCollectionsTest {

  @Test
  public void testOrderedMap() {
    OrderedMap.Standard<String, Integer> map =
        OrderedMap.Standard.from(null, "a", 1, "b", 2, "c", 3);
    assertEquals(3, map.count());
    assertEquals(Integer.valueOf(1), map.lookup("a"));

    // Test iteration order
    Iterator<Entry<String, Integer>> it = map.iterator();
    assertEquals("a", it.next().getKey());
    assertEquals("b", it.next().getKey());
    assertEquals("c", it.next().getKey());
  }

  @Test
  public void testOrderedSet() {
    OrderedSet.Standard<Integer> set = OrderedSet.Standard.from(null, 1, 2, 3, 2, 1);
    assertEquals(3, set.count());
    assertNotNull(set.find(1));

    // Test iteration order
    Iterator<Integer> it = set.iterator();
    assertEquals(Integer.valueOf(1), it.next());
    assertEquals(Integer.valueOf(2), it.next());
    assertEquals(Integer.valueOf(3), it.next());
  }

  @Test
  public void testMutableOrderedMap() {
    OrderedMap.Mutable<String, Integer> map = OrderedMap.Mutable.from(null, "a", 1, "b", 2);
    map.assoc("c", 3);
    assertEquals(3, map.count());

    // Test iteration order
    Iterator<Entry<String, Integer>> it = map.iterator();
    assertEquals("a", it.next().getKey());
    assertEquals("b", it.next().getKey());
    assertEquals("c", it.next().getKey());

    // Test dissoc
    map.dissoc("b");
    assertEquals(2, map.count());
    assertNull(map.lookup("b"));
  }

  @Test
  public void testMutableOrderedSet() {
    OrderedSet.Mutable<Integer> set = OrderedSet.Mutable.from(null, 1, 2);
    set.conj(3);
    assertEquals(3, set.count());

    // Test iteration order
    Iterator<Integer> it = set.iterator();
    assertEquals(Integer.valueOf(1), it.next());
    assertEquals(Integer.valueOf(2), it.next());
    assertEquals(Integer.valueOf(3), it.next());

    // Test dissoc
    set.dissoc(2);
    assertEquals(2, set.count());
    assertNull(set.find(2));
  }
}
