package hara.lang.data;

import org.junit.Test;

import static org.junit.Assert.*;

public class SymbolTest {

  @Test
  public void testSymbolCreation() {
    Symbol s1 = Symbol.create("ns", "name");
    assertEquals("ns", s1.getNamespace());
    assertEquals("name", s1.getName());
    assertEquals("ns/name", s1.pathString());
  }

  @Test
  public void testSymbolCaching() {
    Symbol s1 = Symbol.create("ns", "name");
    Symbol s2 = Symbol.create("ns/name");
    assertSame(s1, s2);
  }

  @Test
  public void testSymbolNoNamespace() {
    Symbol s1 = Symbol.create("name");
    assertNull(s1.getNamespace());
    assertEquals("name", s1.getName());
    assertEquals("name", s1.pathString());
  }

  @Test
  public void testWithMeta() {
    Symbol s1 = Symbol.create("ns", "name");
    Symbol s2 = s1.withMeta(null);
    assertSame(s1, s2);

    Symbol s3 = s1.withMeta(Map.Standard.EMPTY);
    assertNotSame(s1, s3);
    assertEquals(s1, s3);
    assertEquals(s1.hashCode(), s3.hashCode());
  }

  @Test
  public void testDisplay() {
    Symbol s1 = Symbol.create("ns/name");
    assertEquals("ns/name", s1.display());
  }
}
