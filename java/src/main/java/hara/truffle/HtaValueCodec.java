package hara.truffle;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import hara.lang.data.types.ISetType;
import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.regex.Pattern;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;

/** Canonical, dependency-free value encoding used by HTA v1. */
public final class HtaValueCodec {
  private static final byte[] MAGIC = {'H', 'T', 'A', '1'};
  private static final int MAX_FRAME_BYTES = 64 * 1024 * 1024;
  private static final int MAX_NESTING_DEPTH = 256;
  private static final int NIL = 0;
  private static final int FALSE = 1;
  private static final int TRUE = 2;
  private static final int I64 = 3;
  private static final int STRING = 4;
  private static final int BYTES = 5;
  private static final int KEYWORD = 6;
  private static final int SYMBOL = 7;
  private static final int LIST = 8;
  private static final int VECTOR = 9;
  private static final int SET = 10;
  private static final int MAP = 11;
  private static final int HANDLE = 12;
  private static final int F64 = 15;
  private static final int CHARACTER = 19;
  private static final int BIG_INTEGER = 20;
  private static final int DECIMAL = 21;
  private static final int REGEX = 22;

  private HtaValueCodec() {}

  public static byte[] encode(Object value) {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    output.writeBytes(MAGIC);
    write(output, HaraBox.unwrap(value), 0);
    return output.toByteArray();
  }

  public static Object decode(byte[] bytes) {
    return decode(bytes, false);
  }

  /** Decodes list and vector tags to their distinct Hara persistent values for HBC3 constants. */
  public static Object decodeCanonical(byte[] bytes) {
    return decode(bytes, true);
  }

  private static Object decode(byte[] bytes, boolean canonicalCollections) {
    if (bytes.length > MAX_FRAME_BYTES) throw malformed("frame too large");
    if (bytes.length < MAGIC.length) throw malformed("missing HTA1 header");
    for (int i = 0; i < MAGIC.length; i++) {
      if (bytes[i] != MAGIC[i]) throw malformed("invalid HTA1 header");
    }
    Reader reader = new Reader(bytes, MAGIC.length, canonicalCollections);
    Object value = reader.read(0);
    if (reader.remaining() != 0) throw malformed("trailing bytes");
    return value;
  }

  private static void write(ByteArrayOutputStream output, Object value, int depth) {
    if (depth > MAX_NESTING_DEPTH) throw malformed("nesting depth exceeded");
    if (value == null || value == HaraNull.SINGLETON) {
      output.write(NIL);
    } else if (value instanceof Boolean) {
      output.write((Boolean) value ? TRUE : FALSE);
    } else if (value instanceof Byte
        || value instanceof Short
        || value instanceof Integer
        || value instanceof Long) {
      output.write(I64);
      writeLong(output, ((Number) value).longValue());
    } else if (value instanceof Float || value instanceof Double) {
      output.write(F64);
      writeLong(output, Double.doubleToRawLongBits(((Number) value).doubleValue()));
    } else if (value instanceof Character) {
      output.write(CHARACTER);
      writeInt(output, (Character) value);
    } else if (value instanceof BigInteger) {
      output.write(BIG_INTEGER);
      writeText(output, value.toString());
    } else if (value instanceof BigDecimal) {
      output.write(DECIMAL);
      writeText(output, value.toString());
    } else if (value instanceof Pattern) {
      output.write(REGEX);
      writeText(output, ((Pattern) value).pattern());
    } else if (value instanceof String) {
      output.write(STRING);
      writeBytes(output, ((String) value).getBytes(StandardCharsets.UTF_8));
    } else if (value instanceof byte[]) {
      output.write(BYTES);
      writeBytes(output, (byte[]) value);
    } else if (value instanceof Keyword) {
      Keyword keyword = (Keyword) value;
      output.write(KEYWORD);
      writeText(output, qualified(keyword.getNamespace(), keyword.getName()));
    } else if (value instanceof Symbol) {
      Symbol symbol = (Symbol) value;
      output.write(SYMBOL);
      writeText(output, qualified(symbol.getNamespace(), symbol.getName()));
    } else if (value instanceof HtaHandle) {
      HtaHandle handle = (HtaHandle) value;
      if (handle.released()) throw new HaraException("hta/handle-released: " + handle);
      output.write(HANDLE);
      writeText(output, handle.owner());
      writeText(output, handle.type());
      writeLong(output, handle.id());
    } else if (value instanceof IMapType<?, ?>) {
      writeMap(output, ((IMapType<?, ?>) value).iterator(), depth);
    } else if (value instanceof Map<?, ?>) {
      writeMap(output, ((Map<?, ?>) value).entrySet().iterator(), depth);
    } else if (value instanceof ISetType<?>) {
      writeSet(output, ((ISetType<?>) value).iterator(), depth);
    } else if (value instanceof java.util.Set<?>) {
      writeSet(output, ((java.util.Set<?>) value).iterator(), depth);
    } else if (value instanceof hara.lang.data.List<?>) {
      output.write(LIST);
      writeCollection(output, (ILinearType<?>) value, depth);
    } else if (value instanceof ILinearType<?>) {
      output.write(VECTOR);
      writeCollection(output, (ILinearType<?>) value, depth);
    } else if (value instanceof List<?>) {
      output.write(VECTOR);
      writeCollection(output, (List<?>) value, depth);
    } else if (value instanceof Collection<?>) {
      output.write(LIST);
      writeCollection(output, (Collection<?>) value, depth);
    } else {
      throw new HaraException("hta/value-unsupported: " + value.getClass().getName());
    }
  }

