package hara.truffle;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;

/** Explicit numeric conversions used at the Hara language boundary. */
final class HaraNumericConversions {
  private HaraNumericConversions() {}

  @TruffleBoundary
  static long toLong(Object input) {
    Object value = input;
    if (value instanceof Byte
        || value instanceof Short
        || value instanceof Integer
        || value instanceof Long) {
      return ((Number) value).longValue();
    }
    if (value instanceof Double || value instanceof Float) {
      double floating = ((Number) value).doubleValue();
      if (Double.isFinite(floating)
          && floating >= Long.MIN_VALUE
          && floating < 0x1.0p63) {
        return (long) floating;
      }
    }
    throw cannotConvert("long", input);
  }

  @TruffleBoundary
  static double toDouble(Object input) {
    Object value = input;
    if (value instanceof Byte
        || value instanceof Short
        || value instanceof Integer
        || value instanceof Long
        || value instanceof Float
        || value instanceof Double) {
      return ((Number) value).doubleValue();
    }
    throw cannotConvert("double", input);
  }

  private static HaraException cannotConvert(String target, Object value) {
    return new HaraException(
        target + " expects a numeric value, got " + (value == null ? "nil" : value));
  }
}
