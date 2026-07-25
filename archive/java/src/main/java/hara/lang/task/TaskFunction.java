package hara.lang.task;

@FunctionalInterface
public interface TaskFunction {
  Object apply(Object[] arguments) throws Exception;

  default int minimumArity() {
    return 4;
  }

  default boolean variadic() {
    return false;
  }
}
