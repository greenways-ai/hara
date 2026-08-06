package hara.kernel.command;

import hara.kernel.Foundation;
import hara.kernel.NativeMode;
import hara.lang.base.Ex;
import org.junit.Test;

import java.util.Arrays;

import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

public class CoreNativeModeTest {

  @Test
  public void testCompileRejectedInNativeMode() {
    String previous = System.getProperty(NativeMode.PROPERTY);
    System.setProperty(NativeMode.PROPERTY, "true");
    try {
      Core.runCompile(new Foundation(), Arrays.asList("(+ 1 2)"));
      fail("Expected native mode to reject COMPILE");
    } catch (Ex.Unsupported e) {
      assertTrue(e.getMessage().contains("runtime compilation"));
    } finally {
      restore(previous);
    }
  }

  private static void restore(String previous) {
    if (previous == null) {
      System.clearProperty(NativeMode.PROPERTY);
    } else {
      System.setProperty(NativeMode.PROPERTY, previous);
    }
  }
}
