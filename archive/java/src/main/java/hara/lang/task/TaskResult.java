package hara.lang.task;

public final class TaskResult {
  public enum Status { RETURN, ERROR, CANCELLED, TIMEOUT }
  private final Status status;
  private final Object value;
  private final Throwable error;
  private final long elapsedMillis;

  private TaskResult(Status status, Object value, Throwable error, long elapsedMillis) {
    this.status = status; this.value = value; this.error = error; this.elapsedMillis = elapsedMillis;
  }
  public static TaskResult returned(Object value, long elapsed) { return new TaskResult(Status.RETURN, value, null, elapsed); }
  public static TaskResult failed(Throwable error, long elapsed) { return new TaskResult(Status.ERROR, null, error, elapsed); }
  public static TaskResult cancelled(long elapsed) { return new TaskResult(Status.CANCELLED, null, null, elapsed); }
  public static TaskResult timeout(long elapsed) { return new TaskResult(Status.TIMEOUT, null, null, elapsed); }
  public Status status() { return status; }
  public Object value() { return value; }
  public Throwable error() { return error; }
  public long elapsedMillis() { return elapsedMillis; }
}
