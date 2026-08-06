package hara.lang.base;

import hara.lang.protocol.IEquality;
import org.junit.Test;

import java.util.Arrays;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

public class EqTest {

  @Test
  public void testEqNumbers() {
    assertTrue(Eq.eq(1, 1));
    assertFalse(Eq.eq(1, 2));
    assertTrue(Eq.eq(1.0, 1.0));
    assertFalse(Eq.eq(1.0, 1.1));
  }

  @Test
  public void testEqStrings() {
    assertTrue(Eq.eq("hello", "hello"));
    assertFalse(Eq.eq("hello", "world"));
  }

  @Test
  public void testEqLists() {
    assertTrue(Eq.eq(Arrays.asList(1, 2, 3), Arrays.asList(1, 2, 3)));
    assertFalse(Eq.eq(Arrays.asList(1, 2, 3), Arrays.asList(3, 2, 1)));
  }

  @Test
  public void testEqNull() {
    assertTrue(Eq.eq(null, null));
    assertFalse(Eq.eq(1, null));
    assertFalse(Eq.eq(null, 1));
    IEquality nilLike = other -> other == null;
    assertTrue(Eq.eq(nilLike, null));
    assertTrue(Eq.eq(null, nilLike));
  }
}
