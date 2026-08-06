package hara.lang.protocol;

import hara.lang.base.Ex;

import java.util.Map;

public interface IPair<K, V> extends Map.Entry<K, V> {
  @Override
  default V setValue(V value) {
    throw new Ex.Unsupported();
  }
}
