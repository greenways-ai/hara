package hara.lang.base;

import java.math.BigDecimal;
import java.math.BigInteger;

public class NumUtils {

  public static final NumOps.BigDecimalOps BIGDECIMAL_OPS = new NumOps.BigDecimalOps();
  public static final NumOps.BigIntegerOps BIGINT_OPS = new NumOps.BigIntegerOps();
  public static final NumOps.DoubleOps DOUBLE_OPS = new NumOps.DoubleOps();
  public static final NumOps.LongOps LONG_OPS = new NumOps.LongOps();

  public static enum Category {
    DECIMAL,
    FLOATING,
    INTEGER
  }

  public static BigDecimal normalizeDecimal(BigDecimal value) {
    return value.signum() == 0 ? BigDecimal.ZERO : value.stripTrailingZeros();
  }

  public static BigDecimal toBigDecimal(Object x) {
    if (x instanceof BigDecimal) return normalizeDecimal((BigDecimal) x);
    else if (x instanceof BigInteger) return new BigDecimal((BigInteger) x);
    else if (x instanceof Double || x instanceof Float)
      throw new IllegalArgumentException(
          "BigDecimal and floating-point values require an explicit conversion");
    else return BigDecimal.valueOf(((Number) x).longValue());
  }

  public static BigInteger toBigInteger(Object x) {
    if (x instanceof BigInteger) return (BigInteger) x;
    else return BigInteger.valueOf(((Number) x).longValue());
  }

  public static int throwIntOverflow() {
    throw new ArithmeticException("integer overflow");
  }

  public static long gcd(long u, long v) {
    while (v != 0) {
      long r = u % v;
      u = v;
      v = r;
    }
    return u;
  }

  public static long add(long x, long y) {
    long ret = x + y;
    if ((ret ^ x) < 0 && (ret ^ y) < 0) return throwIntOverflow();
    return ret;
  }

  public static long dec(long x) {
    if (x == Long.MIN_VALUE) return throwIntOverflow();
    return x - 1;
  }

  public static long inc(long x) {
    if (x == Long.MAX_VALUE) return throwIntOverflow();
    return x + 1;
  }

  public static long multiply(long x, long y) {
    if (x == Long.MIN_VALUE && y < 0) return throwIntOverflow();
    long ret = x * y;
    if (y != 0 && ret / y != x) return throwIntOverflow();
    return ret;
  }

  public static long minus(long x) {
    if (x == Long.MIN_VALUE) return throwIntOverflow();
    return -x;
  }

  public static long minus(long x, long y) {
    long ret = x - y;
    if (((ret ^ x) < 0 && (ret ^ ~y) < 0)) return throwIntOverflow();
    return ret;
  }

  public static long unchecked_add(long x, long y) {
    return x + y;
  }

  public static long unchecked_dec(long x) {
    return x - 1;
  }

  public static long unchecked_inc(long x) {
    return x + 1;
  }

  public static long unchecked_multiply(long x, long y) {
    return x * y;
  }

  public static long unchecked_minus(long x) {
    return -x;
  }

  public static BigInteger bitOpsCast(Object x) {
    Class<? extends Object> xc = x.getClass();

    if (xc == Long.class || xc == Integer.class || xc == Short.class || xc == Byte.class)
      return BigInteger.valueOf(((Number) x).longValue());
    else if (x instanceof BigInteger) return (BigInteger) x;
    // no bignums, no decimals
    throw new IllegalArgumentException("bit operation not supported for: " + xc);
  }
}
