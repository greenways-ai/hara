package hara.lang.test;

public final class HaraTestResult {
  public enum Status {
    PASS,
    FAIL
  }

  private final HaraTestCase test;
  private final Status status;
  private final Object value;
  private final Throwable error;
  private final long elapsedMillis;

  public HaraTestResult(
      HaraTestCase test, Status status, Object value, Throwable error, long elapsedMillis) {
    this.test = test;
    this.status = status;
    this.value = value;
    this.error = error;
    this.elapsedMillis = elapsedMillis;
  }

  public HaraTestCase test() {
    return test;
  }

  public Status status() {
    return status;
  }

  public Object value() {
    return value;
  }

  public Throwable error() {
    return error;
  }

  public long elapsedMillis() {
    return elapsedMillis;
  }
}
