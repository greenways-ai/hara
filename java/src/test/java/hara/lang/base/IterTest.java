package hara.lang.base;

import org.junit.Test;

import java.util.Arrays;
import java.util.Iterator;
import java.util.List;
import java.util.Map.Entry;
import java.util.concurrent.atomic.AtomicInteger;

import hara.lang.base.iter.CloseableIterator;

import static org.junit.Assert.*;

public class IterTest {

  @Test
  public void testIter() {
    Integer[] arr = {1, 2, 3};
    Iterator<Integer> iter = Iter.iter(arr);
    assertTrue(iter.hasNext());
    assertEquals(Integer.valueOf(1), iter.next());
    assertTrue(iter.hasNext());
    assertEquals(Integer.valueOf(2), iter.next());
    assertTrue(iter.hasNext());
    assertEquals(Integer.valueOf(3), iter.next());
    assertFalse(iter.hasNext());
  }

  @Test
  public void testReduce() {
    List<Integer> list = Arrays.asList(1, 2, 3, 4, 5);
    Integer result = Iter.reduce(list.iterator(), (a, b) -> a + b);
    assertEquals(Integer.valueOf(15), result);
  }

  @Test
  public void testMap() {
    List<Integer> list = Arrays.asList(1, 2, 3);
    Iterator<String> iter = Iter.map(list.iterator(), n -> "v" + n);
    assertEquals("v1", iter.next());
    assertEquals("v2", iter.next());
    assertEquals("v3", iter.next());
    assertFalse(iter.hasNext());
  }

  @Test
  public void testFilter() {
    List<Integer> list = Arrays.asList(1, 2, 3, 4, 5);
    Iterator<Integer> iter = Iter.filter(list.iterator(), n -> n % 2 == 0);
    assertTrue(iter.hasNext());
    assertEquals(Integer.valueOf(2), iter.next());
    assertTrue(iter.hasNext());
    assertEquals(Integer.valueOf(4), iter.next());
    assertFalse(iter.hasNext());
  }

  @Test
  public void testConcat() {
    List<Integer> list1 = Arrays.asList(1, 2);
    List<Integer> list2 = Arrays.asList(3, 4);
    Iterator<Integer> iter = Iter.concat(list1.iterator(), list2.iterator());
    assertEquals(Integer.valueOf(1), iter.next());
    assertEquals(Integer.valueOf(2), iter.next());
    assertEquals(Integer.valueOf(3), iter.next());
    assertEquals(Integer.valueOf(4), iter.next());
    assertFalse(iter.hasNext());
  }

  @Test
  public void testConcatDoesNotRequestLaterSourcesUntilNeeded() {
    final int[] requested = {0};
    Iterator<Iterator<Integer>> sources =
        new Iterator<Iterator<Integer>>() {
          private int sourceIndex;

          @Override
          public boolean hasNext() {
            return sourceIndex < 2;
          }

          @Override
          public Iterator<Integer> next() {
            requested[0]++;
            int start = sourceIndex++ * 2;
            return Arrays.asList(start, start + 1).iterator();
          }
        };

    Iterator<Integer> iter = Iter.concat(sources);
    assertEquals(0, requested[0]);
    assertEquals(Integer.valueOf(0), iter.next());
    assertEquals(1, requested[0]);
    assertEquals(Integer.valueOf(1), iter.next());
    assertEquals(1, requested[0]);
    assertEquals(Integer.valueOf(2), iter.next());
    assertEquals(2, requested[0]);
  }

  @Test
  public void testConcatClosesExhaustedAndExplicitlyClosedSources() {
    AtomicInteger closed = new AtomicInteger();
    Iterator<Integer> first = closeable(Arrays.asList(1).iterator(), closed);
    Iterator<Integer> second = closeable(Arrays.asList(2).iterator(), closed);

    CloseableIterator<Integer> concatenated =
        (CloseableIterator<Integer>) Iter.concat(first, second);
    assertEquals(Integer.valueOf(1), concatenated.next());
    assertEquals(Integer.valueOf(2), concatenated.next());
    assertEquals(1, closed.get());
    concatenated =
        (CloseableIterator<Integer>)
            Iter.concat(closeable(Arrays.asList(3).iterator(), closed), second);
    assertEquals(Integer.valueOf(3), concatenated.next());
    concatenated.close();
    assertEquals(2, closed.get());
  }

  private static <E> Iterator<E> closeable(Iterator<E> iterator, AtomicInteger closed) {
    return new CloseableIterator<E>() {
      @Override
      public boolean hasNext() {
        return iterator.hasNext();
      }

      @Override
      public E next() {
        return iterator.next();
      }

      @Override
      public void close() {
        closed.incrementAndGet();
      }
    };
  }

  @Test
  public void testZipDefersSourceAcquisitionUntilFirstConsumption() {
    final int[] requested = {0};
    Iterator<Iterator<Object>> sources =
        new Iterator<Iterator<Object>>() {
          private int index;

          @Override
          public boolean hasNext() {
            return index < 2;
          }

          @Override
          public Iterator<Object> next() {
            requested[0]++;
            return Arrays.<Object>asList(index++).iterator();
          }
        };

    Iterator<Object[]> zipped = Iter.zip(sources);
    assertEquals(0, requested[0]);
    assertArrayEquals(new Object[] {0, 1}, zipped.next());
    assertEquals(2, requested[0]);
  }

  @Test
  public void testDropAppliesWhenNextIsCalledDirectly() {
    Iterator<Integer> dropped = Iter.drop(Arrays.asList(1, 2, 3).iterator(), 2);
    assertEquals(Integer.valueOf(3), dropped.next());
    assertFalse(dropped.hasNext());
  }

  @Test
  public void testCycleDefersSourceAcquisitionUntilFirstConsumption() {
    final int[] requested = {0};
    Iterator<Integer> cycled =
        Iter.cycle(
            () -> {
              requested[0]++;
              return Arrays.asList(1).iterator();
            });

    assertEquals(0, requested[0]);
    assertEquals(Integer.valueOf(1), cycled.next());
    assertEquals(1, requested[0]);
    assertEquals(Integer.valueOf(1), cycled.next());
    assertEquals(1, requested[0]);
  }

  @Test
  public void testCycleHasExactLookaheadAndRejectsAnEmptyCache() {
    Iterator<Integer> empty = Iter.cycle(() -> java.util.Collections.emptyIterator());
    assertFalse(empty.hasNext());

    Iterator<Integer> cycled = Iter.cycle(() -> Arrays.asList(1, 2).iterator());
    assertTrue(cycled.hasNext());
    assertTrue(cycled.hasNext());
    assertEquals(Integer.valueOf(1), cycled.next());
    assertEquals(Integer.valueOf(2), cycled.next());
    assertEquals(Integer.valueOf(1), cycled.next());
  }

  @Test
  public void testPartitionPairSupportsDirectNextAndDropsOddTail() {
    Iterator<Entry<Integer, Integer>> pairs = Iter.partitionPair(Arrays.asList(1, 2, 3).iterator());

    Entry<Integer, Integer> pair = pairs.next();
    assertEquals(Integer.valueOf(1), pair.getKey());
    assertEquals(Integer.valueOf(2), pair.getValue());
    assertFalse(pairs.hasNext());
  }
}
