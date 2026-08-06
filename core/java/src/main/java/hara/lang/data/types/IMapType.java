package hara.lang.data.types;

import hara.lang.base.Eq;
import hara.lang.base.G;
import hara.lang.base.Iter;
import hara.lang.protocol.*;

import java.util.Iterator;
import java.util.Map;
import java.util.Map.Entry;
import java.util.function.BiFunction;
import java.util.function.Function;

public interface IMapType<K, V>
    extends IColl<Entry<K, V>>,
        IObjType,
        IMetadata,
        ILookup<K, V>,
        IAssoc<K, V>,
        IDissoc<K>,
        IFind<K, Entry<K, V>>,
        IUnOrderedType<Entry<K, V>>,
        IFn<V, K, V> {
  default java.util.Map<K, V> asJavaMap() {
    return null;
  }

  @Override
  default Constant.MetaType getMetatype() {
    return Constant.MetaType.MAP;
  }

  @Override
  default Constant.ObjType getObjType() {
    return Constant.ObjType.MAP;
  }

  @Override
  default IMapType<K, V> conj(Entry<K, V> e) {
    return (IMapType<K, V>) assoc(e.getKey(), e.getValue());
  }

  @Override
  default V lookup(K key) {
    Entry<K, V> e = find(key);
    return (e == null) ? null : e.getValue();
  }

  @Override
  default V lookup(K key, V notFound) {
    Entry<K, V> e = find(key);
    return (e == null) ? notFound : e.getValue();
  }

  @Override
  default Iterator<K> keys() {
    return Iter.map(iterator(), (n) -> n.getKey());
  }

  @Override
  default Iterator<V> vals() {
    return Iter.map(iterator(), (n) -> n.getValue());
  }

  @Override
  default String startString() {
    return "{";
  }

  @Override
  default String endString() {
    return "}";
  }

  @Override
  default String sepString() {
    return ", ";
  }

  @Override
  default String display() {
    return Iter.toString(
        iterator(),
        startString(),
        endString(),
        sepString(),
        (o) -> G.display(o.getKey()) + " " + G.display(o.getValue()));
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  @Override
  default boolean equality(Object obj) {

    if (obj instanceof IMapType) {
      return (count() == ((IMapType) obj).count())
          && Iter.every(
              this.iterator(),
              (e) -> {
                Map.Entry oe = (Entry) ((IMapType) obj).find(e.getKey());
                return oe != null && Eq.eq(oe.getValue(), e.getValue());
              });
    } else if (obj instanceof java.util.Map) {
      return (this.count() == ((java.util.Map) obj).size())
          && Iter.every(
              ((java.util.Map) obj).entrySet().iterator(),
              (e) -> {
                Map.Entry oe = (Map.Entry) e;
                Map.Entry te = (Map.Entry) ((IMapType) this).find(oe.getKey());
                return te != null && Eq.eq(te.getValue(), oe.getValue());
              });
    } else {
      return false;
    }
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  @Override
  public default Function getArg1() {
    return key -> lookup((K) key);
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  @Override
  public default BiFunction getArg2() {
    return (key, notFound) -> lookup((K) key, (V) notFound);
  }
}
