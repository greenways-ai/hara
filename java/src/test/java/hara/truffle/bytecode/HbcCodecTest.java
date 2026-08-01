package hara.truffle.bytecode;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

import hara.truffle.HtaValueCodec;
import hara.truffle.HaraLanguage;
import hara.lang.base.G;
import hara.truffle.bytecode.HbcProgram.Function;
import hara.truffle.bytecode.HbcProgram.Instruction;
import hara.truffle.bytecode.HbcProgram.Opcode;
import hara.truffle.bytecode.HbcProgram.Primitive;
import java.util.Arrays;
import java.util.List;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Source;
import org.graalvm.polyglot.io.ByteSequence;
import org.junit.Test;

public class HbcCodecTest {
  @Test
  public void hbc3ProgramsRoundTripCanonically() {
    HbcProgram program = arithmeticProgram();
    byte[] first = HbcCodec.encode(program);
    assertArrayEquals(new byte[] {'H', 'B', 'C', '3'}, Arrays.copyOf(first, 4));
    HbcProgram decoded = HbcCodec.decode(first);
    assertEquals(program, decoded);
    assertArrayEquals(first, HbcCodec.encode(decoded));
  }

  @Test
  public void corruptionIsRejectedBeforePayloadDecode() {
    byte[] artifact = HbcCodec.encode(arithmeticProgram());
    artifact[12] ^= 1;
    HbcFormatException failure = assertThrows(HbcFormatException.class, () -> HbcCodec.decode(artifact));
    assertEquals("bytecode artifact checksum mismatch", failure.getMessage());
  }

  @Test
  public void canonicalHtaSupportsFloatingConstants() {
    byte[] encoded = HtaValueCodec.encode(1.5d);
    assertEquals(1.5d, (Double) HtaValueCodec.decodeCanonical(encoded), 0.0d);
  }

