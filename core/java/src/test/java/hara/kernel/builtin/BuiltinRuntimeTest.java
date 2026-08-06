package hara.kernel.builtin;

import hara.kernel.base.RT;
import hara.kernel.base.Var;
import hara.lang.data.Symbol;
import org.junit.Test;
import static org.junit.Assert.*;

public class BuiltinRuntimeTest {

  @Test
  public void testVarOperations() {
    RT.Instance rt = new RT.Instance(null, "test");
    Symbol sym = Symbol.create("test.ns", "v");

    // Ensure namespace exists
    BuiltinNamespace.nsCreate(rt, Symbol.create("test.ns"));

    Var v = BuiltinRuntime.varCreate(rt, sym);
    assertNotNull(v);

    BuiltinRuntime.varSet(rt, sym, "val");
    assertEquals("val", BuiltinRuntime.varGet(rt, sym).deref());

    assertNotNull(BuiltinRuntime.varList(rt));
  }

  @Test
  public void testEval() {
    RT.Instance rt = new RT.Instance(null, "test");
    // Simple eval test if possible without complex setup
    // rt.eval(null) -> null usually
    assertNull(BuiltinRuntime.eval(rt, null));
  }

  @Test
  public void testSysPath() {
    RT.Instance rt = new RT.Instance(null, "test");
    assertNotNull(BuiltinRuntime.sysPath(rt));
  }

  @Test
  public void testSysGlobals() {
    RT.Instance rt = new RT.Instance(null, "test");
    assertNotNull(BuiltinRuntime.sysGlobals(rt));
  }

  @Test
  public void testSysLoader() {
    RT.Instance rt = new RT.Instance(null, "test");
    assertNotNull(BuiltinRuntime.sysloader(rt));
  }

  @Test
  public void testSysRoot() {
    RT.Instance rt = new RT.Instance(null, "test");
    // sysRoot returns _root, which we passed as null
    assertNull(BuiltinRuntime.sysRoot(rt));
  }

  @Test
  public void testSysCache() {
    RT.Instance rt = new RT.Instance(null, "test");
    assertNotNull(BuiltinRuntime.sysCache(rt));

    BuiltinRuntime.sysCacheAdd(rt, "Foo", String.class);
    // Verify add?
  }
}
