package hara.kernel.command;

import hara.kernel.Foundation;
import org.junit.Test;

import java.util.List;

import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertTrue;

public class OsTest {

  @Test
  public void testOsHelp() {
    Foundation f = new Foundation();
    // Test empty args defaults to help
    List result = (List) f.call("OS");
    assertNotNull(result);
    assertTrue(result.contains("PWD"));
    assertTrue(result.contains("LS"));
  }

  @Test
  public void testOsPwd() {
    Foundation f = new Foundation();
    Object result = f.call("OS", "PWD");
    assertNotNull(result);
    // Should contain the current working directory
    assertTrue(result.toString().length() > 0);
  }
}
