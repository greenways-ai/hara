package hara.truffle;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.types.IMapType;
import hara.lang.data.types.ISequentialType;
import hara.lang.data.types.ISetType;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.IObjType;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.function.Function;

/** Static persistent-data implementation exported exclusively as {@code std.lib.walk/*}. */
public final class StdLibWalk {
  private StdLibWalk() {}

  @HaraExport(
      name = "walk",
      doc = "Traverses form, an arbitrary data structure",
      arglists = {"[inner outer form]"})
  public static Object walk(HaraContext context, Object[] values) {
    exactly("walk", values, 3);
    return walkInternal(
        value -> invoke(context, values[0], value),
        value -> invoke(context, values[1], value),
        values[2]);
  }

  @HaraExport(
      name = "postwalk",
      doc = "Performs a depth-first, post-order traversal of form",
      arglists = {"[f form]"})
  public static Object postwalk(HaraContext context, Object[] values) {
    exactly("postwalk", values, 2);
    return postwalkInternal(context, values[0], values[1]);
  }

  @HaraExport(
      name = "prewalk",
      doc = "Like postwalk, but does pre-order traversal.",
      arglists = {"[f form]"})
  public static Object prewalk(HaraContext context, Object[] values) {
    exactly("prewalk", values, 2);
    return prewalkInternal(context, values[0], values[1]);
  }

  @HaraExport(
      name = "keywordize-keys",
      doc = "Recursively transforms all map keys from strings to keywords.",
      arglists = {"[m]"})
  public static Object keywordizeKeys(HaraContext context, Object[] values) {
    exactly("keywordize-keys", values, 1);
    return postwalkInternal(
        function(
            value ->
                transformMapKeys(
                    value,
                    key ->
                        key instanceof String || key instanceof Symbol
                            ? keyword(key)
                            : key)),
        values[0]);
  }

  @HaraExport(
      name = "keyword-spearify-keys",
      doc = "recursively transfroms all map keys to spearcase",
      arglists = {"[m]"})
  public static Object keywordSpearifyKeys(HaraContext context, Object[] values) {
    exactly("keyword-spearify-keys", values, 1);
    return postwalkInternal(
        function(
            value ->
                transformMapKeys(
                    value,
                    key ->
                        key instanceof String text
                            ? Keyword.create(text.replace('_', '-'))
                            : key)),
        values[0]);
  }

  @HaraExport(
      name = "stringify-keys",
      doc = "Recursively transforms all map keys from keywords to strings.",
      arglists = {"[m]"})
  public static Object stringifyKeys(HaraContext context, Object[] values) {
    exactly("stringify-keys", values, 1);
    return postwalkInternal(
        function(
            value ->
                transformMapKeys(
                    value,
                    key -> key instanceof Keyword keyword ? keyword.getName() : key)),
        values[0]);
  }

  @HaraExport(
      name = "string-snakify-keys",
      doc = "recursively transforms keyword to string keys",
      arglists = {"[m]"})
  public static Object stringSnakifyKeys(HaraContext context, Object[] values) {
    exactly("string-snakify-keys", values, 1);
    return postwalkInternal(
        function(
            value ->
                transformMapKeys(
                    value,
                    key ->
                        key instanceof Keyword keyword
                            ? keyword.getName().replace('-', '_')
                            : key)),
        values[0]);
  }

  @HaraExport(
      name = "prewalk-replace",
      doc = "Recursively transforms form by replacing keys in smap with their values.",
      arglists = {"[smap form]"})
  public static Object prewalkReplace(HaraContext context, Object[] values) {
    exactly("prewalk-replace", values, 2);
    return prewalkInternal(lookupFunction(values[0]), values[1]);
  }

  @HaraExport(
      name = "postwalk-replace",
      doc = "Recursively transforms form by replacing keys in smap with their values.",
      arglists = {"[smap form]"})
  public static Object postwalkReplace(HaraContext context, Object[] values) {
    exactly("postwalk-replace", values, 2);
    return postwalkInternal(lookupFunction(values[0]), values[1]);
  }

