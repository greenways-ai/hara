package hara.lang.base.primitive;

import hara.lang.protocol.IDeref;
import hara.lang.protocol.IDisplay;
import hara.lang.protocol.IReset;

public final class Volatile<V> implements IDeref<V>, IReset<V>, IDisplay {

  public volatile V _val;

  public Volatile(V val) {
    _val = val;
  }

  @Override
  public V deref() {
    return _val;
  }

  @Override
  public String display() {
    return "#vol <" + _val + ">";
  }

  @Override
  public V reset(V newval) {
    return _val = newval;
  }
}
