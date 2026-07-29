package hara.lang.protocol;

import java.util.Iterator;

/** Produces a fresh traversal cursor for a value. */
public interface IIter<E> {
  Iterator<E> iter();
}
