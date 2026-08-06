package hara.lang.protocol;

public interface IHashCached extends IHash {

  long hashCurrent();

  @Override
  default long hashGet() {
    long h = hashCurrent();
    if (h == 0) {
      h = hashCalc();
      hashPut(h);
    }
    return h;
  }

  @Override
  default long hashGet(Constant.HashType t) {
    return (hashType() == t) ? hashGet() : hashCalc(t);
  }

  void hashPut(long hash);
}
