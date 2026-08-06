package hara.lang.base.primitive;

import java.math.BigInteger;

public interface Cast {

  public static char charCast(Object x) {
    if (x instanceof Character) return ((Character) x).charValue();

    long n = ((Number) x).longValue();
    if (n < Character.MIN_VALUE || n > Character.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for char: " + x);

    return (char) n;
  }

  public static char charCast(byte x) {
    char i = (char) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for char: " + x);
    return i;
  }

  public static char charCast(short x) {
    char i = (char) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for char: " + x);
    return i;
  }

  public static char charCast(char x) {
    return x;
  }

  public static char charCast(int x) {
    char i = (char) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for char: " + x);
    return i;
  }

  public static char charCast(long x) {
    char i = (char) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for char: " + x);
    return i;
  }

  public static char charCast(float x) {
    if (x >= Character.MIN_VALUE && x <= Character.MAX_VALUE) return (char) x;
    throw new IllegalArgumentException("Value out of range for char: " + x);
  }

  public static char charCast(double x) {
    if (x >= Character.MIN_VALUE && x <= Character.MAX_VALUE) return (char) x;
    throw new IllegalArgumentException("Value out of range for char: " + x);
  }

  public static boolean booleanCast(Object x) {
    if (x instanceof Boolean) return ((Boolean) x).booleanValue();
    return x != null;
  }

  public static boolean booleanCast(boolean x) {
    return x;
  }

  public static byte byteCast(Object x) {
    if (x instanceof Byte) return ((Byte) x).byteValue();
    long n = longCast(x);
    if (n < Byte.MIN_VALUE || n > Byte.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for byte: " + x);

    return (byte) n;
  }

  public static byte byteCast(byte x) {
    return x;
  }

  public static byte byteCast(short x) {
    byte i = (byte) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for byte: " + x);
    return i;
  }

  public static byte byteCast(int x) {
    byte i = (byte) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for byte: " + x);
    return i;
  }

  public static byte byteCast(long x) {
    byte i = (byte) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for byte: " + x);
    return i;
  }

  public static byte byteCast(float x) {
    if (x >= Byte.MIN_VALUE && x <= Byte.MAX_VALUE) return (byte) x;
    throw new IllegalArgumentException("Value out of range for byte: " + x);
  }

  public static byte byteCast(double x) {
    if (x >= Byte.MIN_VALUE && x <= Byte.MAX_VALUE) return (byte) x;
    throw new IllegalArgumentException("Value out of range for byte: " + x);
  }

  public static short shortCast(Object x) {
    if (x instanceof Short) return ((Short) x).shortValue();
    long n = longCast(x);
    if (n < Short.MIN_VALUE || n > Short.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for short: " + x);

    return (short) n;
  }

  public static short shortCast(byte x) {
    return x;
  }

  public static short shortCast(short x) {
    return x;
  }

  public static short shortCast(int x) {
    short i = (short) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for short: " + x);
    return i;
  }

  public static short shortCast(long x) {
    short i = (short) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for short: " + x);
    return i;
  }

  public static short shortCast(float x) {
    if (x >= Short.MIN_VALUE && x <= Short.MAX_VALUE) return (short) x;
    throw new IllegalArgumentException("Value out of range for short: " + x);
  }

  public static short shortCast(double x) {
    if (x >= Short.MIN_VALUE && x <= Short.MAX_VALUE) return (short) x;
    throw new IllegalArgumentException("Value out of range for short: " + x);
  }

  public static int intCast(Object x) {
    if (x instanceof Integer) return ((Integer) x).intValue();
    if (x instanceof Number) {
      long n = longCast(x);
      return intCast(n);
    }
    return ((Character) x).charValue();
  }

  public static int intCast(char x) {
    return x;
  }

  public static int intCast(byte x) {
    return x;
  }

  public static int intCast(short x) {
    return x;
  }

  public static int intCast(int x) {
    return x;
  }

  public static int intCast(float x) {
    if (x < Integer.MIN_VALUE || x > Integer.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for int: " + x);
    return (int) x;
  }

  public static int intCast(long x) {
    int i = (int) x;
    if (i != x) throw new IllegalArgumentException("Value out of range for int: " + x);
    return i;
  }

  public static int intCast(double x) {
    if (x < Integer.MIN_VALUE || x > Integer.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for int: " + x);
    return (int) x;
  }

  public static long longCast(Object x) {
    if (x instanceof Integer || x instanceof Long) return ((Number) x).longValue();
    else if (x instanceof BigInteger) {
      BigInteger bi = (BigInteger) x;
      if (bi.bitLength() < 64) return bi.longValue();
      else throw new IllegalArgumentException("Value out of range for long: " + x);
    } else if (x instanceof Byte || x instanceof Short) return ((Number) x).longValue();
    else if (x instanceof Character) return longCast(((Character) x).charValue());
    else return longCast(((Number) x).doubleValue());
  }

  public static long longCast(byte x) {
    return x;
  }

  public static long longCast(short x) {
    return x;
  }

  public static long longCast(int x) {
    return x;
  }

  public static long longCast(float x) {
    if (x < Long.MIN_VALUE || x > Long.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for long: " + x);
    return (long) x;
  }

  public static long longCast(long x) {
    return x;
  }

  public static long longCast(double x) {
    if (x < Long.MIN_VALUE || x > Long.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for long: " + x);
    return (long) x;
  }

  public static float floatCast(Object x) {
    if (x instanceof Float) return ((Float) x).floatValue();

    double n = ((Number) x).doubleValue();
    if (n < -Float.MAX_VALUE || n > Float.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for float: " + x);

    return (float) n;
  }

  public static float floatCast(byte x) {
    return x;
  }

  public static float floatCast(short x) {
    return x;
  }

  public static float floatCast(int x) {
    return x;
  }

  public static float floatCast(float x) {
    return x;
  }

  public static float floatCast(long x) {
    return x;
  }

  public static float floatCast(double x) {
    if (x < -Float.MAX_VALUE || x > Float.MAX_VALUE)
      throw new IllegalArgumentException("Value out of range for float: " + x);

    return (float) x;
  }

  public static double doubleCast(Object x) {
    return ((Number) x).doubleValue();
  }

  public static double doubleCast(byte x) {
    return x;
  }

  public static double doubleCast(short x) {
    return x;
  }

  public static double doubleCast(int x) {
    return x;
  }

  public static double doubleCast(float x) {
    return x;
  }

  public static double doubleCast(long x) {
    return x;
  }

  public static double doubleCast(double x) {
    return x;
  }

  public static byte uncheckedByteCast(Object x) {
    return ((Number) x).byteValue();
  }

  public static byte uncheckedByteCast(byte x) {
    return x;
  }

  public static byte uncheckedByteCast(short x) {
    return (byte) x;
  }

  public static byte uncheckedByteCast(int x) {
    return (byte) x;
  }

  public static byte uncheckedByteCast(long x) {
    return (byte) x;
  }

  public static byte uncheckedByteCast(float x) {
    return (byte) x;
  }

  public static byte uncheckedByteCast(double x) {
    return (byte) x;
  }

  public static short uncheckedShortCast(Object x) {
    return ((Number) x).shortValue();
  }

  public static short uncheckedShortCast(byte x) {
    return x;
  }

  public static short uncheckedShortCast(short x) {
    return x;
  }

  public static short uncheckedShortCast(int x) {
    return (short) x;
  }

  public static short uncheckedShortCast(long x) {
    return (short) x;
  }

  public static short uncheckedShortCast(float x) {
    return (short) x;
  }

  public static short uncheckedShortCast(double x) {
    return (short) x;
  }

  public static char uncheckedCharCast(Object x) {
    if (x instanceof Character) return ((Character) x).charValue();
    return (char) ((Number) x).longValue();
  }

  public static char uncheckedCharCast(byte x) {
    return (char) x;
  }

  public static char uncheckedCharCast(short x) {
    return (char) x;
  }

  public static char uncheckedCharCast(char x) {
    return x;
  }

  public static char uncheckedCharCast(int x) {
    return (char) x;
  }

  public static char uncheckedCharCast(long x) {
    return (char) x;
  }

  public static char uncheckedCharCast(float x) {
    return (char) x;
  }

  public static char uncheckedCharCast(double x) {
    return (char) x;
  }

  public static int uncheckedIntCast(Object x) {
    if (x instanceof Number) return ((Number) x).intValue();
    return ((Character) x).charValue();
  }

  public static int uncheckedIntCast(byte x) {
    return x;
  }

  public static int uncheckedIntCast(short x) {
    return x;
  }

  public static int uncheckedIntCast(char x) {
    return x;
  }

  public static int uncheckedIntCast(int x) {
    return x;
  }

  public static int uncheckedIntCast(long x) {
    return (int) x;
  }

  public static int uncheckedIntCast(float x) {
    return (int) x;
  }

  public static int uncheckedIntCast(double x) {
    return (int) x;
  }

  public static long uncheckedLongCast(Object x) {
    return ((Number) x).longValue();
  }

  public static long uncheckedLongCast(byte x) {
    return x;
  }

  public static long uncheckedLongCast(short x) {
    return x;
  }

  public static long uncheckedLongCast(int x) {
    return x;
  }

  public static long uncheckedLongCast(long x) {
    return x;
  }

  public static long uncheckedLongCast(float x) {
    return (long) x;
  }

  public static long uncheckedLongCast(double x) {
    return (long) x;
  }

  public static float uncheckedFloatCast(Object x) {
    return ((Number) x).floatValue();
  }

  public static float uncheckedFloatCast(byte x) {
    return x;
  }

  public static float uncheckedFloatCast(short x) {
    return x;
  }

  public static float uncheckedFloatCast(int x) {
    return x;
  }

  public static float uncheckedFloatCast(long x) {
    return x;
  }

  public static float uncheckedFloatCast(float x) {
    return x;
  }

  public static float uncheckedFloatCast(double x) {
    return (float) x;
  }

  public static double uncheckedDoubleCast(Object x) {
    return ((Number) x).doubleValue();
  }

  public static double uncheckedDoubleCast(byte x) {
    return x;
  }

  public static double uncheckedDoubleCast(short x) {
    return x;
  }

  public static double uncheckedDoubleCast(int x) {
    return x;
  }

  public static double uncheckedDoubleCast(long x) {
    return x;
  }

  public static double uncheckedDoubleCast(float x) {
    return x;
  }

  public static double uncheckedDoubleCast(double x) {
    return x;
  }
}
