package hara.lang.base.primitive;

import org.junit.Test;
import static org.junit.Assert.*;
import java.nio.charset.StandardCharsets;

public class RapidHashTest {

  @Test
  public void testHelloWorld() {
    byte[] data = "hello world".getBytes(StandardCharsets.UTF_8);
    long expected = -948262298241389037L;
    long actual = RapidHash.hash(data);
    assertEquals(expected, actual);
  }

  @Test
  public void testEmpty() {
    byte[] data = new byte[0];
    long expected = 6516417773221693515L;
    long actual = RapidHash.hash(data);
    assertEquals(expected, actual);
  }

  @Test
  public void testShort() {
    byte[] data = "abc".getBytes(StandardCharsets.UTF_8);
    long expected = 236166369188498817L;
    long actual = RapidHash.hash(data);
    assertEquals(expected, actual);
  }

  @Test
  public void testLarge() {
    byte[] data = new byte[100];
    for (int i = 0; i < 100; i++) data[i] = (byte) i;
    long expected = -8961987896191723928L;
    long actual = RapidHash.hash(data);
    assertEquals(expected, actual);
  }
}
