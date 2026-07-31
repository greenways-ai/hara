package hara.truffle;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import hara.lang.data.types.ISetType;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;
import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.EOFException;
import java.io.IOException;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Map.Entry;

/** Deterministic, host-neutral binary representation of a Hara source module. */
final class HalcArtifact {
  static final int FORMAT_VERSION = 1;
  static final int EXECUTABLE_FOUNDATION_FLAG = 1;
  static final String FOUNDATION_RESOURCE = "std/foundation.hal";
  static final String FOUNDATION_HALC_RESOURCE = "std/foundation.halc";

  private static final byte[] MAGIC = {'H', 'A', 'L', 'C'};
  private static final byte[] LEGACY_MAGIC = {'H', 'I', 'R', 0};
  private static final int HASH_BYTES = 32;
  private static final int MAX_PAYLOAD_BYTES = 64 * 1024 * 1024;
  private static final int MAX_COLLECTION_ITEMS = 1_000_000;

  private static final int NIL = 0;
  private static final int FALSE = 1;
  private static final int TRUE = 2;
  private static final int LONG = 3;
  private static final int DOUBLE = 4;
  private static final int BIG_INTEGER = 5;
  private static final int BIG_DECIMAL = 6;
  private static final int STRING = 7;
  private static final int CHARACTER = 8;
  private static final int SYMBOL = 9;
  private static final int KEYWORD = 10;
  private static final int LIST = 11;
  private static final int VECTOR = 12;
  private static final int MAP = 13;
  private static final int SET = 14;
  private static final int ORDERED_MAP = 15;
  private static final int ORDERED_SET = 16;
  private static final int REGEX = 17;

  private HalcArtifact() {}

  static byte[] encode(String namespace, String resource, byte[] source, Object[] forms) {
    try {
      ByteArrayOutputStream payloadBytes = new ByteArrayOutputStream();
      try (DataOutputStream payload = new DataOutputStream(payloadBytes)) {
        writeString(payload, namespace);
        writeString(payload, resource);
        payload.write(sha256(source));
        writeCount(payload, forms.length);
        for (Object form : forms) writeValue(payload, form);
      }
      byte[] encodedPayload = payloadBytes.toByteArray();
      ByteArrayOutputStream artifactBytes = new ByteArrayOutputStream();
      try (DataOutputStream artifact = new DataOutputStream(artifactBytes)) {
        artifact.write(MAGIC);
        artifact.writeShort(FORMAT_VERSION);
        artifact.writeShort(EXECUTABLE_FOUNDATION_FLAG);
        artifact.writeInt(encodedPayload.length);
        artifact.write(sha256(encodedPayload));
        artifact.write(encodedPayload);
      }
      return artifactBytes.toByteArray();
    } catch (IOException error) {
      throw new HaraException("Unable to encode HALC: " + error.getMessage());
    }
  }

  static Module decode(byte[] artifactBytes) {
    try (DataInputStream artifact =
        new DataInputStream(new ByteArrayInputStream(artifactBytes))) {
      byte[] magic = artifact.readNBytes(MAGIC.length);
      Origin origin;
      if (Arrays.equals(MAGIC, magic)) {
        origin = Origin.HALC;
      } else if (Arrays.equals(LEGACY_MAGIC, magic)) {
        origin = Origin.LEGACY_HIR;
      } else {
        throw invalid("bad magic");
      }
      int version = artifact.readUnsignedShort();
      if (version != FORMAT_VERSION) {
        throw invalid("unsupported format version " + version);
      }
      int flags = artifact.readUnsignedShort();
      if (flags != EXECUTABLE_FOUNDATION_FLAG) {
        throw invalid("unsupported flags " + flags);
      }
      int payloadLength = artifact.readInt();
      if (payloadLength < 0 || payloadLength > MAX_PAYLOAD_BYTES) {
        throw invalid("invalid payload length " + payloadLength);
      }
      byte[] expectedHash = artifact.readNBytes(HASH_BYTES);
      if (expectedHash.length != HASH_BYTES) throw invalid("truncated payload hash");
      byte[] payloadBytes = artifact.readNBytes(payloadLength);
      if (payloadBytes.length != payloadLength) throw invalid("truncated payload");
      if (artifact.read() != -1) throw invalid("trailing bytes");
      if (!MessageDigest.isEqual(expectedHash, sha256(payloadBytes))) {
        throw invalid("payload checksum mismatch");
      }
      try (DataInputStream payload =
          new DataInputStream(new ByteArrayInputStream(payloadBytes))) {
        String namespace = readString(payload);
        String resource = readString(payload);
        byte[] sourceHash = payload.readNBytes(HASH_BYTES);
        if (sourceHash.length != HASH_BYTES) throw invalid("truncated source hash");
        int count = readCount(payload);
        Object[] forms = new Object[count];
        for (int index = 0; index < count; index++) forms[index] = readValue(payload);
        if (payload.read() != -1) throw invalid("trailing payload bytes");
        return new Module(namespace, resource, sourceHash, forms, origin);
      }
    } catch (EOFException error) {
      throw invalid("truncated artifact");
    } catch (IOException error) {
      throw invalid(error.getMessage());
    }
  }

