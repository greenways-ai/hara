package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import hara.lang.data.List;
import hara.lang.data.Map;
import hara.lang.data.OrderedMap;
import hara.lang.data.OrderedSet;
import hara.lang.data.Queue;
import hara.lang.data.Set;
import hara.lang.data.SortedMap;
import hara.lang.data.SortedSet;
import hara.lang.data.Tuple;
import hara.lang.data.Vector;
import hara.lang.protocol.IConj;
import java.util.Iterator;
import java.util.Map.Entry;
import org.junit.Test;

/** Protocol-level collection checks shared by Hara-native and Java-backed values. */
public class CollectionProtocolConformanceTest {
  @Test
  public void lookupPreservesMissingAndPresentNullValues() {
    HaraProtocol lookup = new HaraProtocol("ILookup", java.util.Map.of("lookup", -1));
    HaraJavaAdapters.installLookup(lookup);
    Map.Standard<String, String> map = Map.Standard.from(null, "present", null);

    assertEquals(null, lookup.invoke("lookup", map, new Object[] {"present"}));
    assertEquals("missing", lookup.invoke("lookup", map, new Object[] {"missing", "missing"}));
  }

  @Test
  public void countAndIterationCoverPersistentMutableAndOrderedShapes() {
    HaraProtocol count = new HaraProtocol("ICount", java.util.Map.of("count", 1));
    HaraJavaAdapters.installCount(count);
    HaraProtocol collection =
        new HaraProtocol(
            "IColl",
            java.util.Map.of("start-string", 1, "end-string", 1, "sep-string", 1, "iterator", 1));
    HaraJavaAdapters.installCollection(collection);

    Object[][] values = {
      {Map.Standard.from(null, "a", 1, "b", 2), 2L},
      {Map.Mutable.from(null, "a", 1, "b", 2), 2L},
      {OrderedMap.Standard.from(null, "a", 1, "b", 2), 2L},
      {SortedMap.Standard.from(null, "b", 2, "a", 1), 2L},
      {Set.Standard.from(null, "a", "b"), 2L},
      {OrderedSet.Standard.from(null, "a", "b"), 2L},
      {SortedSet.Standard.from(null, "b", "a"), 2L},
      {Vector.Standard.from(null, "a", "b"), 2L},
      {List.Standard.from(null, "a", "b"), 2L},
      {Queue.Standard.from(null, "a", "b"), 2L},
      {new Tuple.Tup2.L<>(null, "a", "b"), 2L}
    };

    for (Object[] value : values) {
      Object receiver = value[0];
      assertEquals(value[1], count.invoke("count", receiver, new Object[0]));
      Iterator<?> iterator = (Iterator<?>) collection.invoke("iterator", receiver, new Object[0]);
      assertTrue(iterator.hasNext());
      iterator.next();
    }
  }

  @Test
  public void mapInvocationAndSetInvocationUseNotFoundArguments() {
    HaraProtocol ifn = new HaraProtocol("IFn", java.util.Map.of("invoke", -1));
    HaraJavaAdapters.installIFn(ifn);

    Map.Standard<String, String> map = Map.Standard.from(null, "key", "value");
    assertEquals("value", ifn.invoke("invoke", map, new Object[] {"key"}));
    assertEquals("missing", ifn.invoke("invoke", map, new Object[] {"missing", "missing"}));

    Set.Standard<String> set = Set.Standard.from(null, "present");
    assertEquals("present", ifn.invoke("invoke", set, new Object[] {"present"}));
    assertEquals("missing", ifn.invoke("invoke", set, new Object[] {"missing", "missing"}));
  }

  @Test
  public void persistentUpdatesDoNotMutateTheOriginalValue() {
    HaraProtocol assoc = new HaraProtocol("IAssoc", java.util.Map.of("assoc", 3));
    HaraJavaAdapters.installAssoc(assoc);
    HaraProtocol lookup = new HaraProtocol("ILookup", java.util.Map.of("lookup", -1));
    HaraJavaAdapters.installLookup(lookup);

    Vector.Standard<String> original = Vector.Standard.from(null, "old");
    Object updated = assoc.invoke("assoc", original, new Object[] {0, "new"});
    assertEquals("old", lookup.invoke("lookup", original, new Object[] {0L}));
    assertEquals("new", lookup.invoke("lookup", updated, new Object[] {0L}));
  }

  @Test
  public void emptyAndConsRemainProtocolOperations() {
    HaraProtocol empty = new HaraProtocol("IEmpty", java.util.Map.of("empty", 1));
    HaraJavaAdapters.installEmpty(empty);
    HaraProtocol cons = new HaraProtocol("ICons", java.util.Map.of("cons", 2));
    HaraJavaAdapters.installCons(cons);
    HaraProtocol count = new HaraProtocol("ICount", java.util.Map.of("count", 1));
    HaraJavaAdapters.installCount(count);

    List.Standard<String> list = List.Standard.from(null, "tail");
    Object emptyList = empty.invoke("empty", list, new Object[0]);
    assertEquals(0L, count.invoke("count", emptyList, new Object[0]));
    Object prepended = cons.invoke("cons", list, new Object[] {"head"});
    assertEquals(2L, count.invoke("count", prepended, new Object[0]));
    assertFalse(count.invoke("count", list, new Object[0]).equals(0L));
  }

