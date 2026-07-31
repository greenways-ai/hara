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
  public void mapsAndSetsEncodeInCanonicalOrder() {
    byte[] mapA =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {
              hara.lang.data.Map.Standard.from(null, new Object[] {"b", 2L, "a", 1L, "c", 3L})
            });
    byte[] mapB =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {
              hara.lang.data.Map.Standard.from(null, new Object[] {"c", 3L, "a", 1L, "b", 2L})
            });
    assertArrayEquals(mapA, mapB);

    byte[] setA =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {hara.lang.data.Set.Standard.from(null, new Object[] {3L, 1L, 2L})});
    byte[] setB =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {hara.lang.data.Set.Standard.from(null, new Object[] {2L, 3L, 1L})});
    assertArrayEquals(setA, setB);

    // Entry order in the payload follows the canonical encoded-byte order, not
    // the host map/set iteration order. For longs 1, 100, -1 the canonical order
    // is 1 < 100 < -1 (unsigned lexicographic on the 8-byte big-endian encoding).
    byte[] one = {3, 0, 0, 0, 0, 0, 0, 0, 1};
    byte[] hundred = {3, 0, 0, 0, 0, 0, 0, 0, 100};
    byte[] minusOne = {3, -1, -1, -1, -1, -1, -1, -1, -1};

    byte[] mapEncoded =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {
              hara.lang.data.Map.Standard.from(
                  null, new Object[] {1L, "a", -1L, "b", 100L, "c"})
            });
    assertTrue(indexOf(mapEncoded, one) >= 0);
    assertTrue(indexOf(mapEncoded, one) < indexOf(mapEncoded, hundred));
    assertTrue(indexOf(mapEncoded, hundred) < indexOf(mapEncoded, minusOne));

    byte[] setEncoded =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {hara.lang.data.Set.Standard.from(null, new Object[] {1L, -1L, 100L})});
    assertTrue(indexOf(setEncoded, one) >= 0);
    assertTrue(indexOf(setEncoded, one) < indexOf(setEncoded, hundred));
    assertTrue(indexOf(setEncoded, hundred) < indexOf(setEncoded, minusOne));

    // Ordered collections keep insertion order: it is semantic there.
    byte[] orderedA =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {
              hara.lang.data.OrderedMap.Standard.from(null, new Object[] {"b", 2L, "a", 1L})
            });
    byte[] orderedB =
        HirArtifact.encode(
            "t",
            "t",
            new byte[0],
            new Object[] {
              hara.lang.data.OrderedMap.Standard.from(null, new Object[] {"a", 1L, "b", 2L})
            });
    assertTrue(!java.util.Arrays.equals(orderedA, orderedB));
  }

  private static int indexOf(byte[] haystack, byte[] needle) {
    outer:
    for (int i = 0; i + needle.length <= haystack.length; i++) {
      for (int j = 0; j < needle.length; j++) {
        if (haystack[i + j] != needle[j]) continue outer;
      }
      return i;
    }
    return -1;
  }

  @Test
  public void regexValuesRoundTripPortably() {
    java.util.regex.Pattern pattern = java.util.regex.Pattern.compile("a+b");
    HirArtifact.Module module =
        HirArtifact.decode(HirArtifact.encode("t", "t", new byte[0], new Object[] {pattern}));
    assertTrue(module.forms[0] instanceof java.util.regex.Pattern);
    assertEquals("a+b", ((java.util.regex.Pattern) module.forms[0]).pattern());

    java.util.regex.Pattern flagged =
        java.util.regex.Pattern.compile("a+b", java.util.regex.Pattern.CASE_INSENSITIVE);
    HaraException error =
        assertThrows(
            HaraException.class,
            () -> HirArtifact.encode("t", "t", new byte[0], new Object[] {flagged}));
    assertTrue(error.getMessage().contains("regex flags"));
  }

  @Test
  public void goldenBytesLockThePortableFormat() {
    // One form per opcode (0-17). Any change to the byte layout, the opcode
    // numbering, or the canonical collection ordering must update this golden
    // value and specs/99-archive/planning/runtime/hir.md together.
    Object[] forms =
        new Object[] {
          null,
          false,
          true,
          42L,
          2.5d,
          new java.math.BigInteger("123456789012345678901234567890"),
          new java.math.BigDecimal("3.14159"),
          "hárà",
          'x',
          hara.lang.data.Symbol.create("my.ns", "my-sym"),
          hara.lang.data.Keyword.create("kw"),
          hara.lang.data.List.Standard.from(null, new Object[] {1L, "a"}),
          hara.lang.data.Vector.Standard.from(null, new Object[] {1L, "a"}),
          hara.lang.data.Map.Standard.from(null, new Object[] {2L, "b", 1L, "a"}),
          hara.lang.data.Set.Standard.from(null, new Object[] {2L, 1L}),
          hara.lang.data.OrderedMap.Standard.from(null, new Object[] {2L, "b", 1L, "a"}),
          hara.lang.data.OrderedSet.Standard.from(null, new Object[] {2L, 1L}),
          java.util.regex.Pattern.compile("a+b")
        };
    byte[] expected =
        hexBytes(
            "48495200000100010000014b7640e14591506ea3c5e004467edc15b2ea8bb319"
                + "3b48a4596d99c242ca5531a000000001740000000174e3b0c44298fc1c149afb"
                + "f4c8996fb92427ae41e4649b934ca495991b7852b85500000012000102030000"
                + "00000000002a044004000000000000050000001e313233343536373839303132"
                + "3334353637383930313233343536373839300600000007332e31343135390700"
                + "00000668c3a172c3a008000000780901000000056d792e6e73000000066d792d"
                + "73796d000a00000000026b77000b000000020300000000000000010700000001"
                + "61000c00000002030000000000000001070000000161000d0000000203000000"
                + "0000000001070000000161030000000000000002070000000162000e00000002"
                + "030000000000000001030000000000000002000f000000020300000000000000"
                + "0207000000016203000000000000000107000000016100100000000203000000"
                + "0000000002030000000000000001001100000003612b62");
    byte[] encoded = HirArtifact.encode("t", "t", new byte[0], forms);
    assertArrayEquals(expected, encoded);

    // The golden artifact must also remain decodable.
    HirArtifact.Module module = HirArtifact.decode(expected);
    assertEquals("t", module.namespace);
    assertEquals(18, module.forms.length);
    assertEquals("a+b", ((java.util.regex.Pattern) module.forms[17]).pattern());
  }

  private static byte[] hexBytes(String hex) {
    byte[] bytes = new byte[hex.length() / 2];
    for (int i = 0; i < bytes.length; i++) {
      bytes[i] = (byte) Integer.parseInt(hex.substring(i * 2, i * 2 + 2), 16);
    }
    return bytes;
  }

  @Test
  public void foundationArtifactIsDeterministicAndRoundTripsForms() throws Exception {
    Path source = Path.of("lib/src/std/foundation.hal");
    byte[] sourceBytes = Files.readAllBytes(source);
    Object[] forms =
        HaraLanguage.readAll(
            Files.readString(source, StandardCharsets.UTF_8),
            HirArtifact.FOUNDATION_RESOURCE);

    byte[] first =
        HirArtifact.encode(
            "std.foundation", HirArtifact.FOUNDATION_RESOURCE, sourceBytes, forms);
    byte[] second =
        HirArtifact.encode(
            "std.foundation", HirArtifact.FOUNDATION_RESOURCE, sourceBytes, forms);
    assertArrayEquals(first, second);

    HirArtifact.Module decoded = HirArtifact.decode(first);
    assertEquals("std.foundation", decoded.namespace);
    assertEquals(HirArtifact.FOUNDATION_RESOURCE, decoded.resource);
    assertEquals(forms.length, decoded.forms.length);
    for (int index = 0; index < forms.length; index++) {
      assertEquals(G.display(forms[index]), G.display(decoded.forms[index]));
    }
  }

  @Test
  public void rejectsCorruptAndTruncatedArtifacts() throws Exception {
    byte[] source = "(ns std.foundation)".getBytes(StandardCharsets.UTF_8);
    Object[] forms =
        HaraLanguage.readAll(new String(source, StandardCharsets.UTF_8), "foundation.hal");
    byte[] artifact =
        HirArtifact.encode(
            "std.foundation", HirArtifact.FOUNDATION_RESOURCE, source, forms);

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
                "lib/src/std/foundation.hal",
                "--output",
                output.toString()
              },
              new PrintStream(stdout, true, StandardCharsets.UTF_8),
              new PrintStream(stderr, true, StandardCharsets.UTF_8));
      assertEquals(stderr.toString(StandardCharsets.UTF_8), 0, status);
      assertEquals("std.foundation", HirArtifact.decode(Files.readAllBytes(output)).namespace);
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
              .eval(HaraLanguage.ID, "((std.foundation/comp inc inc) 40)")
              .asLong());
      assertEquals(
          "[2 3]",
          context
              .eval(HaraLanguage.ID, "(vec (std.foundation/map inc [1 2]))")
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
                      + " (vec ((map inc) [1 2]))"
                      + " ((juxt inc dec) 8)]")
              .toString());
      assertEquals(
          "[[f g] [f g h] [f g h l & more]]",
          context.eval(HaraLanguage.ID, "(:arglists (meta (var comp)))").toString());
      assertEquals(
          42,
          context.eval(HaraLanguage.ID, "((comp inc inc inc inc) 38)").asLong());
    }
  }
}
