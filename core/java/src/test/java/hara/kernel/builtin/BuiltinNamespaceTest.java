package hara.kernel.builtin;

import hara.kernel.base.Namespace;
import hara.lang.data.Symbol;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Collections;

public class BuiltinNamespaceTest {

  @Test
  public void testNsName() {
    Namespace ns = new Namespace(Symbol.create("test.ns"));
    assertEquals(Symbol.create("test.ns"), BuiltinNamespace.nsName(ns));
  }

  @Test
  public void testNsMap() {
    Namespace ns = new Namespace(Symbol.create("test.ns"));
    assertNotNull(BuiltinNamespace.nsMap(ns));
  }

  @Test
  public void testNsAliases() {
    Namespace ns = new Namespace(Symbol.create("test.ns"));
    assertNotNull(BuiltinNamespace.nsAliases(ns));
  }

  @Test
  public void testNsImports() {
    Namespace ns = new Namespace(Symbol.create("test.ns"));
    assertNotNull(BuiltinNamespace.nsImports(ns));
  }
}
