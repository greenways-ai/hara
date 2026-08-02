package hara.truffle;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;

/** Machine-readable persistent-process benchmark adapter used by bench/runtime. */
final class HaraRuntimeBenchmark {
  private HaraRuntimeBenchmark() {}

  static int run(String[] args, java.io.PrintStream output, java.io.PrintStream error) {
    if (args.length != 6) {
      error.println("benchmark expects RUNTIME ID SOURCE_BASE64URL EXPECTED WINDOWS CALLS");
      return 2;
    }
    String runtime = args[0];
    String id = args[1];
    String source = new String(Base64.getUrlDecoder().decode(args[2]), StandardCharsets.UTF_8);
    String expected = args[3];
    int windows = Integer.parseInt(args[4]);
    int calls = Integer.parseInt(args[5]);
    Context.Builder builder =
        Context.newBuilder(HaraLanguage.ID).option("engine.WarnInterpreterOnly", "false");
    if (Boolean.getBoolean("hara.benchmark.experimental")) {
      builder.allowExperimentalOptions(true);
    }
    try (Context context = builder.build()) {
      long firstStart = System.nanoTime();
      Value first = context.eval(HaraLanguage.ID, source);
      long firstNanos = System.nanoTime() - firstStart;
      assertValue(id, expected, first);
      long[] samples = new long[windows];
      for (int window = 0; window < windows; window++) {
        long started = System.nanoTime();
        for (int call = 0; call < calls; call++) {
          assertValue(id, expected, context.eval(HaraLanguage.ID, source));
        }
        samples[window] = (System.nanoTime() - started) / calls;
      }
      output.print("{\"runtime\":\"");
      output.print(json(runtime));
      output.print("\",\"workload\":\"");
      output.print(json(id));
      output.print("\",\"first_ns\":");
      output.print(firstNanos);
      output.print(",\"samples_ns\":[");
      for (int index = 0; index < samples.length; index++) {
        if (index > 0) output.print(',');
        output.print(samples[index]);
      }
      output.println("]}");
      return 0;
    } catch (Exception failure) {
      error.println(id + ": " + failure.getMessage());
      return 1;
    }
  }

  private static void assertValue(String id, String expected, Value value) {
    String actual = Main.display(value);
    if (!expected.equals(actual)) {
      throw new IllegalStateException(id + ": expected " + expected + ", got " + actual);
    }
  }

  private static String json(String value) {
    return value.replace("\\", "\\\\").replace("\"", "\\\"");
  }
}
