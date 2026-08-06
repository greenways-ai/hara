package hara.lang.base.iter;

import java.util.Iterator;
import java.util.NoSuchElementException;

public class BooleanArrayIterator implements Iterator<Boolean> {
  final boolean[] _array;
  int _i;

  public BooleanArrayIterator(boolean[] array, int i) {
    _array = array;
    _i = i;
  }

  @Override
  public boolean hasNext() {
    return _array != null && _i < _array.length;
  }

  @Override
  public Boolean next() {
    if (_array != null && _i < _array.length) return Boolean.valueOf(_array[_i++]);
    throw new NoSuchElementException();
  }

  @Override
  public void remove() {
    throw new UnsupportedOperationException("remove() not supported");
  }
}
