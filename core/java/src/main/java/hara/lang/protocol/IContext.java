package hara.lang.protocol;

/** Callable root context with optional context-runtime operations. */
public interface IContext {
  Object call(Object... args);

  default Object rawEval(String source) {
    return source;
  }

  default Object initPtr(IPointer pointer) {
    return null;
  }

  default Object tagsPtr(IPointer pointer) {
    return null;
  }

  default Object derefPtr(IPointer pointer) {
    return pointer;
  }

  default Object displayPtr(IPointer pointer) {
    return pointer;
  }

  default Object invokePtr(IPointer pointer, Object[] args) {
    return call(args == null ? new Object[0] : args);
  }

  default Object transformInPtr(IPointer pointer, Object[] args) {
    return args;
  }

  default Object transformOutPtr(IPointer pointer, Object value) {
    return value;
  }
}
