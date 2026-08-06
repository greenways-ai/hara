package hara.kernel.jvm;

import hara.kernel.base.RT;
import hara.lang.base.Ex;
import hara.lang.base.Iter;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.types.ObjFn;
import hara.lang.protocol.IFn;
import java.util.function.Function;

/** Capability-gated implementation of hara.native.jvm/set!. */
public final class JvmSetFunction extends ObjFn implements IFn<Object, Object, Object> {
  private final RT.Instance<?> runtime;

  public JvmSetFunction(RT.Instance<?> runtime) {
    this.runtime = runtime;
  }

  @Override
  public Function<Object, Object> getArgN() {
    return arguments -> {
      Object[] values = Iter.toArray(Iter.iter(arguments));
      if (values.length != 3) throw new Ex.Arity(values.length, "set! expects 3 arguments");
      String member;
      if (values[1] instanceof Symbol) member = ((Symbol) values[1]).getName();
      else if (values[1] instanceof Keyword) member = ((Keyword) values[1]).getName();
      else if (values[1] instanceof String) member = (String) values[1];
      else throw new Ex.Runtime("set! member must be a symbol, keyword, or string");
      return runtime
          .nativeProvider()
          .writeMember(values[0], member, values[2], runtime.nativeAccess());
    };
  }
}
