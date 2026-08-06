package hara.lang.data;

import hara.lang.base.G;
import hara.lang.protocol.*;

import java.util.Iterator;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicReference;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Predicate;

public interface Atom<V> {

  public abstract class Struct<R, V> implements Swap<R, V>, IReset<V>, IDisplay {
    final AtomicReference<V> _state;

    public Struct(V init) {
      _state = new AtomicReference<V>(init);
    }

    @Override
    public V deref() {
      return _state.get();
    }

    @Override
    public boolean cas(V oldVal, V newVal) {
      return _state.compareAndSet(oldVal, newVal);
    }

    @Override
    public V reset(V newVal) {
      validate(newVal);
      V oldVal = _state.getAndSet(newVal);
      notifyWatches(oldVal, newVal);
      return newVal;
    }

    @Override
    public String display() {
      return "#atom <" + G.display(_state.get()) + ">";
    }
  }

  @SuppressWarnings("rawtypes")
  public final class Basic<V> extends Struct<Atom, V> {
    public Basic(V state) {
      super(state);
    }
  }

  @SuppressWarnings("rawtypes")
  public final class Standard<V> extends Struct<Atom, V> implements Swap<Atom, V> {

    final Predicate<V> _validator;
    final ConcurrentHashMap<Object, Consumer<WatchEntry<Atom, V>>> _watches =
        new ConcurrentHashMap<Object, Consumer<WatchEntry<Atom, V>>>();

    public Standard(V init) {
      super(init);
      _validator = null;
    }

    public Standard(V init, Predicate<V> validator) {
      super(init);
      _validator = validator;
    }

    @Override
    public Predicate<V> getValidator() {
      return _validator;
    }

    @Override
    public void addWatch(Object key, Consumer<WatchEntry<Atom, V>> f) {
      _watches.put(key, f);
    }

    @Override
    public void removeWatch(Object key) {
      _watches.remove(key);
    }

    @Override
    public Iterator<Map.Entry<Object, Consumer<WatchEntry<Atom, V>>>> getWatches() {
      return _watches.entrySet().iterator();
    }
  }

  public interface Swap<R, V> extends IWatch<R, V>, IDeref<V>, ICas<V> {

    default Predicate<V> getValidator() {
      return null;
    }

    default boolean validate(V value) {
      Predicate<V> validator = getValidator();
      if (validator != null && !validator.test(value)) {
        throw new IllegalStateException("Validator rejected value: " + value);
      }
      return true;
    }

    @Override
    boolean cas(V oldVal, V newVal);

    default Object swap(Function<V, V> f) {
      for (; ; ) {
        var v = deref();
        var newVal = f.apply(v);
        if (validate(newVal) && cas(v, newVal)) {
          notifyWatches(v, newVal);
          return newVal;
        }
      }
    }

    default Object swap(BiFunction<V, Object, V> f, Object arg) {
      for (; ; ) {
        var v = deref();
        var newVal = f.apply(v, arg);
        if (validate(newVal) && cas(v, newVal)) {
          notifyWatches(v, newVal);
          return newVal;
        }
      }
    }

    default Object swap(BiFunction<V, Object[], V> f, Object... vargs) {
      for (; ; ) {
        var v = deref();
        var newVal = f.apply(v, vargs);
        if (validate(newVal) && cas(v, newVal)) {
          notifyWatches(v, newVal);
          return newVal;
        }
      }
    }
  }
}
