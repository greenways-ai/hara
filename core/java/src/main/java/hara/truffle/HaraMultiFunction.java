package hara.truffle;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.interop.ArityException;
import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.TruffleObject;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import hara.lang.base.Eq;
import java.util.ArrayList;
import java.util.List;

/** A Clojure-shaped multimethod: dispatches on the result of a function call. */
@ExportLibrary(InteropLibrary.class)
public final class HaraMultiFunction implements TruffleObject {
  private final HaraFunction dispatchFunction;
  private final List<Method> methods = new ArrayList<>();
  private HaraFunction defaultMethod;

  public HaraMultiFunction(HaraFunction dispatchFunction) {
    this.dispatchFunction = dispatchFunction;
  }

  @TruffleBoundary
  public void addMethod(Object dispatchValue, HaraFunction method) {
    if (dispatchValue instanceof hara.lang.data.Keyword
        && ((hara.lang.data.Keyword) dispatchValue).getNamespace() == null
        && ((hara.lang.data.Keyword) dispatchValue).getName().equals("default")) {
      defaultMethod = method;
      return;
    }
    for (int i = 0; i < methods.size(); i++) {
      if (Eq.eq(dispatchValue, methods.get(i).dispatchValue)) {
        methods.set(i, new Method(dispatchValue, method));
        return;
      }
    }
    methods.add(new Method(dispatchValue, method));
  }

  @TruffleBoundary
  public Object invoke(Object[] arguments) {
    HaraFunction dispatch = dispatchFunction.resolveArity(arguments.length);
    if (dispatch == null) {
      throw new HaraException(
          "Multimethod dispatch function does not accept " + arguments.length + " arguments");
    }
    Object dispatchValue = dispatch.callTarget().call(dispatch.callArguments(arguments));
    HaraFunction selected = null;
    for (Method method : methods) {
      if (Eq.eq(dispatchValue, method.dispatchValue)) {
        selected = method.function;
        break;
      }
    }
    if (selected == null) selected = defaultMethod;
    if (selected == null) {
      throw new HaraException("No multimethod method for dispatch value " + dispatchValue);
    }
    HaraFunction method = selected.resolveArity(arguments.length);
    if (method == null) {
      throw new HaraException(
          "Multimethod method does not accept " + arguments.length + " arguments");
    }
    return method.callTarget().call(method.callArguments(arguments));
  }

  @ExportMessage
  boolean isExecutable() {
    return true;
  }

  @ExportMessage
  Object execute(Object[] arguments) throws ArityException {
    return HaraBox.export(invoke(arguments));
  }

  @Override
  public String toString() {
    return "#<multifn>";
  }

  private static final class Method {
    private final Object dispatchValue;
    private final HaraFunction function;

    private Method(Object dispatchValue, HaraFunction function) {
      this.dispatchValue = dispatchValue;
      this.function = function;
    }
  }
}
