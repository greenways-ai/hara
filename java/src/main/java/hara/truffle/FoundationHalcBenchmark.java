package hara.truffle;

import com.sun.management.ThreadMXBean;
import java.lang.management.ManagementFactory;
import java.util.Arrays;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Engine;
import org.graalvm.polyglot.Value;

/** Fork-friendly benchmark for source versus HALC foundation loading. */
final class FoundationHalcBenchmark {
  private static final String PROBE = "(std.foundation/get-in {:a {:b 42}} [:a :b])";

  private FoundationHalcBenchmark() {}

  static int run(String[] args, java.io.PrintStream output, java.io.PrintStream error) {
    int samples = args.length == 0 ? 20 : Integer.parseInt(args[0]);
    if (samples < 1 || args.length > 1) {
      error.println("foundation-halc-benchmark expects an optional positive sample count");
      return 2;
    }
    String previous = System.getProperty("hara.HalcMode");
    try {
      for (int index = 0; index < 2; index++) {
        sample("off");
        sample("strict");
      }
      Result source = measure("off", samples);
      Result hir = measure("strict", samples);
      Result shared = measureSharedEngine(samples);
      output.println(
          "{\"benchmark\":\"foundation-load\","
              + "\"samples\":"
              + samples
              + ",\"source\":"
              + source.json()
              + ",\"hir\":"
              + hir.json()
              + ",\"hir_shared_engine\":"
              + shared.json()
              + ",\"load_speedup\":"
              + ratio(source.medianLoad(), hir.medianLoad())
              + ",\"shared_engine_load_speedup\":"
              + ratio(source.medianLoad(), shared.medianLoad())
              + ",\"allocation_reduction\":"
              + reduction(source.medianAllocation(), hir.medianAllocation())
              + "}");
      return 0;
    } catch (RuntimeException failure) {
      error.println("Foundation HALC benchmark failed: " + failure.getMessage());
      return 1;
    } finally {
      if (previous == null) System.clearProperty("hara.HalcMode");
      else System.setProperty("hara.HalcMode", previous);
    }
  }

  private static Result measureSharedEngine(int samples) {
    System.setProperty("hara.HalcMode", "strict");
    long[] context = new long[samples];
    long[] load = new long[samples];
    long[] allocation = new long[samples];
    try (Engine engine = Engine.newBuilder()
        .option("engine.WarnInterpreterOnly", "false").build()) {
      sample("strict", engine);
      for (int index = 0; index < samples; index++) {
        Sample sample = sample("strict", engine);
        context[index] = sample.contextNanos;
        load[index] = sample.loadNanos;
        allocation[index] = sample.allocatedBytes;
      }
    }
    return new Result(context, load, allocation);
  }

  private static Result measure(String mode, int samples) {
    long[] context = new long[samples];
    long[] load = new long[samples];
    long[] allocation = new long[samples];
    for (int index = 0; index < samples; index++) {
      Sample sample = sample(mode);
      context[index] = sample.contextNanos;
      load[index] = sample.loadNanos;
      allocation[index] = sample.allocatedBytes;
    }
    return new Result(context, load, allocation);
  }

  private static Sample sample(String mode) {
    return sample(mode, null);
  }

  private static Sample sample(String mode, Engine engine) {
    System.setProperty("hara.HalcMode", mode);
    ThreadMXBean bean =
        ManagementFactory.getThreadMXBean() instanceof ThreadMXBean
            ? (ThreadMXBean) ManagementFactory.getThreadMXBean()
            : null;
    long thread = Thread.currentThread().threadId();
    long allocatedBefore =
        bean != null && bean.isThreadAllocatedMemorySupported()
            ? bean.getThreadAllocatedBytes(thread)
            : -1L;
    long contextStarted = System.nanoTime();
    Context.Builder builder = Context.newBuilder(HaraLanguage.ID);
    if (engine == null) builder.option("engine.WarnInterpreterOnly", "false");
    else builder.engine(engine);
    try (Context context = builder.build()) {
      long contextNanos = System.nanoTime() - contextStarted;
      long loadStarted = System.nanoTime();
      Value value = context.eval(HaraLanguage.ID, PROBE);
      long loadNanos = System.nanoTime() - loadStarted;
      if (value.asLong() != 42L) throw new IllegalStateException("foundation probe mismatch");
      long allocatedAfter =
          allocatedBefore >= 0L ? bean.getThreadAllocatedBytes(thread) : -1L;
      return new Sample(
          contextNanos,
          loadNanos,
          allocatedBefore < 0L ? -1L : allocatedAfter - allocatedBefore);
    }
  }

  private static String ratio(long numerator, long denominator) {
    if (denominator <= 0L) return "null";
    return String.format(java.util.Locale.ROOT, "%.3f", (double) numerator / denominator);
  }

  private static String reduction(long source, long hir) {
    if (source <= 0L || hir < 0L) return "null";
    return String.format(
        java.util.Locale.ROOT, "%.3f", 1.0d - ((double) hir / (double) source));
  }

  private record Sample(long contextNanos, long loadNanos, long allocatedBytes) {}

  private record Result(long[] context, long[] load, long[] allocation) {
    long medianLoad() {
      return median(load);
    }

    long medianAllocation() {
      return median(allocation);
    }

    String json() {
      return "{\"context_median_ns\":"
          + median(context)
          + ",\"load_median_ns\":"
          + median(load)
          + ",\"allocated_median_bytes\":"
          + median(allocation)
          + "}";
    }

    private static long median(long[] values) {
      long[] sorted = values.clone();
      Arrays.sort(sorted);
      return sorted[sorted.length / 2];
    }
  }
}
