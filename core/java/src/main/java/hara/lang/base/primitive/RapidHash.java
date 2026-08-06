package hara.lang.base.primitive;

import java.nio.charset.StandardCharsets;

/**
 * rapidhash - Very fast, high quality, platform-independent hashing algorithm.
 *
 * <p>Ported from https://github.com/Nicoshev/rapidhash
 */
public interface RapidHash {

  long RAPID_SEED = 0xbdd89aa982704029L;

  long RAPID_SECRET_0 = 0x2d358dccaa6c78a5L;
  long RAPID_SECRET_1 = 0x8bb84b93962eacc9L;
  long RAPID_SECRET_2 = 0x4b33a62ed433d4a3L;

  static long rapid_mix(long A, long B) {
    long low = A * B;
    long high = Math.unsignedMultiplyHigh(A, B);
    return low ^ high;
  }

  static long read64(byte[] data, int offset) {
    return (data[offset] & 0xFFL)
        | ((data[offset + 1] & 0xFFL) << 8)
        | ((data[offset + 2] & 0xFFL) << 16)
        | ((data[offset + 3] & 0xFFL) << 24)
        | ((data[offset + 4] & 0xFFL) << 32)
        | ((data[offset + 5] & 0xFFL) << 40)
        | ((data[offset + 6] & 0xFFL) << 48)
        | ((data[offset + 7] & 0xFFL) << 56);
  }

  static long read32(byte[] data, int offset) {
    return (data[offset] & 0xFFL)
        | ((data[offset + 1] & 0xFFL) << 8)
        | ((data[offset + 2] & 0xFFL) << 16)
        | ((data[offset + 3] & 0xFFL) << 24);
  }

  static long readSmall(byte[] data, int offset, int k) {
    return ((data[offset] & 0xFFL) << 56)
        | ((data[offset + (k >> 1)] & 0xFFL) << 32)
        | (data[offset + k - 1] & 0xFFL);
  }

  static long hash(String s) {
    return hash(s.getBytes(StandardCharsets.UTF_8));
  }

  static long hash(byte[] data) {
    return hash(data, RAPID_SEED);
  }

  static long hash(byte[] data, long seed) {
    int len = data.length;
    return hash(data, 0, len, seed);
  }

  static long hash(byte[] data, int offset, int len, long seed) {
    long secret0 = RAPID_SECRET_0;
    long secret1 = RAPID_SECRET_1;
    long secret2 = RAPID_SECRET_2;

    seed ^= rapid_mix(seed ^ secret0, secret1) ^ len;
    long a, b;

    if (len <= 16) {
      if (len >= 4) {
        int plast = offset + len - 4;
        a = (read32(data, offset) << 32) | read32(data, plast);

        // delta = (len & 24) >> (len >> 3)
        int delta = (len & 24) >> (len >> 3);
        b = (read32(data, offset + delta) << 32) | read32(data, plast - delta);
      } else if (len > 0) {
        a = readSmall(data, offset, len);
        b = 0;
      } else {
        a = 0;
        b = 0;
      }
    } else {
      int i = len;
      int p = offset;
      if (i > 48) {
        long see1 = seed, see2 = seed;
        do {
          seed = rapid_mix(read64(data, p) ^ secret0, read64(data, p + 8) ^ seed);
          see1 = rapid_mix(read64(data, p + 16) ^ secret1, read64(data, p + 24) ^ see1);
          see2 = rapid_mix(read64(data, p + 32) ^ secret2, read64(data, p + 40) ^ see2);
          p += 48;
          i -= 48;
        } while (i >= 48);
        seed ^= see1 ^ see2;
      }
      if (i > 16) {
        seed = rapid_mix(read64(data, p) ^ secret2, read64(data, p + 8) ^ seed ^ secret1);
        if (i > 32) {
          seed = rapid_mix(read64(data, p + 16) ^ secret2, read64(data, p + 24) ^ seed);
        }
      }
      a = read64(data, p + i - 16);
      b = read64(data, p + i - 8);
    }

    a ^= secret1;
    b ^= seed;

    long low = a * b;
    long high = Math.unsignedMultiplyHigh(a, b);

    return rapid_mix(low ^ secret0 ^ len, high ^ secret1);
  }
}
