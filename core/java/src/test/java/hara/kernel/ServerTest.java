package hara.kernel;

import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import java.io.IOException;
import java.net.Socket;
import java.util.concurrent.TimeUnit;
import static org.junit.Assert.*;

public class ServerTest {

  private Foundation foundation;
  private Server server;

  @Before
  public void setUp() {
    foundation = new Foundation();
    // Use port 0 for ephemeral port
    server = new Server(foundation, "TEST_SERVER", 0);
  }

  @After
  public void tearDown() {
    if (server != null && server.isStarted()) {
      server.stop();
    }
  }

  @Test
  public void testServerLifecycle() throws InterruptedException {
    assertFalse(server.isStarted());
    server.start();
    assertTrue(server.isStarted());

    // Allow some time for the server thread to start
    Thread.sleep(100);

    server.stop();
    assertFalse(server.isStarted());
  }

  @Test
  public void testClientConnection() throws IOException, InterruptedException {
    server.start();
    Thread.sleep(100);

    int port = server._socket.getLocalPort();

    try (Socket socket = new Socket("localhost", port)) {
      assertTrue(socket.isConnected());

      Conn conn = new Conn(socket);
      conn.write("PING");

      // Wait for response
      Object response = conn.read();
      String responseStr = new String((byte[]) response);
      assertEquals("PONG", responseStr);
    }
  }

  @Test
  public void testMultipleClients() throws IOException, InterruptedException {
    server.start();
    Thread.sleep(100);

    int port = server._socket.getLocalPort();
    int clientCount = 5;
    Socket[] sockets = new Socket[clientCount];
    Conn[] conns = new Conn[clientCount];

    for (int i = 0; i < clientCount; i++) {
      sockets[i] = new Socket("localhost", port);
      conns[i] = new Conn(sockets[i]);
      conns[i].write("PING");
    }

    for (int i = 0; i < clientCount; i++) {
      Object response = conns[i].read();
      String responseStr = new String((byte[]) response);
      assertEquals("PONG", responseStr);
      sockets[i].close();
    }

    // Give server time to update client count
    Thread.sleep(100);
  }
}
