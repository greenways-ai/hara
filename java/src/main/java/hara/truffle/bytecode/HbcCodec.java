package hara.truffle.bytecode;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.truffle.HtaValueCodec;
import hara.truffle.bytecode.HbcProgram.CatchEntry;
import hara.truffle.bytecode.HbcProgram.Function;
import hara.truffle.bytecode.HbcProgram.Instruction;
import hara.truffle.bytecode.HbcProgram.MetadataEntry;
import hara.truffle.bytecode.HbcProgram.MetadataValue;
import hara.truffle.bytecode.HbcProgram.Opcode;
import hara.truffle.bytecode.HbcProgram.Position;
import hara.truffle.bytecode.HbcProgram.Primitive;
import hara.truffle.bytecode.HbcProgram.TaggedMetadata;
import hara.truffle.bytecode.HbcProgram.TryEntry;
import java.io.ByteArrayOutputStream;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.CharacterCodingException;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.regex.Pattern;

/** Canonical HBC3 encoder/decoder shared with {@code rust/src/vm/artifact.rs}. */
public final class HbcCodec {
  private static final byte[] MAGIC = {'H', 'B', 'C', '3'};
  private static final int DIGEST_BYTES = 32;

  private HbcCodec() {}

  public static HbcProgram decode(byte[] artifact) {
    if (artifact.length < MAGIC.length + Integer.BYTES + DIGEST_BYTES) {
      throw malformed("bytecode artifact is truncated");
    }
    if (!Arrays.equals(MAGIC, Arrays.copyOf(artifact, MAGIC.length))) {
      throw malformed("bytecode artifact has invalid magic");
    }
    long payloadLength = Integer.toUnsignedLong(ByteBuffer.wrap(artifact, 4, 4).getInt());
    long payloadEnd = 8L + payloadLength;
    if (payloadEnd + DIGEST_BYTES != artifact.length) {
      throw malformed("bytecode artifact length mismatch");
    }
    byte[] payload = Arrays.copyOfRange(artifact, 8, (int) payloadEnd);
    byte[] expected = Arrays.copyOfRange(artifact, (int) payloadEnd, artifact.length);
    if (!MessageDigest.isEqual(sha256(payload), expected)) {
      throw malformed("bytecode artifact checksum mismatch");
    }

    Reader in = new Reader(payload);
    int entry = in.u16();
    List<Object> constants = in.many(reader -> HtaValueCodec.decodeCanonical(reader.bytes()));
    List<List<MetadataEntry>> metadata = in.many(HbcCodec::readMetadata);
    List<Function> functions = in.many(HbcCodec::readFunction);
    in.finish();
    HbcProgram program = new HbcProgram(constants, metadata, functions, entry);
    HbcValidator.validate(program);
    return program;
  }

  public static byte[] encode(HbcProgram program) {
    HbcValidator.validate(program);
    Writer out = new Writer();
    out.u16(program.entry());
    out.many(program.constants(), value -> out.bytes(HtaValueCodec.encode(value)));
    out.many(program.varMetadata(), entries -> writeMetadata(out, entries));
    out.many(program.functions(), function -> writeFunction(out, function));
    byte[] payload = out.toByteArray();
    Writer artifact = new Writer();
    artifact.raw(MAGIC);
    artifact.u32(payload.length);
    artifact.raw(payload);
    artifact.raw(sha256(payload));
    return artifact.toByteArray();
  }

  private static Function readFunction(Reader in) {
    String name = in.optionalString();
    boolean asyncFunction = in.bool();
    int arity = in.u16();
    boolean variadic = in.bool();
    int captureCount = in.u16();
    int localCount = in.u16();
    int maxStack = in.u16();
    List<Instruction> code = in.many(HbcCodec::readInstruction);
    List<Position> sourceMap =
        in.many(reader -> reader.bool() ? new Position(reader.u32(), reader.u32(), reader.u32()) : null);
    List<TryEntry> handlers =
        in.many(
            reader -> {
              long start = reader.u32();
              long end = reader.u32();
              int depth = reader.u16();
              List<CatchEntry> catches =
                  reader.many(r -> new CatchEntry(r.string(), r.u16(), r.u32()));
              return new TryEntry(
                  start,
                  end,
                  depth,
                  catches,
                  reader.optionalU32(),
                  reader.optionalU16(),
                  reader.optionalU16());
            });
    return new Function(
        name,
        asyncFunction,
        arity,
        variadic,
        captureCount,
        localCount,
        maxStack,
        code,
        sourceMap,
        handlers);
  }

