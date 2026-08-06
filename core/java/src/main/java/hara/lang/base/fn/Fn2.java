package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.BiFunction;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Fn2<R, T1, T2> extends ObjFn implements IFn<R, T1, T2> {

  final BiFunction<T1, T2, R> _f2;

  public Fn2(BiFunction<T1, T2, R> f2) {
    this(null, f2);
  }

  public Fn2(IMetadata meta, BiFunction<T1, T2, R> f2) {
    super(meta);
    _f2 = f2;
  }

  @Override
  public BiFunction<T1, T2, R> getArg2() {
    return _f2;
  }
}
