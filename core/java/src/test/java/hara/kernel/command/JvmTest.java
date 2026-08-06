package hara.kernel.command;

import hara.kernel.Foundation;
import org.junit.Test;

import java.util.Arrays;
import java.util.List;

import static org.junit.Assert.*;

public class JvmTest {

  @Test
  public void testJvmHelp() {
    Foundation f = new Foundation();
    List<Object> args = Arrays.asList("JVM", "HELP");
    List result = (List) f.call(args.toArray());

    assertNotNull(result);
    assertTrue(result.contains("VERSION"));
    assertTrue(result.contains("PROPS"));
    // "HELP" should be included because we explicitly list keys in Foundation logic
    assertTrue(result.contains("HELP"));
  }

  @Test
  public void testJvmVersion() {
    Foundation f = new Foundation();
    // JVM VERSION
    Object result = f.call("JVM", "VERSION");
    assertEquals(System.getProperty("java.version"), result);
  }
}
