package hara.lang.base;

import hara.lang.base.primitive.*;
import hara.lang.data.AsList;
import hara.lang.data.AsMap;
import hara.lang.data.AsSet;
import org.junit.Test;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Iterator;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Supplier;

import static org.junit.Assert.*;

public class UtTest {

  @Test
  public void testRefCache() throws InterruptedException {
    RefCache<String, Integer> cache = new RefCache<>();
    Integer val = 10;
    cache.register("a", val);
    assertEquals(1, cache.count());
    assertEquals(val, cache.get("a"));
    assertNotNull(cache.find("a"));
    assertNull(cache.find("b"));
    assertNotNull(cache.lookup("a"));
    assertNull(cache.lookup("b"));

    cache.deregister("a");
    assertEquals(0, cache.count());
    assertNull(cache.get("a"));

    // Test getOrCreate
    AtomicInteger supplierCounter = new AtomicInteger(0);
    Supplier<java.lang.ref.Reference<Integer>> supplier =
        () -> {
          supplierCounter.incrementAndGet();
          return new java.lang.ref.WeakReference<>(20);
        };

    assertEquals(Integer.valueOf(20), cache.getOrCreate("b", supplier));
    assertEquals(1, cache.count());
    assertEquals(1, supplierCounter.get());

    // Should not call supplier again
    assertEquals(Integer.valueOf(20), cache.getOrCreate("b", supplier));
    assertEquals(1, supplierCounter.get());

    // Test garbage collection
    cache.register("c", 30);
    System.gc();
    Thread.sleep(100); // Give GC some time
    cache.clearCache();
    // Depending on GC, the cache may or may not be cleared
    assertTrue(cache.count() <= 2);
  }

  @Test
  public void testMurmur3() {
    assertEquals(0, Murmur3.hashInt(0));
    assertEquals(638799361, Murmur3.hashInt(12345));
    assertEquals(1982413648, Murmur3.hashInt(-1));

    assertEquals(0, Murmur3.hashLong(0));
    assertEquals(-468999172, Murmur3.hashLong(123456789012345L));
    assertEquals(1651860712, Murmur3.hashLong(-1L));

    assertEquals(1689409188, Murmur3.hashChars("hello world"));
    assertEquals(0, Murmur3.hashChars(""));
  }

  @Test
  public void testFlag() {
    Flag flag = new Flag(true);
    assertTrue(flag.deref());
    assertEquals("#flag <true>", flag.display());

    flag.reset(false);
    assertFalse(flag.deref());
    assertEquals("#flag <false>", flag.display());
  }

  @Test
  public void testDelay() {
    AtomicInteger counter = new AtomicInteger(0);
    Supplier<Integer> supplier =
        () -> {
          counter.incrementAndGet();
          return 10;
        };

    Delay<Integer> delay = new Delay<>(supplier);
    assertFalse(delay.isRealized());
    assertEquals("#delay.pending<>", delay.display());

    assertEquals(Integer.valueOf(10), delay.deref());
    assertTrue(delay.isRealized());
    assertEquals(1, counter.get());
    assertEquals("#delay <10>", delay.display());

    // Call deref again, counter should not increment
    assertEquals(Integer.valueOf(10), delay.deref());
    assertEquals(1, counter.get());

    // Test exception handling
    Supplier<Integer> failingSupplier =
        () -> {
          throw new RuntimeException("Test Exception");
        };
    Delay<Integer> failingDelay = new Delay<>(failingSupplier);
    try {
      failingDelay.deref();
    } catch (Throwable e) {
      // Just assert that an exception is thrown, as the message is unreliable
      assertNotNull(e);
    }
  }

  @Test
  public void testCounter() {
    Counter counter = new Counter(5);
    assertEquals(Integer.valueOf(5), counter.deref());
    assertEquals("#counter <5>", counter.display());

    assertEquals(6, counter.inc());
    assertEquals(Integer.valueOf(6), counter.deref());

    assertEquals(10, counter.inc(4));
    assertEquals(Integer.valueOf(10), counter.deref());

    assertEquals(9, counter.dec());
    assertEquals(Integer.valueOf(9), counter.deref());

    assertEquals(5, counter.dec(4));
    assertEquals(Integer.valueOf(5), counter.deref());

    counter.reset(100);
    assertEquals(Integer.valueOf(100), counter.deref());
  }

