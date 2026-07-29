package hara.lang.protocol;

/** Reduces a value with or without an explicit initial accumulator. */
public interface IReduce {
  Object reduce(Object function);

  Object reduce(Object function, Object initial);
}
