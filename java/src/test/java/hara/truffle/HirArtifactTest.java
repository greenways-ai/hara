package hara.truffle;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import hara.lang.base.G;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.junit.Test;

public class HirArtifactTest {
  @Test
  public void foundationArtifactIsDeterministicAndRoundTripsForms() throws Exception {
    Path source = Path.of("lib/src/std/lib/foundation.hal");
    byte[] sourceBytes = Files.readAllBytes(source);
    Object[] forms =
        HaraLanguage.readAll(
            Files.readString(source, StandardCharsets.UTF_8),
            HirArtifact.FOUNDATION_RESOURCE);

    byte[] first =
        HirArtifact.encode(
            "std.lib.foundation", HirArtifact.FOUNDATION_RESOURCE, sourceBytes, forms);
    byte[] second =
        HirArtifact.encode(
            "std.lib.foundation", HirArtifact.FOUNDATION_RESOURCE, sourceBytes, forms);
    assertArrayEquals(first, second);

    HirArtifact.Module decoded = HirArtifact.decode(first);
    assertEquals("std.lib.foundation", decoded.namespace);
    assertEquals(HirArtifact.FOUNDATION_RESOURCE, decoded.resource);
    assertEquals(forms.length, decoded.forms.length);
    for (int index = 0; index < forms.length; index++) {
      assertEquals(G.display(forms[index]), G.display(decoded.forms[index]));
    }
  }

  @Test
  public void rejectsCorruptAndTruncatedArtifacts() throws Exception {
    byte[] source = "(ns std.lib.foundation)".getBytes(StandardCharsets.UTF_8);
    Object[] forms =
        HaraLanguage.readAll(new String(source, StandardCharsets.UTF_8), "foundation.hal");
    byte[] artifact =
        HirArtifact.encode(
            "std.lib.foundation", HirArtifact.FOUNDATION_RESOURCE, source, forms);

    byte[] corrupt = artifact.clone();
    corrupt[corrupt.length - 1] ^= 1;
    assertTrue(
        assertThrows(HaraException.class, () -> HirArtifact.decode(corrupt))
            .getMessage()
            .contains("checksum"));

    byte[] truncated = java.util.Arrays.copyOf(artifact, artifact.length - 1);
    assertTrue(
        assertThrows(HaraException.class, () -> HirArtifact.decode(truncated))
            .getMessage()
            .contains("truncated"));

    byte[] missingExecutableFlag = artifact.clone();
    missingExecutableFlag[6] = 0;
    missingExecutableFlag[7] = 0;
    assertTrue(
        assertThrows(HaraException.class, () -> HirArtifact.decode(missingExecutableFlag))
            .getMessage()
            .contains("unsupported flags"));
  }

  @Test
  public void compileCommandWritesLoadableFoundationArtifact() throws Exception {
    Path output = Files.createTempFile("foundation-", ".hir");
    try {
      ByteArrayOutputStream stdout = new ByteArrayOutputStream();
      ByteArrayOutputStream stderr = new ByteArrayOutputStream();
      int status =
          Main.run(
              new String[] {
                "compile-hir",
                "lib/src/std/lib/foundation.hal",
                "--output",
                output.toString()
              },
              new PrintStream(stdout, true, StandardCharsets.UTF_8),
              new PrintStream(stderr, true, StandardCharsets.UTF_8));
      assertEquals(stderr.toString(StandardCharsets.UTF_8), 0, status);
      assertEquals("std.lib.foundation", HirArtifact.decode(Files.readAllBytes(output)).namespace);
    } finally {
      Files.deleteIfExists(output);
    }
  }

  @Test
  public void strictAndOffModesBothPreserveFoundationSemantics() {
    String previous = System.getProperty("hara.HirMode");
    try {
      System.setProperty("hara.HirMode", "strict");
      long before = FoundationHirLowerer.compilationCount();
      assertFoundation();
      assertTrue(FoundationHirLowerer.compilationCount() > before);
      System.setProperty("hara.HirMode", "off");
      assertFoundation();
    } finally {
      if (previous == null) System.clearProperty("hara.HirMode");
      else System.setProperty("hara.HirMode", previous);
    }
  }

  private static void assertFoundation() {
    try (Context context =
        Context.newBuilder(HaraLanguage.ID)
            .option("engine.WarnInterpreterOnly", "false")
            .build()) {
      assertEquals(
          42,
          context
              .eval(HaraLanguage.ID, "((std.lib.foundation/comp2 inc inc) 40)")
              .asLong());
      assertEquals(
          "[2 3]",
          context
              .eval(HaraLanguage.ID, "(vec (std.lib.foundation/map inc [1 2]))")
              .toString());
      assertEquals(
          "[42 {:a {:b 42}} [0 1 2 3] [7 7 7] [2 3] [9 7]]",
          context
              .eval(
                  HaraLanguage.ID,
                  "[(get-in {:a {:b 42}} [:a :b])"
                      + " (assoc-in {} [:a :b] 42)"
                      + " (vec (range 4))"
                      + " (vec (repeat 3 7))"
                      + " ((map inc) [1 2])"
                      + " ((juxt inc dec) 8)]")
              .toString());
      assertEquals(
          "[[f g] [f g h]]",
          context.eval(HaraLanguage.ID, "(:arglists (meta (var comp)))").toString());
    }
  }
}
