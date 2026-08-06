package hara.lang.data;

import org.junit.Test;

import static org.junit.Assert.*;

public class AtomTest {

  @Test
  public void testAtom() {
    Atom.Standard<Integer> atom = new Atom.Standard<>(10);
    assertEquals(Integer.valueOf(10), atom.deref());

    atom.reset(20);
    assertEquals(Integer.valueOf(20), atom.deref());

    assertTrue(atom.cas(20, 30));
    assertEquals(Integer.valueOf(30), atom.deref());
    assertFalse(atom.cas(20, 40));
    assertEquals(Integer.valueOf(30), atom.deref());

    atom.swap(n -> n + 10);
    assertEquals(Integer.valueOf(40), atom.deref());
  }

  @Test
  public void testAtomValidator() {
    Atom.Standard<Integer> atom = new Atom.Standard<>(10, n -> n >= 0);
    assertEquals(Integer.valueOf(10), atom.deref());

    atom.swap(n -> n + 10);
    assertEquals(Integer.valueOf(20), atom.deref());

    try {
      atom.swap(n -> -1);
      fail("Expected IllegalStateException");
    } catch (IllegalStateException e) {
      // expected
    }
  }

  @Test
  public void testAtomWatcher() {
    Atom.Standard<Integer> atom = new Atom.Standard<>(10);
    Object key = new Object();
    final Integer[] oldVal = new Integer[1];
    final Integer[] newVal = new Integer[1];
    atom.addWatch(
        key,
        e -> {
          oldVal[0] = e.oldVal();
          newVal[0] = e.newVal();
        });

    atom.swap(n -> n + 10);
    assertEquals(Integer.valueOf(10), oldVal[0]);
    assertEquals(Integer.valueOf(20), newVal[0]);

    atom.removeWatch(key);
    atom.swap(n -> n + 10);
    assertEquals(Integer.valueOf(10), oldVal[0]);
    assertEquals(Integer.valueOf(20), newVal[0]);
  }

  @Test
  public void testAtomSwapBiFunction() {
    Atom.Standard<Integer> atom = new Atom.Standard<>(10);
    atom.swap((n, a) -> n + (Integer) a, 10);
    assertEquals(Integer.valueOf(20), atom.deref());
  }

  @Test
  public void testAtomSwapBiFunctionVarArgs() {
    Atom.Standard<Integer> atom = new Atom.Standard<>(10);
    atom.swap(
        (n, args) -> {
          int sum = n;
          for (Object arg : args) {
            sum += (Integer) arg;
          }
          return sum;
        },
        10,
        20);
    assertEquals(Integer.valueOf(40), atom.deref());
  }

  @Test
  public void testAtomDisplay() {
    Atom.Standard<Integer> atom = new Atom.Standard<>(10);
    assertEquals("#atom <10>", atom.display());
  }

  @Test
  public void testBasicAtom() {
    Atom.Basic<Integer> atom = new Atom.Basic<>(10);
    assertEquals(Integer.valueOf(10), atom.deref());

    atom.reset(20);
    assertEquals(Integer.valueOf(20), atom.deref());

    assertTrue(atom.cas(20, 30));
    assertEquals(Integer.valueOf(30), atom.deref());
    assertFalse(atom.cas(20, 40));
    assertEquals(Integer.valueOf(30), atom.deref());

    atom.swap(n -> n + 10);
    assertEquals(Integer.valueOf(40), atom.deref());
  }
}