  private static void writeFunction(Writer out, Function function) {
    out.optionalString(function.name());
    out.bool(function.asyncFunction());
    out.u16(function.arity());
    out.bool(function.variadic());
    out.u16(function.captureCount());
    out.u16(function.localCount());
    out.u16(function.maxStack());
    out.many(function.code(), instruction -> writeInstruction(out, instruction));
    out.many(
        function.sourceMap(),
        position -> {
          out.bool(position != null);
          if (position != null) {
            out.u32(position.offset());
            out.u32(position.line());
            out.u32(position.column());
          }
        });
    out.many(
        function.handlers(),
        handler -> {
          out.u32(handler.start());
          out.u32(handler.end());
          out.u16(handler.depth());
          out.many(
              handler.catches(),
              clause -> {
                out.string(clause.className());
                out.u16(clause.binding());
                out.u32(clause.target());
              });
          out.optionalU32(handler.finallyTarget());
          out.optionalU16(handler.pendingValue());
          out.optionalU16(handler.pendingError());
        });
  }

  private static Instruction readInstruction(Reader in) {
    Opcode opcode = Opcode.fromId(in.u8());
    return switch (opcode) {
      case CONSTANT, JUMP, JUMP_IF_FALSE, GET_GLOBAL, SET_GLOBAL, VAR_GLOBAL,
          DECLARE_GLOBAL, STRUCT_FIELD -> new Instruction(opcode, in.u32(), 0, 0);
      case LOAD_LOCAL, STORE_LOCAL, BUILD_VECTOR, BUILD_MAP, BUILD_SET, BUILD_LIST,
          CONCAT_LIST -> new Instruction(opcode, in.u16(), 0, 0);
      case PRIMITIVE -> new Instruction(opcode, Primitive.fromId(in.u8()).id(), in.u8(), 0);
      case PRIMITIVE_LOCAL_CONST ->
          new Instruction(opcode, Primitive.fromId(in.u8()).id(), in.u16(), in.u32());
      case CLOSURE, CALL_STATIC -> new Instruction(opcode, in.u16(), in.u8(), 0);
      case CALL -> new Instruction(opcode, in.u8(), 0, 0);
      case DEF_GLOBAL, DEF_MACRO ->
          new Instruction(opcode, in.u32(), optionalSentinel(in.optionalU16()), 0);
      case DEF_STRUCT -> new Instruction(opcode, in.u32(), in.u32(), 0);
      case MAKE_MULTI_ARITY -> new Instruction(opcode, in.u32(), in.u8(), 0);
      default -> Instruction.of(opcode);
    };
  }

  private static void writeInstruction(Writer out, Instruction instruction) {
    Opcode opcode = instruction.opcode();
    out.u8(opcode.id());
    switch (opcode) {
      case CONSTANT, JUMP, JUMP_IF_FALSE, GET_GLOBAL, SET_GLOBAL, VAR_GLOBAL,
          DECLARE_GLOBAL, STRUCT_FIELD -> out.u32(instruction.first());
      case LOAD_LOCAL, STORE_LOCAL, BUILD_VECTOR, BUILD_MAP, BUILD_SET, BUILD_LIST,
          CONCAT_LIST -> out.u16(instruction.first());
      case PRIMITIVE -> {
        out.u8(instruction.first());
        out.u8(instruction.second());
      }
      case PRIMITIVE_LOCAL_CONST -> {
        out.u8(instruction.first());
        out.u16(instruction.second());
        out.u32(instruction.third());
      }
      case CLOSURE, CALL_STATIC -> {
        out.u16(instruction.first());
        out.u8(instruction.second());
      }
      case CALL -> out.u8(instruction.first());
      case DEF_GLOBAL, DEF_MACRO -> {
        out.u32(instruction.first());
        out.optionalU16(fromOptionalSentinel(instruction.second()));
      }
      case DEF_STRUCT -> {
        out.u32(instruction.first());
        out.u32(instruction.second());
      }
      case MAKE_MULTI_ARITY -> {
        out.u32(instruction.first());
        out.u8(instruction.second());
      }
      default -> {}
    }
  }

