package hara.lang.base.iter;

import java.util.Iterator;
import java.util.NoSuchElementException;

public class CharArrayIterator implements Iterator<Character> {
  final char[] _array;
  int _i;

  public CharArrayIterator(char[] array, int i) {
    _array = array;
    _i = i;
  }

  @Override
  public boolean hasNext() {
    return _array != null && _i < _array.length;
  }

  @Override
  public Character next() {
    if (_array != null && _i < _array.length) return _array[_i++];
    throw new NoSuchElementException();
  }

  @Override
  public void remove() {
    throw new UnsupportedOperationException("remove() not supported");
  }
}
