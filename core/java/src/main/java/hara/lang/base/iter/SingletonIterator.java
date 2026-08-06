package hara.lang.base.iter;

import hara.lang.base.Ex;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class SingletonIterator<E> implements java.util.Iterator<E> {

  private final E _elem;
  private boolean _next = true;

  public SingletonIterator(E element) {
    this._elem = element;
  }

  @Override
  public boolean hasNext() {
    return _next;
  }

  @Override
  public E next() {
    if (!hasNext()) {
      throw new Ex.NoSuchElement();
    }
    _next = false;
    return _elem;
  }
}
