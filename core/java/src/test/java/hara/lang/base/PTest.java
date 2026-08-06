package hara.lang.base;

import hara.lang.base.primitive.*;
import org.junit.Test;

import java.math.BigInteger;

import static org.junit.Assert.*;

public class PTest {

  @Test
  public void testBitOffset() {
    assertEquals(0, Bits.bitOffset(1L));
    assertEquals(1, Bits.bitOffset(2L));
    assertEquals(2, Bits.bitOffset(4L));
    assertEquals(3, Bits.bitOffset(8L));
    assertEquals(10, Bits.bitOffset(1024L));
    assertEquals(63, Bits.bitOffset(1L << 63));
  }

  @Test
  public void testLowestBit() {
    assertEquals(1L, Bits.lowestBit(1L));
    assertEquals(2L, Bits.lowestBit(2L));
    assertEquals(2L, Bits.lowestBit(6L));
    assertEquals(8L, Bits.lowestBit(24L));
    assertEquals(1, Bits.lowestBit(3));
    assertEquals(4, Bits.lowestBit(12));
  }

  @Test
  public void testHighestBit() {
    assertEquals(1L, Bits.highestBit(1L));
    assertEquals(2L, Bits.highestBit(2L));
    assertEquals(4L, Bits.highestBit(6L));
    assertEquals(16L, Bits.highestBit(24L));
    assertEquals(2, Bits.highestBit(3));
    assertEquals(8, Bits.highestBit(12));
  }

  @Test
  public void testLog2Floor() {
    assertEquals(0, Bits.log2Floor(1L));
    assertEquals(1, Bits.log2Floor(2L));
    assertEquals(2, Bits.log2Floor(4L));
    assertEquals(2, Bits.log2Floor(5L));
    assertEquals(3, Bits.log2Floor(8L));
    assertEquals(3, Bits.log2Floor(15L));
    assertEquals(10, Bits.log2Floor(1024L));
  }

  @Test
  public void testLog2Ceil() {
    assertEquals(0, Bits.log2Ceil(1L));
    assertEquals(1, Bits.log2Ceil(2L));
    assertEquals(2, Bits.log2Ceil(4L));
    assertEquals(3, Bits.log2Ceil(5L));
    assertEquals(3, Bits.log2Ceil(8L));
    assertEquals(4, Bits.log2Ceil(15L));
    assertEquals(10, Bits.log2Ceil(1024L));
  }

  @Test
  public void testMaskBelow() {
    assertEquals(0L, Bits.maskBelow(0));
    assertEquals(1L, Bits.maskBelow(1));
    assertEquals(3L, Bits.maskBelow(2));
    assertEquals(7L, Bits.maskBelow(3));
    assertEquals(Long.MAX_VALUE, Bits.maskBelow(63));
  }

  @Test
  public void testMaskAbove() {
    assertEquals(-1L, Bits.maskAbove(0));
    assertEquals(-2L, Bits.maskAbove(1));
    assertEquals(-4L, Bits.maskAbove(2));
    assertEquals(-8L, Bits.maskAbove(3));
  }

  @Test
  public void testBranchingBit() {
    assertEquals(-1, Bits.branchingBit(5L, 5L));
    assertEquals(0, Bits.branchingBit(5L, 4L));
    assertEquals(2, Bits.branchingBit(5L, 1L));
    assertEquals(3, Bits.branchingBit(0L, 8L));
  }

  @Test
  public void testIsPowerOfTwo() {
    assertTrue(Bits.isPowerOfTwo(1L));
    assertTrue(Bits.isPowerOfTwo(2L));
    assertTrue(Bits.isPowerOfTwo(4L));
    assertTrue(Bits.isPowerOfTwo(1024L));
    assertFalse(Bits.isPowerOfTwo(3L));
    assertFalse(Bits.isPowerOfTwo(6L));
    assertFalse(Bits.isPowerOfTwo(0L));
  }

  @Test
  public void testBox() {
    assertEquals(true, Box.box(true));
    assertEquals(false, Box.box(false));
    assertEquals(Byte.valueOf((byte) 5), Box.box((byte) 5));
    assertEquals(Character.valueOf('a'), Box.box('a'));
    assertEquals(Double.valueOf(3.14), Box.box(3.14));
    assertEquals(Float.valueOf(1.618f), Box.box(1.618f));
    assertEquals(Integer.valueOf(100), Box.box(100));
    assertEquals(Long.valueOf(12345L), Box.box(12345L));
    assertEquals("test", Box.box("test"));
    assertEquals(Short.valueOf((short) 25), Box.box((short) 25));
  }

