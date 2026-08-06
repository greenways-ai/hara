package hara.lang.base.primitive;

public class Clock {

  private static final Clock _clock;

  static {
    _clock = new Clock();
  }

  public static final long currentTimeMicros() {
    return currentTimeNanos() / 1000;
  }

  public static final long currentTimeMillis() {
    return currentTimeNanos() / 1000000;
  }

  public static final long currentTimeNanos() {
    return _clock._tsys + (System.nanoTime() - _clock._toff);
  }

  private final long _toff;

  private final long _tsys;

  private Clock() {
    _tsys = System.currentTimeMillis() * 1000000;

    // typically 36 ns, between these two lines.
    _toff = System.nanoTime();
  }
}
