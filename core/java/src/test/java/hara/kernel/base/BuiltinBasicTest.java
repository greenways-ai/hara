package hara.kernel.base;

import hara.kernel.builtin.BuiltinOps;
import hara.kernel.builtin.BuiltinStruct;
import hara.kernel.builtin.BuiltinLambda;
import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotNull;

/** Simple tests for miscellaneous Builtin methods. */
public class BuiltinBasicTest {

  @Test
  public void testAddNumbers() {
    Number result = BuiltinOps.add(10, 5);
    assertEquals(15L, result.longValue());
  }

  @Test
  public void testMultiplyNumbers() {
    Number result = BuiltinOps.multiply(6, 7);
    assertEquals(42L, result.longValue());
  }

  @Test
  public void testHashMap() {
    Object result = BuiltinStruct.hashMap(java.util.Arrays.asList("key", "value"));
    assertNotNull(result);
  }

  @Test
  public void testVector() {
    Object result = BuiltinStruct.vector(java.util.Arrays.asList(1, 2, 3));
    assertNotNull(result);
  }

  @Test
  public void testIdentity() {
    String input = "test";
    Object result = BuiltinLambda.identity(input);
    assertEquals(input, result);
  }
}
