package hara.truffle;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.TruffleObject;
import com.oracle.truffle.api.interop.UnknownIdentifierException;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import hara.lang.base.primitive.Num;
import java.math.BigDecimal;

/** Exact decimal value exposed through members rather than a lossy polyglot double. */
@ExportLibrary(InteropLibrary.class)
public final class HaraDecimal implements TruffleObject {
  private static final String[] MEMBERS = {"value", "unscaled", "scale", "precision"};

  private final BigDecimal value;

  public HaraDecimal(BigDecimal value) {
    this.value = Num.canonicalDecimal(value);
  }

  public BigDecimal value() {
    return value;
  }

  @ExportMessage
  boolean hasMembers() {
    return true;
  }

  @ExportMessage
  Object getMembers(boolean includeInternal) {
    return new HaraStruct.HaraMemberNames(MEMBERS);
  }

  @ExportMessage
  boolean isMemberReadable(String member) {
    return "value".equals(member)
        || "unscaled".equals(member)
        || "scale".equals(member)
        || "precision".equals(member);
  }

  @ExportMessage
  @TruffleBoundary
  Object readMember(String member) throws UnknownIdentifierException {
    switch (member) {
      case "value":
        return value.toPlainString();
      case "unscaled":
        return HaraBox.export(value.unscaledValue());
      case "scale":
        return value.scale();
      case "precision":
        return value.precision();
      default:
        throw UnknownIdentifierException.create(member);
    }
  }

  @ExportMessage
  @TruffleBoundary
  Object toDisplayString(boolean allowSideEffects) {
    return value.toPlainString() + "M";
  }

  @TruffleBoundary
  @Override
  public boolean equals(Object other) {
    return other instanceof HaraDecimal && value.equals(((HaraDecimal) other).value);
  }

  @TruffleBoundary
  @Override
  public int hashCode() {
    return value.hashCode();
  }

  @TruffleBoundary
  @Override
  public String toString() {
    return value.toPlainString() + "M";
  }
}
