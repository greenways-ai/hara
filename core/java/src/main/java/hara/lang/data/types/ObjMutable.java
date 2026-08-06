package hara.lang.data.types;

import hara.lang.base.Iter;
import hara.lang.protocol.*;

public abstract class ObjMutable implements IObjType, IMutable, IHash {
  public IMetadata _meta;

  public ObjMutable() {
    _meta = null;
  }

  public ObjMutable(IMetadata meta) {
    _meta = meta;
  }

  public IMetadata meta() {
    return _meta;
  }

  public ObjMutable withMeta(IMetadata meta) {
    _meta = meta;
    return this;
  }

  public String display() {
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
