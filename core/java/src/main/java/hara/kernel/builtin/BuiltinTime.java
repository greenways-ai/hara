package hara.kernel.builtin;

import hara.lang.base.Ex;
import hara.kernel.base.Module;
import hara.lang.protocol.IFn;

import java.time.Instant;

@SuppressWarnings({"unchecked", "rawtypes"})
@Module.Ns(name = "global", tag = "time")
public interface BuiltinTime {
  @Module.Fn(name = "bench:fn", complete = true)
  public static long bench(IFn f) {
    long start = hara.lang.base.primitive.Clock.currentTimeNanos();
    f.invoke();
    long end = hara.lang.base.primitive.Clock.currentTimeNanos();
    return end - start;
  }

  //
  //
  //

  @Module.Fn(name = "now", complete = true)
  public static long now() {
    return hara.lang.base.primitive.Clock.currentTimeNanos();
  }
}
