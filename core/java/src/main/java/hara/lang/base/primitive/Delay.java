package hara.lang.base.primitive;

import hara.lang.base.Ex;
import hara.lang.base.G;
import hara.lang.protocol.IDeref;
import hara.lang.protocol.IDisplay;
import hara.lang.protocol.IRealize;

import java.util.function.Supplier;

public class Delay<V> implements IDeref<V>, IRealize<V>, IDisplay {
  volatile Throwable _ex;
  volatile Supplier<V> _fn;
  volatile V _val;

  public Delay(Supplier<V> fn) {
    _fn = fn;
    _val = null;
    _ex = null;
  }

  @Override
  public V deref() {
    if (_fn != null) {
      synchronized (this) {
        // double check
        if (_fn != null) {
          try {
            _val = _fn.get();
          } catch (Throwable t) {
            _ex = t;
          }
          _fn = null;
        }
      }
    }
    if (_ex != null) throw Ex.Sneaky(_ex);
    return _val;
  }

  @Override
  public String display() {
    return isRealized() ? "#delay <" + G.display(_val) + ">" : "#delay.pending<>";
  }

  @Override
  public synchronized boolean isRealized() {
    return _fn == null;
  }

  @Override
  public V realize() {
    return deref();
  }
}
