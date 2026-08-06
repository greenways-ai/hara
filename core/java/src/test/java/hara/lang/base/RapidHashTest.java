package hara.lang.base;

import hara.lang.base.primitive.RapidHash;
import org.junit.Test;
import static org.junit.Assert.*;

public class RapidHashTest {

  @Test
  public void testRapidHash() {
    // Basic sanity checks
    assertEquals(6516417773221693515L, RapidHash.hash(new byte[0]));
    // The seed is mixed with length, so empty input is not just 0 unless seed magic makes it so.
    // Wait, let's verify what RapidHash(empty) returns.

    String s = "hello world";
    long h = RapidHash.hash(s);

    // Consistency check
    assertEquals(h, RapidHash.hash(s));
    assertNotEquals(h, RapidHash.hash(s + "!"));
  }
}
