package std.lib.block;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/** Expression extraction, modifier application, and container values. */
public final class Value {
  private Value() {}

  /** Java representation of a value carrying Clojure-style metadata. */
  public record WithMetadata(Object value, Map<Object, Object> metadata) {}

  public static List<Block> applyModifiers(Iterable<? extends Block> blocks) {
    List<Block> out = new ArrayList<>();
    List<Block.Modifier> modifiers = new ArrayList<>();
    for (Block block : blocks) {
      if (block instanceof Block.Modifier modifier) {
        modifiers.add(0, modifier);
      } else if (modifiers.isEmpty()) {
        out.add(block);
      } else {
        Block.Modifier modifier = modifiers.remove(0);
        out = modifier.modify(out, block);
      }
    }
    return out;
  }

  public static List<Object> childValues(Block.Container block) {
    List<Block> expressions = new ArrayList<>();
    for (Block child : block.children) {
      if (child.expression() || child.modifier()) expressions.add(child);
    }
    List<Object> values = new ArrayList<>();
    for (Block child : applyModifiers(expressions)) {
      if (child instanceof Block.IExpression expression) values.add(expression.value());
    }
    return values;
  }

  public static Object containerValue(Block.Container block) {
    List<Object> values = childValues(block);
    return switch (block.tag) {
      case "root" -> form("do", values);
      case "list" -> List.copyOf(values);
      case "vector" -> java.util.Collections.unmodifiableList(new ArrayList<>(values));
      case "map" -> map(values);
      case "set" -> java.util.Collections.unmodifiableSet(new LinkedHashSet<>(values));
      case "quote" -> unary("quote", values);
      case "deref" -> unary("deref", values);
      case "var" -> unary("var", values);
      case "unquote" -> unary("unquote", values);
      case "unquote-splice" -> unary("unquote-splicing", values);
      case "syntax" -> unary("syntax-quote", values);
      case "meta", "hash-meta" -> metadata(values);
      case "hash-keyword" -> namespacedMap(values);
      case "select" -> selection("?", values);
      case "select-splice" -> selection("?-splicing", values);
      case "hash-eval" -> unary("eval", values);
      case "fn" -> anonymousFunction(values);
      default -> throw new IllegalStateException("No value implementation for block tag: " + block.tag);
    };
  }

  private static List<Object> form(String name, List<Object> values) {
    List<Object> out = new ArrayList<>(values.size() + 1);
    out.add(Symbol.create(name)); out.addAll(values);
    return java.util.Collections.unmodifiableList(out);
  }
  private static List<Object> unary(String name, List<Object> values) {
    if (values.size() != 1) throw new IllegalArgumentException(name + " expects one value");
    return List.of(Symbol.create(name), values.get(0));
  }
  private static Map<Object, Object> map(List<Object> values) {
    if ((values.size() & 1) != 0) throw new IllegalArgumentException("Map requires an even number of forms");
    Map<Object, Object> out = new LinkedHashMap<>();
    for (int i = 0; i < values.size(); i += 2) out.put(values.get(i), values.get(i + 1));
    return java.util.Collections.unmodifiableMap(out);
  }
  private static Object metadata(List<Object> values) {
    if (values.size() != 2) throw new IllegalArgumentException("Metadata expects metadata and a value");
    Object metadata = values.get(0);
    Map<Object, Object> out = new LinkedHashMap<>();
    if (metadata instanceof Map<?, ?> map) out.putAll((Map<?, ?>) map);
    else if (metadata instanceof Keyword keyword) out.put(keyword, true);
    else out.put(Keyword.create("tag"), metadata);
    return new WithMetadata(values.get(1), java.util.Collections.unmodifiableMap(out));
  }
  private static Object namespacedMap(List<Object> values) {
    if (values.size() != 2 || !(values.get(0) instanceof Keyword namespace)
        || !(values.get(1) instanceof Map<?, ?> input))
      throw new IllegalArgumentException("Namespaced map expects a keyword and map");
    String ns = namespace.getName();
    Map<Object, Object> out = new LinkedHashMap<>();
    for (Map.Entry<?, ?> entry : input.entrySet()) {
      Object key = entry.getKey();
      if (key instanceof Keyword keyword && keyword.getNamespace() == null)
        key = Keyword.create(ns, keyword.getName());
      out.put(key, entry.getValue());
    }
    return java.util.Collections.unmodifiableMap(out);
  }
  private static Object selection(String name, List<Object> values) {
    if (values.size() != 1 || !(values.get(0) instanceof List<?> selection))
      throw new IllegalArgumentException("Reader conditional expects a list");
    return List.of(Symbol.create(name), map(new ArrayList<>(selection)));
  }
  private static Object anonymousFunction(List<Object> values) {
    return List.of(Symbol.create("fn*"), List.of(), java.util.Collections.unmodifiableList(values));
  }
}
