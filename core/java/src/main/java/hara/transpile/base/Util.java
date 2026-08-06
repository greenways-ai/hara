package hara.transpile.base;

import hara.lang.data.Symbol;
import hara.lang.data.Keyword;
import java.util.*;

public class Util {
  public static Symbol symId(Object id) {
    if (id instanceof Symbol) return Symbol.create(((Symbol) id).getName());
    return Symbol.create(id.toString());
  }

  public static Symbol symModule(Object id) {
    if (id instanceof Symbol) {
      String ns = ((Symbol) id).getNamespace();
      if (ns != null) return Symbol.create(ns);
    }
    return null;
  }

  public static List<Symbol> symPair(Object id) {
    return Arrays.asList(symModule(id), symId(id));
  }

  public static Symbol symFull(Object module, Object id) {
    return Symbol.create(module.toString(), id.toString());
  }

  public static String symDefaultStr(Object sym) {
    return sym.toString().replace("-", "_");
  }

  public static String symDefaultInverseStr(Object sym) {
    return sym.toString().replace("_", "-");
  }

  public static boolean hashvecQ(Object x) {
    if (x instanceof Set && ((Set) x).size() == 1) {
      Object first = ((Set) x).iterator().next();
      return first instanceof List || first instanceof java.util.Vector;
    }
    return false;
  }

  public static boolean doublevecQ(Object x) {
    if (x instanceof List && ((List) x).size() == 1) {
      Object first = ((List) x).get(0);
      return first instanceof List;
    }
    return false;
  }
}
