package hara.truffle.bytecode;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/** Decoder for the Rust-generated HCC1 bytecode conformance artifact. */
public final class HbcConformanceCorpus {
  private static final byte[] MAGIC = {'H', 'C', 'C', '1'};

  private HbcConformanceCorpus() {}

  public record Case(String id, String expectedDisplay, byte[] artifact) {
    public Case {
      artifact = artifact.clone();
    }

    @Override
    public byte[] artifact() {
      return artifact.clone();
    }
  }

  public static List<Case> decode(byte[] corpus) {
    if (corpus.length < 36 || !Arrays.equals(MAGIC, Arrays.copyOf(corpus, 4))) {
      throw new HbcFormatException("invalid bytecode conformance corpus header");
    }
    byte[] payload = Arrays.copyOfRange(corpus, 36, corpus.length);
    if (!MessageDigest.isEqual(Arrays.copyOfRange(corpus, 4, 36), sha256(payload))) {
      throw new HbcFormatException("bytecode conformance corpus checksum mismatch");
    }
    ByteBuffer input = ByteBuffer.wrap(payload).order(ByteOrder.LITTLE_ENDIAN);
    int count = size(input);
    ArrayList<Case> cases = new ArrayList<>(count);
    for (int i = 0; i < count; i++) {
      cases.add(new Case(text(input), text(input), bytes(input)));
    }
    if (input.hasRemaining()) {
      throw new HbcFormatException("trailing bytes in bytecode conformance corpus");
    }
    return List.copyOf(cases);
  }

  private static String text(ByteBuffer input) {
    return new String(bytes(input), StandardCharsets.UTF_8);
  }

  private static byte[] bytes(ByteBuffer input) {
    int size = size(input);
    if (input.remaining() < size) {
      throw new HbcFormatException("truncated bytecode conformance corpus");
    }
    byte[] value = new byte[size];
    input.get(value);
    return value;
  }

  private static int size(ByteBuffer input) {
    if (input.remaining() < 4) {
      throw new HbcFormatException("truncated bytecode conformance corpus");
    }
    int value = input.getInt();
    if (value < 0) throw new HbcFormatException("bytecode conformance corpus length overflow");
    return value;
  }

  private static byte[] sha256(byte[] value) {
    try {
      return MessageDigest.getInstance("SHA-256").digest(value);
    } catch (NoSuchAlgorithmException impossible) {
      throw new AssertionError(impossible);
    }
  }
}
