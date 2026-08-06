package hara.lang.base.fn;

import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import hara.lang.protocol.IMetadata;

import java.util.function.Consumer;
import java.util.function.Function;

@SuppressWarnings({"rawtypes", "unchecked"})
public class Consume1<T1> extends ObjFn implements IFn<Void, T1, Object> {

  final Consumer<T1> _c1;

  public Consume1(Consumer<T1> p1) {
    this(null, p1);
  }

  public Consume1(IMetadata meta, Consumer<T1> c1) {
    super(meta);
    _c1 = c1;
  }

  @Override
  public Function<T1, Void> getArg1() {
    return (e) -> {
      _c1.accept(e);
      return null;
    };
  }
}
