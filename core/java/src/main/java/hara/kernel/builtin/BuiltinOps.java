package hara.kernel.builtin;

import hara.kernel.base.Module;
import hara.lang.base.Eq;
import hara.lang.base.primitive.Num;

import static hara.kernel.base.Module.ReduceInit.*;
import static hara.kernel.base.Module.ReduceType.*;

@SuppressWarnings({"unchecked", "rawtypes"})
@Module.Ns(name = "global", tag = "ops")
public interface BuiltinOps {
  @Module.Fn(name = "+")
  @Module.Reduce(type = INIT, init = ZERO)
  public static Number add(Number x, Number y) {
    return Num.add(x, y);
  }

  @Module.Fn(name = "b&")
  @Module.Reduce(type = INIT, init = NEG_ONE)
  public static Number bitAnd(Number x, Number y) {
    return Num.and(x, y);
  }

  @Module.Fn(name = "b|")
  @Module.Reduce(type = INIT, init = ZERO)
  public static Number bitOr(Number x, Number y) {
    return Num.or(x, y);
  }

  @Module.Fn(name = "dec")
  public static Number dec(Number x) {
    return Num.minus(x, 1);
  }

  @Module.Fn(name = "ceil")
  public static Number ceil(Number x) {
    return Math.ceil(x.doubleValue());
  }

  @Module.Fn(name = "floor")
  public static Number floor(Number x) {
    return Math.floor(x.doubleValue());
  }

  @Module.Fn(name = "round")
  public static Number round(Number x) {
    return Math.round(x.doubleValue());
  }

  @Module.Fn(name = "abs")
  public static Number abs(Number x) {
    return Num.isNeg(x) ? Num.minus(x) : x;
  }

  @Module.Fn(name = "/")
  @Module.Reduce(type = ARRAY, init = ONE)
  public static Number divide(Number x, Number y) {
    return Num.divide(x, y);
  }

  @Module.Fn(name = "equals")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static boolean equals(Object k1, Object k2) {
    return (k1 == k2) ? true : (k1 != null && k1.equals(k2));
  }

  @Module.Fn(name = "=")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static Boolean equivalent(Object k1, Object k2) {
    return hara.lang.base.Eq.eq(k1, k2);
  }

  @Module.Fn(name = ">")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static Boolean gt(Number x, Number y) {
    return Num.gt(x, y);
  }

  @Module.Fn(name = ">=")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static Boolean gte(Number x, Number y) {
    return Num.gte(x, y);
  }

  @Module.Fn(name = "identical")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static boolean identical(Object k1, Object k2) {
    return k1 == k2;
  }

  @Module.Fn(name = "inc")
  public static Number inc(Number x) {
    return Num.add(x, 1);
  }

  @Module.Fn(name = "neg?")
  public static Boolean isNeg(Number x) {
    return Num.lt(x, 0);
  }

  @Module.Fn(name = "pos?")
  public static Boolean isPos(Number x) {
    return Num.gt(x, 0);
  }

  @Module.Fn(name = "zero?")
  public static Boolean isZero(Number x) {
    return Num.eq(x, 0);
  }

  @Module.Fn(name = "<")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static Boolean lt(Number x, Number y) {
    return Num.lt(x, y);
  }

  @Module.Fn(name = "<=")
  @Module.Reduce(type = COMPARE, init = TRUE)
  public static Boolean lte(Number x, Number y) {
    return Num.lte(x, y);
  }

  @Module.Fn(name = "-")
  @Module.Reduce(type = ARRAY, init = ZERO)
  public static Number minus(Number x, Number y) {
    return Num.minus(x, y);
  }

  @Module.Fn(name = "*")
  @Module.Reduce(type = INIT, init = ONE)
  public static Number multiply(Number x, Number y) {
    return Num.multiply(x, y);
  }

  @Module.Fn(name = "quot")
  public static Number quot(Number x, Number y) {
    return Num.quotient(x, y);
  }

  @Module.Fn(name = "rem")
  public static Number rem(Number x, Number y) {
    return Num.remainder(x, y);
  }

  @Module.Fn(name = "mod")
  public static Number mod(Number x, Number y) {
    return Num.remainder(x, y);
  }

  @Module.Fn(name = "min")
  @Module.Reduce(type = ARRAY)
  public static Object min(Object x, Object y) {
    return hara.lang.base.primitive.Min.min(x, y);
  }

  @Module.Fn(name = "max")
  @Module.Reduce(type = ARRAY)
  public static Object max(Object x, Object y) {
    return hara.lang.base.primitive.Max.max(x, y);
  }

  @Module.Fn(name = "not=")
  @Module.Reduce(type = COMPARE, init = FALSE)
  public static Boolean notEquivalent(Object k1, Object k2) {
    return !hara.lang.base.Eq.eq(k1, k2);
  }
}