  @Test
  public void updateAndNavigationProtocolsPreserveCollectionFamilies() {
    HaraProtocol lookup = new HaraProtocol("ILookup", java.util.Map.of("lookup", -1));
    HaraJavaAdapters.installLookup(lookup);
    HaraProtocol assoc = new HaraProtocol("IAssoc", java.util.Map.of("assoc", 3));
    HaraJavaAdapters.installAssoc(assoc);
    HaraProtocol dissoc = new HaraProtocol("IDissoc", java.util.Map.of("dissoc", 2));
    HaraJavaAdapters.installDissoc(dissoc);
    HaraProtocol conj = new HaraProtocol("IConj", java.util.Map.of("conj", 2));
    HaraJavaAdapters.installConj(conj);
    HaraProtocol cons = new HaraProtocol("ICons", java.util.Map.of("cons", 2));
    HaraJavaAdapters.installCons(cons);
    HaraProtocol nth = new HaraProtocol("INth", java.util.Map.of("nth", 2));
    HaraJavaAdapters.installNth(nth);
    HaraProtocol peekFirst =
        new HaraProtocol("IPeekFirst", java.util.Map.of("peek-first", 1));
    HaraJavaAdapters.installPeekFirst(peekFirst);
    HaraProtocol peekLast =
        new HaraProtocol("IPeekLast", java.util.Map.of("peek-last", 1));
    HaraJavaAdapters.installPeekLast(peekLast);
    HaraProtocol pushFirst =
        new HaraProtocol("IPushFirst", java.util.Map.of("push-first", 2));
    HaraJavaAdapters.installPushFirst(pushFirst);
    HaraProtocol pushLast =
        new HaraProtocol("IPushLast", java.util.Map.of("push-last", 2));
    HaraJavaAdapters.installPushLast(pushLast);

    Map.Standard<String, Long> map = Map.Standard.from(null, "a", 1L);
    Object mapWithB = assoc.invoke("assoc", map, new Object[] {"b", 2L});
    assertEquals(1L, lookup.invoke("lookup", map, new Object[] {"a"}));
    assertEquals(2L, lookup.invoke("lookup", mapWithB, new Object[] {"b"}));
    Object mapWithoutA = dissoc.invoke("dissoc", mapWithB, new Object[] {"a"});
    assertEquals("missing", lookup.invoke("lookup", mapWithoutA, new Object[] {"a", "missing"}));

    Vector.Standard<Long> vector = Vector.Standard.from(null, 1L, 2L);
    Object vectorWith3 = conj.invoke("conj", vector, new Object[] {3L});
    assertEquals(3L, nth.invoke("nth", vectorWith3, new Object[] {2L}));
    List.Standard<Long> list = List.Standard.from(null, 1L, 2L);
    Object listWith0 = cons.invoke("cons", list, new Object[] {0L});
    assertEquals(0L, peekFirst.invoke("peek-first", listWith0, new Object[0]));
    assertEquals(1L, peekFirst.invoke("peek-first", list, new Object[0]));
    assertEquals(2L, peekLast.invoke("peek-last", list, new Object[0]));
    assertEquals(
        0L,
        peekFirst.invoke(
            "peek-first", pushFirst.invoke("push-first", list, new Object[] {0L}), new Object[0]));
    assertEquals(
        3L,
        peekLast.invoke(
            "peek-last", pushLast.invoke("push-last", list, new Object[] {3L}), new Object[0]));

    Map.Mutable<String, Long> mutable = Map.Mutable.from(null, "a", 1L);
    Object sameMutable = assoc.invoke("assoc", mutable, new Object[] {"b", 2L});
    assertTrue(sameMutable == mutable);
    assertEquals(2L, lookup.invoke("lookup", mutable, new Object[] {"b"}));
  }

  @Test
  public void emptyPreservesEverySupportedCollectionFamily() {
    HaraProtocol empty = new HaraProtocol("IEmpty", java.util.Map.of("empty", 1));
    HaraJavaAdapters.installEmpty(empty);
    HaraProtocol count = new HaraProtocol("ICount", java.util.Map.of("count", 1));
    HaraJavaAdapters.installCount(count);

    Object[] persistent = {
      Map.Standard.from(null, "a", 1),
      OrderedMap.Standard.from(null, "a", 1),
      SortedMap.Standard.from(null, "a", 1),
      Set.Standard.from(null, "a"),
      OrderedSet.Standard.from(null, "a"),
      SortedSet.Standard.from(null, "a"),
      Vector.Standard.from(null, "a"),
      List.Standard.from(null, "a"),
      Queue.Standard.from(null, "a"),
      new Tuple.Tup1.L<>(null, "a")
    };
    for (Object receiver : persistent) {
      Object result = empty.invoke("empty", receiver, new Object[0]);
      if (receiver instanceof Tuple.Tup1) {
        assertTrue(result instanceof Tuple.Tup0);
      } else {
        assertEquals(receiver.getClass(), result.getClass());
      }
      assertEquals(0L, count.invoke("count", result, new Object[0]));
      assertEquals(1L, count.invoke("count", receiver, new Object[0]));
    }

    Object[] mutable = {
      Map.Mutable.from(null, "a", 1),
      OrderedMap.Mutable.from(null, "a", 1),
      SortedMap.Mutable.from(null, "a", 1),
      Set.Mutable.from(null, "a"),
      OrderedSet.Mutable.from(null, "a"),
      SortedSet.Mutable.from(null, "a"),
      Vector.Mutable.from(null, "a"),
      List.Mutable.from(null, "a"),
      Queue.Mutable.from(null, "a")
    };
    for (Object receiver : mutable) {
      Object result = empty.invoke("empty", receiver, new Object[0]);
      assertTrue(result == receiver);
      assertEquals(0L, count.invoke("count", receiver, new Object[0]));
    }
  }
}
