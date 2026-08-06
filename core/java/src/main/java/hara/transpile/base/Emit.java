package hara.transpile.base;

import java.util.Map;

public class Emit {
  public static String emit(Object form, Map grammar, String namespace, Map mopts) {
    // Implementation of emit
    return EmitCommon.emitCommonLoop(form, grammar, mopts).toString();
  }
}
