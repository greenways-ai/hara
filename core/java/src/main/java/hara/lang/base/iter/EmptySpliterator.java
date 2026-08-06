package hara.lang.base.iter;

import java.util.Spliterator;
import java.util.function.Consumer;

@SuppressWarnings({"rawtypes", "unchecked"})
public interface EmptySpliterator<V> extends java.util.Spliterator<V> {

  @Override
  default boolean tryAdvance(Consumer<? super V> action) {
    return false;
  }

  @Override
  default Spliterator<V> trySplit() {
    return null;
  }

  @Override
  default long estimateSize() {
    return 0;
  }

  @Override
  default int characteristics() {
    return 0;
  }
}
