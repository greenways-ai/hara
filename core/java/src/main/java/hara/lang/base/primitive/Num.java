package hara.lang.base.primitive;

import hara.lang.base.NumOps;
import hara.lang.base.NumUtils;

import java.math.BigDecimal;
import java.math.BigInteger;

public interface Num {

  static double asDouble(Object value) {
    if (value instanceof BigDecimal) {
      throw new IllegalArgumentException(
          "BigDecimal and floating-point values require an explicit conversion");
    }
    return ((Number) value).doubleValue();
  }

  public static double add(double x, double y) {
    return x + y;
  }

  public static double add(double x, long y) {
    return x + y;
  }

  public static double add(double x, Object y) {
    return add(x, asDouble(y));
  }

  public static double add(long x, double y) {
    return x + y;
  }

  public static long add(long x, long y) {
    return NumUtils.add(x, y);
  }

  public static Number add(long x, Object y) {
    return add((Object) x, y);
  }

  public static double add(Object x, double y) {
    return add(asDouble(x), y);
  }

  public static Number add(Object x, long y) {
    return add(x, (Object) y);
  }

  public static Number add(Object x, Object y) {
    return ops(x).combine(ops(y)).add((Number) x, (Number) y);
  }

  public static double addP(double x, double y) {
    return x + y;
  }

  public static double addP(double x, long y) {
    return x + y;
  }

  public static double addP(double x, Object y) {
    return addP(x, asDouble(y));
  }

  public static double addP(long x, double y) {
    return x + y;
  }

  public static Number addP(long x, long y) {
    long ret = x + y;
    if ((ret ^ x) < 0 && (ret ^ y) < 0) return addP((Number) x, (Number) y);
    return num(ret);
  }

  public static Number addP(long x, Object y) {
    return addP((Object) x, y);
  }

  public static double addP(Object x, double y) {
    return addP(asDouble(x), y);
  }

  public static Number addP(Object x, long y) {
    return addP(x, (Object) y);
  }

  public static Number addP(Object x, Object y) {
    return ops(x).combine(ops(y)).addP((Number) x, (Number) y);
  }

  public static long and(long x, long y) {
    return x & y;
  }

