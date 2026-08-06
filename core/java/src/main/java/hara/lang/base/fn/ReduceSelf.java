package hara.lang.base.fn;

import hara.lang.base.Fn;
import hara.lang.base.Iter;
import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

@SuppressWarnings({"rawtypes", "unchecked"})
public class ReduceSelf<R, E, FN> extends ObjFn implements IFn<R, R, E> {
  final BiFunction<R, E, R> _f2;
  final R _init;

  public ReduceSelf(IMetadata meta, R init, FN f) {
    super(meta);
    _init = init;
    _f2 = Fn.toFn(f).getArg2();
  }

  public ReduceSelf(R init, FN f) {
    this(null, init, f);
  }

  @Override
  public Supplier<R> getArg0() {
    return () -> _init;
  }

  @Override
  public Function<R, R> getArg1() {
    return (e) -> e;
  }

  @Override
  public BiFunction<R, E, R> getArg2() {
    return (e0, e1) -> _f2.apply(e0, e1);
  }

  @Override
  public Function getArgN() {
    return (es) -> Iter.reduceIn(Iter.iter(es), _init, _f2);
  }
}
