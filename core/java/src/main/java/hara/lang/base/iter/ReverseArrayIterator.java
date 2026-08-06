package hara.lang.base.iter;

import java.util.Iterator;
import java.util.NoSuchElementException;

public class ReverseArrayIterator implements Iterator<Object> {
  final Object[] _array;
  int _i;

  public ReverseArrayIterator(Object array, int offset) {
    _array = (Object[]) array;
    _i = _array.length - 1 - offset;
  }

  @Override
  public boolean hasNext() {
    return _array != null && _i >= 0;
  }

  @Override
  public Object next() {
    if (_array != null && _i >= 0) return _array[_i--];
    throw new NoSuchElementException();
  }

  @Override
  public void remove() {
    throw new UnsupportedOperationException("remove() not supported");
  }
}