  private static List<MetadataEntry> readMetadata(Reader in) {
    return in.many(reader -> new MetadataEntry(readMetadataValue(reader), readMetadataValue(reader)));
  }

  private static void writeMetadata(Writer out, List<MetadataEntry> entries) {
    out.many(
        entries,
        entry -> {
          writeMetadataValue(out, entry.key());
          writeMetadataValue(out, entry.value());
        });
  }

  private static MetadataValue readMetadataValue(Reader in) {
    MetadataValue.Kind kind = metadataKind(in.u8());
    Object value =
        switch (kind) {
          case NIL -> null;
          case BOOLEAN -> in.bool();
          case NUMBER -> in.i64();
          case FLOAT -> Double.longBitsToDouble(in.u64());
          case BIG_INTEGER -> new BigInteger(in.string());
          case DECIMAL -> new BigDecimal(in.string());
          case CHARACTER -> requireUnicodeScalar(Math.toIntExact(in.u32()));
          case REGEX -> Pattern.compile(in.string());
          case TAGGED -> new TaggedMetadata(in.string(), readMetadataValue(in));
          case STRING -> in.string();
          case KEYWORD -> Keyword.create(in.string());
          case SYMBOL -> Symbol.create(in.string());
          case VECTOR, LIST, SET -> in.many(HbcCodec::readMetadataValue);
          case MAP -> in.many(reader -> new MetadataEntry(readMetadataValue(reader), readMetadataValue(reader)));
        };
    return new MetadataValue(kind, value);
  }

  @SuppressWarnings("unchecked")
  private static void writeMetadataValue(Writer out, MetadataValue metadata) {
    out.u8(metadata.kind().ordinal());
    Object value = metadata.value();
    switch (metadata.kind()) {
      case NIL -> {}
      case BOOLEAN -> out.bool((Boolean) value);
      case NUMBER -> out.i64(((Number) value).longValue());
      case FLOAT -> out.u64(Double.doubleToRawLongBits(((Number) value).doubleValue()));
      case BIG_INTEGER, DECIMAL -> out.string(value.toString());
      case CHARACTER -> out.u32(((Number) value).longValue());
      case REGEX -> out.string(((Pattern) value).pattern());
      case TAGGED -> {
        TaggedMetadata tagged = (TaggedMetadata) value;
        out.string(tagged.tag());
        writeMetadataValue(out, tagged.value());
      }
      case STRING -> out.string((String) value);
      case KEYWORD -> {
        Keyword keyword = (Keyword) value;
        out.string(qualified(keyword.getNamespace(), keyword.getName()));
      }
      case SYMBOL -> {
        Symbol symbol = (Symbol) value;
        out.string(qualified(symbol.getNamespace(), symbol.getName()));
      }
      case VECTOR, LIST, SET ->
          out.many((List<MetadataValue>) value, item -> writeMetadataValue(out, item));
      case MAP -> out.many((List<MetadataEntry>) value, entry -> {
        writeMetadataValue(out, entry.key());
        writeMetadataValue(out, entry.value());
      });
    }
  }

  private static MetadataValue.Kind metadataKind(int tag) {
    MetadataValue.Kind[] kinds = MetadataValue.Kind.values();
    if (tag >= kinds.length) throw malformed("bytecode artifact contains unknown metadata");
    return kinds[tag];
  }

  private static String qualified(String namespace, String name) {
    return namespace == null ? name : namespace + "/" + name;
  }

  private static int requireUnicodeScalar(int value) {
    if (!Character.isValidCodePoint(value)
        || (value >= Character.MIN_SURROGATE && value <= Character.MAX_SURROGATE)) {
      throw malformed("bytecode artifact contains invalid character scalar");
    }
    return value;
  }

  private static long optionalSentinel(Integer value) {
    return value == null ? -1 : value;
  }

  private static Integer fromOptionalSentinel(long value) {
    return value < 0 ? null : Math.toIntExact(value);
  }

  private static byte[] sha256(byte[] bytes) {
    try {
      return MessageDigest.getInstance("SHA-256").digest(bytes);
    } catch (NoSuchAlgorithmException impossible) {
      throw new AssertionError(impossible);
    }
  }

