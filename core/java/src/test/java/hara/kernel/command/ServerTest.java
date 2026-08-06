package hara.kernel.command;

import hara.kernel.Foundation;
import org.junit.Test;

import static org.junit.Assert.assertNull;

public class ServerTest {

  @Test
  public void testServerCommands() {
    Foundation f = new Foundation();
    assertNull(Server.info(f, null));
    assertNull(Server.list(f, null));
    assertNull(Server.newServer(f, null));
    assertNull(Server.stop(f, null));
    assertNull(Server.exists(f, null));
  }
}
