package hara.lang.protocol;

public interface IRealize<V> {
  boolean isRealized();

  V realize();
}
