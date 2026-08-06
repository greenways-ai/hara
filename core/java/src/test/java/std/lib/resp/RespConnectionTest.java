package std.lib.resp;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.List;
import org.junit.Test;

public class RespConnectionTest {
  @Test
  public void roundTripsAllSupportedResp2Values() throws Exception {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    RespConnection.Encoder encoder = new RespConnection.Encoder(output);
    encoder.write(List.of("héllo", 42L, true, List.of("nested")));
    encoder.flush();

    Object decoded =
        new RespConnection.Parser(new ByteArrayInputStream(output.toByteArray())).parse();
    List<?> values = (List<?>) decoded;
    assertEquals("héllo", text(values.get(0)));
    assertEquals(42L, values.get(1));
    assertEquals("true", text(values.get(2)));
    assertEquals("nested", text(((List<?>) values.get(3)).get(0)));
  }

  @Test
  public void parsesNullBulkAndNullArray() throws Exception {
    assertNull(parse("$-1\r\n"));
    assertNull(parse("*-1\r\n"));
  }

  @Test
  public void preservesBinaryBulkStrings() throws Exception {
    Object value =
        new RespConnection.Parser(
                new ByteArrayInputStream(new byte[] {'$', '3', '\r', '\n', 0, (byte) 255, 1, '\r', '\n'}))
            .parse();
    assertArrayEquals(new byte[] {0, (byte) 255, 1}, (byte[]) value);
  }

  @Test
  public void representsServerErrorsWithoutLosingTheMessage() throws Exception {
    Object value = parse("-ERR broken\r\n");
    assertTrue(value instanceof RespConnection.Parser.ServerError);
    assertEquals("ERR broken", ((Throwable) value).getMessage());
  }

  @Test
  public void rejectsTruncatedAndOversizedValues() {
    assertThrows(IOException.class, () -> parse("$4\r\nab"));
    assertThrows(
        IOException.class,
        () ->
            new RespConnection.Parser(
                    new ByteArrayInputStream("$4\r\nabcd\r\n".getBytes(StandardCharsets.US_ASCII)),
                    3,
                    10)
                .parse());
    assertThrows(
        IOException.class,
        () ->
            new RespConnection.Parser(
                    new ByteArrayInputStream("*2\r\n$1\r\na\r\n$1\r\nb\r\n".getBytes(StandardCharsets.US_ASCII)),
                    10,
                    1)
                .parse());
  }

  @Test
  public void rejectsMalformedLinesAndSimpleStrings() {
    assertThrows(IOException.class, () -> parse(":nope\r\n"));
    assertThrows(IOException.class, () -> parse("+unterminated"));
    assertThrows(
        IllegalArgumentException.class,
        () -> new RespConnection.Encoder(new ByteArrayOutputStream()).writeString("bad\r\nline"));
  }

  private static Object parse(String value) throws IOException {
    return new RespConnection.Parser(
            new ByteArrayInputStream(value.getBytes(StandardCharsets.UTF_8)))
        .parse();
  }

  private static String text(Object value) {
    return new String((byte[]) value, StandardCharsets.UTF_8);
  }
}
