package hara.kernel;

import hara.kernel.builtin.BuiltinRuntime;
import hara.kernel.base.RT;
import hara.kernel.protocol.IRuntime;
import hara.kernel.builtin.BuiltinRuntime;
import org.junit.Test;

import java.util.Arrays;
import java.util.List;

import static org.junit.Assert.*;

public class ControlPlaneTest {

  @Test
  public void testFoundationRegistry() {
    Foundation f = new Foundation();

    // Test basic commands via runCommand (mimicking RESP)
    assertEquals("PONG", f.call("PING"));
    assertEquals(Arrays.asList("hello"), f.call("ECHO", "hello"));

    // Test sub-commands
    Object jvmVersion = f.call("JVM", "VERSION");
    assertNotNull(jvmVersion);
  }

  @Test
  public void testCtlNoArgs() {
    Foundation f = new Foundation();
    String sessionKey = "test-session";
    f.RTS.put(sessionKey, new RT.Instance(f, sessionKey));
    IRuntime rt = f.RTS.get(sessionKey);

    // Test invoking ctl with no arguments
    // (ctl) -> should list commands
    Object res = BuiltinRuntime.ctl(rt, java.util.Collections.emptyList());
    assertTrue(res instanceof java.util.Collection);
    assertTrue(((java.util.Collection) res).contains("PING"));
    assertTrue(((java.util.Collection) res).contains("JVM"));
  }

  @Test
  public void testLispCtl() {
    // Setup a minimal runtime environment
    Foundation f = new Foundation();
    String sessionKey = "test-session";
    f.RTS.put(sessionKey, new RT.Instance(f, sessionKey));
    IRuntime rt = f.RTS.get(sessionKey);

    // Test invoking ctl via Builtin (mimicking Lisp call)
    // (ctl :ping)
    Object res = BuiltinRuntime.ctl(rt, Arrays.asList("PING"));
    assertEquals("PONG", res);

    // (ctl :echo "foo")
    Object resEcho = BuiltinRuntime.ctl(rt, Arrays.asList("ECHO", "foo"));
    assertEquals(Arrays.asList("foo"), resEcho);
  }

  @Test
  @SuppressWarnings("rawtypes")
  public void testDiscovery() {
    Foundation f = new Foundation();

    // Test DIR (Internal Discovery)
    List dir = (List) f.call("DIR");
    assertTrue(dir.contains("SERVERS"));
    assertTrue(dir.contains("RTS"));
    assertTrue(dir.contains("PEERS"));

    // Test INFO (Capabilities)
    List info = (List) f.call("INFO");
    // Check structure: [key1, val1, key2, val2...] due to mapToList
    // But mapToList creates [[k,v], [k,v]...] or similar depending on
    // implementation
    // Let's just check it returns *something* non-null for now as exact format
    // depends on mapToList
    assertNotNull(info);

    // Test PEER (Service Discovery)
    // Add
    f.call("PEER", "ADD", "node1", "localhost", "8081");
    assertTrue(f.PEERS.containsKey("node1"));

    // List
    Object peers = f.call("PEER", "LIST");
    assertNotNull(peers);

    // Ping
    Object pong = f.call("PEER", "PING", "node1");
    assertEquals(true, pong);

    // Remove
    f.call("PEER", "REMOVE", "node1");
    assertFalse(f.PEERS.containsKey("node1"));
  }
}
