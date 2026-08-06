package hara.lang.base.iter;

import hara.lang.base.Ex;

@SuppressWarnings({"rawtypes", "unchecked"})
public interface EmptyListIteratorType<V> extends UnmodifiableListIteratorType<V> {

  @Override
  default boolean hasPrevious() {
    return false;
  }

  @Override
  default V previous() {
    return null;
  }

  @Override
  default int nextIndex() {
    return -1;
  }

  @Override
  default int previousIndex() {
    return -1;
  }

  @Override
  default boolean hasNext() {
    return false;
  }

  @Override
  default V next() {
    throw new Ex.NoSuchElement();
  }
}
