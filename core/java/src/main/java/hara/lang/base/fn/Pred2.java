package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.BiFunction;
import java.util.function.BiPredicate;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Pred2<T1, T2> extends ObjFn implements IFn<Boolean, T1, T2> {

  final BiPredicate<T1, T2> _p2;

  public Pred2(BiPredicate<T1, T2> p2) {
    this(null, p2);
  }

  public Pred2(IMetadata meta, BiPredicate<T1, T2> p2) {
    super(meta);
    _p2 = p2;
  }

  @Override
  public BiFunction<T1, T2, Boolean> getArg2() {
    return (o, e) -> _p2.test(o, e);
  }
}
