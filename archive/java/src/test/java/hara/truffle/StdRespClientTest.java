package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.List;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.io.IOAccess;
import org.junit.Test;
import std.lib.resp.RespConnection;

public class StdRespClientTest {
  @Test
  public void networkCapabilityIsRequired() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'std.resp.client)");
      PolyglotException error =
          org.junit.Assert.assertThrows(
              PolyglotException.class,
              () ->
                  context.eval(
                      HaraLanguage.ID,
                      "(std.resp.client/connect \"127.0.0.1\" 1)"));
      assertTrue(error.getMessage().contains("network access is denied"));
    }
  }

  @Test
  public void callsAndClosesARespEndpoint() throws Exception {
    try (ServerSocket server =
            new ServerSocket(0, 1, InetAddress.getLoopbackAddress());
        Context context = networkContext()) {
      Future<?> peer =
          java.util.concurrent.Executors.newVirtualThreadPerTaskExecutor()
              .submit(
                  () -> {
                    try (Socket socket = server.accept();
                        RespConnection connection = new RespConnection(socket)) {
                      List<?> command = connection.read();
                      assertEquals("PING", text(command.get(0)));
                      connection.writeString("PONG");
                    }
                    return null;
                  });

      String source =
          "(require 'std.resp.client) "
              + "(let [c (std.resp.client/connect \"127.0.0.1\" "
              + server.getLocalPort()
              + ")]"
              + " [(std.resp.client/call c \"PING\")"
              + "  (std.resp.client/open? c)"
              + "  (std.resp.client/close c)"
              + "  (std.resp.client/open? c)])";
      assertEquals("[\"PONG\" true true false]", context.eval(HaraLanguage.ID, source).toString());
      peer.get(5, TimeUnit.SECONDS);
    }
  }

  @Test
  public void pipelinesCommandsAndSupportsBinaryBulkDecoding() throws Exception {
    try (ServerSocket server =
            new ServerSocket(0, 1, InetAddress.getLoopbackAddress());
        Context context = networkContext()) {
      Future<?> peer =
          java.util.concurrent.Executors.newVirtualThreadPerTaskExecutor()
              .submit(
                  () -> {
                    try (Socket socket = server.accept();
                        RespConnection connection = new RespConnection(socket)) {
                      for (int i = 0; i < 2; i++) {
                        List<?> command = connection.read();
                        connection.write((byte[]) command.get(1));
                      }
                    }
                    return null;
                  });

      String source =
          "(require 'std.resp.client) "
              + "(require 'std.foundation.bytes) "
              + "(let [c (std.resp.client/connect \"127.0.0.1\" "
              + server.getLocalPort()
              + " {:decode-bulk :bytes})"
              + "      out (std.resp.client/pipeline c [[\"ECHO\" \"ab\"] [\"ECHO\" \"xyz\"]])]"
              + " (std.resp.client/close c)"
              + " [(std.foundation.bytes/count (nth out 0)) (std.foundation.bytes/count (nth out 1))])";
      assertEquals("[2 3]", context.eval(HaraLanguage.ID, source).toString());
      peer.get(5, TimeUnit.SECONDS);
    }
  }

  private static Context networkContext() {
    return Context.newBuilder(HaraLanguage.ID)
        .allowIO(IOAccess.newBuilder().allowHostSocketAccess(true).build())
        .build();
  }

  private static String text(Object value) {
    return new String((byte[]) value, StandardCharsets.UTF_8);
  }
}
