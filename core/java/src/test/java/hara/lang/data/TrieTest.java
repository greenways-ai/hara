package hara.lang.data;

import org.junit.Test;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Iterator;
import java.util.List;

import static org.junit.Assert.*;

public class TrieTest {

  @Test
  public void testMutableTrie() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    trie.assoc("apple", 1);
    trie.assoc("app", 2);

    assertTrue(trie.has("apple"));
    assertTrue(trie.has("app"));
    assertFalse(trie.has("ap"));

    assertEquals(1, (int) trie.find("apple").getValue());
    assertEquals(2, (int) trie.find("app").getValue());

    trie.dissoc("apple");
    assertFalse(trie.has("apple"));
    assertTrue(trie.has("app"));
  }

  @Test
  public void testStandardTrie() {
    Trie.Standard<Integer> trie = new Trie.Standard<>();
    Trie<Integer> trie1 = trie.assoc("apple", 1);
    Trie<Integer> trie2 = trie1.assoc("app", 2);

    assertTrue(trie2.has("apple"));
    assertTrue(trie2.has("app"));
    assertFalse(trie2.has("ap"));

    assertEquals(1, (int) trie2.find("apple").getValue());
    assertEquals(2, (int) trie2.find("app").getValue());

    Trie<Integer> trie3 = trie2.dissoc("apple");
    assertFalse(trie3.has("apple"));
    assertTrue(trie3.has("app"));
    assertTrue(trie2.has("apple"));
  }

  @Test
  public void testCount() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    assertEquals(0, trie.count());
    trie.assoc("apple", 1);
    assertEquals(1, trie.count());
    trie.assoc("app", 2);
    assertEquals(2, trie.count());
    trie.assoc("apple", 3);
    assertEquals(2, trie.count());
    trie.dissoc("app");
    assertEquals(1, trie.count());
  }

  @Test
  public void testIterator() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    trie.assoc("apple", 1);
    trie.assoc("app", 2);
    trie.assoc("banana", 3);
    trie.assoc("", 4);

    Iterator<String> it = trie.iterator();
    List<String> words = new ArrayList<>();
    while (it.hasNext()) {
      words.add(it.next());
    }
    Collections.sort(words);
    assertArrayEquals(new String[] {"", "app", "apple", "banana"}, words.toArray());
  }

  @Test
  public void testEquality() {
    Trie.Mutable<Integer> trie1 = new Trie.Mutable<>();
    trie1.assoc("apple", 1);
    trie1.assoc("app", 2);

    Trie.Mutable<Integer> trie2 = new Trie.Mutable<>();
    trie2.assoc("app", 2);
    trie2.assoc("apple", 1);

    assertTrue(trie1.equality(trie2));

    trie2.assoc("banana", 3);
    assertFalse(trie1.equality(trie2));

    Trie.Mutable<Integer> trie3 = new Trie.Mutable<>();
    trie3.conj("apple");
    Trie.Mutable<Integer> trie4 = new Trie.Mutable<>();
    trie4.conj("apple");
    assertTrue(trie3.equality(trie4));
  }

  @Test
  public void testConj() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    trie.conj("apple");
    assertTrue(trie.has("apple"));
    assertNull(trie.find("apple").getValue());
  }

  @Test
  public void testEdgeCases() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    trie.assoc("", 1);
    assertTrue(trie.has(""));
    assertEquals(1, (int) trie.find("").getValue());

    trie.dissoc("");
    assertFalse(trie.has(""));
  }

  @Test
  public void testLookup() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    trie.assoc("apple", 1);
    assertEquals(1, (int) trie.lookup("apple"));
    assertNull(trie.lookup("app"));
    assertEquals(42, (int) trie.lookup("app", 42));
  }

  @Test
  public void testKeysAndVals() {
    Trie.Mutable<Integer> trie = new Trie.Mutable<>();
    trie.assoc("apple", 1);
    trie.assoc("app", 2);
    trie.assoc("banana", 3);

    List<String> keys = new ArrayList<>();
    trie.keys().forEachRemaining(keys::add);
    Collections.sort(keys);
    assertArrayEquals(new String[] {"app", "apple", "banana"}, keys.toArray());

    List<Integer> vals = new ArrayList<>();
    trie.vals().forEachRemaining(vals::add);
    Collections.sort(vals);
    assertArrayEquals(new Integer[] {1, 2, 3}, vals.toArray());
  }
}