  @HaraExport(
      name = "macroexpand-all",
      doc = "Recursively performs all possible macroexpansions in form.",
      arglists = {"[form]"})
  public static Object macroexpandAll(HaraContext context, Object[] values) {
    exactly("macroexpand-all", values, 1);
    return prewalkInternal(
        function(
            value ->
                value instanceof hara.lang.data.List<?>
                    ? context.macroExpand(value, true)
                    : value),
        values[0]);
  }

  @HaraExport(
      name = "walk:contains",
      doc = "recursively walks form to check for containment",
      arglists = {"[pred form]"})
  public static Object walkContains(HaraContext context, Object[] values) {
    exactly("walk:contains", values, 2);
    boolean[] found = {false};
    prewalkInternal(
        function(
            value -> {
              try {
                if (truthy(invoke(context, values[0], value))) found[0] = true;
              } catch (RuntimeException ignored) {
                // Foundation's walk predicate helpers deliberately ignore non-matching errors.
              }
              return value;
            }),
        values[1]);
    return found[0];
  }

  @HaraExport(
      name = "walk:find",
      doc = "recursively walks to find all matching forms",
      arglists = {"[pred form]"})
  public static Object walkFind(HaraContext context, Object[] values) {
    exactly("walk:find", values, 2);
    LinkedHashSet<Object> found = new LinkedHashSet<>();
    postwalkInternal(
        function(
            value -> {
              try {
                if (truthy(invoke(context, values[0], value))) found.add(value);
              } catch (RuntimeException ignored) {
                // Foundation's walk predicate helpers deliberately ignore non-matching errors.
              }
              return value;
            }),
        values[1]);
    return hara.lang.data.Set.Standard.from(null, found.toArray());
  }

  @HaraExport(
      name = "walk:keep",
      doc = "recursively walks and keeps all processed forms",
      arglists = {"[f form]"})
  public static Object walkKeep(HaraContext context, Object[] values) {
    exactly("walk:keep", values, 2);
    LinkedHashSet<Object> found = new LinkedHashSet<>();
    postwalkInternal(
        function(
            value -> {
              try {
                Object result = invoke(context, values[0], value);
                if (truthy(result)) found.add(result);
              } catch (RuntimeException ignored) {
                // Foundation's walk predicate helpers deliberately ignore non-matching errors.
              }
              return value;
            }),
        values[1]);
    return hara.lang.data.Set.Standard.from(null, found.toArray());
  }

  private static Object prewalkInternal(HaraContext context, Object callable, Object form) {
    return prewalkInternal(value -> invoke(context, callable, value), form);
  }

  private static Object prewalkInternal(Function<Object, Object> function, Object form) {
    Object transformed = function.apply(form);
    return walkInternal(
        child -> prewalkInternal(function, child), Function.identity(), transformed);
  }

  private static Object postwalkInternal(HaraContext context, Object callable, Object form) {
    return postwalkInternal(value -> invoke(context, callable, value), form);
  }

  private static Object postwalkInternal(Function<Object, Object> function, Object form) {
    return walkInternal(
        child -> postwalkInternal(function, child), function, form);
  }

