package hara.transpile.base;

import hara.lang.data.Symbol;
import hara.lang.data.Keyword;
import java.util.*;

public class EmitHelper {
  public static final Map<String, Object> DEFAULT_GRAMMAR = new HashMap<>();

  public static final Map<Character, String> SYM_REPLACE = new HashMap<>();

  static {
    SYM_REPLACE.put('-', "_");
    SYM_REPLACE.put('*', "__");
    SYM_REPLACE.put('?', "p");
    SYM_REPLACE.put('!', "f");
    SYM_REPLACE.put('=', "_eq");
    SYM_REPLACE.put('<', "_lt");
    SYM_REPLACE.put('>', "_gt");
  }

  public static final EmitFn defaultEmitFn =
      (form, grammar, mopts) -> {
        return hara.lang.base.G.display(form);
      };

  public static String prSingle(String s) {
    return "'"
        + s.replace("'", "\\'")
            .replaceAll("^\"", "'")
            .replaceAll("\"$", "'")
            .replaceAll("\\\\\"", "\"")
        + "'";
  }

  public static Object getOption(Map<String, Object> grammar, List<Object> path, Object option) {
    List<Object> defaultPath = Arrays.asList(Keyword.create("default"), Keyword.create("common"));
    Object val = getIn(grammar, append(path, option));
    if (val != null) return val;
    return getIn(grammar, append(defaultPath, option));
  }

  public static Map<String, Object> getOptions(Map<String, Object> grammar, List<Object> path) {
    Map<String, Object> common =
        (Map<String, Object>)
            getIn(grammar, Arrays.asList(Keyword.create("default"), Keyword.create("common")));
    Map<String, Object> specific = (Map<String, Object>) getIn(grammar, path);

    Map<String, Object> merged = new HashMap<>();
    if (common != null) merged.putAll(common);
    if (specific != null) merged.putAll(specific);
    return merged;
  }

  public static Object getIn(Map map, List path) {
    Object current = map;
    for (Object key : path) {
      if (current instanceof Map) {
        current = ((Map) current).get(key);
      } else {
        return null;
      }
    }
    return current;
  }

  private static List<Object> append(List<Object> list, Object item) {
    List<Object> newList = new ArrayList<>(list);
    newList.add(item);
    return newList;
  }

  public static Object formKeyBase(Object form) {
    if (form instanceof Character)
      return Arrays.asList(Keyword.create("char"), Keyword.create("token"), true);
    if (form instanceof Keyword)
      return Arrays.asList(Keyword.create("keyword"), Keyword.create("token"), true);
    if (form instanceof Symbol)
      return Arrays.asList(Keyword.create("symbol"), Keyword.create("token"), true);

    if (form instanceof List)
      return Arrays.asList(Keyword.create("expression"), Keyword.create("expression"), false);

    if (form instanceof Number)
      return Arrays.asList(Keyword.create("number"), Keyword.create("token"));
    if (form instanceof Boolean)
      return Arrays.asList(Keyword.create("boolean"), Keyword.create("token"));
    if (form instanceof String)
      return Arrays.asList(Keyword.create("string"), Keyword.create("token"));
    if (form == null) return Arrays.asList(Keyword.create("nil"), Keyword.create("token"));

    if (form instanceof Map) return Arrays.asList(Keyword.create("map"), Keyword.create("data"));
    if (form instanceof Set) return Arrays.asList(Keyword.create("set"), Keyword.create("data"));

    throw new RuntimeException("Not Valid: " + form);
  }

  public static String emitToken(Object key, Object token, Map grammar, Map mopts) {
    // Look up grammar token options
    Map tokenOpts = (Map) getIn(grammar, Arrays.asList(Keyword.create("token"), key));
    if (tokenOpts != null) {
      // handle emit, as, custom
      // stub
    }
    return hara.lang.base.G.display(token);
  }

  public static String emitSymbolFull(Symbol sym, String ns, Map grammar) {
    // ... (as before)
    return sym.getName();
  }
}
