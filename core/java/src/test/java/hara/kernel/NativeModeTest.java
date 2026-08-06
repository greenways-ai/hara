package hara.kernel;

import org.junit.Test;

import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

public class NativeModeTest {

  @Test
  public void testFoundationSkipsMavenInNativeMode() {
    String previous = System.getProperty(NativeMode.PROPERTY);
    System.setProperty(NativeMode.PROPERTY, "true");
    try {
      Foundation foundation = new Foundation();
      assertFalse(foundation.REGISTRY.containsKey("MAVEN"));
    } finally {
      restore(previous);
    }
  }

  @Test
  public void testFoundationKeepsMavenOutsideNativeMode() {
    String previous = System.getProperty(NativeMode.PROPERTY);
    System.clearProperty(NativeMode.PROPERTY);
    try {
      Foundation foundation = new Foundation();
      assertTrue(foundation.REGISTRY.containsKey("MAVEN"));
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
