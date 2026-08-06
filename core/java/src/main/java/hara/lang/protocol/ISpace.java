package hara.lang.protocol;

import java.util.List;

public interface ISpace {
  void contextSet(Object context, Object key, Object options);

  void contextUnset(Object context);

  List<?> contextList();

  Object contextGet(Object context);

  List<?> activeRuntimes();

  Object runtimeGet(Object context);

  Object runtimeStart(Object context);

  boolean runtimeStarted(Object context);

  boolean runtimeStopped(Object context);

  void runtimeStop(Object context);
}
