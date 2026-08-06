package hara.lang.data;

import org.junit.Test;

import java.util.Iterator;
import java.util.Map.Entry;

import static org.junit.Assert.*;

public class SortedCollectionsTest {

  @Test
  public void testSortedMap() {
    SortedMap.Standard<String, Integer> map = SortedMap.Standard.from(null, "c", 3, "a", 1, "b", 2);
    assertEquals(3, map.count());
    assertEquals(Integer.valueOf(1), map.lookup("a"));

    // Test iteration order
    Iterator<Entry<String, Integer>> it = map.iterator();
    assertEquals("a", it.next().getKey());
    assertEquals("b", it.next().getKey());
    assertEquals("c", it.next().getKey());
  }

  @Test
  public void testSortedSet() {
    SortedSet.Standard<Integer> set = SortedSet.Standard.from(null, 3, 1, 2, 1, 3);
    assertEquals(3, set.count());
    assertNotNull(set.find(1));

    // Test iteration order
    Iterator<Integer> it = set.iterator();
    assertEquals(Integer.valueOf(1), it.next());
    assertEquals(Integer.valueOf(2), it.next());
    assertEquals(Integer.valueOf(3), it.next());
  }

  @Test
  public void testSortedMapAssocWith() {
    SortedMap.Standard<String, Integer> map = SortedMap.Standard.from(null, "a", 1, "b", 2);
    SortedMap.Standard<String, Integer> updatedMap =
        map.assocWith("a", 3, (oldVal, newVal) -> oldVal + newVal);
    assertEquals(Integer.valueOf(4), updatedMap.lookup("a"));
  }

  @Test
  public void testSortedMapDissocRebalance() {
    // Build a map that requires rebalancing on removal
    SortedMap.Standard<Integer, Integer> map = SortedMap.Standard.from(null);
    for (int i = 0; i < 100; i++) {
      map = map.assoc(i, i);
    }
    assertEquals(100, map.count());
    map = map.dissoc(50);
    assertEquals(99, map.count());
    assertNull(map.lookup(50));
  }

  @Test
  public void testSortedMapNth() {
    SortedMap.Standard<String, Integer> map = SortedMap.Standard.from(null, "c", 3, "a", 1, "b", 2);
    assertEquals("a", map.nth(0).getKey());
    assertEquals("b", map.nth(1).getKey());
    assertEquals("c", map.nth(2).getKey());
  }

  @Test(expected = IndexOutOfBoundsException.class)
  public void testSortedMapNthOutOfBounds() {
    SortedMap.Standard<String, Integer> map = SortedMap.Standard.from(null, "a", 1);
    map.nth(1);
  }

  @Test
  public void testSortedMapIndexOfKey() {
    SortedMap.Standard<String, Integer> map = SortedMap.Standard.from(null, "c", 3, "a", 1, "b", 2);
    assertEquals(0, map.indexOfKey("a"));
    assertEquals(1, map.indexOfKey("b"));
    assertEquals(2, map.indexOfKey("c"));
    assertEquals(-1, map.indexOfKey("d"));
  }

  @Test
  public void testMutableSortedMap() {
    SortedMap.Mutable<String, Integer> map = SortedMap.Mutable.from(null, "c", 3, "a", 1);
    map.assoc("b", 2);
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
  public void testSortedMapWithCustomComparator() {
    SortedMap.Standard<String, Integer> map =
        new SortedMap.Standard<String, Integer>(
            null, SortedMap.Node.EMPTY_NODE, java.util.Comparator.reverseOrder());
    map = map.assoc("c", 3).assoc("a", 1).assoc("b", 2);

    Iterator<Entry<String, Integer>> it = map.iterator();
    assertEquals("c", it.next().getKey());
    assertEquals("b", it.next().getKey());
    assertEquals("a", it.next().getKey());
  }

  @Test
  public void testSortedMapToMutable() {
    SortedMap.Standard<String, Integer> map = SortedMap.Standard.from(null, "a", 1, "b", 2);
    SortedMap.Mutable<String, Integer> mutableMap = map.toMutable();
    assertEquals(2, mutableMap.count());
    mutableMap.assoc("c", 3);
    assertEquals(3, mutableMap.count());
  }

  @Test
  public void testMutableSortedMapToPersistent() {
    SortedMap.Mutable<String, Integer> mutableMap = SortedMap.Mutable.from(null, "a", 1, "b", 2);
    SortedMap.Standard<String, Integer> map = mutableMap.toPersistent();
    assertEquals(2, map.count());
    map.assoc("c", 3);
    assertEquals(2, map.count()); // Persistent map should not be changed
  }

  @Test
  public void testMutableSortedSet() {
    SortedSet.Mutable<Integer> set = SortedSet.Mutable.from(null, 3, 1);
    set.conj(2);
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

  @Test
  public void testSortedSetConjDuplicate() {
    SortedSet.Standard<Integer> set = SortedSet.Standard.from(null, 1, 2, 3);
    SortedSet.Standard<Integer> updatedSet = set.conj(2);
    assertEquals(3, updatedSet.count());
    assertSame(set, updatedSet); // Should return the same instance
  }

  @Test
  public void testSortedSetDissocNonExistent() {
    SortedSet.Standard<Integer> set = SortedSet.Standard.from(null, 1, 2, 3);
    SortedSet.Standard<Integer> updatedSet = set.dissoc(4);
    assertEquals(3, updatedSet.count());
    assertSame(set, updatedSet); // Should return the same instance
  }

  @Test
  public void testSortedSetEmpty() {
    SortedSet.Standard<Integer> set = SortedSet.Standard.from(null, 1, 2, 3);
    SortedSet.Standard<Integer> emptySet = set.empty();
    assertEquals(0, emptySet.count());
  }

  @Test
  public void testSortedSetWithCustomComparator() {
    SortedMap.Standard<Integer, Integer> map =
        new SortedMap.Standard<Integer, Integer>(
            null, SortedMap.Node.EMPTY_NODE, java.util.Comparator.reverseOrder());
    SortedSet.Standard<Integer> set = new SortedSet.Standard<Integer>(null, map);
    set = set.conj(3).conj(1).conj(2);

    Iterator<Integer> it = set.iterator();
    assertEquals(Integer.valueOf(3), it.next());
    assertEquals(Integer.valueOf(2), it.next());
    assertEquals(Integer.valueOf(1), it.next());
  }

  @Test
  public void testSortedSetToMutable() {
    SortedSet.Standard<Integer> set = SortedSet.Standard.from(null, 1, 2);
    SortedSet.Mutable<Integer> mutableSet = set.toMutable();
    assertEquals(2, mutableSet.count());
    mutableSet.conj(3);
    assertEquals(3, mutableSet.count());
  }

  @Test
  public void testMutableSortedSetToPersistent() {
    SortedSet.Mutable<Integer> mutableSet = SortedSet.Mutable.from(null, 1, 2);
    SortedSet.Standard<Integer> set = mutableSet.toPersistent();
    assertEquals(2, set.count());
    set.conj(3);
    assertEquals(2, set.count()); // Persistent set should not be changed
  }
}
