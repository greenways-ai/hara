package hara.truffle.bytecode;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/** Decoder for Rust's deterministic HBB1 Foundation-module container. */
public final class HbcBundleCodec {
  private static final byte[] MAGIC = {'H', 'B', 'B', '1'};

  private HbcBundleCodec() {}

  public record Module(String resource, String namespaceForm, byte[] artifact) {
    public Module {
      artifact = artifact.clone();
    }

    @Override
    public byte[] artifact() {
      return artifact.clone();
    }
  }

  public static List<Module> decode(byte[] bundle) {
    if (bundle.length < 36 || !Arrays.equals(MAGIC, Arrays.copyOf(bundle, 4))) {
      throw new HbcFormatException("invalid foundation bytecode bundle header");
    }
    byte[] payload = Arrays.copyOfRange(bundle, 36, bundle.length);
    if (!MessageDigest.isEqual(Arrays.copyOfRange(bundle, 4, 36), sha256(payload))) {
      throw new HbcFormatException("foundation bytecode bundle checksum mismatch");
    }
    ByteBuffer input = ByteBuffer.wrap(payload).order(ByteOrder.LITTLE_ENDIAN);
    int count = size(input);
    ArrayList<Module> modules = new ArrayList<>(count);
    for (int i = 0; i < count; i++) {
      modules.add(new Module(text(input), text(input), bytes(input)));
    }
    if (input.hasRemaining()) throw new HbcFormatException("trailing bytes in foundation bytecode bundle");
    return List.copyOf(modules);
  }

  private static String text(ByteBuffer input) {
    return new String(bytes(input), StandardCharsets.UTF_8);
  }

  private static byte[] bytes(ByteBuffer input) {
    int size = size(input);
    if (input.remaining() < size) throw new HbcFormatException("truncated foundation bytecode bundle");
    byte[] value = new byte[size];
    input.get(value);
    return value;
  }

  private static int size(ByteBuffer input) {
    if (input.remaining() < 4) throw new HbcFormatException("truncated foundation bytecode bundle");
    int value = input.getInt();
    if (value < 0) throw new HbcFormatException("foundation bytecode bundle length overflow");
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