  private static void writeSet(ByteArrayOutputStream output, Iterator<?> iterator, int depth) {
    ArrayList<byte[]> encoded = new ArrayList<>();
    iterator.forEachRemaining(value -> encoded.add(encodeBare(value, depth + 1)));
    encoded.sort(HtaValueCodec::compareUnsigned);
    output.write(SET);
    writeInt(output, encoded.size());
    encoded.forEach(value -> writeRaw(output, value));
  }

  private static void writeMap(ByteArrayOutputStream output, Iterator<?> iterator, int depth) {
    ArrayList<Map.Entry<byte[], byte[]>> encoded = new ArrayList<>();
    iterator.forEachRemaining(
        item -> {
          Map.Entry<?, ?> entry = (Map.Entry<?, ?>) item;
          encoded.add(
              Map.entry(encodeBare(entry.getKey(), depth + 1), encodeBare(entry.getValue(), depth + 1)));
        });
    encoded.sort((left, right) -> compareUnsigned(left.getKey(), right.getKey()));
    output.write(MAP);
    writeInt(output, encoded.size());
    encoded.forEach(
        entry -> {
          writeRaw(output, entry.getKey());
          writeRaw(output, entry.getValue());
        });
  }

  private static byte[] encodeBare(Object value, int depth) {
    ByteArrayOutputStream output = new ByteArrayOutputStream();
    write(output, HaraBox.unwrap(value), depth);
    return output.toByteArray();
  }

  private static void writeCollection(ByteArrayOutputStream output, Iterable<?> values, int depth) {
    ArrayList<Object> copy = new ArrayList<>();
    values.forEach(copy::add);
    writeInt(output, copy.size());
    copy.forEach(value -> write(output, HaraBox.unwrap(value), depth + 1));
  }

  private static int compareUnsigned(byte[] left, byte[] right) {
    return java.util.Arrays.compareUnsigned(left, right);
  }

  private static String qualified(String namespace, String name) {
    return namespace == null ? name : namespace + "/" + name;
  }

  private static void writeText(ByteArrayOutputStream output, String value) {
    writeBytes(output, value.getBytes(StandardCharsets.UTF_8));
  }

  private static void writeBytes(ByteArrayOutputStream output, byte[] value) {
    if (value.length > MAX_FRAME_BYTES - output.size() - Integer.BYTES) {
      throw malformed("frame too large");
    }
    writeInt(output, value.length);
    output.writeBytes(value);
  }

  private static void writeRaw(ByteArrayOutputStream output, byte[] value) {
    if (value.length > MAX_FRAME_BYTES - output.size()) throw malformed("frame too large");
    output.writeBytes(value);
  }

  private static void writeInt(ByteArrayOutputStream output, int value) {
    output.write((value >>> 24) & 0xff);
    output.write((value >>> 16) & 0xff);
    output.write((value >>> 8) & 0xff);
    output.write(value & 0xff);
  }

  private static void writeLong(ByteArrayOutputStream output, long value) {
    for (int shift = 56; shift >= 0; shift -= 8) output.write((int) (value >>> shift) & 0xff);
  }

