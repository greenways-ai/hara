package hara.transpile.base;

import hara.lang.data.Keyword;
import java.util.*;

public class EmitBlock {
  public static String emitBlock(Object key, Object form, Map grammar, Map mopts) {
    // Simple default block implementation
    List args = (List) form;
    if (args.size() > 1) args = args.subList(1, args.size());

    return "{\n" + emitDo(args, grammar, mopts) + "\n}";
  }

  public static String emitDo(List args, Map grammar, Map mopts) {
    List<String> statements = EmitCommon.emitArray(args, grammar, mopts);
    return String.join(";\n", statements) + ";";
  }

  public static String emitBlockBody(Object key, Object block, List args, Map grammar, Map mopts) {
    return emitDo(args, grammar, mopts);
  }
}
