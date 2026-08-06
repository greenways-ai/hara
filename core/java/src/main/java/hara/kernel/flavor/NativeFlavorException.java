package hara.kernel.flavor;

public final class NativeFlavorException extends RuntimeException {
  public enum Kind {
    DENIED,
    UNSUPPORTED,
    UNRESOLVED
  }

  private final Kind kind;

  public NativeFlavorException(Kind kind, String message) {
    super(message);
    this.kind = kind;
  }

  public NativeFlavorException(Kind kind, String message, Throwable cause) {
    super(message, cause);
    this.kind = kind;
  }

  public Kind kind() {
    return kind;
  }
}
