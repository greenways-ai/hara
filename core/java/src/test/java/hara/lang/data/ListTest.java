package hara.lang.data;

import org.junit.Test;

import static org.junit.Assert.*;

public class ListTest {

  @Test
  public void testMutableListCreation() {
    List.Mutable<Integer> list = new List.Mutable<>();
    assertEquals(0, list.count());
  }

  @Test
  public void testMutablePushLast() {
    List.Mutable<Integer> list = new List.Mutable<>();
    list.pushLast(1);
    list.pushLast(2);
    assertEquals(2, list.count());
    assertEquals(Integer.valueOf(1), list.nth(0));
    assertEquals(Integer.valueOf(2), list.nth(1));
  }

  @Test
  public void testMutablePushFirst() {
    List.Mutable<Integer> list = new List.Mutable<>();
    list.pushFirst(1);
    list.pushFirst(2);
    assertEquals(2, list.count());
    assertEquals(Integer.valueOf(2), list.nth(0));
    assertEquals(Integer.valueOf(1), list.nth(1));
  }

  @Test
  public void testMutablePopLast() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    list.popLast();
    assertEquals(2, list.count());
    assertEquals(Integer.valueOf(2), list.nth(1));
  }

  @Test
  public void testMutablePopFirst() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    list.popFirst();
    assertEquals(2, list.count());
    assertEquals(Integer.valueOf(2), list.nth(0));
  }

  @Test
  public void testMutableAssoc() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    list.assoc(1, 5);
    assertEquals(3, list.count());
    assertEquals(Integer.valueOf(5), list.nth(1));
    list.assoc(3, 4); // This should be equivalent to pushLast
    assertEquals(4, list.count());
    assertEquals(Integer.valueOf(4), list.nth(3));
  }

  @Test
  public void testMutableConj() {
    List.Mutable<Integer> list = new List.Mutable<>();
    list.conj(1);
    list.conj(2);
    assertEquals(2, list.count());
    assertEquals(Integer.valueOf(1), list.nth(0));
    assertEquals(Integer.valueOf(2), list.nth(1));
  }

  @Test
  public void testMutableCons() {
    List.Mutable<Integer> list = new List.Mutable<>();
    list.cons(1);
    list.cons(2);
    assertEquals(2, list.count());
    assertEquals(Integer.valueOf(2), list.nth(0));
    assertEquals(Integer.valueOf(1), list.nth(1));
  }

  @Test
  public void testMutableEmpty() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    list.empty();
    assertEquals(0, list.count());
  }

  @Test
  public void testToPersistent() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    List.Standard<Integer> persistentList = list.toPersistent();
    assertEquals(3, persistentList.count());
    assertEquals(Integer.valueOf(1), persistentList.nth(0));
  }

  @Test
  public void testStandardListCreation() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    assertEquals(3, list.count());
    assertEquals(Integer.valueOf(1), list.nth(0));
  }

  @Test
  public void testStandardPushLast() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2);
    List.Standard<Integer> newList = list.pushLast(3);
    assertEquals(2, list.count()); // Original list is unchanged
    assertEquals(3, newList.count());
    assertEquals(Integer.valueOf(3), newList.nth(2));
  }

  @Test
  public void testStandardPushFirst() {
    List.Standard<Integer> list = List.Standard.from(null, 2, 3);
    List.Standard<Integer> newList = list.pushFirst(1);
    assertEquals(2, list.count());
    assertEquals(3, newList.count());
    assertEquals(Integer.valueOf(1), newList.nth(0));
  }

  @Test
  public void testStandardPopLast() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    List.Standard<Integer> newList = list.popLast();
    assertEquals(3, list.count());
    assertEquals(2, newList.count());
    assertEquals(Integer.valueOf(2), newList.nth(1));
  }

  @Test
  public void testStandardPopFirst() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    List.Standard<Integer> newList = list.popFirst();
    assertEquals(3, list.count());
    assertEquals(2, newList.count());
    assertEquals(Integer.valueOf(2), newList.nth(0));
  }

  @Test
  public void testStandardAssoc() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    List.Standard<Integer> newList = list.assoc(1, 5);
    assertEquals(3, list.count());
    assertEquals(Integer.valueOf(2), list.nth(1));
    assertEquals(3, newList.count());
    assertEquals(Integer.valueOf(5), newList.nth(1));
  }

  @Test
  public void testStandardConj() {
    List.Standard<Integer> list = List.Standard.from(null, 1);
    List.Standard<Integer> newList = list.conj(2);
    assertEquals(1, list.count());
    assertEquals(2, newList.count());
    assertEquals(Integer.valueOf(2), newList.nth(1));
  }

  @Test
  public void testStandardCons() {
    List.Standard<Integer> list = List.Standard.from(null, 2);
    List.Standard<Integer> newList = list.cons(1);
    assertEquals(1, list.count());
    assertEquals(2, newList.count());
    assertEquals(Integer.valueOf(1), newList.nth(0));
  }

  @Test
  public void testStandardEmpty() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    List.Standard<Integer> newList = list.empty();
    assertEquals(3, list.count());
    assertEquals(0, newList.count());
  }

  @Test
  public void testToMutable() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    List.Mutable<Integer> mutableList = list.toMutable();
    assertEquals(3, mutableList.count());
    assertEquals(Integer.valueOf(1), mutableList.nth(0));
  }

  @Test
  public void testMutableIterator() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    java.util.Iterator<Integer> it = list.iterator();
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(1), it.next());
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(2), it.next());
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(3), it.next());
    assertFalse(it.hasNext());
  }

  @Test
  public void testStandardIterator() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    java.util.Iterator<Integer> it = list.iterator();
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(1), it.next());
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(2), it.next());
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(3), it.next());
    assertFalse(it.hasNext());
  }

  @Test(expected = IndexOutOfBoundsException.class)
  public void testMutableNthOutOfBounds() {
    List.Mutable<Integer> list = List.Mutable.from(null, 1, 2, 3);
    list.nth(3);
  }

  @Test(expected = IndexOutOfBoundsException.class)
  public void testStandardNthOutOfBounds() {
    List.Standard<Integer> list = List.Standard.from(null, 1, 2, 3);
    list.nth(3);
  }

  @Test
  public void testMutableResize() {
    List.Mutable<Integer> list = new List.Mutable<>(null, 2);
    list.pushLast(1);
    list.pushLast(2);
    list.pushLast(3);
    assertEquals(3, list.count());
    assertEquals(Integer.valueOf(1), list.nth(0));
    assertEquals(Integer.valueOf(2), list.nth(1));
    assertEquals(Integer.valueOf(3), list.nth(2));
  }

  @Test
  public void testStandardResize() {
    List.Standard<Integer> list = List.Standard.from(null, 1);
    list = list.pushLast(2);
    list = list.pushLast(3);
    list = list.pushLast(4);
    list = list.pushLast(5);
    assertEquals(5, list.count());
    assertEquals(Integer.valueOf(1), list.nth(0));
    assertEquals(Integer.valueOf(5), list.nth(4));
  }

  @Test
  public void testEmptyListOperations() {
    List.Mutable<Integer> list = new List.Mutable<>();
    list.popFirst();
    list.popLast();
    assertEquals(0, list.count());
    List.Standard<Integer> stdList = List.Standard.from(null);
    List.Standard<Integer> afterPopFirst = stdList.popFirst();
    List.Standard<Integer> afterPopLast = stdList.popLast();
    assertEquals(0, stdList.count());
    assertEquals(0, afterPopFirst.count());
    assertEquals(0, afterPopLast.count());
  }
}