  // Cast Tests
  @Test
  public void testCharCast() {
    assertEquals('a', Cast.charCast('a'));
    assertEquals('b', Cast.charCast((byte) 98));
    assertEquals('c', Cast.charCast((short) 99));
    assertEquals('d', Cast.charCast(100));
    assertEquals('e', Cast.charCast(101L));
    assertEquals('f', Cast.charCast(102.0f));
    assertEquals('g', Cast.charCast(103.0));
    assertEquals('h', Cast.charCast(Character.valueOf('h')));
  }

  @Test(expected = IllegalArgumentException.class)
  public void testCharCastOutOfRange() {
    Cast.charCast(Character.MAX_VALUE + 1);
  }

  @Test
  public void testUncheckedCharCast() {
    assertEquals((char) (Character.MAX_VALUE + 1), Cast.uncheckedCharCast(Character.MAX_VALUE + 1));
  }

  @Test
  public void testBooleanCast() {
    assertTrue(Cast.booleanCast(true));
    assertFalse(Cast.booleanCast(false));
    assertTrue(Cast.booleanCast(new Object()));
    assertFalse(Cast.booleanCast(null));
  }

  @Test
  public void testByteCast() {
    assertEquals((byte) 10, Cast.byteCast((byte) 10));
    assertEquals((byte) 20, Cast.byteCast((short) 20));
    assertEquals((byte) 30, Cast.byteCast(30));
    assertEquals((byte) 40, Cast.byteCast(40L));
    assertEquals((byte) 50, Cast.byteCast(50.0f));
    assertEquals((byte) 60, Cast.byteCast(60.0));
    assertEquals((byte) 70, Cast.byteCast(Byte.valueOf((byte) 70)));
  }

  @Test(expected = IllegalArgumentException.class)
  public void testByteCastOutOfRange() {
    Cast.byteCast(Byte.MAX_VALUE + 1);
  }

  @Test
  public void testUncheckedByteCast() {
    assertEquals((byte) (Byte.MAX_VALUE + 1), Cast.uncheckedByteCast(Byte.MAX_VALUE + 1));
  }

  @Test
  public void testShortCast() {
    assertEquals((short) 10, Cast.shortCast((byte) 10));
    assertEquals((short) 20, Cast.shortCast((short) 20));
    assertEquals((short) 30, Cast.shortCast(30));
    assertEquals((short) 40, Cast.shortCast(40L));
    assertEquals((short) 50, Cast.shortCast(50.0f));
    assertEquals((short) 60, Cast.shortCast(60.0));
    assertEquals((short) 70, Cast.shortCast(Short.valueOf((short) 70)));
  }

  @Test(expected = IllegalArgumentException.class)
  public void testShortCastOutOfRange() {
    Cast.shortCast(Short.MAX_VALUE + 1);
  }

  @Test
  public void testUncheckedShortCast() {
    assertEquals((short) (Short.MAX_VALUE + 1), Cast.uncheckedShortCast(Short.MAX_VALUE + 1));
  }

  @Test
  public void testIntCast() {
    assertEquals(10, Cast.intCast((byte) 10));
    assertEquals(20, Cast.intCast((short) 20));
    assertEquals(30, Cast.intCast(30));
    assertEquals(40, Cast.intCast(40L));
    assertEquals(50, Cast.intCast(50.0f));
    assertEquals(60, Cast.intCast(60.0));
    assertEquals(70, Cast.intCast(Integer.valueOf(70)));
    assertEquals(97, Cast.intCast('a'));
  }

  @Test(expected = IllegalArgumentException.class)
  public void testIntCastOutOfRange() {
    Cast.intCast(Integer.MAX_VALUE + 1L);
  }

  @Test
  public void testUncheckedIntCast() {
    assertEquals((int) (Integer.MAX_VALUE + 1L), Cast.uncheckedIntCast(Integer.MAX_VALUE + 1L));
  }

  @Test
  public void testLongCast() {
    assertEquals(10L, Cast.longCast((byte) 10));
    assertEquals(20L, Cast.longCast((short) 20));
    assertEquals(30L, Cast.longCast(30));
    assertEquals(40L, Cast.longCast(40L));
    assertEquals(50L, Cast.longCast(50.0f));
    assertEquals(60L, Cast.longCast(60.0));
    assertEquals(70L, Cast.longCast(Long.valueOf(70)));
  }

  @Test(expected = IllegalArgumentException.class)
  public void testLongCastOutOfRange() {
    Cast.longCast(BigInteger.valueOf(Long.MAX_VALUE).add(BigInteger.ONE));
  }

