package hara.lang.protocol;

import java.util.Iterator;

/** Stateful traversal cursor. */
public interface IIterator<E> extends IIter<E>, Iterator<E> {
  @Override
  default Iterator<E> iter() {
    return this;
  }
}
