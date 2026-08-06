package hara.truffle;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/** Marks an explicitly exported static Java method in a lazy Hara library. */
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.METHOD)
public @interface HaraExport {
  enum Kind {
    FUNCTION,
    VALUE,
    MACRO
  }

  String name();

  String doc() default "";

  String[] arglists() default {};

  Kind kind() default Kind.FUNCTION;

  /** Also make an unqualified macro available as a core-style intrinsic. */
  boolean intrinsic() default false;
}