  private static HbcFormatException malformed(String message) {
    return new HbcFormatException(message);
  }

  @FunctionalInterface
  private interface ReaderFunction<T> {
    T read(Reader reader);
  }

  @FunctionalInterface
  private interface WriterConsumer<T> {
    void write(T value);
  }

  private static final class Reader {
    private final ByteBuffer input;

    Reader(byte[] bytes) {
      input = ByteBuffer.wrap(bytes).order(ByteOrder.BIG_ENDIAN);
    }

    int u8() {
      require(1);
      return Byte.toUnsignedInt(input.get());
    }

    boolean bool() {
      return switch (u8()) {
        case 0 -> false;
        case 1 -> true;
        default -> throw malformed("bytecode artifact contains invalid boolean");
      };
    }

    int u16() {
      require(2);
      return Short.toUnsignedInt(input.getShort());
    }

    long u32() {
      require(4);
      return Integer.toUnsignedLong(input.getInt());
    }

    long u64() {
      require(8);
      return input.getLong();
    }

    long i64() {
      require(8);
      return input.getLong();
    }

    byte[] bytes() {
      int size = checkedSize(u32());
      require(size);
      byte[] result = new byte[size];
      input.get(result);
      return result;
    }

    String string() {
      try {
        return StandardCharsets.UTF_8
            .newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
            .decode(ByteBuffer.wrap(bytes()))
            .toString();
      } catch (CharacterCodingException error) {
        throw malformed("bytecode artifact contains invalid UTF-8");
      }
    }

    String optionalString() {
      return bool() ? string() : null;
    }

    Integer optionalU16() {
      return bool() ? u16() : null;
    }

    Long optionalU32() {
      return bool() ? u32() : null;
    }

    <T> List<T> many(ReaderFunction<T> function) {
      int size = checkedSize(u32());
      ArrayList<T> values = new ArrayList<>(Math.min(size, 4096));
      for (int i = 0; i < size; i++) values.add(function.read(this));
      return values;
    }

    void finish() {
      if (input.hasRemaining()) throw malformed("bytecode artifact has trailing payload bytes");
    }

    private int checkedSize(long size) {
      if (size > Integer.MAX_VALUE) throw malformed("bytecode artifact length overflow");
      return (int) size;
    }

    private void require(int size) {
      if (size < 0 || input.remaining() < size) throw malformed("bytecode artifact is truncated");
    }
  }

  private static final class Writer {
    private final ByteArrayOutputStream output = new ByteArrayOutputStream();

    void u8(long value) {
      requireUnsigned(value, 0xffL, "u8");
      output.write((int) value);
    }

    void bool(boolean value) {
      u8(value ? 1 : 0);
    }

    void u16(long value) {
      requireUnsigned(value, 0xffffL, "u16");
      output.write((int) (value >>> 8));
      output.write((int) value);
    }

    void u32(long value) {
      requireUnsigned(value, 0xffff_ffffL, "u32");
      for (int shift = 24; shift >= 0; shift -= 8) output.write((int) (value >>> shift));
    }

    void u64(long value) {
      for (int shift = 56; shift >= 0; shift -= 8) output.write((int) (value >>> shift));
    }

    void i64(long value) {
      u64(value);
    }

    void bytes(byte[] value) {
      u32(value.length);
      raw(value);
    }

    void string(String value) {
      bytes(value.getBytes(StandardCharsets.UTF_8));
    }

    void optionalString(String value) {
      bool(value != null);
      if (value != null) string(value);
    }

    void optionalU16(Integer value) {
      bool(value != null);
      if (value != null) u16(value);
    }

    void optionalU32(Long value) {
      bool(value != null);
      if (value != null) u32(value);
    }

    <T> void many(List<T> values, WriterConsumer<T> consumer) {
      u32(values.size());
      for (T value : values) consumer.write(value);
    }

    void raw(byte[] value) {
      output.writeBytes(value);
    }

    byte[] toByteArray() {
      return output.toByteArray();
    }

    private void requireUnsigned(long value, long maximum, String type) {
      if (value < 0 || value > maximum) throw malformed("bytecode field does not fit " + type);
    }
  }
}
