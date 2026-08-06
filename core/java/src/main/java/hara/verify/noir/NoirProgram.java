package hara.verify.noir;

import hara.lang.protocol.IDisplay;
import hara.verify.crypto.Hash256;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.regex.Pattern;

/** Immutable Noir source plus the exact toolchain identity that gives it meaning. */
public final class NoirProgram implements IDisplay {
  public static final String DEFAULT_NOIR_VERSION = "1.0.0-beta.25";
  public static final String DEFAULT_BACKEND_VERSION = "bb.js/5.1.0";

  private static final byte[] DOMAIN = "HARA_NOIR_PROGRAM_V1\0".getBytes(StandardCharsets.US_ASCII);
  private static final Pattern NAME = Pattern.compile("[a-z][a-z0-9_]*");

  private final String name;
  private final String source;
  private final String noirVersion;
  private final String backendVersion;
  private final Hash256 sourceHash;
  private final Hash256 cacheKey;

  private NoirProgram(String name, String source, String noirVersion, String backendVersion) {
    if (name == null || !NAME.matcher(name).matches()) {
      throw new IllegalArgumentException("Noir package names must match [a-z][a-z0-9_]*.");
    }
    if (source == null || source.trim().isEmpty()) {
      throw new IllegalArgumentException("Noir source cannot be empty.");
    }
    if (noirVersion == null || noirVersion.isBlank()) {
      throw new IllegalArgumentException("A pinned Noir compiler version is required.");
    }
    if (backendVersion == null || backendVersion.isBlank()) {
      throw new IllegalArgumentException("A pinned proving backend version is required.");
    }
    this.name = name;
    this.source = source;
    this.noirVersion = noirVersion;
    this.backendVersion = backendVersion;
    this.sourceHash = digest(source.getBytes(StandardCharsets.UTF_8));
    this.cacheKey = calculateCacheKey();
  }

  public static NoirProgram create(String name, String source) {
    return new NoirProgram(name, source, DEFAULT_NOIR_VERSION, DEFAULT_BACKEND_VERSION);
  }

  public static NoirProgram create(
      String name, String source, String noirVersion, String backendVersion) {
    return new NoirProgram(name, source, noirVersion, backendVersion);
  }

  public String name() {
    return name;
  }

  public String source() {
    return source;
  }

  public String noirVersion() {
    return noirVersion;
  }

  public String backendVersion() {
    return backendVersion;
  }

  public Hash256 sourceHash() {
    return sourceHash;
  }

  public Hash256 cacheKey() {
    return cacheKey;
  }

  public String manifest() {
    return "[package]\nname = \"" + name + "\"\ntype = \"bin\"\n";
  }

  @Override
  public String display() {
    return "#noir/program {\"" + name + "\" \"" + cacheKey.hex() + "\"}";
  }

  private Hash256 calculateCacheKey() {
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    bytes.writeBytes(DOMAIN);
    append(bytes, name);
    append(bytes, source);
    append(bytes, noirVersion);
    append(bytes, backendVersion);
    return digest(bytes.toByteArray());
  }

  private static void append(ByteArrayOutputStream output, String value) {
    byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
    output.writeBytes(ByteBuffer.allocate(Long.BYTES).putLong(bytes.length).array());
    output.writeBytes(bytes);
  }

  private static Hash256 digest(byte[] bytes) {
    try {
      return Hash256.of(MessageDigest.getInstance("SHA-256").digest(bytes));
    } catch (NoSuchAlgorithmException e) {
      throw new IllegalStateException("The JVM does not provide SHA-256.", e);
    }
  }
}
