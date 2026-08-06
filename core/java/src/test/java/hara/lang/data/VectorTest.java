package hara.lang.data;

import org.junit.Test;

import static org.junit.Assert.*;

public class VectorTest {

  @Test
  public void testMutableVectorCreation() {
    Vector.Mutable<Integer> vector = Vector.Mutable.empty(null);
    assertEquals(0, vector.count());
  }

  @Test
  public void testMutablePushLast() {
    Vector.Mutable<Integer> vector = Vector.Mutable.empty(null);
    vector.pushLast(1);
    vector.pushLast(2);
    assertEquals(2, vector.count());
    assertEquals(Integer.valueOf(1), vector.nth(0));
    assertEquals(Integer.valueOf(2), vector.nth(1));
  }

  @Test
  public void testMutablePopLast() {
    Vector.Mutable<Integer> vector = Vector.Mutable.from(null, 1, 2, 3);
    vector.popLast();
    assertEquals(2, vector.count());
    assertEquals(Integer.valueOf(2), vector.nth(1));
  }

  @Test
  public void testMutableAssoc() {
    Vector.Mutable<Integer> vector = Vector.Mutable.from(null, 1, 2, 3);
    vector.assoc(1, 5);
    assertEquals(3, vector.count());
    assertEquals(Integer.valueOf(5), vector.nth(1));
    vector.assoc(3, 4); // This should be equivalent to pushLast
    assertEquals(4, vector.count());
    assertEquals(Integer.valueOf(4), vector.nth(3));
  }

  @Test
  public void testMutableConj() {
    Vector.Mutable<Integer> vector = Vector.Mutable.empty(null);
    vector.conj(1);
    vector.conj(2);
    assertEquals(2, vector.count());
    assertEquals(Integer.valueOf(1), vector.nth(0));
    assertEquals(Integer.valueOf(2), vector.nth(1));
  }

  @Test
  public void testStandardVectorCreation() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2, 3);
    assertEquals(3, vector.count());
    assertEquals(Integer.valueOf(1), vector.nth(0));
  }

  @Test
  public void testStandardPushLast() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2);
    Vector.Standard<Integer> newVector = vector.pushLast(3);
    assertEquals(2, vector.count()); // Original vector is unchanged
    assertEquals(3, newVector.count());
    assertEquals(Integer.valueOf(3), newVector.nth(2));
  }

  @Test
  public void testStandardPopLast() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2, 3);
    Vector.Standard<Integer> newVector = vector.popLast();
    assertEquals(3, vector.count());
    assertEquals(2, newVector.count());
    assertEquals(Integer.valueOf(2), newVector.nth(1));
  }

  @Test
  public void testStandardAssoc() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2, 3);
    Vector.Standard<Integer> newVector = vector.assoc(1, 5);
    assertEquals(3, vector.count());
    assertEquals(Integer.valueOf(2), vector.nth(1));
    assertEquals(3, newVector.count());
    assertEquals(Integer.valueOf(5), newVector.nth(1));
  }

  @Test
  public void testStandardConj() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1);
    Vector.Base<Integer> newVector = vector.conj(2);
    assertEquals(1, vector.count());
    assertEquals(2, newVector.count());
    assertEquals(Integer.valueOf(2), newVector.nth(1));
  }

  @Test
  public void testMutableIterator() {
    Vector.Mutable<Integer> vector = Vector.Mutable.from(null, 1, 2, 3);
    java.util.Iterator<Integer> it = vector.iterator();
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
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2, 3);
    java.util.Iterator<Integer> it = vector.iterator();
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
    Vector.Mutable<Integer> vector = Vector.Mutable.from(null, 1, 2, 3);
    vector.nth(3);
  }

  @Test(expected = IndexOutOfBoundsException.class)
  public void testStandardNthOutOfBounds() {
    Vector.Standard<Integer> vector = Vector.Standard.from(null, 1, 2, 3);
    vector.nth(3);
  }
}
