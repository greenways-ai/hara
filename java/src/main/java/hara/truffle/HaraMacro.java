package hara.truffle;

import hara.lang.data.List;
import hara.lang.data.Symbol;
import java.util.function.Function;

/** A macro expander backed by an ordinary compiled Hara function or a narrow native expander. */
final class HaraMacro {
  private final HaraContext context;
  private final String namespace;
  private final Symbol name;
  private final Object function;
  private final Function<List<?>, Object> nativeExpander;

  HaraMacro(
      HaraContext context,
      String namespace,
      Symbol name,
      Object function) {
    this.context = context;
    this.namespace = namespace;
    this.name = name;
    this.function = function;
    this.nativeExpander = null;
  }

  private HaraMacro(Symbol name, Function<List<?>, Object> nativeExpander) {
    this.context = null;
    this.namespace = null;
    this.name = name;
    this.function = null;
    this.nativeExpander = nativeExpander;
  }

  static HaraMacro nativeMacro(Symbol name, Function<List<?>, Object> expander) {
    return new HaraMacro(name, expander);
  }

  String namespace() {
    return namespace;
  }

  Symbol name() {
    return name;
  }

  Object expand(List<?> invocation) {
    return expand(invocation, hara.lang.data.Map.Standard.EMPTY);
  }

  Object expand(List<?> invocation, Object macroEnvironment) {
    if (nativeExpander != null) return nativeExpander.apply(invocation);
    Object[] arguments = new Object[(int) invocation.count() + 1];
    arguments[0] = invocation;
    arguments[1] = macroEnvironment;
    for (int index = 1; index < invocation.count(); index++) {
      arguments[index + 1] = invocation.nth(index);
    }
    try {
      return HaraBox.unwrap(context.invokeCallable(function, arguments));
    } catch (HaraException error) {
      throw new HaraException(
          "Unable to expand macro " + namespace + "/" + name.getName() + ": " + error.getMessage());
    }
  }
}
