package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.Supplier;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Fn0<R> extends ObjFn implements IFn<R, Object, Object> {

  final Supplier<R> _f0;

  public Fn0(IMetadata meta, Supplier<R> f0) {
    super(meta);
    _f0 = f0;
  }

  public Fn0(Supplier<R> f0) {
    this(null, f0);
  }

  @Override
  public Supplier<R> getArg0() {
    return _f0;
  }
}
