package hara.transpile.base;

import hara.lang.data.Keyword;
import hara.lang.base.G;
import java.util.*;

public class EmitFnImpl {
  public static String emitFn(Object key, Object form, Map grammar, Map mopts) {
    // Simple fn implementation: function name(args) { body }
    List args = (List) form;

    Object name = null;
    Object params = null;
    List body = null;

    int idx = 1;
    if (args.size() > idx && args.get(idx) instanceof hara.lang.data.Symbol) {
      name = args.get(idx);
      idx++;
    }

    if (idx < args.size()) {
      params = args.get(idx);
      idx++;
    }

    if (idx < args.size()) {
      body = args.subList(idx, args.size());
    } else {
      body = Collections.emptyList();
    }

    String nameStr = name != null ? " " + G.display(name) : "";
    String paramStr = "";
    if (params instanceof List) {
      paramStr = String.join(", ", EmitCommon.emitArray((List) params, grammar, mopts));
    }

    String bodyStr = EmitBlock.emitDo(body, grammar, mopts);

    return "function" + nameStr + "(" + paramStr + ") {\n" + bodyStr + "\n}";
  }
}
