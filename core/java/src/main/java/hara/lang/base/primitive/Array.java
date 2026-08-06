package hara.lang.base.primitive;

import hara.lang.base.G;
import hara.lang.base.Iter;
import hara.lang.base.iter.*;

import java.util.Arrays;
import java.util.Iterator;
import java.util.function.BiFunction;
import java.util.function.Function;
import java.util.function.Predicate;
import java.util.function.Supplier;

public interface Array {

  public static boolean[] booleans(boolean... arr) {
    return arr;
  }

  public static byte[] bytes(byte... arr) {
    return arr;
  }

  public static char[] chars(char... arr) {
    return arr;
  }

  public static short[] shorts(short... arr) {
    return arr;
  }

  public static int[] ints(int... arr) {
    return arr;
  }

  public static long[] longs(long... arr) {
    return arr;
  }

  public static float[] floats(float... arr) {
    return arr;
  }

  public static double[] doubles(double... arr) {
    return arr;
  }

  public static String[] strings(String... arr) {
    return arr;
  }

  public static Object[] objects(Object... arr) {
    return arr;
  }

  public static <E> E reduce(BiFunction<E, E, E> f, E[] elements) {
    E out = elements[0];
    for (int i = 1; i < elements.length; i++) {
      out = f.apply(out, elements[i]);
    }
    return out;
  }

  public static <R, E> R reduce(BiFunction<R, E, R> f, R init, E[] elements) {
    R out = init;
    for (E e : elements) {
      out = f.apply(out, e);
    }
    return out;
  }

  public static <R, E> R reduce(
      BiFunction<R, E, R> f, R init, E[] elements, Supplier<Boolean> end) {
    R out = init;
    for (E e : elements) {
      out = f.apply(out, e);
      if (end.get()) {
        return out;
      }
    }
    return out;
  }

  public static <E> boolean every(Predicate<E> f, E[] elements) {
    for (E e : elements) {
      if (!f.test(e)) return false;
    }
    return true;
  }

  public static <E> boolean any(Predicate<E> f, E[] elements) {
    for (E e : elements) {
      if (f.test(e)) return true;
    }
    return false;
  }

  public static <E> E some(Predicate<E> f, E[] elements) {
    for (E e : elements) {
      if (f.test(e)) return e;
    }
    return null;
  }

  public static <E> String display(E[] elements) {
    String s = "";
    for (E e : elements) {
      s += (G.display(e) + " ");
    }
    return s;
  }

  public static Object[] toArray(Object obj) {
    if (obj instanceof Iterator) {
      return Iter.toArray((Iterator<?>) obj);
    } else if (obj.getClass().isArray()) {
      return (Object[]) obj;
    } else {
      return Iter.toArray(Iter.iter(obj));
    }
  }

  @SuppressWarnings("rawtypes")
  public static Iterator toRevIter(Object[] array) {
    return new ReverseArrayIterator(array, 0);
  }

  @SuppressWarnings("rawtypes")
  public static java.util.List toList(Object[] arr) {
    return Arrays.asList(arr);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  public static Iterator toIter(Object array) {
    if (array == null || java.lang.reflect.Array.getLength(array) == 0) return Iter.emptyIterator();
    Class aclass = array.getClass();
    if (aclass == int[].class) return new IntArrayIterator((int[]) array, 0);
    if (aclass == float[].class) return new FloatArrayIterator((float[]) array, 0);
    if (aclass == double[].class) return new DoubleArrayIterator((double[]) array, 0);
    if (aclass == long[].class) return new LongArrayIterator((long[]) array, 0);
    if (aclass == byte[].class) return new ByteArrayIterator((byte[]) array, 0);
    if (aclass == char[].class) return new CharArrayIterator((char[]) array, 0);
    if (aclass == short[].class) return new ShortArrayIterator((short[]) array, 0);
    if (aclass == boolean[].class) return new BooleanArrayIterator((boolean[]) array, 0);
    return new ArrayIterator((Object[]) array, 0);
  }

  public static <E> E[] concat(E[] first, E[] second, Class<E> type) {
    E[] result = newArray(type, first.length + second.length);
    System.arraycopy(first, 0, result, 0, first.length);
    System.arraycopy(second, 0, result, first.length, second.length);
    return result;
  }

  @SuppressWarnings("unchecked")
  public static <E> E[] newArray(Class<E> type, int length) {
    return (E[]) java.lang.reflect.Array.newInstance(type, length);
  }

  @SuppressWarnings({"unchecked"})
  public static <E> Iterator<E> toIter(E... array) {
    if (array == null || array.length == 0) return (Iterator<E>) Iter.emptyIterator();
    return new ArrayIterator<E>(array, 0);
  }

  @SuppressWarnings({"unchecked"})
  public static <E> Iterator<E> toIter(E[] array, int start, int end) {
    if (array == null || array.length == 0) return (Iterator<E>) Iter.emptyIterator();
    return new ArrayIterator<E>(array, start, end);
  }

  public static void checkPosition(int start, int end, int size) {
    if (start < 0 || end < start || end > size) {
      throw new IndexOutOfBoundsException();
    }
  }

  static <E> void swap(E[] array, int i, int j) {
    E temp = array[i];
    array[i] = array[j];
    array[j] = temp;
  }

  @SafeVarargs
  static <E> E[] checkNotNull(E... array) {
    return checkNotNull(array, array.length);
  }

  static <E> E[] checkNotNull(E[] array, int length) {
    for (int i = 0; i < length; i++) {
      checkNotNull(array[i], i);
    }
    return array;
  }

  public static <E> E[] fillArray(Iterator<E> it, E[] array) {
    int i = 0;
    while (it.hasNext()) {
      array[i++] = it.next();
    }
    return array;
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  public static <E, R> R[] map(Function<E, R> f, Class type, E[] array) {
    var out = (R[]) java.lang.reflect.Array.newInstance(type, array.length);
    return fillArray(Iter.map(toIter(array), f), out);
  }
}
