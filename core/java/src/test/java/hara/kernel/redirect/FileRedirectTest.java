package hara.kernel.redirect;

import hara.lang.data.Vector;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.Arrays;
import java.util.List;

import static org.junit.Assert.assertEquals;

public class FileRedirectTest {

  private static final String TEST_IN_LOG = "test_in.log";
  private static final String TEST_OUT_LOG = "test_out.log";

  @Before
  @After
  public void deleteLogFiles() throws IOException {
    Files.deleteIfExists(Paths.get(TEST_IN_LOG));
    Files.deleteIfExists(Paths.get(TEST_OUT_LOG));
  }

  @Test
  public void testFileRedirect() throws IOException {
    FileRedirect redirect = new FileRedirect(TEST_IN_LOG, TEST_OUT_LOG);

    // Test read
    List<byte[]> command = Arrays.asList("ECHO".getBytes(), "hello".getBytes());
    redirect.read(command);

    // Test write
    Vector.Standard<String> response =
        ((Vector.Mutable<String>) Vector.Mutable.<String>empty(null).conj("hello")).toPersistent();
    redirect.write(response);

    // Test write with byte array
    byte[] byteResponse = new byte[] {1, 2, 3};
    redirect.write(byteResponse);

    redirect.close();

    List<String> inLines = Files.readAllLines(Paths.get(TEST_IN_LOG));
    assertEquals(1, inLines.size());
    assertEquals("COMMAND,[\"4543484f\" \"68656c6c6f\"]", inLines.get(0));

    List<String> outLines = Files.readAllLines(Paths.get(TEST_OUT_LOG));
    assertEquals(2, outLines.size());
    assertEquals("RESPONSE,[\"hello\"]", outLines.get(0));
    assertEquals("RESPONSE,010203", outLines.get(1));
  }
}
