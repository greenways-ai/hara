package hara.lang.data;

import hara.lang.data.types.ILinkedType;
import org.junit.Test;

import java.util.Iterator;

import static org.junit.Assert.*;

public class ConsTest {

  @Test
  public void testCons() {
    Cons<Integer> c3 = new Cons<>(null, 3, null);
    Cons<Integer> c2 = new Cons<>(null, 2, c3);
    Cons<Integer> c1 = new Cons<>(null, 1, c2);

    assertEquals(1, (int) c1.peekFirst());
    assertEquals(3, c1.count());

    ILinkedType<Integer> rest = c1.popFirst();
    assertEquals(2, (int) rest.peekFirst());
    assertEquals(2, rest.count());

    Cons<Integer> c0 = c1.cons(0);
    assertEquals(0, (int) c0.peekFirst());
    assertEquals(4, c0.count());
  }

  @Test
  public void testIterator() {
    Cons<Integer> c3 = new Cons<>(null, 3, null);
    Cons<Integer> c2 = new Cons<>(null, 2, c3);
    Cons<Integer> c1 = new Cons<>(null, 1, c2);

    Iterator<Integer> it = c1.iterator();
    assertTrue(it.hasNext());
    assertEquals(1, (int) it.next());
    assertTrue(it.hasNext());
    assertEquals(2, (int) it.next());
    assertTrue(it.hasNext());
    assertEquals(3, (int) it.next());
    assertFalse(it.hasNext());
  }

  @Test(expected = java.util.NoSuchElementException.class)
  public void testIteratorException() {
    Cons<Integer> c1 = new Cons<>(null, 1, null);
    Iterator<Integer> it = c1.iterator();
    it.next();
    it.next();
  }
}
