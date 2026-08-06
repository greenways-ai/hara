package hara.lang.context;

import hara.lang.protocol.IContext;
import hara.lang.protocol.IPointer;
import java.util.LinkedHashMap;
import java.util.Map;

/** Safe fallback context used when no runtime is active. */
public final class NullContext implements IContext {
  public static final NullContext INSTANCE = new NullContext();

  private NullContext() {}

  @Override
  public Object call(Object... args) {
    throw new IllegalStateException("Context runtime is not active");
  }

  @Override
  public Object rawEval(String source) {
    return source;
  }

  @Override
  public Object initPtr(IPointer pointer) {
    return null;
  }

  @Override
  public Object tagsPtr(IPointer pointer) {
    return null;
  }

  @Override
  public Object derefPtr(IPointer pointer) {
    if (pointer == null) return null;
    Map<Object, Object> values = new LinkedHashMap<>();
    for (Object key : pointer.ptrKeys()) values.put(key, pointer.ptrVal(key));
    return values;
  }

  @Override
  public Object displayPtr(IPointer pointer) {
    return pointer == null ? null : pointer;
  }

  @Override
  public Object invokePtr(IPointer pointer, Object[] args) {
    throw new IllegalStateException("Context runtime is not active");
  }

  @Override
  public Object transformInPtr(IPointer pointer, Object[] args) {
    return args;
  }

  @Override
  public Object transformOutPtr(IPointer pointer, Object value) {
    return value;
  }
}
