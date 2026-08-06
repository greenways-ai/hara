package hara.lang.context;

import hara.lang.data.Keyword;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IApplicable;
import hara.lang.protocol.IContext;
import hara.lang.protocol.IDeref;
import hara.lang.protocol.ILookup;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;
import hara.lang.protocol.IPointer;
import java.util.AbstractMap.SimpleImmutableEntry;
import java.util.Collections;
import java.util.Iterator;
import java.util.Map;
import java.util.function.Function;

/** A context-qualified value that can be invoked or transformed by its runtime. */
public final class Pointer
    implements IPointer, IApplicable, IDeref<Object>, ILookup<Object, Object>, IObjType {
  private final Object context;
  private final Map<?, ?> values;
  private final Function<Object, IContext> resolver;

  public Pointer(Object context, Map<?, ?> values, ContextRegistry registry) {
    this(context, values, (registry == null ? new ContextRegistry() : registry)::get);
  }

  public Pointer(Object context, Map<?, ?> values, Function<Object, IContext> resolver) {
    if (context == null) throw new IllegalArgumentException("Context required");
    this.context = context;
    this.values = values == null ? Collections.emptyMap() : values;
    this.resolver = resolver == null ? ignored -> NullContext.INSTANCE : resolver;
  }

  public Object context() {
    return context;
  }

  public Map<?, ?> values() {
    return values;
  }

  public IContext runtime() {
    return resolver.apply(context);
  }

  @Override
  public Object ptrContext() {
    return context;
  }

  @Override
  public Iterable<?> ptrKeys() {
    return values.keySet();
  }

  @Override
  public Object ptrVal(Object key) {
    return lookup(key);
  }

  @Override
  public Object lookup(Object key) {
    Object value = values.get(key);
    if (value == null && key instanceof Keyword) {
      value = values.get(((Keyword) key).getName());
    }
    return value;
  }

  @Override
  public Object lookup(Object key, Object notFound) {
    Object value = lookup(key);
    return value == null && !containsKey(key) ? notFound : value;
  }

  @Override
  public Map.Entry<Object, Object> find(Object key) {
    return containsKey(key) ? new SimpleImmutableEntry<>(key, lookup(key)) : null;
  }

  @Override
  @SuppressWarnings("unchecked")
  public Iterator<Object> keys() {
    return (Iterator<Object>) (Iterator<?>) values.keySet().iterator();
  }

  @Override
  @SuppressWarnings("unchecked")
  public Iterator<Object> vals() {
    return (Iterator<Object>) (Iterator<?>) values.values().iterator();
  }

  private boolean containsKey(Object key) {
    return values.containsKey(key)
        || (key instanceof Keyword && values.containsKey(((Keyword) key).getName()));
  }

  @Override
  public Object deref() {
    return runtime().derefPtr(this);
  }

  @Override
  public Object applyIn(Object runtime, Object[] args) {
    return resolve(runtime).invokePtr(this, args);
  }

  @Override
  public Object transformIn(Object runtime, Object[] args) {
    return resolve(runtime).transformInPtr(this, args);
  }

  @Override
  public Object transformOut(Object runtime, Object[] args, Object value) {
    return resolve(runtime).transformOutPtr(this, value);
  }

  @Override
  public IMetadata meta() {
    return null;
  }

  @Override
  public IObjType withMeta(IMetadata meta) {
    return this;
  }

  @Override
  public long hashCalc(Constant.HashType type) {
    return System.identityHashCode(this);
  }

  @Override
  public String display() {
    Object tags = runtime().tagsPtr(this);
    Object displayed = runtime().displayPtr(this);
    return "!" + context + (tags == null ? "" : String.valueOf(tags))
        + (displayed == null ? " <no-context>" : "\n" + displayed);
  }

  private IContext resolve(Object runtime) {
    return runtime instanceof IContext ? (IContext) runtime : resolver.apply(runtime == null ? context : runtime);
  }

  @Override
  public Constant.ObjType getObjType() {
    return Constant.ObjType.CLASS;
  }

  @Override
  public String getObjName() {
    return "POINTER";
  }
}
