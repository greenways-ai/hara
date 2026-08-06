package hara.kernel.base;

import hara.kernel.builtin.BuiltinCollection;
import org.junit.Test;

import static org.junit.Assert.*;

/**
 * Tests for collection operations in {@link Builtin.Collection}. These tests target previously
 * uncovered collection methods.
 */
public class BuiltinCollectionTest {

  @Test
  public void testAssocWithMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("a", 1);

    java.util.Map<String, Integer> result = BuiltinCollection.assoc(map, "b", 2);

    assertEquals(Integer.valueOf(1), result.get("a"));
    assertEquals(Integer.valueOf(2), result.get("b"));
  }

  @Test
  public void testConjWithList() {
    java.util.List<Integer> list = new java.util.ArrayList<>();
    list.add(1);
    list.add(2);

    java.util.List<Integer> result = BuiltinCollection.conj(list, 3);

    assertEquals(3, result.size());
    assertEquals(Integer.valueOf(3), result.get(2));
  }

  @Test
  public void testConjWithSet() {
    java.util.Set<Integer> set = new java.util.HashSet<>();
    set.add(1);
    set.add(2);

    java.util.Set<Integer> result = BuiltinCollection.conj(set, 3);

    assertEquals(3, result.size());
    assertTrue(result.contains(3));
  }

  @Test
  public void testConsWithList() {
    java.util.List<Integer> list = new java.util.ArrayList<>();
    list.add(2);
    list.add(3);

    java.util.List<Integer> result = BuiltinCollection.cons(list, 1);

    assertEquals(3, result.size());
    assertEquals(Integer.valueOf(1), result.get(0));
  }

  @Test
  public void testCountWithArray() {
    Object[] arr = {1, 2, 3, 4};
    long count = BuiltinCollection.count(arr);
    assertEquals(4L, count);
  }

  @Test
  public void testCountWithString() {
    long count = BuiltinCollection.count("hello");
    assertEquals(5L, count);
  }

  @Test
  public void testCountWithCollection() {
    java.util.List<Integer> list = java.util.Arrays.asList(1, 2, 3);
    long count = BuiltinCollection.count(list);
    assertEquals(3L, count);
  }

  @Test
  public void testDissocWithMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("a", 1);
    map.put("b", 2);

    java.util.Map<String, Integer> result = BuiltinCollection.dissoc(map, "a");

    assertEquals(1, result.size());
    assertFalse(result.containsKey("a"));
    assertEquals(Integer.valueOf(2), result.get("b"));
  }

  @Test
  public void testDissocWithSet() {
    java.util.Set<Integer> set = new java.util.HashSet<>();
    set.add(1);
    set.add(2);
    set.add(3);

    java.util.Set<Integer> result = BuiltinCollection.dissoc(set, 2);

    assertEquals(2, result.size());
    assertFalse(result.contains(2));
  }

  @Test
  public void testEmptyWithCollection() {
    java.util.List<Integer> list = new java.util.ArrayList<>();
    list.add(1);
    list.add(2);

    java.util.Collection<Integer> result = BuiltinCollection.empty(list);

    assertEquals(0, result.size());
  }

  @Test
  public void testGetWithMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("key", 42);

    Object result = BuiltinCollection.get(map, "key");
    assertEquals(42, result);
  }

  @Test
  public void testGetWithMapAndDefault() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();

    Object result = BuiltinCollection.get(map, "missing", 99);
    assertEquals(99, result);
  }

  @Test
  public void testHasWithMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("key", 42);

    assertTrue(BuiltinCollection.has(map, "key"));
    assertFalse(BuiltinCollection.has(map, "missing"));
  }

  @Test
  public void testHasWithSet() {
    java.util.Set<Integer> set = new java.util.HashSet<>();
    set.add(1);
    set.add(2);

    assertTrue(BuiltinCollection.has(set, 1));
    assertFalse(BuiltinCollection.has(set, 3));
  }

  @Test
  public void testIntoWithList() {
    java.util.List<Integer> list = new java.util.ArrayList<>();
    list.add(1);

    java.util.List<Integer> result = BuiltinCollection.into(list, java.util.Arrays.asList(2, 3));

    assertEquals(3, result.size());
  }

  @Test
  public void testKeysWithMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("a", 1);
    map.put("b", 2);

    java.util.Iterator<String> keys = BuiltinCollection.keys(map);

    int count = 0;
    while (keys.hasNext()) {
      keys.next();
      count++;
    }
    assertEquals(2, count);
  }

  @Test
  public void testValsWithMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("a", 1);
    map.put("b", 2);

    java.util.Iterator<Integer> vals = BuiltinCollection.vals(map);

    int count = 0;
    while (vals.hasNext()) {
      vals.next();
      count++;
    }
    assertEquals(2, count);
  }
}
