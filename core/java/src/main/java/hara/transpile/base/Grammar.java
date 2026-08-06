package hara.transpile.base;

import hara.lang.data.Keyword;
import java.util.Map;
import java.util.HashMap;

public class Grammar {
  public Keyword tag;
  public Map reserved;
  public Map structure;
  public Map sections;
  public Map banned;
  public Map highlight;
  public Map macros;

  // Constructor-like builder
  public static Grammar create(Keyword tag, Map reserved, Map template) {
    Grammar g = new Grammar();
    g.tag = tag;
    g.reserved = reserved;
    // Merge template...
    return g;
  }
}
