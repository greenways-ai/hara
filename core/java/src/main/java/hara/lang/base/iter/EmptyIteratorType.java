package hara.lang.base.iter;

import hara.lang.base.Ex;

@SuppressWarnings({"rawtypes", "unchecked"})
public interface EmptyIteratorType<V> extends UnmodifiableIteratorType<V> {

  @Override
  default boolean hasNext() {
    return false;
  }

  @Override
  default V next() {
    throw new Ex.NoSuchElement();
  }
}