  static String declaredNamespace(Object[] forms) {
    for (Object form : forms) {
      if (!(form instanceof hara.lang.data.List<?> list) || list.count() < 2) continue;
      if (!(list.nth(0) instanceof Symbol operator)
          || operator.getNamespace() != null
          || !"ns".equals(operator.getName())) continue;
      if (!(list.nth(1) instanceof Symbol namespace)
          || namespace.getNamespace() != null) {
        throw new HaraException("HALC source has an invalid ns declaration");
      }
      return namespace.getName();
    }
    throw new HaraException("HALC source does not declare a namespace");
  }

  private static void writeValue(DataOutputStream output, Object value) throws IOException {
    if (value == null) {
      output.writeByte(NIL);
    } else if (Boolean.FALSE.equals(value)) {
      output.writeByte(FALSE);
    } else if (Boolean.TRUE.equals(value)) {
      output.writeByte(TRUE);
    } else if (value instanceof Byte
        || value instanceof Short
        || value instanceof Integer
        || value instanceof Long) {
      output.writeByte(LONG);
      output.writeLong(((Number) value).longValue());
    } else if (value instanceof Float || value instanceof Double) {
      output.writeByte(DOUBLE);
      output.writeDouble(((Number) value).doubleValue());
    } else if (value instanceof BigInteger number) {
      output.writeByte(BIG_INTEGER);
      writeString(output, number.toString());
    } else if (value instanceof BigDecimal number) {
      output.writeByte(BIG_DECIMAL);
      writeString(output, number.toString());
    } else if (value instanceof String string) {
      output.writeByte(STRING);
      writeString(output, string);
    } else if (value instanceof Character character) {
      output.writeByte(CHARACTER);
      output.writeInt(character);
    } else if (value instanceof Symbol symbol) {
      output.writeByte(SYMBOL);
      writeNamespaced(output, symbol.getNamespace(), symbol.getName());
      writeMetadata(output, symbol);
    } else if (value instanceof Keyword keyword) {
      output.writeByte(KEYWORD);
      writeNamespaced(output, keyword.getNamespace(), keyword.getName());
      writeMetadata(output, keyword);
    } else if (value instanceof hara.lang.data.List<?> list) {
      output.writeByte(LIST);
      writeLinear(output, list);
      writeMetadata(output, list);
    } else if (value instanceof hara.lang.data.Vector<?> vector) {
      output.writeByte(VECTOR);
      writeLinear(output, vector);
      writeMetadata(output, vector);
    } else if (value instanceof hara.lang.data.OrderedMap<?, ?> map) {
      output.writeByte(ORDERED_MAP);
      writeMap(output, map, false);
      writeMetadata(output, map);
    } else if (value instanceof IMapType<?, ?> map) {
      output.writeByte(MAP);
      writeMap(output, map, true);
      writeMetadata(output, (IObjType) map);
    } else if (value instanceof hara.lang.data.OrderedSet<?> set) {
      output.writeByte(ORDERED_SET);
      writeSet(output, set, false);
      writeMetadata(output, set);
    } else if (value instanceof ISetType<?> set) {
      output.writeByte(SET);
      writeSet(output, set, true);
      writeMetadata(output, (IObjType) set);
    } else if (value instanceof java.util.regex.Pattern pattern) {
      if (pattern.flags() != 0) {
        throw new HaraException(
            "Unsupported portable HALC regex flags: " + pattern.flags() + " for " + pattern);
      }
      output.writeByte(REGEX);
      writeString(output, pattern.pattern());
    } else {
      throw new HaraException(
          "Unsupported portable HALC constant: " + value.getClass().getName());
    }
  }

