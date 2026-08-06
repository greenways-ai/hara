package hara.lang.data;

import org.junit.Test;

import static org.junit.Assert.*;

public class CollectionsTest {

  @Test
  public void testList() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    assertEquals(3, list.count());
    assertEquals(Integer.valueOf(1), list.nth(0));
    assertEquals(Integer.valueOf(2), list.nth(1));
    assertEquals(Integer.valueOf(3), list.nth(2));

    list = list.conj(4);
    assertEquals(4, list.count());
    assertEquals(Integer.valueOf(4), list.peekLast());

    list = list.popLast();
    assertEquals(3, list.count());
    assertEquals(Integer.valueOf(3), list.peekLast());
  }

  @Test
  public void testVector() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2, 3);
    assertEquals(3, vector.count());
    assertEquals(Integer.valueOf(1), vector.nth(0));
    assertEquals(Integer.valueOf(2), vector.nth(1));
    assertEquals(Integer.valueOf(3), vector.nth(2));

    vector = vector.pushLast(4);
    assertEquals(4, vector.count());
    assertEquals(Integer.valueOf(4), vector.peekLast());

    vector = vector.popLast();
    assertEquals(3, vector.count());
    assertEquals(Integer.valueOf(3), vector.peekLast());
  }

  @Test
  public void testMap() {
    Map.Standard<Keyword, Integer> map =
        Map.Standard.from(null, Keyword.create("a"), 1, Keyword.create("b"), 2);
    assertEquals(2, map.count());
    assertEquals(Integer.valueOf(1), map.lookup(Keyword.create("a")));
    assertEquals(Integer.valueOf(2), map.lookup(Keyword.create("b")));

    map = map.assoc(Keyword.create("c"), 3);
    assertEquals(3, map.count());
    assertEquals(Integer.valueOf(3), map.lookup(Keyword.create("c")));

    map = map.dissoc(Keyword.create("b"));
    assertEquals(2, map.count());
    assertNull(map.lookup(Keyword.create("b")));
  }

  @Test
  public void testSet() {
    Set.Standard<Integer> set = Set.Standard.from(null, 1, 2, 3);
    assertEquals(3, set.count());
    assertNotNull(set.find(1));
    assertNotNull(set.find(2));
    assertNotNull(set.find(3));
    assertNull(set.find(4));

    set = set.conj(4);
    assertEquals(4, set.count());
    assertNotNull(set.find(4));

    set = set.dissoc(2);
    assertEquals(3, set.count());
    assertNull(set.find(2));
  }

  @Test
  public void testMutableVersions() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2);
    list.conj(3);
    assertEquals(3, list.count());

    Vector.Mutable<Integer> vector = Vector.Mutable.from(null, 1, 2);
    vector.conj(3);
    assertEquals(3, vector.count());

    Map.Mutable<Keyword, Integer> map = Map.Mutable.from(null, Keyword.create("a"), 1);
    map.assoc(Keyword.create("b"), 2);
    assertEquals(2, map.count());

    Set.Mutable<Integer> set = Set.Mutable.from(null, 1);
    set.conj(2);
    assertEquals(2, set.count());
  }
}
