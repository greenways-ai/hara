package hara.truffle;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import java.io.InputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.io.IOAccess;
import org.junit.Test;

public class HaraCapabilityTest {
  @Test
  public void fileAndSocketFunctionsResolveButDefaultToDenied() {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      PolyglotException file =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(file/read \"denied.bin\")"));
      assertTrue(file.getMessage().contains("file access is denied"));

      PolyglotException socket =
          assertThrows(
              PolyglotException.class,
              () -> context.eval(HaraLanguage.ID, "(socket/connect \"127.0.0.1\" 1)"));
      assertTrue(socket.getMessage().contains("network access is denied"));
    }
  }

  @Test
  public void explicitFileAccessPreservesBytesAndGatesModules() throws Exception {
    Path directory = Files.createTempDirectory("hara-capability-");
    Path data = directory.resolve("data.bin");
    Path module = directory.resolve("module.hal");
    Files.writeString(module, "(+ 40 2)");
    String dataPath = sourceString(data);
    String modulePath = sourceString(module);

    try (Context denied = Context.newBuilder(HaraLanguage.ID).build()) {
      PolyglotException error =
          assertThrows(
              PolyglotException.class,
              () -> denied.eval(HaraLanguage.ID, "(load-file \"" + modulePath + "\")"));
      assertTrue(error.getMessage().contains("file access is denied"));
    }

    try (Context allowed = context(true, false)) {
      assertEquals(
          255,
          allowed
              .eval(
                  HaraLanguage.ID,
                  "(deref (file/write \""
                      + dataPath
                      + "\" (bytes 0 127 255))) "
                      + "(bytes/get (deref (file/read \""
                      + dataPath
                      + "\")) 2)")
              .asLong());
      assertArrayEquals(new byte[] {0, 127, -1}, Files.readAllBytes(data));
      assertEquals(
          42, allowed.eval(HaraLanguage.ID, "(load-file \"" + modulePath + "\")").asLong());
    } finally {
      Files.deleteIfExists(data);
      Files.deleteIfExists(module);
      Files.deleteIfExists(directory);
    }
  }

  @Test
  public void fileResolveNormalizesAChildPathWhenFileAccessIsGranted() throws Exception {
    Path directory = Files.createTempDirectory("hara-resolve-");
    try (Context context = context(true, false)) {
      String root = sourceString(directory);
      String resolved =
          context
              .eval(HaraLanguage.ID, "(file/resolve \"" + root + "\" \"child/../data.bin\")")
              .asString();
      assertEquals(directory.resolve("data.bin").normalize().toString(), resolved);
    } finally {
      Files.deleteIfExists(directory);
    }
  }

  @Test
  public void explicitSocketAccessSendsBytesOnLoopback() throws Exception {
    try (ServerSocket server = new ServerSocket(0, 1, InetAddress.getLoopbackAddress())) {
      CompletableFuture<byte[]> received =
          CompletableFuture.supplyAsync(
              () -> {
                try (Socket socket = server.accept();
                    InputStream input = socket.getInputStream()) {
                  return input.readNBytes(3);
                } catch (Exception error) {
                  throw new RuntimeException(error);
                }
              });

      try (Context context = context(false, true)) {
        assertEquals(
            3,
            context
                .eval(
                    HaraLanguage.ID,
                    "(let [s (socket/connect \"127.0.0.1\" "
                        + server.getLocalPort()
                        + " nil (fn [error connection] "
                        + "connection))] "
                        + "(let [n (socket/send s (bytes 1 2 255))] "
                        + "(socket/close s) n))")
                .asLong());
      }
      assertArrayEquals(new byte[] {1, 2, -1}, received.get(5, TimeUnit.SECONDS));
    }
  }

  private static Context context(boolean file, boolean socket) {
    IOAccess access =
        IOAccess.newBuilder().allowHostFileAccess(file).allowHostSocketAccess(socket).build();
    return Context.newBuilder(HaraLanguage.ID).allowIO(access).build();
  }

  private static String sourceString(Path path) {
    return path.toString().replace("\\", "\\\\").replace("\"", "\\\"");
  }
}
