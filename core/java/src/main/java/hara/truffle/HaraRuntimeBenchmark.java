package hara.truffle;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Source;
import org.graalvm.polyglot.Value;
import org.graalvm.polyglot.io.ByteSequence;

/** Machine-readable persistent-process benchmark adapter used by bench/runtime. */
final class HaraRuntimeBenchmark {
  private HaraRuntimeBenchmark() {}

  static int run(String[] args, java.io.PrintStream output, java.io.PrintStream error) {
    if (args.length != 7) {
      error.println("benchmark expects RUNTIME REPRESENTATION ID PAYLOAD_BASE64URL EXPECTED WINDOWS CALLS");
      return 2;
    }
    String runtime = args[0];
    String representation = args[1];
    String id = args[2];
    byte[] payload = Base64.getUrlDecoder().decode(args[3]);
    String expected = args[4];
    int windows = Integer.parseInt(args[5]);
    int calls = Integer.parseInt(args[6]);
    try (Context context = Context.newBuilder(HaraLanguage.ID)
        .option("engine.WarnInterpreterOnly", "false").build()) {
      Source source;
      if ("vm".equals(representation)) {
        source = Source.newBuilder(HaraLanguage.ID, ByteSequence.create(payload), id + ".hbc")
            .mimeType(HaraLanguage.BYTECODE_MIME_TYPE).cached(true).build();
      } else if ("full".equals(representation)) {
        source = Source.newBuilder(HaraLanguage.ID,
            new String(payload, StandardCharsets.UTF_8), id + ".hal").cached(true).build();
      } else {
        error.println("benchmark representation must be vm or full");
        return 2;
      }
      long prepareStart = System.nanoTime();
      Value prepared = context.parse(source);
      long prepareNanos = System.nanoTime() - prepareStart;
      long firstStart = System.nanoTime();
      Value first = prepared.execute();
      long firstNanos = System.nanoTime() - firstStart;
      assertValue(id, expected, first);
      long[] samples = new long[windows];
      for (int window = 0; window < windows; window++) {
        long started = System.nanoTime();
        for (int call = 0; call < calls; call++) {
          assertValue(id, expected, prepared.execute());
        }
        samples[window] = (System.nanoTime() - started) / calls;
      }
      output.print("{\"runtime\":\"");
      output.print(json(runtime));
      output.print("\",\"workload\":\"");
      output.print(json(id));
      output.print("\",\"representation\":\"");
      output.print(json(representation));
      output.print("\",\"prepare_ns\":");
      output.print(prepareNanos);
      output.print(",\"first_ns\":");
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
