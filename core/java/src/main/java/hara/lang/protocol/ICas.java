package hara.lang.protocol;

/** Atomically replaces an expected value. */
public interface ICas<V> {
  boolean cas(V oldValue, V newValue);
}
