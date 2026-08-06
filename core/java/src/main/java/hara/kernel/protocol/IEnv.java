package hara.kernel.protocol;

import hara.lang.base.Ex;
import hara.lang.protocol.ILookup;

import java.util.Iterator;
import java.util.Map.Entry;

public interface IEnv<K, V> extends ILookup<K, V> {
  IEnv<K, V> getParent();

  ILookup<K, V> getMap();

  @SuppressWarnings("rawtypes")
  IRuntime getRuntime();

  @Override
  default Entry<K, V> find(K k) {
    Entry<K, V> e = getMap().find(k);
    if (e == null) {
      IEnv<K, V> env = getParent();
      if (env != null) {
        return env.find(k);
      }
    }
    return e;
  }

  @Override
  default Iterator<K> keys() {
    throw new Ex.Unsupported();
  }

  @Override
  default Iterator<V> vals() {
    throw new Ex.Unsupported();
  }
}
