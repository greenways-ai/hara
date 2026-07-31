package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertSame;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import hara.kernel.Conn;
import hara.lang.protocol.IApplicable;
import hara.lang.protocol.IComponent;
import hara.lang.protocol.IContext;
import hara.lang.protocol.IInvokeIn;
import java.net.Socket;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.Test;

public class SessionKernelTest {
  @Test
  public void localAndRespClientsShareRootAcrossListenerRestarts() throws Exception {
    try (SessionKernel kernel = new SessionKernel(false, false)) {
      kernel.root().eval("(def answer 41)");

      try (HaraServer first = new HaraServer(kernel, "127.0.0.1", 0, false)) {
        first.start();
        assertEquals("42", legacyEval(first.port(), "(+ answer 1)"));
      }

      assertEquals("user", kernel.root().currentNamespace());
      kernel.root().eval("(def answer 42)");

      try (HaraServer second = new HaraServer(kernel, "127.0.0.1", 0, false)) {
        second.start();
        assertEquals("42", legacyEval(second.port(), "answer"));
      }
    }
  }

  @Test
  public void respControllerCanStartStopAndRestartWithoutClosingRoot() {
    try (SessionKernel kernel = new SessionKernel(false, false);
        Main.RespController resp = new Main.RespController(kernel, "127.0.0.1", 0, false)) {
      assertEquals("RESP ○ offline", resp.command("/resp"));
      assertTrue(resp.command("/resp start").startsWith("RESP ● 127.0.0.1:"));
      kernel.root().eval("(def retained 42)");
      assertEquals("RESP ○ offline", resp.command("/resp stop"));
      assertTrue(resp.command("/resp restart 0").startsWith("RESP ● 127.0.0.1:"));
      assertEquals("42", kernel.root().eval("retained").toString());
    }
  }

  @Test
  public void sessionsIsolateDefinitionsInsideOneBroker() {
    try (SessionKernel kernel = new SessionKernel(false, false)) {
      SessionKernel.Session alpha = kernel.create("alpha");
      SessionKernel.Session beta = kernel.create("beta");
      alpha.eval("(def answer 41)");
      beta.eval("(def answer 6)");
      assertEquals("41", alpha.eval("answer").toString());
      assertEquals("6", beta.eval("answer").toString());
    }
  }

  @Test
  public void sessionsConformToContextComponentAndApplicativeProtocols() {
    try (SessionKernel kernel = new SessionKernel(false, false)) {
      SessionKernel.Session alpha = kernel.create("alpha");
      SessionKernel.Session beta = kernel.create("beta");

      assertTrue(alpha instanceof IContext);
      assertTrue(alpha instanceof IComponent);
      assertTrue(alpha instanceof IApplicable);
      assertTrue(alpha instanceof IInvokeIn);
      assertTrue(alpha.isStarted());
      assertEquals(
          "user",
          ((SessionKernel.Session.SessionMetadata) alpha.getProps()).namespace);

      assertEquals(41L, alpha.call("(do (ns alpha.core) (def answer 41) answer)"));
      assertEquals("alpha.core", alpha.currentNamespace());
      assertEquals("user", beta.currentNamespace());
      assertSame(alpha, alpha.applyDefault());
      assertEquals(42L, alpha.applyIn(beta, new Object[] {"(+ 20 22)"}));
      assertEquals(42L, alpha.invokeIn(beta, "(+ 40 2)"));
      Object[] arguments = new Object[] {"answer"};
      assertSame(arguments, alpha.transformIn(beta, arguments));
      assertEquals(41L, alpha.transformOut(beta, arguments, 41L));

      alpha.stop();
      assertTrue(alpha.isStopped());
      assertFalse(alpha.isStarted());
      assertThrows(IllegalStateException.class, () -> alpha.call("answer"));
      assertThrows(IllegalStateException.class, alpha::start);
    }
  }

  @Test
  public void filesystemAttachmentConfinesFilesAndResetsSessionState() throws Exception {
    Path root = Files.createTempDirectory("hara-session-files");
    try (SessionKernel kernel = new SessionKernel(true, false)) {
      SessionKernel.Session session = kernel.create("mounted");
      session.eval("(def stale-value 42)");
      kernel.attachFilesystem("mounted", root);
      session.eval("(deref (file/write \"/state.bin\" (bytes 1 2 3)))");
      assertTrue(Files.exists(root.resolve("state.bin")));
      try {
        session.eval("stale-value");
        throw new AssertionError("reattachment must reset namespace state");
      } catch (IllegalArgumentException expected) {
        assertTrue(expected.getMessage().contains("Unbound"));
      }
    } finally {
      Files.deleteIfExists(root.resolve("state.bin"));
      Files.deleteIfExists(root);
    }
  }

  private static String legacyEval(int port, String source) throws Exception {
    try (Socket socket = new Socket("127.0.0.1", port)) {
      Conn conn = new Conn(socket);
      conn.write("EVAL", "ROOT", source);
      return text(conn.read());
    }
  }

  private static String text(Object value) {
    if (value instanceof byte[])
      return new String((byte[]) value, java.nio.charset.StandardCharsets.UTF_8);
    return String.valueOf(value);
  }
}
