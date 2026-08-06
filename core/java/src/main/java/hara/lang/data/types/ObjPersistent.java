package hara.lang.data.types;

import hara.lang.base.G;
import hara.lang.base.Iter;
import hara.lang.protocol.*;
import java.util.Map.Entry;

public abstract class ObjPersistent implements IObjType, IPersistent, IHashCached {
  public final IMetadata _meta;
  private long _hash;

  public ObjPersistent() {
    _meta = null;
  }

  public ObjPersistent(IMetadata meta) {
    _meta = meta;
  }

  public final IMetadata meta() {
    return _meta;
  }

  public long hashCurrent() {
    return _hash;
  }

  public void hashPut(long hash) {
    _hash = hash;
  }

  public String display() {
    if (this instanceof IMapType) {
      IMapType map = (IMapType) this;
      return Iter.toString(
          map.iterator(),
          "{",
          "}",
          " ",
          value -> {
            Entry entry = (Entry) value;
            return G.display(entry.getKey()) + " " + G.display(entry.getValue());
          });
    }
    if (this instanceof IColl) {
      IColl coll = (IColl) this;
      return Iter.display(coll.iterator(), coll.startString(), coll.endString(), coll.sepString());
    }
    return super.toString();
  }

  @SuppressWarnings({"unchecked", "rawtypes"})
  public String toString() {
    if (this instanceof IColl) {
      return getClass().getName()
          + "<"
          + Iter.display(((IColl) this).iterator(), "", "", ",")
          + ">";
    } else {
      return getClass().getName() + "<" + display() + ">";
    }
  }
}
