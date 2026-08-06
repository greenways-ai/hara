package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.Function;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Fn1<R, T1> extends ObjFn implements IFn<R, T1, Object> {

  final Function<T1, R> _f1;

  public Fn1(Function<T1, R> f1) {
    this(null, f1);
  }

  public Fn1(IMetadata meta, Function<T1, R> f1) {
    super(meta);
    _f1 = f1;
  }

  @Override
  public Function<T1, R> getArg1() {
    return _f1;
  }
}
