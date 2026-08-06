package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

@SuppressWarnings({"rawtypes", "unchecked"})
public class FnVargs<R, ITR> extends ObjFn implements IFn<R, Object, Object> {

  final Function<ITR, R> _f;

  public FnVargs(Function<ITR, R> f) {
    this(null, f);
  }

  public FnVargs(IMetadata meta, Function<ITR, R> f) {
    super(meta);
    _f = f;
  }

  @Override
  public Supplier<R> getArg0() {
    return () -> _f.apply((ITR) new Object[] {});
  }

  @Override
  public Function<Object, R> getArg1() {
    return (arg) -> _f.apply((ITR) new Object[] {arg});
  }

  @Override
  public BiFunction<Object, Object, R> getArg2() {
    return (a0, a1) -> _f.apply((ITR) new Object[] {a0, a1});
  }

  @Override
  public Function<Object, R> getArgN() {
    return (Function<Object, R>) _f;
  }
}
