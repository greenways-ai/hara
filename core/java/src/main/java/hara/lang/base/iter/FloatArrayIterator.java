package hara.lang.base.iter;

import java.util.Iterator;
import java.util.NoSuchElementException;

public class FloatArrayIterator implements Iterator<Float> {
  final float[] _array;
  int _i;

  public FloatArrayIterator(float[] array, int i) {
    _array = array;
    _i = i;
  }

  @Override
  public boolean hasNext() {
    return _array != null && _i < _array.length;
  }

  @Override
  public Float next() {
    if (_array != null && _i < _array.length) return _array[_i++];
    throw new NoSuchElementException();
  }

  @Override
  public void remove() {
    throw new UnsupportedOperationException("remove() not supported");
  }
}
