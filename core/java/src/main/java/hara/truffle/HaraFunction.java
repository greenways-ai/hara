package hara.truffle;

import com.oracle.truffle.api.RootCallTarget;
import com.oracle.truffle.api.frame.MaterializedFrame;
import com.oracle.truffle.api.interop.ArityException;
import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import com.oracle.truffle.api.interop.TruffleObject;

@ExportLibrary(InteropLibrary.class)
public final class HaraFunction implements TruffleObject {
  private final RootCallTarget callTarget;
  private final int minimumArity;
  private final boolean variadic;
  private final MaterializedFrame closure;
  private final HaraFunction[] overloads;

  public HaraFunction(
      RootCallTarget callTarget, int minimumArity, boolean variadic, MaterializedFrame closure) {
    this.callTarget = callTarget;
    this.minimumArity = minimumArity;
    this.variadic = variadic;
    this.closure = closure;
    this.overloads = null;
  }

  public HaraFunction(HaraFunction[] overloads) {
    if (overloads.length == 0) {
      throw new IllegalArgumentException("A function must have at least one arity");
    }
    this.callTarget = null;
    this.minimumArity = -1;
    this.variadic = false;
    this.closure = null;
    this.overloads = overloads.clone();
  }

  public RootCallTarget callTarget() {
    if (overloads != null) {
      throw new IllegalStateException("Overloaded function requires arity resolution");
    }
    return callTarget;
  }

  public int arity() {
    return overloads == null ? minimumArity : -1;
  }

  public boolean acceptsArity(int actual) {
    if (overloads != null) {
      return resolveArity(actual) != null;
    }
    return actual >= minimumArity && (variadic || actual == minimumArity);
  }

  public HaraFunction resolveArity(int actual) {
    if (overloads == null) {
      return acceptsArity(actual) ? this : null;
    }
    HaraFunction variadicMatch = null;
    for (HaraFunction overload : overloads) {
      if (overload.variadic() && overload.acceptsArity(actual)) {
        variadicMatch = overload;
      } else if (!overload.variadic() && overload.acceptsArity(actual)) {
        return overload;
      }
    }
    return variadicMatch;
  }

  public boolean variadic() {
    return variadic;
  }

  @ExportMessage
  boolean isExecutable() {
    return true;
  }

  @ExportMessage
  Object execute(Object[] arguments) throws ArityException {
    HaraFunction selected = resolveArity(arguments.length);
    if (selected == null) {
      throw ArityException.create(
          minimumArity < 0 ? 0 : minimumArity,
          variadic ? Integer.MAX_VALUE : minimumArity,
          arguments.length);
    }
    return HaraBox.export(selected.callTarget.call(selected.callArguments(arguments)));
  }

  public Object[] callArguments(Object[] arguments) {
    if (overloads != null) {
      throw new IllegalStateException("Overloaded function requires arity resolution");
    }
    Object[] callArguments = new Object[arguments.length + 1];
    callArguments[0] = closure;
    System.arraycopy(arguments, 0, callArguments, 1, arguments.length);
    return callArguments;
  }

  @Override
  public String toString() {
    if (overloads != null) {
      return "#<fn/multi>";
    }
    return variadic ? "#<fn/" + minimumArity + "+>" : "#<fn/" + minimumArity + ">";
  }
}
