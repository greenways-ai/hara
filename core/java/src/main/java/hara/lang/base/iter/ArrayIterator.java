package hara.lang.base.iter;

import java.util.Iterator;
import java.util.NoSuchElementException;

public class ArrayIterator<E> implements Iterator<E> {
  final E[] _array;
  int _i;
  int _end;

  public ArrayIterator(E[] array, int i, int end) {
    _i = i;
    _array = array;
    _end = end;
  }

  public ArrayIterator(E[] array, int i) {
    _i = i;
    _array = array;
    _end = array.length;
  }

  @Override
  public boolean hasNext() {
    return _array != null && _i < _end;
  }

  @Override
  public E next() {
    if (_array != null && _i < _end) return _array[_i++];
    throw new NoSuchElementException();
  }

  @Override
  public void remove() {
    throw new UnsupportedOperationException("remove() not supported");
  }
}
