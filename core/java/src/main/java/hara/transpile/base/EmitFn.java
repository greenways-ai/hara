package hara.transpile.base;

import java.util.Map;

public interface EmitFn {
  Object apply(Object form, Map<String, Object> grammar, Map<String, Object> mopts);
}
