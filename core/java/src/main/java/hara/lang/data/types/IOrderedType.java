package hara.lang.data.types;

import hara.lang.base.G;
import hara.lang.base.Iter;
import hara.lang.protocol.Constant;
import hara.lang.protocol.IHash;

import java.util.function.Function;

public interface IOrderedType<E> extends Iterable<E>, IHash {

  @Override
  default long hashCalc(Constant.HashType t) {

    Function<Object, Long> f = G.hashFn(t);
    return Iter.reduce(
        iterator(), Long.valueOf(hashSeed().hashCode()), (acc, item) -> (acc * 31) + f.apply(item));
  }
}
