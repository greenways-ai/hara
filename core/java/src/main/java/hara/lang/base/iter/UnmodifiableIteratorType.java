package hara.lang.base.iter;

import hara.lang.base.Ex;

@SuppressWarnings({"rawtypes", "unchecked"})
public interface UnmodifiableIteratorType<V> extends java.util.Iterator<V> {

  @Override
  default void remove() {
    throw new Ex.Unsupported();
  }
}
