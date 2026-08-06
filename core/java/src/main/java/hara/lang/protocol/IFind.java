package hara.lang.protocol;

public interface IFind<K, V> {
  V find(K key);

  default boolean has(K key) {
    return find(key) != null;
  }
}