  private static Object readValue(DataInputStream input) throws IOException {
    return switch (input.readUnsignedByte()) {
      case NIL -> null;
      case FALSE -> Boolean.FALSE;
      case TRUE -> Boolean.TRUE;
      case LONG -> input.readLong();
      case DOUBLE -> input.readDouble();
      case BIG_INTEGER -> new BigInteger(readString(input));
      case BIG_DECIMAL -> new BigDecimal(readString(input));
      case STRING -> readString(input);
      case CHARACTER -> (char) input.readInt();
      case SYMBOL ->
          withMetadata(
              Symbol.create(readNullableString(input), readString(input)), readMetadata(input));
      case KEYWORD ->
          withMetadata(
              Keyword.create(readNullableString(input), readString(input)), readMetadata(input));
      case LIST -> {
        Object[] values = readValues(input);
        yield withMetadata(hara.lang.data.List.Standard.from(null, values), readMetadata(input));
      }
      case VECTOR -> {
        Object[] values = readValues(input);
        yield withMetadata(hara.lang.data.Vector.Standard.from(null, values), readMetadata(input));
      }
      case MAP -> {
        Object[] entries = readEntries(input);
        yield withMetadata(hara.lang.data.Map.Standard.from(null, entries), readMetadata(input));
      }
      case ORDERED_MAP -> {
        Object[] entries = readEntries(input);
        yield withMetadata(
            hara.lang.data.OrderedMap.Standard.from(null, entries), readMetadata(input));
      }
      case SET -> {
        Object[] values = readValues(input);
        yield withMetadata(hara.lang.data.Set.Standard.from(null, values), readMetadata(input));
      }
      case ORDERED_SET -> {
        Object[] values = readValues(input);
        yield withMetadata(
            hara.lang.data.OrderedSet.Standard.from(null, values), readMetadata(input));
      }
      case REGEX -> java.util.regex.Pattern.compile(readString(input));
      default -> throw invalid("unknown value opcode");
    };
  }

  private static void writeLinear(DataOutputStream output, ILinearType<?> values)
      throws IOException {
    writeCount(output, Math.toIntExact(values.count()));
    for (Object value : values) writeValue(output, value);
  }

  private static Object[] readValues(DataInputStream input) throws IOException {
    int count = readCount(input);
    Object[] values = new Object[count];
    for (int index = 0; index < count; index++) values[index] = readValue(input);
    return values;
  }

  private static void writeMap(DataOutputStream output, IMapType<?, ?> map, boolean canonical)
      throws IOException {
    writeCount(output, Math.toIntExact(map.count()));
    int count = Math.toIntExact(map.count());
    if (!canonical) {
      // Ordered maps: entry order is semantic, keep iteration order.
      for (Object item : map) {
        Entry<?, ?> entry = (Entry<?, ?>) item;
        writeValue(output, entry.getKey());
        writeValue(output, entry.getValue());
      }
      return;
    }
    byte[][] encodedKeys = new byte[count][];
    byte[][] encodedEntries = new byte[count][];
    int index = 0;
    for (Object item : map) {
      Entry<?, ?> entry = (Entry<?, ?>) item;
      encodedKeys[index] = encodeValue(entry.getKey());
      encodedEntries[index] = concat(encodedKeys[index], encodeValue(entry.getValue()));
      index++;
    }
    for (int order : sortedOrder(encodedKeys)) output.write(encodedEntries[order]);
  }

  private static Object[] readEntries(DataInputStream input) throws IOException {
    int count = readCount(input);
    Object[] entries = new Object[Math.multiplyExact(count, 2)];
    for (int index = 0; index < entries.length; index++) entries[index] = readValue(input);
    return entries;
  }

  private static void writeSet(DataOutputStream output, ISetType<?> set, boolean canonical)
      throws IOException {
    writeCount(output, Math.toIntExact(set.count()));
    if (!canonical) {
      // Ordered sets: element order is semantic, keep iteration order.
      for (Object value : set) writeValue(output, value);
      return;
    }
    byte[][] encodedValues = new byte[Math.toIntExact(set.count())][];
    int index = 0;
    for (Object value : set) encodedValues[index++] = encodeValue(value);
    for (int order : sortedOrder(encodedValues)) output.write(encodedValues[order]);
  }