  private static Object walkInternal(
      Function<Object, Object> inner, Function<Object, Object> outer, Object form) {
    IMetadata metadata = form instanceof IObjType value ? value.meta() : null;
    if (form instanceof hara.lang.data.List<?> list) {
      return outer.apply(
          hara.lang.data.List.Standard.from(metadata, transform(inner, list).toArray()));
    }
    if (form instanceof Map.Entry<?, ?> entry) {
      return outer.apply(
          hara.lang.data.Vector.Standard.from(
              null, inner.apply(entry.getKey()), inner.apply(entry.getValue())));
    }
    if (form instanceof IMapType<?, ?> map) {
      ArrayList<Object> entries = new ArrayList<>((int) map.count() * 2);
      for (Object value : map) {
        Object transformed = inner.apply(value);
        Object[] pair = pair(transformed);
        entries.add(pair[0]);
        entries.add(pair[1]);
      }
      return outer.apply(rebuildMap(map, entries));
    }
    if (form instanceof ISetType<?> set) {
      return outer.apply(rebuildSet(set, transform(inner, set)));
    }
    if (form instanceof hara.lang.data.Vector<?> vector) {
      return outer.apply(
          hara.lang.data.Vector.Standard.from(metadata, transform(inner, vector).toArray()));
    }
    if (form instanceof ISequentialType<?> sequence) {
      return outer.apply(
          hara.lang.data.List.Standard.from(metadata, transform(inner, sequence).toArray()));
    }
    return outer.apply(form);
  }

  private static ArrayList<Object> transform(
      Function<Object, Object> function, Iterable<?> values) {
    ArrayList<Object> result = new ArrayList<>();
    for (Object value : values) result.add(function.apply(value));
    return result;
  }

  private static Object transformMapKeys(Object value, Function<Object, Object> transform) {
    if (!(value instanceof IMapType<?, ?> map)) return value;
    ArrayList<Object> entries = new ArrayList<>((int) map.count() * 2);
    for (Object entryObject : map) {
      Map.Entry<?, ?> entry = (Map.Entry<?, ?>) entryObject;
      entries.add(transform.apply(entry.getKey()));
      entries.add(entry.getValue());
    }
    return rebuildMap(map, entries);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object rebuildMap(IMapType<?, ?> template, java.util.List<Object> entries) {
    IMapType result = (IMapType) template.empty();
    for (int index = 0; index < entries.size(); index += 2) {
      result = (IMapType) result.assoc(entries.get(index), entries.get(index + 1));
    }
    return result;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object rebuildSet(ISetType<?> template, java.util.List<Object> values) {
    ISetType result = (ISetType) template.empty();
    for (Object value : values) result = (ISetType) result.conj(value);
    return result;
  }

  private static Function<Object, Object> lookupFunction(Object value) {
    if (!(value instanceof IMapType<?, ?> replacements)) {
      throw new HaraException("walk replacement map must be a map");
    }
    return function(
        candidate -> {
          @SuppressWarnings("rawtypes")
          Map.Entry<?, ?> entry = (Map.Entry<?, ?>) ((IMapType) replacements).find(candidate);
          return entry == null ? candidate : entry.getValue();
        });
  }

  private static Object[] pair(Object value) {
    if (value instanceof Map.Entry<?, ?> entry) {
      return new Object[] {entry.getKey(), entry.getValue()};
    }
    if (value instanceof ISequentialType<?> sequence && sequence.count() == 2) {
      java.util.Iterator<?> iterator = sequence.iterator();
      return new Object[] {iterator.next(), iterator.next()};
    }
    throw new HaraException("walk map entry transform must return a pair");
  }

  private static Keyword keyword(Object value) {
    if (value instanceof Symbol symbol) {
      return Keyword.create(symbol.getNamespace(), symbol.getName());
    }
    return Keyword.create((String) value);
  }

  private static Function<Object, Object> function(Function<Object, Object> function) {
    return function;
  }

  private static Object invoke(HaraContext context, Object callable, Object value) {
    return HaraBox.unwrap(context.invokeCallable(callable, new Object[] {value}));
  }

  private static boolean truthy(Object value) {
    Object unwrapped = HaraBox.unwrap(value);
    return unwrapped != null && unwrapped != HaraNull.SINGLETON && !Boolean.FALSE.equals(unwrapped);
  }

  private static void exactly(String name, Object[] values, int count) {
    if (values.length != count) {
      throw new HaraException(name + " expects " + count + " arguments");
    }
  }
}
