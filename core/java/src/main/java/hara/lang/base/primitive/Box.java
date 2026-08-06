package hara.lang.base.primitive;

public interface Box {

  public static Object box(boolean x) {
    return x ? true : false;
  }

  public static Object box(Boolean x) {
    return x;
  }

  public static Number box(byte x) {
    return x;
  }

  public static Character box(char x) {
    return Character.valueOf(x);
  }

  public static Number box(double x) {
    return x;
  }

  public static Number box(float x) {
    return x;
  }

  public static Number box(int x) {
    return x;
  }

  public static Number box(long x) {
    return x;
  }

  public static Object box(Object x) {
    return x;
  }

  public static Number box(short x) {
    return x;
  }
}
