package hara.kernel.base;

import hara.kernel.builtin.BuiltinBasic;
import hara.kernel.builtin.BuiltinRef;
import hara.lang.data.Symbol;
import hara.lang.data.Atom;
import hara.lang.base.primitive.Volatile;
import hara.lang.base.primitive.Counter;
import hara.lang.base.primitive.Num;
import hara.lang.base.Fn;
import java.util.function.BiFunction;
import hara.kernel.builtin.BuiltinBasic;
import hara.kernel.builtin.BuiltinRef;
import org.junit.Test;

import static org.junit.Assert.*;

public class BuiltinRefTest {

  @Test
  public void testAtomSwap() {
    Atom.Standard<Integer> atom = BuiltinBasic.atom(10);
    // Use Fn.toFn or cast if necessary, but here we test Builtin wrapper
    // The BuiltinRef.swap expects IFn

    BuiltinRef.swap(atom, Fn.toFn((java.util.function.Function<Integer, Integer>) i -> i + 1));
    assertEquals(Integer.valueOf(11), atom.deref());
  }

  @Test
  public void testAtomSwapArgs() {
    Atom.Standard<Integer> atom = BuiltinBasic.atom(10);
    BuiltinRef.swap(
        atom,
        Fn.toFn((java.util.function.BiFunction<Integer, Integer, Integer>) (i, n) -> i + n),
        5);
    assertEquals(Integer.valueOf(15), atom.deref());
  }

  @Test
  public void testAtomCompareSet() {
    Atom.Standard<Integer> atom = BuiltinBasic.atom(10);
    assertTrue(BuiltinRef.compareSet(atom, 10, 11));
    assertEquals(Integer.valueOf(11), atom.deref());
    assertFalse(BuiltinRef.compareSet(atom, 10, 12));
    assertEquals(Integer.valueOf(11), atom.deref());
  }

  @Test
  public void testVolatileReset() {
    Volatile<Integer> v = BuiltinBasic.atomVolatile(10);
    BuiltinRef.vreset(v, 20);
    assertEquals(Integer.valueOf(20), v.deref());
  }

  @Test
  public void testVolatileSwap() {
    Volatile<Integer> v = BuiltinBasic.atomVolatile(10);
    BuiltinRef.vswap(v, Fn.toFn((java.util.function.Function<Integer, Integer>) i -> i + 1));
    assertEquals(Integer.valueOf(11), v.deref());
  }

  @Test
  public void testCounter() {
    Counter c = BuiltinBasic.counter(10);
    assertEquals(10, BuiltinRef.counterVal(c));
    assertEquals(11, BuiltinRef.counterInc(c));
    assertEquals(10, BuiltinRef.counterDec(c));
    assertEquals(15, BuiltinRef.counterInc(c, 5));
    assertEquals(10, BuiltinRef.counterDec(c, 5));
  }
}
