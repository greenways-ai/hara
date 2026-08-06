package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import hara.kernel.Conn;
import java.net.Socket;
import java.util.List;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;

public class HaraServerTest {
  private HaraServer server;

  @Before
  public void setUp() throws Exception {
    server = new HaraServer("127.0.0.1", 0, false);
    server.start();
  }

  @After
  public void tearDown() {
    if (server != null) server.close();
  }

  @Test
  public void createsAndAttachesIsolatedSessions() throws Exception {
    try (Socket socket = new Socket("127.0.0.1", server.port())) {
      Conn conn = new Conn(socket);

      conn.write("HELLO", "3");
      List<?> hello = (List<?>) conn.read();
      assertEquals("SERVER", text(hello.get(0)));
      assertEquals("HARA", text(hello.get(1)));

      conn.write("SESSION", "NEW", "APP");
      assertEquals("APP", text(((List<?>) conn.read()).get(2)));
      conn.write("SESSION", "ATTACH", "APP");
      assertEquals("APP", text(((List<?>) conn.read()).get(2)));

      conn.write("EVAL", "REQ-1", "(def answer 41)");
      conn.read();
      conn.read();
      conn.write("EVAL", "REQ-2", "(+ answer 1)");
      List<?> result = (List<?>) conn.read();
      assertEquals("42", text(result.get(2)));
      conn.read();
    }

    assertTrue(server.sessionNames().contains("ROOT"));
    assertTrue(server.sessionNames().contains("APP"));
  }

  @Test
  public void legacyClientsCanEvaluateBySessionWithoutHello() throws Exception {
    try (Socket socket = new Socket("127.0.0.1", server.port())) {
      Conn conn = new Conn(socket);
      conn.write("EVAL", "ROOT", "(+ 19 23)");
      assertEquals("42", text(conn.read()));
    }
  }

  @Test
  public void sessionsAreIsolated() throws Exception {
    try (Socket socket = new Socket("127.0.0.1", server.port())) {
      Conn conn = new Conn(socket);
      conn.write("SESSION", "NEW", "OTHER");
      conn.read();
      conn.write("SESSION", "ATTACH", "OTHER");
      conn.read();
      conn.write("EVAL", "OTHER", "answer");
      Object response = conn.read();
      assertTrue(text(response).contains("ERROR") || text(response).contains("Unbound"));
    }
  }

  @Test
  public void twoTruffleServersExposeIndependentRespEndpoints() throws Exception {
    try (HaraServer peer = new HaraServer("127.0.0.1", 0, false, true);
        HaraServer client = new HaraServer("127.0.0.1", 0, false, true)) {
      peer.start();
      client.start();
      try (Socket peerSocket = new Socket("127.0.0.1", peer.port());
          Socket clientSocket = new Socket("127.0.0.1", client.port())) {
        Conn peerConn = new Conn(peerSocket);
        Conn clientConn = new Conn(clientSocket);
        peerConn.write("HELLO", "3", "CLIENT", "PEER");
        peerConn.read();
        clientConn.write("HELLO", "3", "CLIENT", "CLIENT");
        clientConn.read();
        peerConn.write("SESSION", "NEW", "PEER-SESSION");
        peerConn.read();
        clientConn.write("EVAL", "REQ-CLIENT", "(+ 19 23)");
        clientConn.read();
        clientConn.read();
      }
      assertTrue(peer.sessionNames().contains("PEER-SESSION"));
      assertEquals(java.util.Set.of("ROOT"), client.sessionNames());
    }
  }

  @Test
  public void protocolFourUsesCorrelatedResultAndDoneFrames() throws Exception {
    try (Socket socket = new Socket("127.0.0.1", server.port())) {
      Conn conn = new Conn(socket);
      conn.write("HELLO", "4", "CLIENT", "TEST");
      List<?> hello = (List<?>) conn.read();
      assertEquals("4", text(hello.get(5)));

      conn.write("SESSION", "SESSION-1", "LIST");
      List<?> sessions = (List<?>) conn.read();
      assertEquals("RESULT", text(sessions.get(0)));
      assertEquals("SESSION-1", text(sessions.get(1)));
      assertEquals("DONE", text(((List<?>) conn.read()).get(0)));

      conn.write("EVAL", "EVAL-1", "(+ 20 22)");
      List<?> result = (List<?>) conn.read();
      assertEquals("42", text(result.get(2)));
      List<?> done = (List<?>) conn.read();
      assertEquals("EVAL-1", text(done.get(1)));
      assertEquals("OK", text(done.get(2)));
    }
  }

  @Test
  public void protocolFourPreservesEvaluationSourceAndReturnsStructuredDocs() throws Exception {
    try (Socket socket = new Socket("127.0.0.1", server.port())) {
      Conn conn = new Conn(socket);
      conn.write("HELLO", "4", "CLIENT", "TEST");
      conn.read();

      conn.write(
          "EVAL",
          "EVAL-SOURCE",
          "(defn located \"A located function.\" [value] value)",
          "FILE",
          "/tmp/sample.hal",
          "LINE",
          "12",
          "COLUMN",
          "3");
      conn.read();
      conn.read();

      conn.write("DOC", "DOC-1", "located");
      List<?> response = (List<?>) conn.read();
      List<?> documentation = (List<?>) response.get(2);
      assertEquals("located", text(plist(documentation, "SYMBOL")));
      assertEquals("A located function.", text(plist(documentation, "DOC")));
      assertEquals("/tmp/sample.hal", text(plist(documentation, "FILE")));
      assertEquals("12", text(plist(documentation, "LINE")));
      assertEquals("3", text(plist(documentation, "COLUMN")));
      assertTrue(plist(documentation, "ARGLISTS") instanceof List<?>);
      conn.read();
    }
  }

  @Test
  public void protocolFourReturnsStableErrorsAndRegisteredOperations() throws Exception {
    server.close();
    server = new HaraServer("127.0.0.1", 0, false);
    server.registerHandler(
        new HaraServer.Handler() {
          @Override
          public String operation() {
            return "TEST-ECHO";
          }

          @Override
          public void handle(HaraServer.Request request, HaraServer.Responder responder)
              throws Exception {
            responder.result(List.of(request.connectionId(), request.argument(0)));
          }
        });
    server.start();

    try (Socket socket = new Socket("127.0.0.1", server.port())) {
      Conn conn = new Conn(socket);
      conn.write("HELLO", "4");
      conn.read();
      conn.write("TEST-ECHO", "CUSTOM-1", "hello");
      List<?> result = (List<?>) conn.read();
      assertEquals("hello", text(((List<?>) result.get(2)).get(1)));
      conn.read();

      conn.write("DOES-NOT-EXIST", "BAD-1");
      List<?> error = (List<?>) conn.read();
      assertEquals("ERROR", text(error.get(0)));
      assertEquals("UNKNOWN_OP", text(error.get(2)));
      List<?> done = (List<?>) conn.read();
      assertEquals("ERROR", text(done.get(2)));
    }
  }

  private static String text(Object value) {
    if (value instanceof byte[])
      return new String((byte[]) value, java.nio.charset.StandardCharsets.UTF_8);
    return String.valueOf(value);
  }

  private static Object plist(List<?> values, String key) {
    for (int index = 0; index + 1 < values.size(); index += 2) {
      if (key.equals(text(values.get(index)))) return values.get(index + 1);
    }
    return null;
  }
}
