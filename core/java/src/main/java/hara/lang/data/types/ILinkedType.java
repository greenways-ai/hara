package hara.lang.data.types;

import hara.lang.protocol.*;

public interface ILinkedType<E>
    extends IColl<E>, IPushFirst<E>, IPopFirst, IPeekFirst<E>, ICons<E>, IConj<E>, ICount {

  @Override
  default ILinkedType<E> cons(E e) {
    return (ILinkedType<E>) pushFirst(e);
  }

  @Override
  default ILinkedType<E> conj(E e) {
    return (ILinkedType<E>) pushFirst(e);
  }

  @Override
  default String startString() {
    return "(";
  }

  @Override
  default String endString() {
    return ")";
  }
}
