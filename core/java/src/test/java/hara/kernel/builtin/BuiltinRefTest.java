package hara.kernel.builtin;

import hara.lang.data.Atom;
import hara.lang.base.primitive.Volatile;
import hara.lang.protocol.IFn;
import org.junit.Test;
import static org.junit.Assert.*;

public class BuiltinRefTest {

  @Test
  public void testAtom() {
    Atom.Standard<Integer> a = new Atom.Standard<>(10);
    assertEquals(Integer.valueOf(10), a.deref());
  }

  @Test
  public void testAtomSwap() {
    Atom.Standard<Integer> a = new Atom.Standard<>(10);
    IFn inc = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a1) -> (Integer) a1 + 1);
    assertEquals(Integer.valueOf(11), BuiltinRef.swap(a, inc));
    assertEquals(Integer.valueOf(11), a.deref());
  }

  @Test
  public void testAtomWatch() {
    Atom.Standard<Integer> a = new Atom.Standard<>(10);
    final boolean[] called = {false};
    IFn watcher =
        hara.lang.base.Fn.toFnVargs(
            (java.util.function.Function)
                (args) -> {
                  called[0] = true;
                  return null;
                });

    BuiltinRef.watchAdd(a, "key", watcher);

    // Use swap to trigger watch (reset doesn't notify in Struct)
    IFn inc = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a1) -> (Integer) a1 + 10);
    BuiltinRef.swap(a, inc);

    assertTrue(called[0]);

    BuiltinRef.watchRemove(a, "key");
    called[0] = false;
    BuiltinRef.swap(a, inc);
    assertFalse(called[0]);
  }

  @Test
  public void testVolatile() {
    Volatile<Integer> v = new Volatile<>(10);
    assertEquals(Integer.valueOf(20), BuiltinRef.vreset(v, 20));
    assertEquals(Integer.valueOf(20), v.deref());

    IFn inc = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a1) -> (Integer) a1 + 1);
    assertEquals(Integer.valueOf(21), BuiltinRef.vswap(v, inc));
  }
}
