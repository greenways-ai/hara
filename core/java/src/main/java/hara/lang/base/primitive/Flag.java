package hara.lang.base.primitive;

import hara.lang.protocol.IDeref;
import hara.lang.protocol.IDisplay;
import hara.lang.protocol.IReset;

public class Flag implements IDeref<Boolean>, IReset<Boolean>, IDisplay {
  private boolean _val;

  public Flag(boolean val) {
    _val = val;
  }

  @Override
  public Boolean deref() {
    return _val;
  }

  @Override
  public String display() {
    return "#flag <" + _val + ">";
  }

  @Override
  public Boolean reset(Boolean val) {
    return _val = val;
  }
}
