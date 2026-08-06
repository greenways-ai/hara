package hara.lang.data.types;

import hara.lang.base.Iter;
import hara.lang.protocol.*;

import java.util.Iterator;
import java.util.Map.Entry;

public interface ISequentialLookupType<E>
    extends ISequentialType<E>,
        Iterable<E>,
        ICount,
        INth<E>,
        ILookup<Long, E>,
        IPeekFirst<E>,
        IPeekLast<E> {

  @Override
  default Entry<Long, E> find(Long idx) {
    if (idx >= 0 && idx < count()) {
      E out = nth(idx);
      return new Entry<Long, E>() {
        @Override
        public Long getKey() {
          return idx;
        }

        @Override
        public E getValue() {
          return out;
        }

        @Override
        public E setValue(E value) {
          throw new UnsupportedOperationException("Not Supported");
        }
      };
    }
    throw new IndexOutOfBoundsException();
  }

  @Override
  default Iterator<Long> keys() {
    return Iter.range(0, count());
  }

  @Override
  default E lookup(Long idx) {
    return nth(idx);
  }

  @Override
  default E peekFirst() {
    return nth(0);
  }

  @Override
  default E peekLast() {
    return nth(count() - 1);
  }

  @Override
  default Iterator<E> vals() {
    return this.iterator();
  }
}
