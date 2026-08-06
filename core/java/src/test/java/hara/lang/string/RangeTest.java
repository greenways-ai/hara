package hara.lang.string;

import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Comparator;

public class RangeTest {

  @Test
  public void testBetween() {
    Range<Integer> range = Range.between(1, 10);
    assertTrue(range.contains(1));
    assertTrue(range.contains(10));
    assertTrue(range.contains(5));
    assertFalse(range.contains(0));
    assertFalse(range.contains(11));

    assertEquals(Integer.valueOf(1), range.getMinimum());
    assertEquals(Integer.valueOf(10), range.getMaximum());
  }

  @Test
  public void testIs() {
    Range<Integer> range = Range.is(5);
    assertTrue(range.contains(5));
    assertFalse(range.contains(4));
    assertFalse(range.contains(6));
  }

  @Test
  public void testWithComparator() {
    Comparator<String> lengthComp = (s1, s2) -> Integer.compare(s1.length(), s2.length());
    Range<String> range = Range.between("a", "ccc", lengthComp);

    assertTrue(range.contains("bb"));
    assertFalse(range.contains("dddd"));
  }

  @Test
  public void testIntersection() {
    Range<Integer> r1 = Range.between(1, 10);
    Range<Integer> r2 = Range.between(5, 15);

    Range<Integer> intersection = r1.intersectionWith(r2);
    assertEquals(Integer.valueOf(5), intersection.getMinimum());
    assertEquals(Integer.valueOf(10), intersection.getMaximum());
  }

  @Test
  public void testFit() {
    Range<Integer> range = Range.between(1, 10);
    assertEquals(Integer.valueOf(1), range.fit(0));
    assertEquals(Integer.valueOf(10), range.fit(11));
    assertEquals(Integer.valueOf(5), range.fit(5));
  }

  @Test
  public void testContainsRange() {
    Range<Integer> r1 = Range.between(1, 10);
    Range<Integer> r2 = Range.between(2, 8);
    assertTrue(r1.containsRange(r2));
    assertFalse(r2.containsRange(r1));
  }

  @Test
  public void testIsAfterBefore() {
    Range<Integer> range = Range.between(1, 10);
    assertTrue(range.isAfter(0));
    assertTrue(range.isBefore(11));
    assertFalse(range.isAfter(5));
  }
}