  private static HaraException malformed(String message) {
    return new HaraException("hta/value-malformed: " + message);
  }

  private static final class Reader {
    private final ByteBuffer input;
    private final boolean canonicalCollections;

    private Reader(byte[] bytes, int offset, boolean canonicalCollections) {
      input =
          ByteBuffer.wrap(bytes, offset, bytes.length - offset).slice().order(ByteOrder.BIG_ENDIAN);
      this.canonicalCollections = canonicalCollections;
    }

    private int remaining() {
      return input.remaining();
    }

    private Object read(int depth) {
      if (depth > MAX_NESTING_DEPTH) throw malformed("nesting depth exceeded");
      require(1);
      int tag = Byte.toUnsignedInt(input.get());
      switch (tag) {
        case NIL:
          return HaraNull.SINGLETON;
        case FALSE:
          return false;
        case TRUE:
          return true;
        case I64:
          require(8);
          return input.getLong();
        case F64:
          require(8);
          return Double.longBitsToDouble(input.getLong());
        case CHARACTER:
          require(4);
          int codePoint = input.getInt();
          if (!Character.isValidCodePoint(codePoint)
              || (codePoint >= Character.MIN_SURROGATE && codePoint <= Character.MAX_SURROGATE)) {
            throw malformed("invalid character scalar");
          }
          return Character.isBmpCodePoint(codePoint)
              ? Character.valueOf((char) codePoint)
              : new String(Character.toChars(codePoint));
        case BIG_INTEGER:
          return new BigInteger(text());
        case DECIMAL:
          return new BigDecimal(text());
        case REGEX:
          return Pattern.compile(text());
        case STRING:
          return text();
        case BYTES:
          return bytes();
        case KEYWORD:
          return Keyword.create(text());
        case SYMBOL:
          return Symbol.create(text());
        case LIST:
          return sequence(depth + 1, false);
        case VECTOR:
          return sequence(depth + 1, true);
        case SET:
          return set(depth + 1);
        case MAP:
          return map(depth + 1);
        case HANDLE:
          String owner = text();
          String type = text();
          require(8);
          return new HtaHandle(owner, type, input.getLong());
        default:
          throw malformed("unknown value tag");
      }
    }

    private Object sequence(int depth, boolean vector) {
      int size = size();
      requireContainerItems(size, 1, "sequence");
      ArrayList<Object> result = new ArrayList<>(size);
      for (int i = 0; i < size; i++) result.add(read(depth));
      if (!canonicalCollections) return result;
      Object[] values = result.toArray();
      return vector
          ? hara.lang.data.Vector.Standard.from(null, values)
          : hara.lang.data.List.Standard.from(null, values);
    }

    private Object set(int depth) {
      int size = size();
      requireContainerItems(size, 1, "set");
      LinkedHashSet<Object> result = new LinkedHashSet<>();
      for (int i = 0; i < size; i++) result.add(read(depth));
      return canonicalCollections
          ? hara.lang.data.Set.Standard.from(null, result.toArray())
          : result;
    }

    private Object map(int depth) {
      int size = size();
      requireContainerItems(size, 2, "map");
      LinkedHashMap<Object, Object> result = new LinkedHashMap<>();
      for (int i = 0; i < size; i++) result.put(read(depth), read(depth));
      if (!canonicalCollections) return result;
      Object[] entries = new Object[result.size() * 2];
      int index = 0;
      for (java.util.Map.Entry<Object, Object> entry : result.entrySet()) {
        entries[index++] = entry.getKey();
        entries[index++] = entry.getValue();
      }
      return hara.lang.data.Map.Standard.from(null, entries);
    }

    private String text() {
      return new String(bytes(), StandardCharsets.UTF_8);
    }

    private byte[] bytes() {
      int size = size();
      require(size);
      byte[] result = new byte[size];
      input.get(result);
      return result;
    }

    private int size() {
      require(4);
      int size = input.getInt();
      if (size < 0) throw malformed("negative length");
      return size;
    }

    private void requireContainerItems(int count, int minimumBytes, String kind) {
      if (count > input.remaining() / minimumBytes) {
        throw malformed("impossible " + kind + " length");
      }
    }

    private void require(int amount) {
      if (amount < 0 || input.remaining() < amount) throw malformed("truncated value");
    }
  }
}
