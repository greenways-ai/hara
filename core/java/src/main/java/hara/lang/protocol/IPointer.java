package hara.lang.protocol;

public interface IPointer {
  Object ptrContext();

  Iterable<?> ptrKeys();

  Object ptrVal(Object key);
}
