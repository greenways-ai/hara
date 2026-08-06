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
public class ReduceArray<E, FN> extends ObjFn implements IFn<E, E, E> {
  final BiFunction<E, E, E> _f2;
  final E _init;

  public ReduceArray(E init, FN f) {
    this(null, init, f);
  }

  public ReduceArray(IMetadata meta, E init, FN f) {
    super(meta);
    _init = init;
    _f2 = Fn.toFn(f).getArg2();
  }

  @Override
  public Supplier<E> getArg0() {
    return () -> _init;
  }

  @Override
  public Function<E, E> getArg1() {
    return (e) -> _f2.apply(_init, e);
  }

  @Override
  public BiFunction<E, E, E> getArg2() {
    return (e0, e1) -> _f2.apply(e0, e1);
  }

  @Override
  public Function getArgN() {
    return (es) -> Iter.reduce(Iter.iter(es), _f2);
  }
}
