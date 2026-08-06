package hara.lang.protocol;

public interface ICons<E> {
  ICons<E> cons(E e);
}
