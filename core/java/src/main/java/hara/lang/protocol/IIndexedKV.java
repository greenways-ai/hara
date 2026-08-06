package hara.lang.protocol;

public interface IIndexedKV<K, V> {
  long indexOfKey(K key);

  long indexOfVal(V val);
}
