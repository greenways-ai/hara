package hara.lang.base.iter;

import hara.lang.base.Ex;

@SuppressWarnings({"rawtypes", "unchecked"})
public interface UnmodifiableListIteratorType<V> extends java.util.ListIterator<V> {

  @Override
  default void remove() {
    throw new Ex.Unsupported();
  }

  @Override
  default void set(V e) {
    throw new Ex.Unsupported();
  }

  @Override
  default void add(V e) {
    throw new Ex.Unsupported();
  }
}
