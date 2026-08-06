package hara.kernel.command;

import hara.kernel.Foundation;
import org.junit.Test;

import java.util.Arrays;
import java.util.List;

import static org.junit.Assert.*;

public class SessionTest {

  @Test
  public void testSessionLifecycle() {
    Foundation f = new Foundation();
    String sessionName = "test-session";

    // New
    Object res = Session.newSession(f, Arrays.asList(sessionName));
    assertEquals(sessionName, res);
    assertTrue((Boolean) Session.exists(f, Arrays.asList(sessionName)));

    // Get (should return existing)
    Object resGet = Session.get(f, Arrays.asList(sessionName));
    assertEquals(sessionName, resGet);

    // List
    List<?> list = (List<?>) Session.list(f, null);
    assertTrue(list.contains(sessionName));

    // Kill
    Session.kill(f, Arrays.asList(sessionName));
    assertFalse((Boolean) Session.exists(f, Arrays.asList(sessionName)));
  }
}