  // Canonical collection ordering: entries are sorted by the unsigned
  // lexicographic order of their canonical encodings, so the artifact does
  // not depend on any host map/set iteration order.
  private static byte[] encodeValue(Object value) throws IOException {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    try (DataOutputStream output = new DataOutputStream(bytes)) {
      writeValue(output, value);
    }
    return bytes.toByteArray();
  }

  private static byte[] concat(byte[] first, byte[] second) {
    byte[] both = Arrays.copyOf(first, first.length + second.length);
    System.arraycopy(second, 0, both, first.length, second.length);
    return both;
  }

  private static int[] sortedOrder(byte[][] encoded) {
    Integer[] order = new Integer[encoded.length];
    for (int index = 0; index < order.length; index++) order[index] = index;
    Arrays.sort(order, (a, b) -> Arrays.compareUnsigned(encoded[a], encoded[b]));
    int[] result = new int[order.length];
    for (int index = 0; index < order.length; index++) result[index] = order[index];
    return result;
  }

  private static void writeNamespaced(DataOutputStream output, String namespace, String name)
      throws IOException {
    writeNullableString(output, namespace);
    writeString(output, name);
  }

  private static void writeMetadata(DataOutputStream output, IObjType value) throws IOException {
    IMetadata metadata = value.meta();
    if (metadata == null) {
      output.writeBoolean(false);
    } else if (metadata instanceof IMapType<?, ?>) {
      output.writeBoolean(true);
      writeValue(output, metadata);
    } else {
      throw new HaraException(
          "Unsupported portable HALC metadata: " + metadata.getClass().getName());
    }
  }

  private static IMetadata readMetadata(DataInputStream input) throws IOException {
    if (!input.readBoolean()) return null;
    Object metadata = readValue(input);
    if (!(metadata instanceof IMetadata)) throw invalid("metadata is not metadata-capable");
    return (IMetadata) metadata;
  }

  @SuppressWarnings("unchecked")
  private static <T extends IObjType> T withMetadata(T value, IMetadata metadata) {
    return metadata == null ? value : (T) value.withMeta(metadata);
  }

  private static void writeString(DataOutputStream output, String value) throws IOException {
    byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
    output.writeInt(bytes.length);
    output.write(bytes);
  }

  private static String readString(DataInputStream input) throws IOException {
    int length = input.readInt();
    if (length < 0 || length > MAX_PAYLOAD_BYTES) throw invalid("invalid string length " + length);
    byte[] bytes = input.readNBytes(length);
    if (bytes.length != length) throw invalid("truncated string");
    return new String(bytes, StandardCharsets.UTF_8);
  }

  private static void writeNullableString(DataOutputStream output, String value)
      throws IOException {
    output.writeBoolean(value != null);
    if (value != null) writeString(output, value);
  }

  private static String readNullableString(DataInputStream input) throws IOException {
    return input.readBoolean() ? readString(input) : null;
  }

  private static void writeCount(DataOutputStream output, int count) throws IOException {
    if (count < 0 || count > MAX_COLLECTION_ITEMS) {
      throw new HaraException("HALC collection is too large: " + count);
    }
    output.writeInt(count);
  }

  private static int readCount(DataInputStream input) throws IOException {
    int count = input.readInt();
    if (count < 0 || count > MAX_COLLECTION_ITEMS) {
      throw invalid("invalid collection count " + count);
    }
    return count;
  }

  private static byte[] sha256(byte[] value) {
    try {
      return MessageDigest.getInstance("SHA-256").digest(value);
    } catch (NoSuchAlgorithmException impossible) {
      throw new AssertionError(impossible);
    }
  }

  private static HaraException invalid(String detail) {
    return new HaraException("Invalid HALC artifact: " + detail);
  }

  static final class Module {
    final String namespace;
    final String resource;
    final byte[] sourceHash;
    final Object[] forms;
    final Origin origin;

    Module(String namespace, String resource, byte[] sourceHash, Object[] forms, Origin origin) {
      this.namespace = namespace;
      this.resource = resource;
      this.sourceHash = sourceHash.clone();
      this.forms = forms.clone();
      this.origin = origin;
    }
  }

  enum Origin {
    HALC,
    LEGACY_HIR
  }
}
