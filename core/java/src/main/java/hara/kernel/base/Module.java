package hara.kernel.base;

import java.lang.annotation.Documented;
import java.lang.annotation.Retention;
import java.lang.annotation.Target;

import static java.lang.annotation.ElementType.METHOD;
import static java.lang.annotation.ElementType.TYPE;
import static java.lang.annotation.RetentionPolicy.RUNTIME;

public interface Module {

  public enum ReduceType {
    SELF,
    INIT,
    ARRAY,
    COMPARE
  }

  public enum ReduceInit {
    NIL,
    ZERO,
    ONE,
    NEG_ONE,
    TRUE,
    FALSE,
    EMPTY_MAP,
    EMPTY_LIST,
    EMPTY_VECTOR,
    EMPTY_QUEUE,
  }

  @Retention(RUNTIME)
  @Target({METHOD})
  @Documented
  public @interface Var {
    boolean control() default false;

    boolean macro() default false;

    boolean dynamic() default false;
  }

  @Retention(RUNTIME)
  @Target({METHOD})
  @Documented
  public @interface Fn {
    String name() default "";

    String doc() default "";

    boolean protocol() default false;

    String method() default "";

    boolean fallback() default false;

    boolean option() default false;

    boolean helper() default false;

    boolean complete() default false;

    boolean vargs() default false;

    boolean rt() default false;

    boolean env() default false;
  }

  @Retention(RUNTIME)
  @Target({METHOD})
  @Documented
  @SuppressWarnings("rawtypes")
  public @interface RT {
    Class protocol() default Object.class;

    String method() default "";
  }

  @Retention(RUNTIME)
  @Target({TYPE})
  @Documented
  public @interface Ns {
    String name() default "";

    String tag() default "";
  }

  @Retention(RUNTIME)
  @Target({METHOD})
  @Documented
  public @interface Reduce {
    ReduceType type() default ReduceType.INIT;

    ReduceInit init() default ReduceInit.NIL;
  }
}
