package hara.kernel.command;

import hara.kernel.Foundation;
import hara.lang.base.Ex;
import org.junit.Test;

import java.util.Arrays;

public class MavenTest {

  @Test(expected = Ex.Runtime.class)
  public void testLoadNoSession() {
    Foundation f = new Foundation();
    // Should throw because session "unknown" does not exist
    Maven.load(f, Arrays.asList("unknown", "lib"));
  }
}