  public static BigInteger and(long x, Object y) {
    return and(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger and(Object x, long y) {
    return and(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger and(Object x, Object y) {
    return NumUtils.bitOpsCast(x).and(NumUtils.bitOpsCast(y));
  }

  public static long andNot(long x, long y) {
    return x & ~y;
  }

  public static BigInteger andNot(long x, Object y) {
    return andNot(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger andNot(Object x, long y) {
    return andNot(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger andNot(Object x, Object y) {
    return NumUtils.bitOpsCast(x).andNot(NumUtils.bitOpsCast(y));
  }

  // @WarnBoxedMath(false)
  public static boolean[] booleans(Object array) {
    return (boolean[]) array;
  }

  // @WarnBoxedMath(false)
  public static byte[] bytes(Object array) {
    return (byte[]) array;
  }

  static NumUtils.Category category(Object x) {
    Class<? extends Object> xc = x.getClass();

    if (xc == Integer.class) return NumUtils.Category.INTEGER;
    else if (xc == Double.class) return NumUtils.Category.FLOATING;
    else if (xc == Long.class) return NumUtils.Category.INTEGER;
    else if (xc == Float.class) return NumUtils.Category.FLOATING;
    else if (xc == BigDecimal.class) return NumUtils.Category.DECIMAL;
    else return NumUtils.Category.INTEGER;
  }

  // @WarnBoxedMath(false)
  public static char[] chars(Object array) {
    return (char[]) array;
  }

  public static long clearBit(long x, long n) {
    return x & ~(1L << n);
  }

  public static BigInteger clearBit(long x, Object y) {
    return clearBit(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger clearBit(Object x, long y) {
    return clearBit(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger clearBit(Object x, Object y) {
    return NumUtils.bitOpsCast(x).clearBit(NumUtils.bitOpsCast(y).intValue());
  }

  public static int compare(Number x, Number y) {
    NumOps ops = ops(x).combine(ops(y));
    if (ops.lt(x, y)) return -1;
    else if (ops.lt(y, x)) return 1;
    return 0;
  }

  public static double dec(double x) {
    return x - 1;
  }

  public static long dec(long x) {
    return NumUtils.dec(x);
  }

  public static Number dec(Object x) {
    return ops(x).dec((Number) x);
  }

  public static double decP(double x) {
    return x - 1;
  }

  public static Number decP(long x) {
    if (x == Long.MIN_VALUE) return NumUtils.BIGINT_OPS.dec(x);
    return num(x - 1);
  }

  public static Number decP(Object x) {
    return ops(x).decP((Number) x);
  }

  public static Number divide(BigInteger n, BigInteger d) {
    if (d.equals(BigInteger.ZERO)) throw new ArithmeticException("Divide by zero");
    return n.divide(d);
  }

  public static BigDecimal canonicalDecimal(BigDecimal value) {
    return NumUtils.normalizeDecimal(value);
  }

  public static BigInteger toBigInteger(Number value) {
    if (value instanceof BigDecimal) return ((BigDecimal) value).toBigInteger();
    if (value instanceof BigInteger) return (BigInteger) value;
    return BigInteger.valueOf(value.longValue());
  }

  public static BigDecimal toBigDecimal(Number value) {
    if (value instanceof BigDecimal) return canonicalDecimal((BigDecimal) value);
    if (value instanceof BigInteger) return canonicalDecimal(new BigDecimal((BigInteger) value));
    if (value instanceof Double || value instanceof Float)
      return canonicalDecimal(BigDecimal.valueOf(value.doubleValue()));
    return canonicalDecimal(BigDecimal.valueOf(value.longValue()));
  }

  public static double divide(double x, double y) {
    return x / y;
  }

  public static double divide(double x, long y) {
    return x / y;
  }

  public static double divide(double x, Object y) {
    return x / asDouble(y);
  };

  public static double divide(long x, double y) {
    return x / y;
  }

  public static Number divide(long x, long y) {
    return divide((Number) x, (Number) y);
  }

  public static Number divide(long x, Object y) {
    return divide((Object) x, y);
  }

  public static double divide(Object x, double y) {
    return asDouble(x) / y;
  }

  public static Number divide(Object x, long y) {
    return divide(x, (Object) y);
  }

  public static Number divide(Object x, Object y) {
    if (isNaN(x)) {
      return (Number) x;
    } else if (isNaN(y)) {
      return (Number) y;
    }
    NumOps yops = ops(y);
    if (yops.isZero((Number) y)) throw new ArithmeticException("Divide by zero");
    return ops(x).combine(yops).divide((Number) x, (Number) y);
  }

  // @WarnBoxedMath(false)
  public static double[] doubles(Object array) {
    return (double[]) array;
  }

  public static boolean eq(double x, double y) {
    return x == y;
  }

  public static boolean eq(double x, long y) {
    return x == y;
  }

  public static boolean eq(double x, Object y) {
    return x == asDouble(y);
  }

  public static boolean eq(long x, double y) {
    return x == y;
  }

  public static boolean eq(long x, long y) {
    return x == y;
  }

  public static boolean eq(long x, Object y) {
    return eq((Object) x, y);
  }

  public static boolean eq(Number x, Number y) {
    return ops(x).combine(ops(y)).eq(x, y);
  }

  public static boolean eq(Object x, double y) {
    return asDouble(x) == y;
  }

  public static boolean eq(Object x, long y) {
    return eq(x, (Object) y);
  }

  public static boolean eq(Object x, Object y) {
    return eq((Number) x, (Number) y);
  }

  public static boolean equal(Number x, Number y) {
    return category(x) == category(y) && ops(x).combine(ops(y)).eq(x, y);
  }

  public static long flipBit(long x, long n) {
    return x ^ (1L << n);
  }

  public static BigInteger flipBit(long x, Object y) {
    return flipBit(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger flipBit(Object x, long y) {
    return flipBit(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger flipBit(Object x, Object y) {
    return NumUtils.bitOpsCast(x).flipBit(NumUtils.bitOpsCast(y).intValue());
  }

  // @WarnBoxedMath(false)
  public static float[] floats(Object array) {
    return (float[]) array;
  }

  public static boolean gt(double x, double y) {
    return x > y;
  }

  public static boolean gt(double x, long y) {
    return x > y;
  }

  public static boolean gt(double x, Object y) {
    return x > asDouble(y);
  }

  public static boolean gt(long x, double y) {
    return x > y;
  }

  public static boolean gt(long x, long y) {
    return x > y;
  }

  public static boolean gt(long x, Object y) {
    return gt((Object) x, y);
  }

  public static boolean gt(Object x, double y) {
    return asDouble(x) > y;
  }

  public static boolean gt(Object x, long y) {
    return gt(x, (Object) y);
  }

  public static boolean gt(Object x, Object y) {
    return ops(x).combine(ops(y)).lt((Number) y, (Number) x);
  }

  public static boolean gte(double x, double y) {
    return x >= y;
  }

  public static boolean gte(double x, long y) {
    return x >= y;
  }

  public static boolean gte(double x, Object y) {
    return x >= asDouble(y);
  }

  public static boolean gte(long x, double y) {
    return x >= y;
  }

  public static boolean gte(long x, long y) {
    return x >= y;
  }

  public static boolean gte(long x, Object y) {
    return gte((Object) x, y);
  }

  public static boolean gte(Object x, double y) {
    return asDouble(x) >= y;
  }

  public static boolean gte(Object x, long y) {
    return gte(x, (Object) y);
  }

  public static boolean gte(Object x, Object y) {
    return ops(x).combine(ops(y)).gte((Number) x, (Number) y);
  }

  public static double inc(double x) {
    return x + 1;
  }

  public static long inc(long x) {
    return NumUtils.inc(x);
  }

  public static Number inc(Object x) {
    return ops(x).inc((Number) x);
  }

  public static double incP(double x) {
    return x + 1;
  }

  public static Number incP(long x) {
    if (x == Long.MAX_VALUE) return NumUtils.BIGINT_OPS.inc(x);
    return num(x + 1);
  }

  public static Number incP(Object x) {
    return ops(x).incP((Number) x);
  }

  // @WarnBoxedMath(false)
  public static int[] ints(Object array) {
    return (int[]) array;
  }

  static boolean isNaN(Object x) {
    return (x instanceof Double) && ((Double) x).isNaN()
        || (x instanceof Float) && ((Float) x).isNaN();
  }

  public static boolean isNeg(double x) {
    return x < 0;
  }

  public static boolean isNeg(long x) {
    return x < 0;
  }

  public static boolean isNeg(Object x) {
    return ops(x).isNeg((Number) x);
  }

  public static boolean isPos(double x) {
    return x > 0;
  }

  public static boolean isPos(long x) {
    return x > 0;
  }

  public static boolean isPos(Object x) {
    return ops(x).isPos((Number) x);
  }

  public static boolean isZero(double x) {
    return x == 0;
  }

  public static boolean isZero(long x) {
    return x == 0;
  }

  public static boolean isZero(Object x) {
    return ops(x).isZero((Number) x);
  }

  // @WarnBoxedMath(false)
  public static long[] longs(Object array) {
    return (long[]) array;
  }

  public static boolean lt(double x, double y) {
    return x < y;
  }

  public static boolean lt(double x, long y) {
    return x < y;
  }

  public static boolean lt(double x, Object y) {
    return x < asDouble(y);
  }

  public static boolean lt(long x, double y) {
    return x < y;
  }

  public static boolean lt(long x, long y) {
    return x < y;
  }

  public static boolean lt(long x, Object y) {
    return lt((Object) x, y);
  }

  public static boolean lt(Object x, double y) {
    return asDouble(x) < y;
  }

  public static boolean lt(Object x, long y) {
    return lt(x, (Object) y);
  }

  public static boolean lt(Object x, Object y) {
    return ops(x).combine(ops(y)).lt((Number) x, (Number) y);
  }

  public static boolean lte(double x, double y) {
    return x <= y;
  }

  public static boolean lte(double x, long y) {
    return x <= y;
  }

  public static boolean lte(double x, Object y) {
    return x <= asDouble(y);
  }

  public static boolean lte(long x, double y) {
    return x <= y;
  }

  public static boolean lte(long x, long y) {
    return x <= y;
  }

  public static boolean lte(long x, Object y) {
    return lte((Object) x, y);
  }

  public static boolean lte(Object x, double y) {
    return asDouble(x) <= y;
  }

  public static boolean lte(Object x, long y) {
    return lte(x, (Object) y);
  }

  public static boolean lte(Object x, Object y) {
    return ops(x).combine(ops(y)).lte((Number) x, (Number) y);
  }

  public static double minus(double x) {
    return -x;
  }

  public static double minus(double x, double y) {
    return x - y;
  }

  public static double minus(double x, long y) {
    return x - y;
  }

  public static double minus(double x, Object y) {
    return minus(x, asDouble(y));
  }

  public static long minus(long x) {
    return NumUtils.minus(x);
  }

  public static double minus(long x, double y) {
    return x - y;
  }

  public static long minus(long x, long y) {
    return NumUtils.minus(x, y);
  }

  public static Number minus(long x, Object y) {
    return minus((Object) x, y);
  }

  public static Number minus(Object x) {
    return ops(x).negate((Number) x);
  }

  public static double minus(Object x, double y) {
    return minus(asDouble(x), y);
  }

  public static Number minus(Object x, long y) {
    return minus(x, (Object) y);
  }

  public static Number minus(Object x, Object y) {
    NumOps yops = ops(y);
    return ops(x).combine(yops).add((Number) x, yops.negate((Number) y));
  }

  public static double minusP(double x) {
    return -x;
  }

  public static double minusP(double x, double y) {
    return x - y;
  }

  public static double minusP(double x, long y) {
    return x - y;
  }

  public static double minusP(double x, Object y) {
    return minus(x, asDouble(y));
  }

  public static Number minusP(long x) {
    if (x == Long.MIN_VALUE) return BigInteger.valueOf(x).negate();
    return num(-x);
  }

  public static double minusP(long x, double y) {
    return x - y;
  }

  public static Number minusP(long x, long y) {
    long ret = x - y;
    if (((ret ^ x) < 0 && (ret ^ ~y) < 0)) return minusP((Number) x, (Number) y);
    return num(ret);
  }

  public static Number minusP(long x, Object y) {
    return minusP((Object) x, y);
  }

  public static Number minusP(Object x) {
    return ops(x).negateP((Number) x);
  }

  public static double minusP(Object x, double y) {
    return minus(asDouble(x), y);
  }

  public static Number minusP(Object x, long y) {
    return minusP(x, (Object) y);
  }

  public static Number minusP(Object x, Object y) {
    NumOps yops = ops(y);
    Number negativeY = yops.negateP((Number) y);
    NumOps negativeYOps = ops(negativeY);
    return ops(x).combine(negativeYOps).addP((Number) x, negativeY);
  }

  public static double multiply(double x, double y) {
    return x * y;
  }

  public static double multiply(double x, long y) {
    return x * y;
  }

  public static double multiply(double x, Object y) {
    return multiply(x, asDouble(y));
  }

  public static double multiply(long x, double y) {
    return x * y;
  }

  public static long multiply(long x, long y) {
    return NumUtils.multiply(x, y);
  }

  public static Number multiply(long x, Object y) {
    return multiply((Object) x, y);
  }

  public static double multiply(Object x, double y) {
    return multiply(asDouble(x), y);
  }

  public static Number multiply(Object x, long y) {
    return multiply(x, (Object) y);
  }

  public static Number multiply(Object x, Object y) {
    return ops(x).combine(ops(y)).multiply((Number) x, (Number) y);
  }

  public static double multiplyP(double x, double y) {
    return x * y;
  }

  public static double multiplyP(double x, long y) {
    return x * y;
  }

  public static double multiplyP(double x, Object y) {
    return multiplyP(x, asDouble(y));
  }

  public static double multiplyP(long x, double y) {
    return x * y;
  }

  public static Number multiplyP(long x, long y) {
    if (x == Long.MIN_VALUE && y < 0) return multiplyP((Number) x, (Number) y);
    long ret = x * y;
    if (y != 0 && ret / y != x) return multiplyP((Number) x, (Number) y);
    return num(ret);
  }

  public static Number multiplyP(long x, Object y) {
    return multiplyP((Object) x, y);
  }

  public static double multiplyP(Object x, double y) {
    return multiplyP(asDouble(x), y);
  }

  public static Number multiplyP(Object x, long y) {
    return multiplyP(x, (Object) y);
  }

  public static Number multiplyP(Object x, Object y) {
    return ops(x).combine(ops(y)).multiplyP((Number) x, (Number) y);
  }

  public static long not(long x) {
    return ~x;
  }

  public static BigInteger not(Object x) {
    return NumUtils.bitOpsCast(x).not();
  }

  public static Number num(double x) {
    return Double.valueOf(x);
  }

  public static Number num(float x) {
    return Float.valueOf(x);
  }

  public static Number num(long x) {
    return Long.valueOf(x);
  }

  public static Number num(Object x) {
    return (Number) x;
  }

  static NumOps ops(Object x) {
    Class<? extends Object> xc = x.getClass();

    if (xc == Long.class || xc == Integer.class || xc == Short.class || xc == Byte.class)
      return NumUtils.LONG_OPS;
    else if (xc == Double.class || xc == Float.class) return NumUtils.DOUBLE_OPS;
    else if (xc == BigInteger.class) return NumUtils.BIGINT_OPS;
    else if (xc == BigDecimal.class) return NumUtils.BIGDECIMAL_OPS;
    else return NumUtils.BIGINT_OPS;
  }

  public static long or(long x, long y) {
    return x | y;
  }

  public static BigInteger or(long x, Object y) {
    return or(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger or(Object x, long y) {
    return or(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger or(Object x, Object y) {
    return NumUtils.bitOpsCast(x).or(NumUtils.bitOpsCast(y));
  }

  public static double quotient(double n, double d) {
    if (d == 0) throw new ArithmeticException("Divide by zero");

    double q = n / d;
    if (q <= Long.MAX_VALUE && q >= Long.MIN_VALUE) {
      return (long) q;
    } else { // bigint quotient
      return new BigDecimal(q).toBigInteger().doubleValue();
    }
  }

  public static double quotient(double x, long y) {
    return quotient(x, (double) y);
  }

  public static Number quotient(double x, Object y) {
    return quotient((Object) x, y);
  }

  public static double quotient(long x, double y) {
    return quotient((double) x, y);
  }

  public static long quotient(long x, long y) {
    return x / y;
  }

  public static Number quotient(long x, Object y) {
    return quotient((Object) x, y);
  }

  public static Number quotient(Object x, double y) {
    return quotient(x, (Object) y);
  }

  public static Number quotient(Object x, long y) {
    return quotient(x, (Object) y);
  }

  public static Number quotient(Object x, Object y) {
    NumOps yops = ops(y);
    if (yops.isZero((Number) y)) throw new ArithmeticException("Divide by zero");
    return ops(x).combine(yops).quotient((Number) x, (Number) y);
  }

  // @WarnBoxedMath(false)
  public static Number rationalize(Number x) {
    if (x instanceof Float || x instanceof Double)
      return canonicalDecimal(BigDecimal.valueOf(x.doubleValue()));
    else if (x instanceof BigDecimal) return canonicalDecimal((BigDecimal) x);
    return x;
  }

  public static double remainder(double n, double d) {
    if (d == 0) throw new ArithmeticException("Divide by zero");

    double q = n / d;
    if (q <= Long.MAX_VALUE && q >= Long.MIN_VALUE) {
      return (n - ((long) q) * d);
    } else { // bigint quotient
      Number bq = new BigDecimal(q).toBigInteger();
      return (n - bq.doubleValue() * d);
    }
  }

  public static double remainder(double x, long y) {
    return remainder(x, (double) y);
  }

  public static Number remainder(double x, Object y) {
    return remainder((Object) x, y);
  }

  public static double remainder(long x, double y) {
    return remainder((double) x, y);
  }

  public static long remainder(long x, long y) {
    if (y == 0) throw new ArithmeticException("Divide by zero");
    return x % y;
  }

  public static Number remainder(long x, Object y) {
    return remainder((Object) x, y);
  }

  public static Number remainder(Object x, double y) {
    return remainder(x, (Object) y);
  }

  public static Number remainder(Object x, long y) {
    return remainder(x, (Object) y);
  }

  public static Number remainder(Object x, Object y) {
    NumOps yops = ops(y);
    if (yops.isZero((Number) y)) throw new ArithmeticException("Divide by zero");
    return ops(x).combine(yops).remainder((Number) x, (Number) y);
  }

  public static long setBit(long x, long n) {
    return x | (1L << n);
  }

  public static BigInteger setBit(long x, Object y) {
    return setBit(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger setBit(Object x, long y) {
    return setBit(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger setBit(Object x, Object y) {
    return NumUtils.bitOpsCast(x).setBit(NumUtils.bitOpsCast(y).intValue());
  }

  public static long shiftLeft(long x, long n) {
    return x << n;
  }

  public static BigInteger shiftLeft(long x, Object y) {
    return shiftLeft(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger shiftLeft(Object x, long y) {
    return shiftLeft(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger shiftLeft(Object x, Object y) {
    return NumUtils.bitOpsCast(x).shiftLeft(NumUtils.bitOpsCast(y).intValue());
  }

  public static int shiftLeftInt(int x, int n) {
    return x << n;
  }

  public static long shiftRight(long x, long n) {
    return x >> n;
  }

  public static BigInteger shiftRight(long x, Object y) {
    return shiftRight(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger shiftRight(Object x, long y) {
    return shiftRight(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger shiftRight(Object x, Object y) {
    return NumUtils.bitOpsCast(x).shiftRight(NumUtils.bitOpsCast(y).intValue());
  }

  public static int shiftRightInt(int x, int n) {
    return x >> n;
  }

  // @WarnBoxedMath(false)
  public static short[] shorts(Object array) {
    return (short[]) array;
  }

  public static boolean testBit(long x, long n) {
    return (x & (1L << n)) != 0;
  }

  public static boolean testBit(long x, Object y) {
    return testBit(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static boolean testBit(Object x, long y) {
    return testBit(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static boolean testBit(Object x, Object y) {
    return NumUtils.bitOpsCast(x).testBit(NumUtils.bitOpsCast(y).intValue());
  }

  public static int throwIntOverflow() {
    throw new ArithmeticException("integer overflow");
  }

  public static double unchecked_add(double x, double y) {
    return add(x, y);
  }

  public static double unchecked_add(double x, long y) {
    return add(x, y);
  }

  public static double unchecked_add(double x, Object y) {
    return add(x, y);
  }

  public static double unchecked_add(long x, double y) {
    return add(x, y);
  }

  public static long unchecked_add(long x, long y) {
    return x + y;
  }

  public static Number unchecked_add(long x, Object y) {
    return unchecked_add((Object) x, y);
  }

  public static double unchecked_add(Object x, double y) {
    return add(x, y);
  }

  public static Number unchecked_add(Object x, long y) {
    return unchecked_add(x, (Object) y);
  }

  public static Number unchecked_add(Object x, Object y) {
    return ops(x).combine(ops(y)).unchecked_add((Number) x, (Number) y);
  }

  public static double unchecked_dec(double x) {
    return dec(x);
  }

  public static long unchecked_dec(long x) {
    return x - 1;
  }

  public static Number unchecked_dec(Object x) {
    return ops(x).unchecked_dec((Number) x);
  }

  public static double unchecked_inc(double x) {
    return inc(x);
  }

  public static long unchecked_inc(long x) {
    return x + 1;
  }

  public static Number unchecked_inc(Object x) {
    return ops(x).unchecked_inc((Number) x);
  }

  public static int unchecked_int_add(int x, int y) {
    return x + y;
  }

  public static int unchecked_int_dec(int x) {
    return x - 1;
  }

  public static int unchecked_int_divide(int x, int y) {
    return x / y;
  }

  public static int unchecked_int_inc(int x) {
    return x + 1;
  }

  public static int unchecked_int_multiply(int x, int y) {
    return x * y;
  }

  public static int unchecked_int_negate(int x) {
    return -x;
  }

  public static int unchecked_int_remainder(int x, int y) {
    return x % y;
  }

  public static int unchecked_int_subtract(int x, int y) {
    return x - y;
  }

  public static double unchecked_minus(double x) {
    return minus(x);
  }

  public static double unchecked_minus(double x, double y) {
    return minus(x, y);
  }

  public static double unchecked_minus(double x, long y) {
    return minus(x, y);
  }

  public static double unchecked_minus(double x, Object y) {
    return minus(x, y);
  }

  public static long unchecked_minus(long x) {
    return -x;
  }

  public static double unchecked_minus(long x, double y) {
    return minus(x, y);
  }

  public static long unchecked_minus(long x, long y) {
    return x - y;
  }

  public static Number unchecked_minus(long x, Object y) {
    return unchecked_minus((Object) x, y);
  }

  public static Number unchecked_minus(Object x) {
    return ops(x).unchecked_negate((Number) x);
  }

  public static double unchecked_minus(Object x, double y) {
    return minus(x, y);
  }

  public static Number unchecked_minus(Object x, long y) {
    return unchecked_minus(x, (Object) y);
  }

  public static Number unchecked_minus(Object x, Object y) {
    NumOps yops = ops(y);
    return ops(x).combine(yops).unchecked_add((Number) x, yops.unchecked_negate((Number) y));
  }

  public static double unchecked_multiply(double x, double y) {
    return multiply(x, y);
  }

  public static double unchecked_multiply(double x, long y) {
    return multiply(x, y);
  }

  public static double unchecked_multiply(double x, Object y) {
    return multiply(x, y);
  }

  public static double unchecked_multiply(long x, double y) {
    return multiply(x, y);
  }

  public static long unchecked_multiply(long x, long y) {
    return x * y;
  }

  public static Number unchecked_multiply(long x, Object y) {
    return unchecked_multiply((Object) x, y);
  }

  public static double unchecked_multiply(Object x, double y) {
    return multiply(x, y);
  }

  public static Number unchecked_multiply(Object x, long y) {
    return unchecked_multiply(x, (Object) y);
  }

  public static Number unchecked_multiply(Object x, Object y) {
    return ops(x).combine(ops(y)).unchecked_multiply((Number) x, (Number) y);
  }

  public static long unsignedShiftRight(long x, long n) {
    return x >>> n;
  }

  public static BigInteger unsignedShiftRight(long x, Object y) {
    return unsignedShiftRight(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger unsignedShiftRight(Object x, long y) {
    return unsignedShiftRight(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger unsignedShiftRight(Object x, Object y) {
    // BigInteger doesn't have unsigned shift because it has no sign bit in the standard sense (it's
    // arbitrary precision).
    // Standard shiftRight is effectively arithmetic shift.
    // For positive numbers, they are the same.
    // For negative numbers, BigInteger semantics differ from Java's >>>.
    // Given we are upscaling to BigInt, we likely want standard shiftRight behavior for "infinite"
    // bits,
    // or we need to simulate >>> by manipulating the bit count if we assume a fixed width (which we
    // don't here).
    // I will map it to shiftRight for now as it's the closest analogue in infinite precision.
    return NumUtils.bitOpsCast(x).shiftRight(NumUtils.bitOpsCast(y).intValue());
  }

  public static int unsignedShiftRightInt(int x, int n) {
    return x >>> n;
  }

  public static long xor(long x, long y) {
    return x ^ y;
  }

  public static BigInteger xor(long x, Object y) {
    return xor(BigInteger.valueOf(x), NumUtils.bitOpsCast(y));
  }

  public static BigInteger xor(Object x, long y) {
    return xor(NumUtils.bitOpsCast(x), BigInteger.valueOf(y));
  }

  public static BigInteger xor(Object x, Object y) {
    return NumUtils.bitOpsCast(x).xor(NumUtils.bitOpsCast(y));
  }
}
