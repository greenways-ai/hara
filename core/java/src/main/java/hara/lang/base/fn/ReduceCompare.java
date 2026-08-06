package hara.lang.base.fn;

import hara.lang.base.Fn;
import hara.lang.base.Iter;
import hara.lang.data.Tuple;
import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.Iterator;
import java.util.function.BiFunction;
import java.util.function.Function;

@SuppressWarnings({"rawtypes", "unchecked"})
public class ReduceCompare<E, FN> extends ObjFn implements IFn<Boolean, E, E> {
  final BiFunction<E, E, Boolean> _c2;
  final boolean _def;

  public ReduceCompare(boolean def, FN f) {
    this(null, def, f);
  }

  public ReduceCompare(IMetadata meta, boolean def, FN f) {
    super(meta);
    _def = def;
    _c2 = Fn.toFn(f).getArg2();
  }

  @Override
  public BiFunction<E, E, Boolean> getArg2() {
    return (e0, e1) -> _c2.apply(e0, e1);
  }

  @Override
  public Function getArgN() {
    return (es) -> {
      Iterator<E> it = Iter.iter(es);
      Tuple.Tup2.L<Boolean, E> init = new Tuple.Tup2.L(null, _def, it.next());
      return Iter.reduce(
              it,
              init,
              (p, e) -> new Tuple.Tup2.L(null, _c2.apply(p.B(), e), e),
              (p) -> p.A() != _def)
          .A();
    };
  }
}
