package hara.kernel.base;

import hara.kernel.builtin.BuiltinTime;
import org.junit.Test;

import static org.junit.Assert.assertTrue;

/**
 * Tests for time-related utility methods in {@link Builtin.Time}. These tests target previously
 * uncovered methods.
 */
public class BuiltinTimeTest {

  @Test
  public void testNowReturnsCurrentTime() {
    long before = System.currentTimeMillis();
    long now = BuiltinTime.now();
    long after = System.currentTimeMillis();

    // BuiltinTime.now() returns nanos, but synced to epoch.
    // So we compare it against epoch millis * 1,000,000
    assertTrue(
        "now() should return current time. now=" + now + ", before=" + (before * 1_000_000),
        now >= (before - 100) * 1_000_000 && now <= (after + 100) * 1_000_000);
  }

  @Test
  public void testBenchMeasuresExecutionTime() {
    long duration =
        BuiltinTime.bench(
            new hara.lang.protocol.IFn() {
              @Override
              public Object invoke(Object... args) {
                try {
                  Thread.sleep(50);
                } catch (InterruptedException e) {
                  Thread.currentThread().interrupt();
                }
                return null;
              }

              @Override
              public Object invoke() {
                return invoke(new Object[0]);
              }
            });

    // Should take at least 50ms (50 * 1,000,000 nanos)
    assertTrue("bench() should measure execution time accurately", duration >= 50 * 1_000_000);
  }
}
