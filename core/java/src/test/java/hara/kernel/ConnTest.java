package hara.kernel;

import org.junit.Test;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.util.Arrays;
import java.util.List;
import static org.junit.Assert.*;

public class ConnTest {

  @Test
  public void testEncoder() throws Exception {
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    Conn.Encoder encoder = new Conn.Encoder(out);

    encoder.writeString("OK");
    assertEquals("+OK\r\n", out.toString());
    out.reset();

    encoder.write(123L);
    assertEquals(":123\r\n", out.toString());
    out.reset();

    encoder.write("hello".getBytes());
    assertEquals("$5\r\nhello\r\n", out.toString());
    out.reset();

    encoder.write(new RuntimeException("Error"));
    assertEquals("-Error\r\n", out.toString());
    out.reset();

    encoder.write(Arrays.asList("a", 1L));
    assertEquals("*2\r\n$1\r\na\r\n:1\r\n", out.toString());
  }

  @Test
  public void testParser() throws Exception {
    String input = "+OK\r\n:123\r\n$5\r\nhello\r\n-Error\r\n*2\r\n$1\r\na\r\n:1\r\n";
    ByteArrayInputStream in = new ByteArrayInputStream(input.getBytes());
    Conn.Parser parser = new Conn.Parser(in);

    assertArrayEquals("OK".getBytes(), (byte[]) parser.parse());
    assertEquals(123L, parser.parse());
    assertArrayEquals("hello".getBytes(), (byte[]) parser.parse());

    Object err = parser.parse();
    assertTrue(err instanceof Conn.Parser.ServerError);
    assertEquals("Error", ((Throwable) err).getMessage());

    List<?> list = (List<?>) parser.parse();
    assertEquals(2, list.size());
    assertArrayEquals("a".getBytes(), (byte[]) list.get(0));
    assertEquals(1L, list.get(1));
  }

  @Test
  public void testParserNulls() throws Exception {
    String input = "$-1\r\n*-1\r\n";
    ByteArrayInputStream in = new ByteArrayInputStream(input.getBytes());
    Conn.Parser parser = new Conn.Parser(in);

    assertNull(parser.parse()); // Bulk string null
    assertNull(parser.parse()); // Array null
  }

  @Test(expected = Conn.Parser.ProtocolException.class)
  public void testProtocolException() throws Exception {
    String input = "INVALID\r\n";
    ByteArrayInputStream in = new ByteArrayInputStream(input.getBytes());
    Conn.Parser parser = new Conn.Parser(in);
    parser.parse();
  }

  @Test
  public void testPipeline() throws java.io.IOException {
    ByteArrayOutputStream out = new ByteArrayOutputStream();
    ByteArrayInputStream in = new ByteArrayInputStream("+OK\r\n".getBytes());

    java.net.Socket socket =
        new java.net.Socket() {
          @Override
          public java.io.InputStream getInputStream() {
            return in;
          }

          @Override
          public java.io.OutputStream getOutputStream() {
            return out;
          }
        };

    Conn conn = new Conn(socket);
    Conn.Pipeline pipeline = conn.pipeline();

    pipeline.call("PING");
    assertEquals(1, pipeline.counter());

    // Check output
    assertEquals("*1\r\n$4\r\nPING\r\n", out.toString());

    // Read response
    List<Object> results = pipeline.read();
    assertEquals(1, results.size());
    assertArrayEquals("OK".getBytes(), (byte[]) results.get(0));
  }
}
