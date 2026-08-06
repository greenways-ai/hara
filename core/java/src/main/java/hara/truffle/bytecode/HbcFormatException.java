package hara.truffle.bytecode;

/** Deterministic rejection raised before malformed HBC3 reaches Truffle execution. */
public final class HbcFormatException extends RuntimeException {
  public HbcFormatException(String message) {
    super(message);
  }
}
