package hara.lang.base.primitive;

public interface Max {

  public static double max(double x, double y) {
    return Math.max(x, y);
  }

  public static Object max(double x, long y) {
    if (Double.isNaN(x)) {
      return x;
    }
    if (x > y) {
      return x;
    } else {
      return y;
    }
  }

  public static Object max(double x, Object y) {
    if (Double.isNaN(x)) {
      return x;
    } else if (Num.isNaN(y)) {
      return y;
    }
    if (x > ((Number) y).doubleValue()) {
      return x;
    } else {
      return y;
    }
  }

  public static Object max(long x, double y) {
    if (Double.isNaN(y)) {
      return y;
    }
    if (x > y) {
      return x;
    } else {
      return y;
    }
  }

  public static long max(long x, long y) {
    if (x > y) {
      return x;
    } else {
      return y;
    }
  }

  public static Object max(long x, Object y) {
    if (Num.isNaN(y)) {
      return y;
    }
    if (Num.gt(x, y)) {
      return x;
    } else {
      return y;
    }
  }

  public static Object max(Object x, double y) {
    if (Num.isNaN(x)) {
      return x;
    } else if (Double.isNaN(y)) {
      return y;
    }
    if (((Number) x).doubleValue() > y) {
      return x;
    } else {
      return y;
    }
  }

  public static Object max(Object x, long y) {
    if (Num.isNaN(x)) {
      return x;
    }
    if (Num.gt(x, y)) {
      return x;
    } else {
      return y;
    }
  }

  public static Object max(Object x, Object y) {
    if (Num.isNaN(x)) {
      return x;
    } else if (Num.isNaN(y)) {
      return y;
    }
    if (Num.gt(x, y)) {
      return x;
    } else {
      return y;
    }
  }
}
