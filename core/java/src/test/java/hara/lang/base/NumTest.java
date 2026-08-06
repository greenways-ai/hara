package hara.lang.base;

import hara.lang.base.primitive.Num;
import java.math.BigDecimal;
import java.math.BigInteger;
import org.junit.Test;

import static org.junit.Assert.*;

public class NumTest {

  @Test
  public void testAdd() {
    assertEquals(5L, Num.add(2, 3));
    assertEquals(5.0, (double) Num.add(2.0, 3.0), 0.0);
    assertEquals(5L, Num.add(2L, 3L));
    assertEquals(5L, Num.add((Object) 2L, (Object) 3L));
    assertEquals(5.0, Num.add((Object) 2.0, (Object) 3.0).doubleValue(), 0.0);
  }

  @Test
  public void testMinus() {
    assertEquals(-1L, Num.minus(2, 3));
    assertEquals(-1.0, (double) Num.minus(2.0, 3.0), 0.0);
    assertEquals(-1L, Num.minus(2L, 3L));
  }

  @Test
  public void testMultiply() {
    assertEquals(6L, Num.multiply(2, 3));
    assertEquals(6.0, (double) Num.multiply(2.0, 3.0), 0.0);
    assertEquals(6L, Num.multiply(2L, 3L));
  }

  @Test
  public void testDivide() {
    assertEquals(2.0, (double) Num.divide(6.0, 3.0), 0.0);
    assertEquals(2L, Num.divide(5L, 2L));
    assertEquals(BigInteger.valueOf(2), Num.divide(BigInteger.valueOf(5), BigInteger.valueOf(2)));
    assertEquals(
        new BigDecimal("0.3333333333333333333333333333333333"),
        Num.divide(BigDecimal.ONE, BigDecimal.valueOf(3)));
  }

  @Test
  public void testCanonicalDecimalsAndPromotion() {
    BigDecimal first = Num.canonicalDecimal(new BigDecimal("1.0"));
    BigDecimal second = Num.canonicalDecimal(new BigDecimal("1.00"));
    assertEquals(first, second);
    assertEquals(first.hashCode(), second.hashCode());
    assertEquals(
        BigInteger.valueOf(Long.MAX_VALUE).add(BigInteger.ONE), Num.addP(Long.MAX_VALUE, 1L));
  }

  @Test
  public void testLanguageNumericHashNormalizesEqualRepresentations() {
    assertEquals(G.hashRapid(1L), G.hashRapid(BigInteger.ONE));
    assertEquals(G.hashRapid(1L), G.hashRapid(new BigDecimal("1.00")));
    assertEquals(G.hashRapid(0.0d), G.hashRapid(-0.0d));
    assertTrue(Num.eq(-0.0d, 0.0d));
  }

  @Test(expected = IllegalArgumentException.class)
  public void testDecimalAndFloatingPointRequireExplicitConversion() {
    Num.add(new BigDecimal("1.5"), 2.0);
  }

  @Test(expected = IllegalArgumentException.class)
  public void testFloatingPointAndDecimalRequireExplicitConversion() {
    Num.multiply(2.0, new BigDecimal("1.5"));
  }

  @Test
  public void testInc() {
    assertEquals(3L, Num.inc(2L));
    assertEquals(3.0, (double) Num.inc(2.0), 0.0);
  }

  @Test
  public void testDec() {
    assertEquals(1L, Num.dec(2L));
    assertEquals(1.0, (double) Num.dec(2.0), 0.0);
  }

  @Test
  public void testQuotient() {
    assertEquals(2L, Num.quotient(7L, 3L));
    assertEquals(2.0, (double) Num.quotient(7.0, 3.0), 0.0);
  }

  @Test
  public void testRemainder() {
    assertEquals(1L, Num.remainder(7L, 3L));
    assertEquals(1.0, (double) Num.remainder(7.0, 3.0), 0.0);
  }

  @Test
  public void testLt() {
    assertTrue(Num.lt(2, 3));
    assertTrue(Num.lt(2.0, 3.0));
    assertFalse(Num.lt(3, 2));
  }

  @Test
  public void testLte() {
    assertTrue(Num.lte(2, 3));
    assertTrue(Num.lte(2.0, 3.0));
    assertFalse(Num.lte(3, 2));
    assertTrue(Num.lte(2, 2));
  }

  @Test
  public void testGt() {
    assertFalse(Num.gt(2, 3));
    assertTrue(Num.gt(3, 2));
    assertTrue(Num.gt(3.0, 2.0));
  }

  @Test
  public void testGte() {
    assertFalse(Num.gte(2, 3));
    assertTrue(Num.gte(3, 2));
    assertTrue(Num.gte(2, 2));
  }

  @Test
  public void testEq() {
    assertTrue(Num.eq(2, 2));
    assertFalse(Num.eq(2, 3));
  }

  @Test
  public void testIsZero() {
    assertTrue(Num.isZero(0));
    assertFalse(Num.isZero(1));
  }

  @Test
  public void testIsPos() {
    assertTrue(Num.isPos(1));
    assertFalse(Num.isPos(0));
    assertFalse(Num.isPos(-1));
  }

  @Test
  public void testIsNeg() {
    assertTrue(Num.isNeg(-1));
    assertFalse(Num.isNeg(0));
    assertFalse(Num.isNeg(1));
  }
}
