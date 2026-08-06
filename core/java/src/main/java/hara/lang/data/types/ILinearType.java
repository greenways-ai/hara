package hara.lang.data.types;

import hara.lang.base.Ex;
import hara.lang.protocol.*;

public interface ILinearType<E>
    extends IColl<E>,
        IPushFirst<E>,
        IPushLast<E>,
        IPopFirst,
        IPopLast,
        IPeekFirst<E>,
        IPeekLast<E>,
        ICons<E>,
        IConj<E>,
        INth<E>,
        ICount {

  @Override
  default ILinearType<E> cons(E e) {
    return (ILinearType<E>) pushFirst(e);
  }

  @Override
  default ILinearType<E> conj(E e) {
    return (ILinearType<E>) pushLast(e);
  }

  @Override
  default IPushFirst<E> pushFirst(E e) {
    throw new Ex.Unsupported();
  }

  @Override
  default IPopFirst popFirst() {
    throw new Ex.Unsupported();
  }

  @Override
  default String startString() {
    return "[";
  }

  @Override
  default String endString() {
    return "]";
  }
}
