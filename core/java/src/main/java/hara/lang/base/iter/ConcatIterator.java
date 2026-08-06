package hara.lang.base.iter;

import hara.lang.base.Ex;
import hara.lang.base.Iter;

import java.util.ArrayDeque;
import java.util.Iterator;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class ConcatIterator<E> implements CloseableIterator<E> {

  private final Iterator<? extends Iterator<? extends E>> sources;
  private final ArrayDeque<Iterator<? extends E>> appended = new ArrayDeque<>();
  private Iterator<? extends E> current;
  private boolean closed;

  public ConcatIterator(java.util.Iterator<? extends java.util.Iterator<? extends E>> iterators) {
    this.sources = iterators;
  }

  @Override
  public boolean hasNext() {
    if (closed) {
      return false;
    }
    while (true) {
      if (current != null) {
        if (current.hasNext()) {
          return true;
        }
        // Release exhausted resource-backed children before acquiring the next
        // source. This keeps lazy concatenation from retaining completed sources.
        Iter.close(current);
        current = null;
      }
      if (sources.hasNext()) {
        current = sources.next();
      } else if (!appended.isEmpty()) {
        current = appended.removeFirst();
      } else {
        Iter.close(sources);
        return false;
      }
    }
  }

  @Override
  public E next() {
    if (!hasNext()) {
      throw new Ex.NoSuchElement();
    }
    return current.next();
  }

  public Iterator<E> concat(java.util.Iterator<? extends E> that) {
    if (closed) {
      throw new IllegalStateException("Iterator is closed");
    }
    appended.addLast(that);
    return this;
  }

  @Override
  public void close() {
    if (closed) {
      return;
    }
    closed = true;
    Iter.close(current);
    Iter.close(sources);
    for (Iterator<? extends E> iterator : appended) {
      Iter.close(iterator);
    }
    appended.clear();
    current = null;
  }
}
