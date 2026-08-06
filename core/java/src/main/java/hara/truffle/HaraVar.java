package hara.truffle;

import com.oracle.truffle.api.CompilerDirectives.TruffleBoundary;
import com.oracle.truffle.api.interop.InteropLibrary;
import com.oracle.truffle.api.interop.TruffleObject;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import hara.lang.protocol.IDeref;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;
import hara.lang.protocol.IReset;
import hara.lang.data.Keyword;
import hara.lang.protocol.Constant;
import hara.lang.protocol.ILookup;
import hara.lang.data.Map;
import hara.lang.data.types.IVarType;
import java.util.ArrayDeque;
import java.util.Deque;

@ExportLibrary(InteropLibrary.class)
public final class HaraVar
    implements TruffleObject, IDeref<Object>, IReset<Object>, IVarType, IObjType {
  enum Origin {
    SOURCE,
    HAL_FALLBACK,
    JAVA_LIBRARY,
    RUNTIME_PRIMITIVE
  }

  private final String namespace;
  private final String name;
  private volatile IMetadata metadata;
  private volatile Object value;
  private volatile Origin origin;
  private final ThreadLocal<Deque<Object>> dynamicBindings =
      ThreadLocal.withInitial(ArrayDeque::new);
  private static final Object NIL_BINDING = new Object();

  HaraVar(String namespace, String name, Object value) {
    this(namespace, name, value, Map.Standard.EMPTY, Origin.SOURCE);
  }

  HaraVar(String namespace, String name, Object value, IMetadata metadata) {
    this(namespace, name, value, metadata, Origin.SOURCE);
  }

  HaraVar(String namespace, String name, Object value, IMetadata metadata, Origin origin) {
    this.namespace = namespace;
    this.name = name;
    this.metadata = metadata == null ? Map.Standard.EMPTY : metadata;
    this.value = value;
    this.origin = origin == null ? Origin.SOURCE : origin;
  }

  public Object get() {
    return value;
  }

  String namespaceName() {
    return namespace;
  }

  String symbolName() {
    return name;
  }

  @TruffleBoundary
  @Override
  public Object deref() {
    Deque<Object> bindings = dynamicBindings.get();
    if (bindings.isEmpty()) return get();
    Object value = bindings.peekLast();
    return value == NIL_BINDING ? null : value;
  }

  @Override
  public Object reset(Object value) {
    set(value);
    return value;
  }

  @TruffleBoundary
  public void bind(Object value) {
    dynamicBindings.get().addLast(value == null ? NIL_BINDING : value);
  }

  @TruffleBoundary
  public void unbind() {
    Deque<Object> bindings = dynamicBindings.get();
    if (bindings.isEmpty()) throw new IllegalStateException("Var has no dynamic binding");
    bindings.removeLast();
    if (bindings.isEmpty()) dynamicBindings.remove();
  }

  @Override
  public IMetadata meta() {
    return metadata;
  }

  @Override
  public HaraVar withMeta(IMetadata metadata) {
    return new HaraVar(namespace, name, value, metadata, origin);
  }

  @Override
  @TruffleBoundary
  public Boolean isDynamic() {
    if (!(metadata instanceof ILookup<?, ?>)) return false;
    Object value = ((ILookup<Object, Object>) metadata).lookup(Keyword.create("dynamic"));
    return Boolean.TRUE.equals(value);
  }

  @Override
  @TruffleBoundary
  public Boolean isMacro() {
    if (!(metadata instanceof ILookup<?, ?>)) return false;
    Object value = ((ILookup<Object, Object>) metadata).lookup(Keyword.create("macro"));
    return Boolean.TRUE.equals(value);
  }

  @Override
  @TruffleBoundary
  public Boolean isControl() {
    if (!(metadata instanceof ILookup<?, ?>)) return false;
    Object value = ((ILookup<Object, Object>) metadata).lookup(Keyword.create("control"));
    return Boolean.TRUE.equals(value);
  }

  @Override
  public long hashCalc(Constant.HashType type) {
    return System.identityHashCode(this);
  }

  @Override
  public String display() {
    return displayName();
  }

  void set(Object value) {
    this.value = value;
  }

  void setMetadata(IMetadata metadata) {
    this.metadata = metadata == null ? Map.Standard.EMPTY : metadata;
  }

  Origin origin() {
    return origin;
  }

  void setOrigin(Origin origin) {
    this.origin = origin == null ? Origin.SOURCE : origin;
  }

  @ExportMessage
  Object toDisplayString(boolean allowSideEffects) {
    return displayName();
  }

  @Override
  public String toString() {
    return displayName();
  }

  @TruffleBoundary
  private String displayName() {
    return "#'" + namespace + "/" + name;
  }
}
