package hara.truffle;

import hara.lang.data.Symbol;

/** Optimized public collection operations shared by core and HAL libraries. */
public final class StdFoundationCollection {
  private StdFoundationCollection() {}

  @HaraExport(
      name = "remove",
      doc = "Returns values for which predicate is false.",
      arglists = {"[predicate value]"})
  public static Object remove(HaraContext context, Object[] values) {
    return context.removeValues(values);
  }

  @HaraExport(
      name = "reduce-kv",
      doc = "Reduces key/value entries with function and an initial value.",
      arglists = {"[function initial map]"})
  public static Object reduceKeyValues(HaraContext context, Object[] values) {
    return context.reduceKeyValues(values);
  }

  @HaraExport(
      name = "merge",
      doc = "Combines maps from left to right.",
      arglists = {"[& maps]"})
  public static Object merge(HaraContext context, Object[] values) {
    return context.mergeMaps(values);
  }

  @HaraExport(
      name = "select-keys",
      doc = "Returns a map containing the requested keys that are present.",
      arglists = {"[map keys]"})
  public static Object selectKeys(HaraContext context, Object[] values) {
    return context.selectKeys(values);
  }

  @HaraExport(
      name = "var-sym",
      doc = "converts a var to a symbol",
      arglists = {"[var]"})
  public static Object varSymbol(HaraContext context, Object[] values) {
    if (values.length != 1 || !(HaraBox.unwrap(values[0]) instanceof HaraVar variable)) {
      throw new HaraException("var-sym expects a var");
    }
    return Symbol.create(variable.namespaceName(), variable.symbolName());
  }
}
