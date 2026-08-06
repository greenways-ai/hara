package hara.lang.base.primitive;

public interface Murmur3 {

  public static final int C1 = 0xcc9e2d51;

  public static final int C2 = 0x1b873593;

  public static final int seed = 0;

  public static int hashChars(CharSequence input) {
    int h1 = seed;

    // step through the CharSequence 2 chars at a time
    for (int i = 1; i < input.length(); i += 2) {
      int k1 = input.charAt(i - 1) | (input.charAt(i) << 16);
      k1 = mixK1(k1);
      h1 = mixH1(h1, k1);
    }

    // deal with any remaining characters
    if ((input.length() & 1) == 1) {
      int k1 = input.charAt(input.length() - 1);
      k1 = mixK1(k1);
      h1 ^= k1;
    }

    return mixFull(h1, 2 * input.length());
  }

  public static int hashInt(int input) {
    if (input == 0) return 0;
    int k1 = mixK1(input);
    int h1 = mixH1(seed, k1);

    return mixFull(h1, 4);
  }

  public static int hashLong(long input) {
    if (input == 0) return 0;
    int low = (int) input;
    int high = (int) (input >>> 32);

    int k1 = mixK1(low);
    int h1 = mixH1(seed, k1);

    k1 = mixK1(high);
    h1 = mixH1(h1, k1);

    return mixFull(h1, 8);
  }

  // Finalization mix - force all bits of a hash block to avalanche
  public static int mixFull(int h1, int length) {
    h1 ^= length;
    h1 ^= h1 >>> 16;
    h1 *= 0x85ebca6b;
    h1 ^= h1 >>> 13;
    h1 *= 0xc2b2ae35;
    h1 ^= h1 >>> 16;
    return h1;
  }

  public static int mixH1(int h1, int k1) {
    h1 ^= k1;
    h1 = Integer.rotateLeft(h1, 13);
    h1 = h1 * 5 + 0xe6546b64;
    return h1;
  }

  public static int mixHash(int hash, int count) {
    int h1 = seed;
    int k1 = mixK1(hash);
    h1 = mixH1(h1, k1);
    return mixFull(h1, count);
  }

  public static int mixK1(int k1) {
    k1 *= C1;
    k1 = Integer.rotateLeft(k1, 15);
    k1 *= C2;
    return k1;
  }
}
