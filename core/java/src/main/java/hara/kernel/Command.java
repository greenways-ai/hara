package hara.kernel;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import java.util.List;
import java.util.function.BiFunction;

public interface Command {

  @Retention(RetentionPolicy.RUNTIME)
  @Target({ElementType.METHOD, ElementType.TYPE})
  public @interface Fn {
    String name();
  }

  @Retention(RetentionPolicy.RUNTIME)
  @Target(ElementType.METHOD)
  public @interface Sub {
    String name();
  }

  @FunctionalInterface
  public interface Type extends BiFunction<Foundation, List<Object>, Object> {}
}
