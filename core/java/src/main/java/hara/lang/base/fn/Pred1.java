package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.Function;
import java.util.function.Predicate;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Pred1<T1> extends ObjFn implements IFn<Boolean, T1, Object> {

  final Predicate<T1> _p1;

  public Pred1(IMetadata meta, Predicate<T1> p1) {
    super(meta);
    _p1 = p1;
  }

  public Pred1(Predicate<T1> p1) {
    this(null, p1);
  }

  @Override
  public Function<T1, Boolean> getArg1() {
    return (e) -> _p1.test(e);
  }
}
