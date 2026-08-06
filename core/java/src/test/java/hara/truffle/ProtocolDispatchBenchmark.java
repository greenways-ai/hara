package hara.truffle;

import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;

/**
 * Manual protocol-dispatch benchmark.
 *
 * <p>Run this class with a GraalVM/JVMCI-enabled Java runtime after test compilation. It is kept
 * outside the JUnit naming convention because timing assertions are not stable build tests.
 */
public final class ProtocolDispatchBenchmark {
  private static final int DEFAULT_WARMUP = 5;
  private static final int DEFAULT_ITERATIONS = 100_000;

  private ProtocolDispatchBenchmark() {}

  public static void main(String[] args) {
    int warmup = args.length > 0 ? Integer.parseInt(args[0]) : DEFAULT_WARMUP;
    int iterations = args.length > 1 ? Integer.parseInt(args[1]) : DEFAULT_ITERATIONS;

    runCase("arithmetic", ARITHMETIC_PROGRAM, new String[] {"1"}, 2, warmup, iterations);
    runCase("direct-call", DIRECT_CALL_PROGRAM, new String[] {"1"}, 2, warmup, iterations);
    runCase(
        "protocol-monomorphic",
        MONOMORPHIC_PROGRAM,
        new String[] {"(Cell 1)"},
        1,
        warmup,
        iterations);
    runCase(
        "protocol-polymorphic",
        POLYMORPHIC_PROGRAM,
        new String[] {"(CellA 1)", "(CellB 1)"},
        1,
        warmup,
        iterations);
    runCase("multimethod", MULTIMETHOD_PROGRAM, new String[] {"1"}, 2, warmup, iterations);
    runCase("lookup", LOOKUP_PROGRAM, new String[] {"{:key 1}"}, 1, warmup, iterations);
    runCase("iterator", ITERATOR_PROGRAM, new String[] {"[1]"}, 2, warmup, iterations);
  }

  private static void runCase(
      String name,
      String program,
      String[] receiverExpressions,
      long expected,
      int warmup,
      int iterations) {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      Value benchmark = context.eval(HaraLanguage.ID, program);
      Value[] receivers = new Value[receiverExpressions.length];
      for (int i = 0; i < receiverExpressions.length; i++) {
        receivers[i] = context.eval(HaraLanguage.ID, receiverExpressions[i]);
      }
      for (int i = 0; i < warmup; i++) {
        benchmark.execute(receivers[i % receivers.length]);
      }

      long start = System.nanoTime();
      Value result = null;
      for (int i = 0; i < iterations; i++) {
        result = benchmark.execute(receivers[i % receivers.length]);
      }
      long elapsed = System.nanoTime() - start;
      if (result == null || result.asLong() != expected) {
        throw new AssertionError("benchmark result mismatch: " + result);
      }
      System.out.printf(
          "benchmark=%s iterations=%d elapsed_ns=%d calls_per_sec=%.1f%n",
          name, iterations, elapsed, iterations / (elapsed / 1_000_000_000.0));
    }
  }

  private static final String ARITHMETIC_PROGRAM = "(fn [value] (+ value 1))";

  private static final String DIRECT_CALL_PROGRAM =
      "(defn add1 [value] (+ value 1)) (fn [value] (add1 value))";

  private static final String COMMON_PROGRAM =
      "(defprotocol Value (value [self])) "
          + "(defstruct Cell [value]) "
          + "(extend-type Cell Value (value [self] (field self :value))) ";

  private static final String MONOMORPHIC_PROGRAM =
      COMMON_PROGRAM + "(fn [receiver] (value receiver))";

  private static final String POLYMORPHIC_PROGRAM =
      "(defprotocol Value (value [self])) "
          + "(defstruct CellA [value]) "
          + "(defstruct CellB [value]) "
          + "(extend-type CellA Value (value [self] (field self :value))) "
          + "(extend-type CellB Value (value [self] (field self :value))) "
          + "(fn [receiver] (value receiver))";

  private static final String MULTIMETHOD_PROGRAM =
      "(defmulti kind (fn [value] value)) "
          + "(defmethod kind 1 [value] (+ value 1)) "
          + "(fn [value] (kind value))";

  private static final String LOOKUP_PROGRAM =
      "(fn [value] (ILookup/lookup value :key))";

  private static final String ITERATOR_PROGRAM =
      "(fn [value] (let [it (iter-map (fn [x] (+ x 1)) value)] (iter-next it)))";
}
