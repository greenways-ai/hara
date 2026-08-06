package hara.kernel.base;

import hara.kernel.NativeMode;
import hara.lang.base.Ex;
import org.junit.Test;

import java.net.MalformedURLException;
import java.net.URL;

import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

public class RTNativeModeTest {

  @Test
  public void testPathAddRejectedInNativeMode() throws MalformedURLException {
    String previous = System.getProperty(NativeMode.PROPERTY);
    System.setProperty(NativeMode.PROPERTY, "true");
    try {
      RT.Instance<Object> rt = new RT.Instance<>(null, "test");
      rt.pathAdd(new URL("file:///tmp/"));
      fail("Expected native mode to reject classpath mutation");
    } catch (Ex.Unsupported e) {
      assertTrue(e.getMessage().contains("classpath mutation"));
    } finally {
      restore(previous);
    }
  }

  @Test
  public void testPathAddStringsRejectedInNativeMode() {
    String previous = System.getProperty(NativeMode.PROPERTY);
    System.setProperty(NativeMode.PROPERTY, "true");
    try {
      RT.Instance<Object> rt = new RT.Instance<>(null, "test");
      rt.pathAdd(new String[] {"file:///tmp/"});
      fail("Expected native mode to reject classpath mutation");
    } catch (Ex.Unsupported e) {
      assertTrue(e.getMessage().contains("classpath mutation"));
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
