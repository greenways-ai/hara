package hara.kernel.base;

import hara.kernel.builtin.BuiltinOps;
import org.junit.Test;
import static org.junit.Assert.*;

/**
 * Tests for numeric operations in {@link Builtin.Ops}. These tests target previously uncovered
 * arithmetic and comparison methods.
 */
public class BuiltinOpsTest {

  @Test
  public void testMinMax() {
    assertEquals(1L, BuiltinOps.min(1L, 2L));
    assertEquals(2L, BuiltinOps.max(1L, 2L));
    assertEquals(1.0, BuiltinOps.min(1.0, 2.0));
    assertEquals(2.0, BuiltinOps.max(1.0, 2.0));
  }

  @Test
  public void testRounding() {
    assertEquals(1.0, ((Number) BuiltinOps.floor(1.5)).doubleValue(), 0.001);
    assertEquals(2.0, ((Number) BuiltinOps.ceil(1.5)).doubleValue(), 0.001);
    assertEquals(2L, ((Number) BuiltinOps.round(1.5)).longValue());
    assertEquals(1L, ((Number) BuiltinOps.round(1.4)).longValue());
  }

  @Test
  public void testAbs() {
    assertEquals(1.0, ((Number) BuiltinOps.abs(-1.0)).doubleValue(), 0.001);
    assertEquals(1.0, ((Number) BuiltinOps.abs(1.0)).doubleValue(), 0.001);
    assertEquals(1L, ((Number) BuiltinOps.abs(-1L)).longValue());
    assertEquals(1L, ((Number) BuiltinOps.abs(1L)).longValue());
  }

  @Test
  public void testQuotRemMod() {
    assertEquals(3L, ((Number) BuiltinOps.quot(10L, 3L)).longValue());
    assertEquals(1L, ((Number) BuiltinOps.rem(10L, 3L)).longValue());
    assertEquals(1L, ((Number) BuiltinOps.mod(10L, 3L)).longValue());
  }

  @Test
  public void testIncDecDouble() {
    assertEquals(2.0, ((Number) BuiltinOps.inc(1.0)).doubleValue(), 0.001);
    assertEquals(0.0, ((Number) BuiltinOps.dec(1.0)).doubleValue(), 0.001);
  }
}
