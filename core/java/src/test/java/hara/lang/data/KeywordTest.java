package hara.lang.data;

import org.junit.Test;

import java.util.HashMap;
import java.util.Map;

import static org.junit.Assert.*;

public class KeywordTest {

  @Test
  public void testCreate() {
    Keyword k1 = Keyword.create("test");
    assertEquals("test", k1.getName());
    assertNull(k1.getNamespace());

    Keyword k2 = Keyword.create("ns/test");
    assertEquals("test", k2.getName());
    assertEquals("ns", k2.getNamespace());

    Keyword k3 = Keyword.create("ns", "test");
    assertEquals("test", k3.getName());
    assertEquals("ns", k3.getNamespace());

    assertSame(k2, k3);

    Keyword k4 = Keyword.create("test");
    assertSame(k1, k4);
  }

  @Test
  public void testCompareTo() {
    Keyword k1 = Keyword.create("a");
    Keyword k2 = Keyword.create("b");
    Keyword k3 = Keyword.create("ns/a");
    Keyword k4 = Keyword.create("ns/b");

    assertTrue(k1.compareTo(k2) < 0);
    assertTrue(k2.compareTo(k1) > 0);
    assertTrue(k1.compareTo(k1) == 0);
    assertTrue(k3.compareTo(k4) < 0);
    assertTrue(k4.compareTo(k3) > 0);
    assertTrue(k3.compareTo(k3) == 0);
    assertTrue(k2.compareTo(k3) < 0);
  }

  @Test
  public void testDisplay() {
    Keyword k1 = Keyword.create("test");
    assertEquals(":test", k1.display());

    Keyword k2 = Keyword.create("ns/test");
    assertEquals(":ns/test", k2.display());
  }

  @Test
  @SuppressWarnings("unchecked")
  public void testLookup() {
    Keyword k1 = Keyword.create("test");
    Map<Keyword, String> map = new HashMap<>();
    map.put(k1, "value");

    assertEquals("value", k1.getArg1().apply(map));

    Keyword k2 = Keyword.create("not-found");
    assertNull(k2.getArg1().apply(map));

    assertEquals("default", k2.getArg2().apply(map, "default"));

    assertNull(k1.getArg1().apply(null));
    assertEquals("default", k2.getArg2().apply(null, "default"));

    Set.Standard<Keyword> set = Set.Standard.from(null, k1);
    assertSame(k1, k1.getArg1().apply(set));
    assertNull(k2.getArg1().apply(set));
    assertEquals("default", k2.getArg2().apply(set, "default"));
  }

  @Test
  public void testInvalidInputs() {
    try {
      Keyword.create("too/many/slashes");
    } catch (IllegalArgumentException e) {
      assertEquals("Keyword name can only contain one slash.", e.getMessage());
    }

    try {
      Keyword.create("/starts-with-slash");
    } catch (IllegalArgumentException e) {
      assertEquals("Keyword name cannot start with a slash.", e.getMessage());
    }

    try {
      Keyword.create("ends-with-slash/");
    } catch (IllegalArgumentException e) {
      assertEquals("Keyword name cannot end with a slash.", e.getMessage());
    }

    try {
      Keyword.create("");
    } catch (IllegalArgumentException e) {
      assertEquals("Keyword name cannot be empty.", e.getMessage());
    }

    try {
      Keyword.create("/");
    } catch (IllegalArgumentException e) {
      assertEquals("Keyword name cannot be a single slash.", e.getMessage());
    }
  }

  @Test
  public void testEqualsAndHashCode() {
    Keyword k1 = Keyword.create("test");
    Keyword k2 = Keyword.create("test");
    Keyword k3 = Keyword.create("ns/test");
    Keyword k4 = Keyword.create("ns/test");

    assertEquals(k1, k2);
    assertEquals(k3, k4);
    assertEquals(k1.hashCode(), k2.hashCode());
    assertEquals(k3.hashCode(), k4.hashCode());
  }
}
