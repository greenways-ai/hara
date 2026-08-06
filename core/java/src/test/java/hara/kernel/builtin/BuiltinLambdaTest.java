package hara.kernel.builtin;

import hara.lang.protocol.IFn;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

public class BuiltinLambdaTest {

  @Test
  public void testApply() {
    // apply requires varargs support if passed iterator
    IFn f =
        hara.lang.base.Fn.toFnVargs(
            (java.util.function.Function)
                (args) -> {
                  Iterator it = (Iterator) args;
                  int a = (Integer) it.next();
                  int b = (Integer) it.next();
                  return a + b;
                });
    // apply(f, args) treats args as list of arguments. Last argument must be a
    // sequence.
    // We want apply(f, [1, 2]). So we pass [[1, 2]].
    assertEquals((Object) 3, BuiltinLambda.apply(f, Arrays.asList(Arrays.asList(1, 2))));
  }

  @Test
  public void testCall() {
    IFn f =
        hara.lang.base.Fn.toFn(
            null, (java.util.function.BiFunction) (a, b) -> (Integer) a + (Integer) b);
    // call(obj, f, args) -> f(obj, args...)
    // f expects 2 args. obj is 1st, args[0] is 2nd.
    assertEquals((Object) 3, BuiltinLambda.call(1, f, Arrays.asList(2)));
  }

  @Test
  public void testComp() {
    IFn f1 = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a) -> (Integer) a + 1);
    IFn f2 = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a) -> (Integer) a * 2);
    IFn comp = BuiltinLambda.comp(Arrays.asList(f1, f2));
    // f1(f2(2)) = (2*2) + 1 = 5
    assertEquals((Object) 5, comp.invoke(2));
  }

  @Test
  public void testIdentity() {
    assertEquals("a", BuiltinLambda.identity("a"));
  }

  @Test
  public void testJuxt() {
    IFn f1 = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a) -> (Integer) a + 1);
    IFn f2 = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a) -> (Integer) a * 2);
    IFn juxt = BuiltinLambda.juxt(Arrays.asList(f1, f2));
    // juxt returns Vector, which implements Iterable/ISequential but maybe not
    // java.util.List
    Iterable res = (Iterable) juxt.invoke(2);
    Iterator it = res.iterator();
    assertEquals((Object) 3, it.next());
    assertEquals((Object) 4, it.next());
  }

  @Test
  public void testMap() {
    IFn f = hara.lang.base.Fn.toFn(null, (java.util.function.Function) (a) -> (Integer) a + 1);
    Iterator it = BuiltinLambda.map(f, Arrays.asList(1, 2));
    assertEquals((Object) 2, it.next());
    assertEquals((Object) 3, it.next());
  }

  @Test
  public void testReduce() {
    IFn f =
        hara.lang.base.Fn.toFn(
            null, (java.util.function.BiFunction) (a, b) -> (Integer) a + (Integer) b);
    assertEquals((Object) 3, BuiltinLambda.reduce(f, 0, Arrays.asList(1, 2)));
  }

  @Test
  public void testPartial() {
    IFn f =
        hara.lang.base.Fn.toFn(
            null, (java.util.function.BiFunction) (a, b) -> (Integer) a + (Integer) b);
    IFn p = BuiltinLambda.partial(f, Arrays.asList(1));
    assertEquals((Object) 3, p.invoke(2));
  }
}
