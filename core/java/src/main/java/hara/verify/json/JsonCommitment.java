package hara.verify.json;

import hara.verify.crypto.Hash256;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.Map;

/** The stable semantic commitment ABI for provable JSON v1. */
public final class JsonCommitment {
  private static final byte[] DOMAIN = "HARA_JSON_V1\0".getBytes(StandardCharsets.US_ASCII);

  private JsonCommitment() {}

  public static Hash256 commit(Object value) {
    return commit(JsonValue.fromHara(value));
  }

  public static Hash256 commit(JsonValue value) {
    return Hash256.of(hash(encoded(value)));
  }

  private static byte[] encoded(JsonValue value) {
    var bytes = new ByteArrayOutputStream();
    bytes.writeBytes(DOMAIN);
    if (value instanceof JsonValue.Null) {
      bytes.write(0);
    } else if (value instanceof JsonValue.Bool) {
      bytes.write(((JsonValue.Bool) value).value() ? 2 : 1);
    } else if (value instanceof JsonValue.Integer) {
      bytes.write(3);
      bytes.writeBytes(longBytes(((JsonValue.Integer) value).value()));
    } else if (value instanceof JsonValue.String) {
      bytes.write(4);
      writeBytes(bytes, ((JsonValue.String) value).value().getBytes(StandardCharsets.UTF_8));
    } else if (value instanceof JsonValue.Array) {
      bytes.write(5);
      JsonValue.Array array = (JsonValue.Array) value;
      bytes.writeBytes(longBytes(array.values().size()));
      for (var child : array.values()) bytes.writeBytes(commit(child).bytes());
    } else {
      bytes.write(6);
      var entries = new ArrayList<>(((JsonValue.Object) value).values().entrySet());
      entries.sort(Comparator.comparing(Map.Entry::getKey, JsonCommitment::compareUtf8));
      bytes.writeBytes(longBytes(entries.size()));
      for (var entry : entries) {
        writeBytes(bytes, entry.getKey().getBytes(StandardCharsets.UTF_8));
        bytes.writeBytes(commit(entry.getValue()).bytes());
      }
    }
    return bytes.toByteArray();
  }

  private static void writeBytes(ByteArrayOutputStream output, byte[] value) {
    output.writeBytes(longBytes(value.length));
    output.writeBytes(value);
  }

  private static byte[] longBytes(long value) {
    return ByteBuffer.allocate(Long.BYTES).putLong(value).array();
  }

  private static byte[] hash(byte[] value) {
    try {
      return MessageDigest.getInstance("SHA-256").digest(value);
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException("The JVM does not provide SHA-256.", e);
    }
  }

  private static int compareUtf8(String left, String right) {
    return java.util.Arrays.compareUnsigned(
        left.getBytes(StandardCharsets.UTF_8), right.getBytes(StandardCharsets.UTF_8));
  }
}
