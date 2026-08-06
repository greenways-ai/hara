package hara.lang.base.primitive;

import hara.lang.protocol.IDeref;
import hara.lang.protocol.IDisplay;
import hara.lang.protocol.IReset;

public class Counter implements IDeref<Integer>, IReset<Integer>, IDisplay {

  private int _c;

  public Counter(int count) {
    _c = count;
  }

  public int dec() {
    return _c -= 1;
  }

  public int dec(int n) {
    return _c -= n;
  }

  @Override
  public Integer deref() {
    return _c;
  }

  @Override
  public String display() {
    return "#counter <" + _c + ">";
  }

  public int inc() {
    return _c += 1;
  }

  public int inc(int n) {
    return _c += n;
  }

  @Override
  public Integer reset(Integer count) {
    return _c = count;
  }
}
