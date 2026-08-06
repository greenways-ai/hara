package hara.kernel.builtin;

import hara.lang.protocol.IFn;
import org.junit.Test;
import static org.junit.Assert.*;

public class BuiltinTimeTest {

  @Test
  public void testNow() {
    assertTrue(BuiltinTime.now() > 0);
  }

  @Test
  public void testBench() {
    IFn f =
        hara.lang.base.Fn.toFnVargs(
            (java.util.function.Function)
                (args) -> {
                  try {
                    Thread.sleep(10);
                  } catch (InterruptedException e) {
                  }
                  return null;
                });

    long duration = BuiltinTime.bench(f);
    assertTrue(duration >= 10_000_000); // 10ms in nanos
  }
}
