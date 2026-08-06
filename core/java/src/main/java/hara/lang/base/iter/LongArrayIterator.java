package hara.lang.base.iter;

import java.util.Iterator;
import java.util.NoSuchElementException;

public class LongArrayIterator implements Iterator<Long> {
  final long[] _array;
  int _i;

  public LongArrayIterator(long[] array, int i) {
    _array = array;
    _i = i;
  }

  @Override
  public boolean hasNext() {
    return _array != null && _i < _array.length;
  }

  @Override
  public Long next() {
    if (_array != null && _i < _array.length) return Long.valueOf(_array[_i++]);
    throw new NoSuchElementException();
  }

  @Override
  public void remove() {
    throw new UnsupportedOperationException("remove() not supported");
  }
}
