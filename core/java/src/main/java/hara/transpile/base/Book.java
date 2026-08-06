package hara.transpile.base;

import hara.lang.data.Symbol;
import hara.lang.data.Keyword;
import java.util.*;

public class Book {
  public Keyword lang;
  public Map meta;
  public Grammar grammar;
  public Map modules;
  public Keyword parent;

  public Book(Keyword lang, Map meta, Grammar grammar, Map modules, Keyword parent) {
    this.lang = lang;
    this.meta = meta;
    this.grammar = grammar;
    this.modules = modules;
    this.parent = parent;
  }
}
