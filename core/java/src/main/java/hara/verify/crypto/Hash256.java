package hara.verify.crypto;

import hara.lang.protocol.IDisplay;

import java.util.Arrays;
import java.util.HexFormat;

/** An immutable 256-bit digest. */
public final class Hash256 implements IDisplay {
  public static final int LENGTH = 32;

  private final byte[] bytes;

  private Hash256(byte[] bytes) {
    this.bytes = bytes;
  }

  public static Hash256 of(byte[] bytes) {
    if (bytes == null || bytes.length != LENGTH) {
      throw new IllegalArgumentException("A Hash256 must contain exactly 32 bytes.");
    }
    return new Hash256(bytes.clone());
  }

  public static Hash256 parse(String hex) {
    if (hex == null || hex.length() != LENGTH * 2) {
      throw new IllegalArgumentException("A Hash256 hex value must contain exactly 64 characters.");
    }
    try {
      return of(HexFormat.of().parseHex(hex));
    } catch (IllegalArgumentException e) {
      throw new IllegalArgumentException("Invalid Hash256 hex value.", e);
    }
  }

  public byte[] bytes() {
    return bytes.clone();
  }

  public String hex() {
    return HexFormat.of().formatHex(bytes);
  }

  @Override
  public String display() {
    return "#hash/sha256 \"" + hex() + "\"";
  }

  @Override
  public String toString() {
    return hex();
  }

  @Override
  public boolean equals(Object other) {
    return other instanceof Hash256 && Arrays.equals(bytes, ((Hash256) other).bytes);
  }

  @Override
  public int hashCode() {
    return Arrays.hashCode(bytes);
  }
}
