package hara.lang.protocol;

public interface IDerefTimeout<V> {
  V derefTimeout(long ms, V timeoutVal);
}