  @Test
  public void testUncheckedLongCast() {
    assertEquals(
        Long.MIN_VALUE,
        Cast.uncheckedLongCast(BigInteger.valueOf(Long.MAX_VALUE).add(BigInteger.ONE)));
  }

  @Test
  public void testFloatCast() {
    assertEquals(10.0f, Cast.floatCast((byte) 10), 0.0f);
    assertEquals(20.0f, Cast.floatCast((short) 20), 0.0f);
    assertEquals(30.0f, Cast.floatCast(30), 0.0f);
    assertEquals(40.0f, Cast.floatCast(40L), 0.0f);
    assertEquals(50.0f, Cast.floatCast(50.0f), 0.0f);
    assertEquals(60.0f, Cast.floatCast(60.0), 0.0f);
    assertEquals(70.0f, Cast.floatCast(Float.valueOf(70.0f)), 0.0f);
  }

  @Test(expected = IllegalArgumentException.class)
  public void testFloatCastOutOfRange() {
    Cast.floatCast(Double.MAX_VALUE);
  }

  @Test
  public void testUncheckedFloatCast() {
    assertEquals(Float.POSITIVE_INFINITY, Cast.uncheckedFloatCast(Double.MAX_VALUE), 0.0f);
  }

  @Test
  public void testDoubleCast() {
    assertEquals(10.0, Cast.doubleCast((byte) 10), 0.0);
    assertEquals(20.0, Cast.doubleCast((short) 20), 0.0);
    assertEquals(30.0, Cast.doubleCast(30), 0.0);
    assertEquals(40.0, Cast.doubleCast(40L), 0.0);
    assertEquals(50.0, Cast.doubleCast(50.0f), 0.0);
    assertEquals(60.0, Cast.doubleCast(60.0), 0.0);
    assertEquals(70.0, Cast.doubleCast(Double.valueOf(70.0)), 0.0);
  }

  // Complex Tests
  @Test
  public void testComplexArithmetic() {
    Complex a = new Complex(5.0, 6.0);
    Complex b = new Complex(-3.0, 4.0);

    Complex sum = a.plus(b);
    assertEquals(2.0, sum.re(), 0.001);
    assertEquals(10.0, sum.im(), 0.001);

    Complex difference = a.minus(b);
    assertEquals(8.0, difference.re(), 0.001);
    assertEquals(2.0, difference.im(), 0.001);

    Complex product = a.times(b);
    assertEquals(-39.0, product.re(), 0.001);
    assertEquals(2.0, product.im(), 0.001);

    Complex quotient = a.divides(b);
    assertEquals(0.36, quotient.re(), 0.001);
    assertEquals(-1.52, quotient.im(), 0.001);

    Complex scaled = a.scale(2.0);
    assertEquals(10.0, scaled.re(), 0.001);
    assertEquals(12.0, scaled.im(), 0.001);
  }

  @Test
  public void testComplexProperties() {
    Complex a = new Complex(3.0, 4.0);
    assertEquals(5.0, a.abs(), 0.001);
    assertEquals(0.927, a.phase(), 0.001);
  }

  @Test
  public void testComplexConjugateReciprocal() {
    Complex a = new Complex(3.0, 4.0);

    Complex conjugate = a.conjugate();
    assertEquals(3.0, conjugate.re(), 0.001);
    assertEquals(-4.0, conjugate.im(), 0.001);

    Complex reciprocal = a.reciprocal();
    assertEquals(0.12, reciprocal.re(), 0.001);
    assertEquals(-0.16, reciprocal.im(), 0.001);
  }

  @Test
  public void testComplexTrigonometric() {
    Complex a = new Complex(0.5, 0.2);

    Complex exp = a.exp();
    assertEquals(1.616, exp.re(), 0.001);
    assertEquals(0.327, exp.im(), 0.001);

    Complex sin = a.sin();
    assertEquals(0.489, sin.re(), 0.001);
    assertEquals(0.177, sin.im(), 0.001);

    Complex cos = a.cos();
    assertEquals(0.895, cos.re(), 0.001);
    assertEquals(-0.096, cos.im(), 0.001);

    Complex tan = a.tan();
    assertEquals(0.519, tan.re(), 0.001);
    assertEquals(0.253, tan.im(), 0.001);
  }

  @Test
  public void testComplexEqualsAndHashCode() {
    Complex a1 = new Complex(1.0, 2.0);
    Complex a2 = new Complex(1.0, 2.0);
    Complex b = new Complex(2.0, 3.0);

    assertTrue(a1.equals(a2));
    assertFalse(a1.equals(b));
    assertEquals(a1.hashCode(), a2.hashCode());
  }
}
