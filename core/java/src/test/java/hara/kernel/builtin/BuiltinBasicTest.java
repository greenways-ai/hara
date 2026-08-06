package hara.kernel.builtin;

import hara.lang.data.Atom;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.base.primitive.Counter;
import hara.lang.base.primitive.Flag;
import org.junit.Test;
import static org.junit.Assert.*;

public class BuiltinBasicTest {

  @Test
  public void testAtom() {
    Atom.Standard<Integer> a = BuiltinBasic.atom(10);
    assertEquals((Object) 10, a.deref());
  }

  @Test
  public void testCompare() {
    assertEquals(0, BuiltinBasic.compare(1, 1));
    assertEquals(-1, BuiltinBasic.compare(1, 2));
    assertEquals(1, BuiltinBasic.compare(2, 1));
    assertEquals(0, BuiltinBasic.compare("a", "a"));
  }

  @Test
  public void testCounter() {
    Counter c = BuiltinBasic.counter();
    assertEquals(-1, ((Integer) c.deref()).intValue());

    c = BuiltinBasic.counter(10);
    assertEquals(10, ((Integer) c.deref()).intValue());
  }

  @Test
  public void testDeref() {
    Atom.Standard<Integer> a = BuiltinBasic.atom(10);
    assertEquals((Object) 10, BuiltinBasic.deref(a));
  }

  @Test
  public void testFlag() {
    Flag f = BuiltinBasic.flag();
    assertFalse(f.deref());

    f = BuiltinBasic.flag(true);
    assertTrue(f.deref());
  }

  @Test
  public void testHash() {
    assertEquals("a".hashCode(), BuiltinBasic.hash("a"));
  }

  @Test
  public void testKeyword() {
    assertEquals(Keyword.create("foo"), BuiltinBasic.keyword("foo"));
    assertEquals(Keyword.create("ns", "name"), BuiltinBasic.keyword("ns", "name"));
  }

  @Test
  public void testSymbol() {
    assertEquals(Symbol.create("foo"), BuiltinBasic.symbol("foo"));
    assertEquals(Symbol.create("ns", "name"), BuiltinBasic.symbol("ns", "name"));
  }

  @Test
  public void testType() {
    assertEquals(String.class, BuiltinBasic.type("a"));
    assertNull(BuiltinBasic.type(null));
  }
}
