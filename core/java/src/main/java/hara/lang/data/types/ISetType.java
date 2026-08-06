package hara.lang.data.types;

import hara.lang.base.Iter;
import hara.lang.protocol.*;

public interface ISetType<E>
    extends IColl<E>, IObjType, IDissoc<E>, IFind<E, E>, IUnOrderedType<E>, IFn<E, E, E> {

  default java.util.Set<E> asJavaSet() {
    return null;
  }

  @Override
  default Constant.ObjType getObjType() {
    return Constant.ObjType.SET;
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  @Override
  default boolean equality(Object obj) {
    if (obj instanceof ISetType) {
      return (count() == ((ISetType) obj).count())
          && Iter.every(this.iterator(), (e) -> ((ISetType) obj).find(e) != null);
    } else if (obj instanceof java.util.Set) {
      return (this.count() == ((java.util.Set) obj).size())
          && Iter.every(((java.util.Set) obj).iterator(), (e) -> this.find((E) e) != null);
    } else {
      return false;
    }
  }

  @Override
  default String startString() {
    return "#{";
  }

  @Override
  default String endString() {
    return "}";
  }

  @Override
  public default E invoke(E key) {
    return find(key);
  }

  @Override
  public default E invoke(E key, E notFound) {
    var ret = find(key);
    return (ret == null) ? notFound : ret;
  }
}
