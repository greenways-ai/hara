package hara.lang.base.iter;

import java.util.Iterator;

/** Iterator with explicit lifecycle semantics for lazy or resource-backed sources. */
public interface CloseableIterator<E> extends Iterator<E>, AutoCloseable {
  @Override
  void close();
}
