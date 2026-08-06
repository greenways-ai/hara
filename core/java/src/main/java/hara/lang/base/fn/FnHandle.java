package hara.lang.base.fn;

import hara.lang.base.Ex;
import hara.lang.base.Iter;
import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.lang.invoke.MethodHandle;
import java.util.Arrays;
import java.util.LinkedList;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

@SuppressWarnings({"rawtypes", "unchecked"})
public class FnHandle<R> extends ObjFn implements IFn<R, Object, Object> {
  final MethodHandle _mh;
  final int _num;

  public FnHandle(IMetadata meta, MethodHandle mh, int num) {
    super(meta);
    _mh = mh;
    _num = num;
  }

  public FnHandle(MethodHandle mh, int num) {
    this(null, mh, num);
  }

  public void checkArgs(int size) {
    if (size != _num) {
      throw new Ex.Arity(size, "Only " + _num + " Args supported");
    }
  }

  @Override
  public Supplier<R> getArg0() {
    checkArgs(0);
    return () -> invokeHandle(new LinkedList());
  }

  @Override
  public Function<Object, R> getArg1() {
    checkArgs(1);
    return (arg) -> invokeHandle(Arrays.asList(arg));
  }

  @Override
  public BiFunction<Object, Object, R> getArg2() {
    checkArgs(2);
    return (a1, a2) -> invokeHandle(Arrays.asList(a1, a2));
  }

  @Override
  public Function<Object, R> getArgN() {
    return (vargs) -> {
      java.util.List args = Iter.toArrayList(Iter.iter(vargs));
      checkArgs(args.size());
      return invokeHandle(args);
    };
  }

  public R invokeHandle(java.util.List args) {
    try {
      return (R) _mh.invokeWithArguments(args);
    } catch (Throwable t) {
      throw Ex.Sneaky(t);
    }
  }
}
