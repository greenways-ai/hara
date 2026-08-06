package hara.kernel.jvm;

import hara.kernel.base.RT;
import hara.kernel.flavor.NativeFlavorProvider;
import hara.lang.base.Ex;
import hara.lang.base.Iter;
import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

/** Interpreter callable used by the explicit hara.native.jvm namespace family. */
public final class JvmNativeFunction extends ObjFn implements IFn<Object, Object, Object> {
  private final String name;
  private final Function<Object[], Object> implementation;

  public JvmNativeFunction(String name, Function<Object[], Object> implementation) {
    this.name = name;
    this.implementation = implementation;
  }

  public static JvmFlavorProvider provider(RT.Instance<?> runtime) {
    NativeFlavorProvider provider = runtime.nativeProvider();
    if (!(provider instanceof JvmFlavorProvider)) {
      throw new Ex.Runtime("JVM native operation requires an ns :flavor :jvm declaration");
    }
    return (JvmFlavorProvider) provider;
  }

  public static void requireArity(String name, Object[] values, int expected) {
    if (values.length != expected) {
      throw new Ex.Arity(values.length, name + " expects " + expected + " arguments");
    }
  }

  @Override
  public Supplier<Object> getArg0() {
    return () -> implementation.apply(new Object[0]);
  }

  @Override
  public Function<Object, Object> getArg1() {
    return value -> implementation.apply(new Object[] {value});
  }

  @Override
  public BiFunction<Object, Object, Object> getArg2() {
    return (first, second) -> implementation.apply(new Object[] {first, second});
  }

  @Override
  public Function<Object, Object> getArgN() {
    return arguments -> implementation.apply(Iter.toArray(Iter.iter(arguments)));
  }

  @Override
  public String toString() {
    return "#<jvm-native " + name + ">";
  }
}
