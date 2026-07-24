package hara.truffle;

import hara.lang.data.Keyword;
import hara.lang.data.List;
import hara.lang.data.Symbol;
import hara.lang.data.Vector;
import hara.lang.data.types.IMapType;
import hara.lang.data.types.ILinearType;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;

/** Fully validated, immutable interpretation of an ns declaration. */
final class HaraNamespaceDeclaration {
  private static final Set<String> INTRINSIC_LIBRARIES =
      Set.of("string", "bytes", "promise", "coroutine", "file", "os");

  final Symbol name;
  final boolean blank;
  final java.util.List<Symbol> builtins;
  final Set<String> excludedIntrinsics;
  final Map<String, String> intrinsicAliases;
  final Object[] structuralClauses;

  private HaraNamespaceDeclaration(
      Symbol name,
      boolean blank,
      java.util.List<Symbol> builtins,
      Set<String> excludedIntrinsics,
      Map<String, String> intrinsicAliases,
      Object[] structuralClauses) {
    this.name = name;
    this.blank = blank;
    this.builtins = java.util.List.copyOf(builtins);
    this.excludedIntrinsics = Set.copyOf(excludedIntrinsics);
    this.intrinsicAliases = Map.copyOf(intrinsicAliases);
    this.structuralClauses = structuralClauses.clone();
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  static HaraNamespaceDeclaration parse(Symbol name, Object[] clauses) {
    if (name.getNamespace() != null) {
      throw new HaraException("ns name must be an unqualified symbol");
    }
    boolean configSeen = false;
    boolean blank = false;
    ArrayList<Symbol> builtins = new ArrayList<>();
    LinkedHashSet<String> excluded = new LinkedHashSet<>();
    LinkedHashMap<String, String> aliases = new LinkedHashMap<>();
    ArrayList<Object> structural = new ArrayList<>();

    for (Object clauseValue : clauses) {
      if (!(clauseValue instanceof List<?> clause) || clause.count() == 0) {
        throw new HaraException("ns clauses must be non-empty lists");
      }
      if (!(clause.nth(0) instanceof Keyword keyword) || keyword.getNamespace() != null) {
        throw new HaraException("ns clause must start with an unqualified keyword");
      }
      String clauseName = keyword.getName();
      if ("config".equals(clauseName)) {
        if (configSeen) throw new HaraException("ns accepts only one :config clause");
        configSeen = true;
        if (clause.count() != 2 || !(clause.nth(1) instanceof IMapType<?, ?>)) {
          throw new HaraException(":config expects one map");
        }
        IMapType options = (IMapType) clause.nth(1);
        Iterator<?> iterator = options.iterator();
        while (iterator.hasNext()) {
          java.util.Map.Entry<?, ?> entry = (java.util.Map.Entry<?, ?>) iterator.next();
          if (!(entry.getKey() instanceof Keyword option) || option.getNamespace() != null) {
            throw new HaraException(":config keys must be unqualified keywords");
          }
          if (!Set.of("blank", "builtins", "intrinsics").contains(option.getName())) {
            throw new HaraException("Unsupported :config option: :" + option.getName());
          }
        }
        Object blankValue = options.lookup(Keyword.create("blank"));
        if (blankValue != null) {
          if (!(blankValue instanceof Boolean)) {
            throw new HaraException(":config :blank expects a boolean");
          }
          blank = (Boolean) blankValue;
        }
        Object builtinValue = options.lookup(Keyword.create("builtins"));
        if (builtinValue != null) parseBuiltins(builtinValue, builtins);
        Object intrinsicValue = options.lookup(Keyword.create("intrinsics"));
        if (intrinsicValue != null) parseIntrinsics(intrinsicValue, excluded, aliases);
      } else if ("require".equals(clauseName)
          || "flavor".equals(clauseName)
          || "import".equals(clauseName)) {
        structural.add(clause);
      } else if ("intrinsics".equals(clauseName) || "builtins".equals(clauseName)) {
        throw new HaraException(":" + clauseName + " is valid only inside ns :config");
      } else {
        throw new HaraException("Unsupported ns clause: :" + clauseName);
      }
    }
    for (String library : aliases.keySet()) {
      if (excluded.contains(library)) {
        throw new HaraException(
            "Intrinsic library cannot be both excluded and aliased: " + library);
      }
    }
    return new HaraNamespaceDeclaration(
        name, blank, builtins, excluded, aliases, structural.toArray());
  }

  private static void parseBuiltins(Object value, java.util.List<Symbol> output) {
    if (!(value instanceof Vector<?> vector)) {
      throw new HaraException(":config :builtins expects a vector of symbols");
    }
    LinkedHashSet<String> seen = new LinkedHashSet<>();
    for (Object item : vector) {
      if (!(item instanceof Symbol symbol) || symbol.getNamespace() != null) {
        throw new HaraException(":config :builtins expects unqualified symbols");
      }
      if (!seen.add(symbol.getName())) {
        throw new HaraException("Duplicate builtin declaration: " + symbol.getName());
      }
      output.add(symbol);
    }
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static void parseIntrinsics(
      Object value, Set<String> excluded, Map<String, String> aliases) {
    if (Keyword.create("all").equals(value)) return;
    if (!(value instanceof IMapType<?, ?> options)) {
      throw new HaraException(":config :intrinsics expects :all or an options map");
    }
    Iterator<?> iterator = options.iterator();
    while (iterator.hasNext()) {
      java.util.Map.Entry<?, ?> entry = (java.util.Map.Entry<?, ?>) iterator.next();
      if (!(entry.getKey() instanceof Keyword option) || option.getNamespace() != null) {
        throw new HaraException(":config :intrinsics keys must be unqualified keywords");
      }
      if (!"exclude".equals(option.getName()) && !"alias".equals(option.getName())) {
        throw new HaraException(
            "Unsupported :config :intrinsics option: :" + option.getName());
      }
    }
    Object excludeValue = ((IMapType) options).lookup(Keyword.create("exclude"));
    if (excludeValue != null) {
      if (!(excludeValue instanceof Vector<?> vector)) {
        throw new HaraException(":config :intrinsics :exclude expects a vector");
      }
      for (Object item : vector) {
        String library = libraryName(item, ":config :intrinsics :exclude");
        if (!excluded.add(library)) {
          throw new HaraException("Duplicate intrinsic exclusion: " + library);
        }
      }
    }
    Object aliasValue = ((IMapType) options).lookup(Keyword.create("alias"));
    if (aliasValue != null) {
      if (!(aliasValue instanceof IMapType<?, ?> aliasMap)) {
        throw new HaraException(":config :intrinsics :alias expects a map");
      }
      LinkedHashSet<String> usedAliases = new LinkedHashSet<>();
      for (Object entryValue : aliasMap) {
        java.util.Map.Entry<?, ?> entry = (java.util.Map.Entry<?, ?>) entryValue;
        String library = libraryName(entry.getKey(), ":config :intrinsics :alias");
        if (!(entry.getValue() instanceof Symbol alias) || alias.getNamespace() != null) {
          throw new HaraException("Intrinsic aliases must be unqualified symbols");
        }
        if (!usedAliases.add(alias.getName())) {
          throw new HaraException("Duplicate intrinsic alias target: " + alias.getName());
        }
        if (aliases.put(library, alias.getName()) != null) {
          throw new HaraException("Duplicate intrinsic alias: " + library);
        }
      }
    }
  }

  private static String libraryName(Object value, String operation) {
    if (!(value instanceof Symbol symbol) || symbol.getNamespace() != null) {
      throw new HaraException(operation + " expects unqualified library symbols");
    }
    if (!INTRINSIC_LIBRARIES.contains(symbol.getName())) {
      throw new HaraException("Unknown intrinsic library: " + symbol.getName());
    }
    return symbol.getName();
  }
}
