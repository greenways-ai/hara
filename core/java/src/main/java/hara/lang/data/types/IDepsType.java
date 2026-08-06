package hara.lang.data.types;

import hara.lang.protocol.IContext;

import java.util.Iterator;

public interface IDepsType<K, E> {
  E depGet(IContext ctx, K id);

  ISetType<E> depEntries(IContext ctx, K id);

  Iterator<K> depIds(IContext ctx);
}
