package hara.kernel.builtin;

import org.junit.Test;
import static org.junit.Assert.*;

public class BuiltinOpsTest {

  @Test
  public void testAdd() {
    assertEquals(3, BuiltinOps.add(1, 2).intValue());
  }

  @Test
  public void testBitOps() {
    assertEquals(1, BuiltinOps.bitAnd(3, 1).intValue());
    assertEquals(3, BuiltinOps.bitOr(1, 2).intValue());
  }

  @Test
  public void testDecInc() {
    assertEquals(0, BuiltinOps.dec(1).intValue());
    assertEquals(2, BuiltinOps.inc(1).intValue());
  }

  @Test
  public void testMath() {
    assertEquals(2.0, BuiltinOps.ceil(1.1).doubleValue(), 0.001);
    assertEquals(1.0, BuiltinOps.floor(1.9).doubleValue(), 0.001);
    assertEquals(2.0, BuiltinOps.round(1.6).doubleValue(), 0.001);
    assertEquals(1, BuiltinOps.abs(-1).intValue());
  }

  @Test
  public void testDivide() {
    assertEquals(2, BuiltinOps.divide(4, 2).intValue());
  }

  @Test
  public void testComparison() {
    assertTrue(BuiltinOps.equals(1, 1));
    assertFalse(BuiltinOps.equals(1, 2));

    assertTrue(BuiltinOps.equivalent(1, 1));

    assertTrue(BuiltinOps.gt(2, 1));
    assertTrue(BuiltinOps.gte(2, 2));

    assertTrue(BuiltinOps.identical("a", "a")); // String pool

    assertTrue(BuiltinOps.lt(1, 2));
    assertTrue(BuiltinOps.lte(2, 2));

    assertTrue(BuiltinOps.notEquivalent(1, 2));
  }

  @Test
  public void testChecks() {
    assertTrue(BuiltinOps.isNeg(-1));
    assertTrue(BuiltinOps.isPos(1));
    assertTrue(BuiltinOps.isZero(0));
  }

  @Test
  public void testMinusMultiply() {
    assertEquals(1, BuiltinOps.minus(3, 2).intValue());
    assertEquals(6, BuiltinOps.multiply(2, 3).intValue());
  }

  @Test
  public void testQuotRemMod() {
    assertEquals(2, BuiltinOps.quot(5, 2).intValue());
    assertEquals(1, BuiltinOps.rem(5, 2).intValue());
    assertEquals(1, BuiltinOps.mod(5, 2).intValue());
  }

  @Test
  public void testMinMax() {
    assertEquals(1, BuiltinOps.min(1, 2));
    assertEquals(2, BuiltinOps.max(1, 2));
  }
}
