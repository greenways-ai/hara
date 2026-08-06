package hara.transpile.base;

import hara.lang.data.Symbol;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

public class UtilTest {

  @Test
  public void testSymId() {
    Symbol sym = Symbol.create("ns", "name");
    assertEquals(Symbol.create("name"), Util.symId(sym));
    assertEquals(Symbol.create("foo"), Util.symId("foo"));
  }

  @Test
  public void testSymModule() {
    Symbol sym = Symbol.create("ns", "name");
    assertEquals(Symbol.create("ns"), Util.symModule(sym));
    assertNull(Util.symModule("foo"));
    assertNull(Util.symModule(Symbol.create("name")));
  }

  @Test
  public void testSymPair() {
    Symbol sym = Symbol.create("ns", "name");
    List<Symbol> pair = Util.symPair(sym);
    assertEquals(Symbol.create("ns"), pair.get(0));
    assertEquals(Symbol.create("name"), pair.get(1));
  }

  @Test
  public void testSymFull() {
    assertEquals(Symbol.create("ns", "name"), Util.symFull("ns", "name"));
  }

  @Test
  public void testSymDefaultStr() {
    assertEquals("foo_bar", Util.symDefaultStr("foo-bar"));
  }

  @Test
  public void testSymDefaultInverseStr() {
    assertEquals("foo-bar", Util.symDefaultInverseStr("foo_bar"));
  }

  @Test
  public void testHashvecQ() {
    Set<Object> set = new HashSet<>();
    set.add(Arrays.asList(1, 2));
    assertTrue(Util.hashvecQ(set));

    set.clear();
    set.add("foo");
    assertFalse(Util.hashvecQ(set));

    set.clear();
    set.add(Arrays.asList(1));
    set.add(Arrays.asList(2));
    assertFalse(Util.hashvecQ(set)); // size != 1
  }

  @Test
  public void testDoublevecQ() {
    List<Object> list = Arrays.asList(Arrays.asList(1, 2));
    assertTrue(Util.doublevecQ(list));

    assertFalse(Util.doublevecQ(Arrays.asList("foo")));
    assertFalse(Util.doublevecQ(Arrays.asList(Arrays.asList(1), Arrays.asList(2)))); // size != 1
  }
}