  @Test
  public void invalidStackProgramsNeverReachExecution() {
    Function invalid =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            0,
            List.of(Instruction.of(Opcode.RETURN)),
            Arrays.asList((HbcProgram.Position) null),
            List.of());
    HbcFormatException failure =
        assertThrows(
            HbcFormatException.class,
            () -> HbcValidator.validate(new HbcProgram(List.of(), List.of(), List.of(invalid), 0)));
    assertTrue(failure.getMessage().contains("return with stack height 0"));
  }

  @Test
  public void polyglotExecutesEncodedHbc3() throws Exception {
    Source source =
        Source.newBuilder(
                HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(arithmeticProgram())), "sum.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(42L, context.eval(source).asLong());
    }
  }

  @Test
  public void closuresAndCallsExecuteInsideThePortableMachine() throws Exception {
    Function addCaptured =
        new Function(
            "add-captured",
            false,
            1,
            false,
            1,
            2,
            2,
            List.of(
                new Instruction(Opcode.LOAD_LOCAL, 0, 0, 0),
                new Instruction(Opcode.LOAD_LOCAL, 1, 0, 0),
                new Instruction(Opcode.PRIMITIVE, Primitive.ADD.id(), 2, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null),
            List.of());
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            2,
            List.of(
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                new Instruction(Opcode.CLOSURE, 1, 1, 0),
                new Instruction(Opcode.CONSTANT, 1, 0, 0),
                new Instruction(Opcode.CALL, 1, 0, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null, null),
            List.of());
    HbcProgram program = new HbcProgram(List.of(19L, 23L), List.of(), List.of(entry, addCaptured), 0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "closure.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(42L, context.eval(source).asLong());
    }
  }

  @Test
  public void decodesEveryArtifactInTheTrackedRustFoundationBundle() throws Exception {
    byte[] bundle = Files.readAllBytes(Path.of("rust/assets/std.foundation.hbb"));
    assertArrayEquals(new byte[] {'H', 'B', 'B', '1'}, Arrays.copyOf(bundle, 4));
    byte[] payload = Arrays.copyOfRange(bundle, 36, bundle.length);
    assertArrayEquals(Arrays.copyOfRange(bundle, 4, 36), MessageDigest.getInstance("SHA-256").digest(payload));
    ByteBuffer input = ByteBuffer.wrap(payload).order(ByteOrder.LITTLE_ENDIAN);
    int modules = input.getInt();
    assertTrue(modules > 0);
    for (int i = 0; i < modules; i++) {
      takeBundleField(input); // resource name
      takeBundleField(input); // namespace form
      byte[] artifact = takeBundleField(input);
      HbcProgram decoded = HbcCodec.decode(artifact);
      assertTrue(decoded.functions().size() > 0);
      assertTrue(HbcDisassembler.disassemble(decoded).startsWith("HBC3 entry="));
    }
    assertEquals(0, input.remaining());
  }

  @Test
  public void rustTryTableCatchesThrownGuestValues() throws Exception {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            1,
            1,
            List.of(
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                Instruction.of(Opcode.THROW),
                new Instruction(Opcode.LOAD_LOCAL, 0, 0, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null),
            List.of(
                new HbcProgram.TryEntry(
                    0,
                    2,
                    0,
                    List.of(new HbcProgram.CatchEntry("Exception", 0, 2)),
                    null,
                    null,
                    null)));
    HbcProgram program = new HbcProgram(List.of("boom"), List.of(), List.of(entry), 0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "catch.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals("boom", context.eval(source).asString());
    }
  }

  @Test
  public void executesTheRustFoundationBundleAndExposesItsDefinitions() throws Exception {
    List<HbcBundleCodec.Module> modules =
        HbcBundleCodec.decode(Files.readAllBytes(Path.of("rust/assets/std.foundation.hbb")));
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      for (HbcBundleCodec.Module module : modules) {
        context.eval(HaraLanguage.ID, module.namespaceForm());
        Source source =
            Source.newBuilder(
                    HaraLanguage.ID,
                    ByteSequence.create(module.artifact()),
                    module.resource() + ".hbc")
                .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
                .build();
        context.eval(source);
      }
      assertEquals("HARA", context.eval(HaraLanguage.ID, "(std.foundation.string/upper \"hara\")").asString());
      assertEquals(42L, context.eval(HaraLanguage.ID, "(std.foundation/if-not false 42)").asLong());
    }
  }

  @Test
  public void hostCallAndSettledAwaitUseTheSharedHostPromiseBoundary() throws Exception {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            3,
            List.of(
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                new Instruction(Opcode.CONSTANT, 1, 0, 0),
                new Instruction(Opcode.BUILD_VECTOR, 0, 0, 0),
                Instruction.of(Opcode.HOST_CALL),
                Instruction.of(Opcode.AWAIT),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null, null, null),
            List.of());
    HbcProgram program = new HbcProgram(List.of("host", "describe"), List.of(), List.of(entry), 0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "host.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      org.graalvm.polyglot.Value result = context.eval(source);
      assertTrue(result.hasHashEntries() || result.hasMembers());
    }
  }

  @Test
  public void asyncBytecodeFunctionsReturnPromisesThatAwaitToTheirValue() throws Exception {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            1,
            List.of(
                new Instruction(Opcode.CLOSURE, 1, 0, 0),
                new Instruction(Opcode.CALL, 0, 0, 0),
                Instruction.of(Opcode.AWAIT),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null),
            List.of());
    Function async =
        new Function(
            "answer",
            true,
            0,
            false,
            0,
            0,
            1,
            List.of(new Instruction(Opcode.CONSTANT, 0, 0, 0), Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null),
            List.of());
    HbcProgram program = new HbcProgram(List.of(42L), List.of(), List.of(entry, async), 0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "async.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(42L, context.eval(source).asLong());
    }
  }

  @Test
  public void staticBytecodeRecursionDoesNotConsumeTheJavaStack() throws Exception {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            1,
            List.of(
                new Instruction(Opcode.CONSTANT, 2, 0, 0),
                new Instruction(Opcode.CALL_STATIC, 1, 1, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null),
            List.of());
    Function recursive =
        new Function(
            "count-down",
            false,
            1,
            false,
            0,
            1,
            2,
            List.of(
                new Instruction(Opcode.LOAD_LOCAL, 0, 0, 0),
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                new Instruction(Opcode.PRIMITIVE, Primitive.LESS.id(), 2, 0),
                new Instruction(Opcode.JUMP_IF_FALSE, 6, 0, 0),
                new Instruction(Opcode.CONSTANT, 1, 0, 0),
                Instruction.of(Opcode.RETURN),
                new Instruction(Opcode.LOAD_LOCAL, 0, 0, 0),
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                new Instruction(Opcode.PRIMITIVE, Primitive.SUBTRACT.id(), 2, 0),
                new Instruction(Opcode.CALL_STATIC, 1, 1, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null, null, null, null, null, null, null, null),
            List.of());
    HbcProgram program =
        new HbcProgram(List.of(1L, 0L, 10_000L), List.of(), List.of(entry, recursive), 0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "deep.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(0L, context.eval(source).asLong());
    }
  }

  @Test
  public void exceptionsUnwindAcrossExplicitBytecodeCallFrames() throws Exception {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            1,
            1,
            List.of(
                new Instruction(Opcode.CALL_STATIC, 1, 0, 0),
                Instruction.of(Opcode.RETURN),
                new Instruction(Opcode.LOAD_LOCAL, 0, 0, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null),
            List.of(
                new HbcProgram.TryEntry(
                    0,
                    1,
                    0,
                    List.of(new HbcProgram.CatchEntry("Exception", 0, 2)),
                    null,
                    null,
                    null)));
    Function throwing =
        new Function(
            "throwing",
            false,
            0,
            false,
            0,
            0,
            1,
            List.of(new Instruction(Opcode.CONSTANT, 0, 0, 0), Instruction.of(Opcode.THROW)),
            Arrays.asList(null, null),
            List.of());
    HbcProgram program = new HbcProgram(List.of(42L), List.of(), List.of(entry, throwing), 0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "unwind.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      assertEquals(42L, context.eval(source).asLong());
    }
  }

  @Test
  public void defGlobalPreservesRustArtifactMetadata() throws Exception {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            1,
            List.of(
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                new Instruction(Opcode.DEF_GLOBAL, 1, 0, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null),
            List.of());
    HbcProgram.MetadataValue doc =
        new HbcProgram.MetadataValue(HbcProgram.MetadataValue.Kind.KEYWORD, hara.lang.data.Keyword.create("doc"));
    HbcProgram.MetadataValue text =
        new HbcProgram.MetadataValue(HbcProgram.MetadataValue.Kind.STRING, "portable metadata");
    HbcProgram program =
        new HbcProgram(
            List.of(42L, "answer"),
            List.of(List.of(new HbcProgram.MetadataEntry(doc, text))),
            List.of(entry),
            0);
    Source source =
        Source.newBuilder(HaraLanguage.ID, ByteSequence.create(HbcCodec.encode(program)), "meta.hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
            .build();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(source);
      assertEquals(
          "portable metadata",
          context.eval(HaraLanguage.ID, "(get (meta #'answer) :doc)").asString());
    }
  }

  @Test
  public void executesEveryRustProducedDisplayConformanceArtifact() throws Exception {
    List<HbcConformanceCorpus.Case> cases =
        HbcConformanceCorpus.decode(
            Files.readAllBytes(Path.of("rust/assets/bytecode-conformance.hcc")));
    assertTrue(cases.size() >= 80);
    for (HbcConformanceCorpus.Case testCase : cases) {
      Source source =
          Source.newBuilder(
                  HaraLanguage.ID,
                  ByteSequence.create(testCase.artifact()),
                  testCase.id() + ".hbc")
              .mimeType(HaraLanguage.BYTECODE_MIME_TYPE)
              .build();
      try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
        org.graalvm.polyglot.Value actual;
        try {
          actual = context.eval(source);
        } catch (RuntimeException failure) {
          throw new AssertionError(testCase.id(), failure);
        }
        String display =
            actual.isNull()
                ? "nil"
                : actual.isString() ? G.display(actual.asString()) : actual.toString();
        assertEquals(testCase.id(), testCase.expectedDisplay(), display);
      }
    }
  }

  private static byte[] takeBundleField(ByteBuffer input) {
    int size = input.getInt();
    byte[] value = new byte[size];
    input.get(value);
    return value;
  }

  private static HbcProgram arithmeticProgram() {
    Function entry =
        new Function(
            null,
            false,
            0,
            false,
            0,
            0,
            2,
            List.of(
                new Instruction(Opcode.CONSTANT, 0, 0, 0),
                new Instruction(Opcode.CONSTANT, 1, 0, 0),
                new Instruction(Opcode.PRIMITIVE, Primitive.ADD.id(), 2, 0),
                Instruction.of(Opcode.RETURN)),
            Arrays.asList(null, null, null, null),
            List.of());
    return new HbcProgram(List.of(19L, 23L), List.of(), List.of(entry), 0);
  }
}
