package hara.lang.protocol;

import java.util.Iterator;
import java.util.Map;

public interface ILookup<K, V> extends IFind<K, Map.Entry<K, V>> {

  Iterator<K> keys();

  default V lookup(K key) {
    return lookup(key, null);
  }

  default V lookup(K key, V notFound) {
    Map.Entry<K, V> ret = find(key);
    return (ret == null) ? notFound : ret.getValue();
  }

  Iterator<V> vals();
}
