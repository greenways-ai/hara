package hara.lang.data;

import hara.lang.data.Tuple.Tup0;
import hara.lang.data.Tuple.Tup1;
import hara.lang.data.Tuple.Tup2;
import hara.lang.data.Tuple.Tup3;
import org.junit.Test;

import java.util.Iterator;

import static org.junit.Assert.*;

public class TupleTest {

  @Test
  public void testTup0() {
    Tup0 t = Tup0.EMPTY;
    assertEquals(0, t.count());
    assertFalse(t.iterator().hasNext());
  }

  @Test
  public void testTup1() {
    Tup1.L<Integer> t = new Tup1.L<>(null, 1);
    assertEquals(1, t.count());
    assertEquals(1, (int) t.A());
    assertEquals(1, t.nth(0));

    Iterator it = t.iterator();
    assertTrue(it.hasNext());
    assertEquals(1, it.next());
    assertFalse(it.hasNext());
  }

  @Test
  public void testTup2() {
    Tup2.L<Integer, String> t = new Tup2.L<>(null, 1, "a");
    assertEquals(2, t.count());
    assertEquals(1, (int) t.A());
    assertEquals("a", t.B());

    assertEquals(1, t.nth(0));
    assertEquals("a", t.nth(1));
  }

  @Test
  public void testPushPop() {
    Tup0 t0 = Tup0.EMPTY;

    Tup1 t1 = (Tup1) t0.pushLast(1);
    assertEquals(1, t1.A());

    Tup2 t2 = (Tup2) t1.pushLast(2);
    assertEquals(1, t2.A());
    assertEquals(2, t2.B());

    Tup3 t3 = (Tup3) t2.pushLast(3);
    assertEquals(3, t3.C());

    Tup2 t2_pop = (Tup2) t3.popLast();
    assertEquals(1, t2_pop.A());
    assertEquals(2, t2_pop.B());
  }
}
