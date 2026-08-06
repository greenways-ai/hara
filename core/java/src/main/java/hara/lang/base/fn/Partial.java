package hara.lang.base.fn;

import hara.lang.base.Fn;
import hara.lang.base.Iter;
import hara.lang.data.List;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IOFn;

import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Supplier;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Partial<FN, ITR> implements IOFn {

  final List.Standard _args;
  final IFn _f;

  public Partial(FN f, ITR vars) {
    _f = Fn.toFn(f);
    _args = List.Standard.into(Iter.iter(vars));
  }

  @Override
  public Supplier getArg0() {
    return () -> _f.apply(_args);
  }

  @Override
  public Function getArg1() {
    return (x) -> _f.apply(_args.conj(x));
  }

  @Override
  public BiFunction getArg2() {
    return (x, y) -> _f.apply(_args.conj(x).conj(y));
  }

  @Override
  public Function getArgN() {
    return (args) -> _f.apply(Iter.concat(Iter.iter(_args), Iter.iter(args)));
  }
}
