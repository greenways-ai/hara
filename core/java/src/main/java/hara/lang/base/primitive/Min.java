package hara.lang.base.primitive;

public interface Min {
  public static double min(double x, double y) {
    return Math.min(x, y);
  }

  public static Object min(double x, long y) {
    if (Double.isNaN(x)) {
      return x;
    }
    if (x < y) {
      return x;
    } else {
      return y;
    }
  }

  public static Object min(double x, Object y) {
    if (Double.isNaN(x)) {
      return x;
    } else if (Num.isNaN(y)) {
      return y;
    }
    if (x < ((Number) y).doubleValue()) {
      return x;
    } else {
      return y;
    }
  }

  public static Object min(long x, double y) {
    if (Double.isNaN(y)) {
      return y;
    }
    if (x < y) {
      return x;
    } else {
      return y;
    }
  }

  public static long min(long x, long y) {
    if (x < y) {
      return x;
    } else {
      return y;
    }
  }

  public static Object min(long x, Object y) {
    if (Num.isNaN(y)) {
      return y;
    }
    if (Num.lt(x, y)) {
      return x;
    } else {
      return y;
    }
  }

  public static Object min(Object x, double y) {
    if (Num.isNaN(x)) {
      return x;
    } else if (Double.isNaN(y)) {
      return y;
    }
    if (((Number) x).doubleValue() < y) {
      return x;
    } else {
      return y;
    }
  }

  public static Object min(Object x, long y) {
    if (Num.isNaN(x)) {
      return x;
    }
    if (Num.lt(x, y)) {
      return x;
    } else {
      return y;
    }
  }

  public static Object min(Object x, Object y) {
    if (Num.isNaN(x)) {
      return x;
    } else if (Num.isNaN(y)) {
      return y;
    }
    if (Num.lt(x, y)) {
      return x;
    } else {
      return y;
    }
  }
}
