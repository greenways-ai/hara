package hara.truffle;

/** Optimized public sequence operations backed by the Truffle iterator runtime. */
public final class StdFoundationSequence {
  private StdFoundationSequence() {}

  public static Object map(HaraContext context, Object[] values) {
    return context.mapValues(values);
  }

  @HaraExport(
      name = "reduce",
      doc = "Reduces a collection with function and an optional initial value.",
      arglists = {"[function value]", "[function initial value]"})
  public static Object reduce(HaraContext context, Object[] values) {
    return context.reduceIterator(values);
  }

  @HaraExport(
      name = "cycle",
      doc = "Returns a lazy sequence that repeats the values in a collection.",
      arglists = {"[value]"})
  public static Object cycle(HaraContext context, Object[] values) {
    HaraContext.requireMethodArity("cycle", values, 1);
    return context.seqValue(new Object[] {context.iterCycle(values[0])});
  }

  @HaraExport(
      name = "partition",
      doc = "Partitions input lazily, or creates an eager partition transform.",
      arglists = {"[amount]", "[amount value]"})
  public static Object partition(HaraContext context, Object[] values) {
    return context.partitionValues(values, false);
  }

  @HaraExport(
      name = "partition-all",
      doc = "Partitions input lazily including a partial tail, or creates an eager transform.",
      arglists = {"[amount]", "[amount value]"})
  public static Object partitionAll(HaraContext context, Object[] values) {
    return context.partitionValues(values, true);
  }

  @HaraExport(
      name = "filter",
      doc = "Returns an iterator transform or eagerly filters a collection.",
      arglists = {"[predicate]", "[predicate value]"})
  public static Object filter(HaraContext context, Object[] values) {
    return context.filterValues(values);
  }

  @HaraExport(
      name = "take",
      doc = "Returns an iterator transform or eagerly takes from a collection.",
      arglists = {"[amount]", "[amount value]"})
  public static Object take(HaraContext context, Object[] values) {
    return context.takeValues(values);
  }

  @HaraExport(
      name = "drop",
      doc = "Returns an iterator transform or eagerly drops from a collection.",
      arglists = {"[amount]", "[amount value]"})
  public static Object drop(HaraContext context, Object[] values) {
    return context.dropValues(values);
  }
}
