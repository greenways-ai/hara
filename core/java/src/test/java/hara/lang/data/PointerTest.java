package hara.lang.data;

import hara.lang.protocol.Constant;
import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertSame;

public class PointerTest {

  @Test
  public void testPointer() {
    Pointer p = new Pointer(null, "ns", "name");
    assertEquals("ns", p.getNamespace());
    assertEquals("name", p.getName());
    assertEquals("#'ns/name", p.display());
    assertEquals(Constant.ObjType.POINTER, p.getObjType());
  }

  @Test
  public void testWithMeta() {
    Pointer p = new Pointer(null, "ns", "name");
    Pointer p2 = p.withMeta(null);
    assertSame(p, p2);
  }
}
