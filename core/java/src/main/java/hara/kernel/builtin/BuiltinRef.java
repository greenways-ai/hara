package hara.kernel.builtin;

import hara.kernel.base.Module;
import hara.lang.base.Fn;
import hara.lang.protocol.IFn;
import hara.lang.data.Atom;
import hara.lang.protocol.IWatch;

import java.util.Iterator;

@SuppressWarnings({"unchecked", "rawtypes"})
@Module.Ns(name = "global", tag = "ref")
public interface BuiltinRef {

  //
  // Atom
  //

  @Module.Fn(name = "compare:set!", protocol = true)
  public static <V> boolean compareSet(Atom.Swap<Atom, V> atom, V oldVal, V newVal) {
    return atom.cas(oldVal, newVal);
  }

  @Module.Fn(name = "swap!", vargs = true, protocol = true)
  public static <V> V swap(Atom.Swap<Atom, V> atom, IFn f, Object... args) {
    return (V)
        atom.swap(
            (java.util.function.BiFunction<V, Object[], V>)
                (v, a) -> {
                  Object[] allArgs = new Object[a.length + 1];
                  allArgs[0] = v;
                  System.arraycopy(a, 0, allArgs, 1, a.length);
                  return (V) f.apply(allArgs);
                },
            args);
  }

  @Module.Fn(name = "watch-add", protocol = true)
  public static <V> Atom.Standard<V> watchAdd(
      Atom.Standard<V> atom, Object key, IFn<Object, IWatch.WatchEntry<Atom, V>, Object> f) {
    atom.addWatch(key, (e) -> f.invoke(e.A(), e.B(), e.D(), e.E()));
    return atom;
  }

  @Module.Fn(name = "watch-remove", protocol = true)
  public static <V> Atom.Standard<V> watchRemove(Atom.Standard<V> atom, Object key) {
    atom.removeWatch(key);
    return atom;
  }

  @Module.Fn(name = "watch-list", protocol = true)
  public static <V> Iterator watchList(Atom.Standard<V> atom) {
    return atom.getWatches();
  }

  //
  // Volatile
  //

  @Module.Fn(name = "vreset!", protocol = true)
  public static <V> V vreset(hara.lang.base.primitive.Volatile<V> v, V val) {
    return v.reset(val);
  }

  @Module.Fn(name = "vswap!", protocol = true, vargs = true)
  public static <V> V vswap(hara.lang.base.primitive.Volatile<V> v, IFn f, Object... args) {
    synchronized (v) {
      Object[] allArgs = new Object[args.length + 1];
      allArgs[0] = v.deref();
      System.arraycopy(args, 0, allArgs, 1, args.length);
      V newVal = (V) f.apply(allArgs);
      v.reset(newVal);
      return newVal;
    }
  }

  //
  // Counter
  //

  @Module.Fn(name = "counter:inc")
  public static long counterInc(hara.lang.base.primitive.Counter c) {
    return c.inc();
  }

  @Module.Fn(name = "counter:inc")
  public static long counterInc(hara.lang.base.primitive.Counter c, long n) {
    return c.inc((int) n);
  }

  @Module.Fn(name = "counter:dec")
  public static long counterDec(hara.lang.base.primitive.Counter c) {
    return c.dec();
  }

  @Module.Fn(name = "counter:dec")
  public static long counterDec(hara.lang.base.primitive.Counter c, long n) {
    return c.dec((int) n);
  }

  @Module.Fn(name = "counter:val")
  public static long counterVal(hara.lang.base.primitive.Counter c) {
    return c.deref();
  }
}