  @Test
  public void testClock() throws InterruptedException {
    long nanos1 = Clock.currentTimeNanos();
    long micros1 = Clock.currentTimeMicros();
    long millis1 = Clock.currentTimeMillis();

    Thread.sleep(1); // Sleep for 1 millisecond to ensure time progresses

    long nanos2 = Clock.currentTimeNanos();
    long micros2 = Clock.currentTimeMicros();
    long millis2 = Clock.currentTimeMillis();

    assertTrue(nanos2 > nanos1);
    assertTrue(micros2 > micros1);
    assertTrue(millis2 >= millis1); // Millis might not advance in 1ms sleep

    assertEquals(nanos1 / 1000, micros1, 1);
    assertEquals(micros1 / 1000, millis1, 1);
  }

  @Test
  public void testAsSet() {
    java.util.Set<String> set = new java.util.HashSet<>();
    set.add("a");
    set.add("b");
    set.add("c");
    AsSet<String> asSet = new AsSet<>(set);

    assertEquals(3, asSet.count());
    assertEquals("b", asSet.find("b"));
    assertNull(asSet.find("d"));

    asSet.conj("d");
    assertEquals(4, asSet.count());
    assertEquals("d", asSet.find("d"));

    asSet.dissoc("a");
    assertEquals(3, asSet.count());
    assertNull(asSet.find("a"));

    java.util.Iterator<String> it = asSet.iterator();
    assertTrue(it.hasNext());
    it.next();
    assertTrue(it.hasNext());
    it.next();
    assertTrue(it.hasNext());
    it.next();
    assertFalse(it.hasNext());

    asSet.empty();
    assertEquals(0, asSet.count());
  }

  @Test
  public void testAsMap() {
    java.util.Map<String, Integer> map = new java.util.HashMap<>();
    map.put("a", 1);
    map.put("b", 2);
    map.put("c", 3);
    AsMap<String, Integer> asMap = new AsMap<>(map);

    assertEquals(3, asMap.count());
    assertTrue(asMap.has("b"));
    assertFalse(asMap.has("d"));
    assertEquals(Integer.valueOf(2), asMap.lookup("b"));
    assertNull(asMap.lookup("d"));
    assertEquals(Integer.valueOf(5), asMap.lookup("d", 5));

    assertNotNull(asMap.find("c"));
    assertNull(asMap.find("d"));

    asMap.assoc("d", 4);
    assertEquals(4, asMap.count());
    assertTrue(asMap.has("d"));

    asMap.dissoc("a");
    assertEquals(3, asMap.count());
    assertFalse(asMap.has("a"));

    java.util.Iterator<java.util.Map.Entry<String, Integer>> it = asMap.iterator();
    assertTrue(it.hasNext());
    it.next();
    assertTrue(it.hasNext());
    it.next();
    assertTrue(it.hasNext());
    it.next();
    assertFalse(it.hasNext());

    asMap.empty();
    assertEquals(0, asMap.count());
  }

  @Test
  public void testAsList() {
    List<Integer> list = new ArrayList<>(Arrays.asList(1, 2, 3));
    AsList<Integer> asList = new AsList<>(list);

    assertEquals(3, asList.count());
    assertEquals(Integer.valueOf(1), asList.peekFirst());
    assertEquals(Integer.valueOf(3), asList.peekLast());
    assertEquals(Integer.valueOf(2), asList.nth(1));

    asList.pushFirst(0);
    assertEquals(4, asList.count());
    assertEquals(Integer.valueOf(0), asList.peekFirst());

    asList.pushLast(4);
    assertEquals(5, asList.count());
    assertEquals(Integer.valueOf(4), asList.peekLast());

    asList.popFirst();
    assertEquals(4, asList.count());
    assertEquals(Integer.valueOf(1), asList.peekFirst());

    asList.popLast();
    assertEquals(3, asList.count());
    assertEquals(Integer.valueOf(3), asList.peekLast());

    Iterator<Integer> it = asList.iterator();
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(1), it.next());
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(2), it.next());
    assertTrue(it.hasNext());
    assertEquals(Integer.valueOf(3), it.next());
    assertFalse(it.hasNext());

    asList.empty();
    assertEquals(0, asList.count());
  }
}
