package hara.kernel.base;

import hara.lang.data.Keyword;
import hara.lang.data.Map;
import hara.lang.data.Symbol;
import hara.lang.data.Vector;
import hara.lang.protocol.IMetadata;
import org.junit.Test;

import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

public class EnvTest {

  @Test
  public void testLoadStatic() {
    Map<Symbol, Var> staticVars = Env.loadStatic();
    assertNotNull(staticVars);
    assertTrue(staticVars.count() > 0);

    // Check for a few important symbols
    assertNotNull(staticVars.find(Symbol.create("+")));
    assertNotNull(staticVars.find(Symbol.create("map")));
    assertNotNull(staticVars.find(Symbol.create("reduce")));
  }

  @Test
  public void testFlagArgLists() {
    hara.lang.data.Map<Symbol, Var> fns = Env.loadStatic();
    Var flagVar = fns.lookup(Symbol.create("flag"));
    assertNotNull("flag var should exist", flagVar);

    IMetadata meta = flagVar.meta();
    assertNotNull("metadata should not be null", meta);
    assertTrue("metadata should be a map", meta instanceof hara.lang.data.Map);

    hara.lang.data.Map metaMap = (hara.lang.data.Map) meta;
    Object arglists = metaMap.lookup(Keyword.create("arglists"));
    assertNotNull("arglists should be in metadata", arglists);

    // Check structure
    assertTrue("arglists should be a Vector", arglists instanceof Vector);
    Vector argsVec = (Vector) arglists;

    // We expect signatures for flag() and flag(val)
    // With -parameters flag in pom.xml, the parameter name should be "val"

    boolean foundEmpty = false;
    boolean foundVal = false;

    for (Object o : argsVec) {
      Vector sig = (Vector) o;
      if (sig.count() == 0) {
        foundEmpty = true;
      } else if (sig.count() == 1) {
        Symbol s = (Symbol) sig.nth(0);
        if (s.getName().equals("val")) {
          foundVal = true;
        }
      }
    }

    assertTrue("Should find empty signature", foundEmpty);
    assertTrue("Should find (val) signature", foundVal);
  }
}
