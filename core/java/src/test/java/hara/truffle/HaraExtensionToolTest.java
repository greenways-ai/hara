package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.junit.Assume;
import org.junit.Test;

public class HaraExtensionToolTest {
  private static final Path PACKAGE =
      Path.of("rust/extensions/ledger-noir/target/package/ledger/noir")
          .toAbsolutePath()
          .normalize();

  @Test
  public void checksACompleteBuiltPackage() {
    Assume.assumeTrue(Files.isRegularFile(PACKAGE.resolve("hara.extension.edn")));
    Result result = run("extension", "check", PACKAGE.toString());
    assertEquals(result.error, 0, result.status);
    assertTrue(result.output.contains("ledger.noir [hara/ledger.noir] 0.1.0 is valid"));
  }

  @Test
  public void nodeHandshakeRequiresAndAcceptsExplicitProcessAuthority() {
    Assume.assumeTrue(Files.isRegularFile(PACKAGE.resolve("hara.extension.edn")));
    Result denied = run("extension", "test", PACKAGE.toString());
    assertEquals(1, denied.status);
    assertTrue(denied.error.contains("capability-denied"));

    Result allowed = run("--allow-process", "extension", "test", PACKAGE.toString());
    assertEquals(allowed.error, 0, allowed.status);
    assertTrue(allowed.output.contains("passed the Node HTA handshake"));
  }

  private static Result run(String... arguments) {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    ByteArrayOutputStream error = new ByteArrayOutputStream();
    int status =
        Main.run(
            arguments,
            new PrintStream(output, true, StandardCharsets.UTF_8),
            new PrintStream(error, true, StandardCharsets.UTF_8));
    return new Result(
        status,
        output.toString(StandardCharsets.UTF_8),
        error.toString(StandardCharsets.UTF_8));
  }

  private record Result(int status, String output, String error) {}
}
