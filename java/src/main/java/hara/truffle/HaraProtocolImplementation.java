package hara.truffle;

/** A registered implementation together with its optional Hara function specialization. */
public final class HaraProtocolImplementation {
  private final HaraProtocolInvoker invoker;
  private final HaraFunction function;
  private final boolean intrinsic;

  HaraProtocolImplementation(HaraProtocolInvoker invoker, HaraFunction function) {
    this(invoker, function, false);
  }

  HaraProtocolImplementation(HaraProtocolInvoker invoker, HaraFunction function, boolean intrinsic) {
    this.invoker = invoker;
    this.function = function;
    this.intrinsic = intrinsic;
  }

  public HaraProtocolInvoker invoker() {
    return invoker;
  }

  /**
   * Whether this implementation is one of the runtime's own built-in invokers (as opposed to a
   * user or extension registration). Specialized nodes may inline the receiver operations of an
   * intrinsic implementation because its behavior is fixed by the runtime.
   */
  public boolean intrinsic() {
    return intrinsic;
  }

  public HaraFunction function() {
    return function;
  }

  public Object invoke(Object receiver, Object[] arguments) {
    return invoker.invoke(receiver, arguments);
  }
}
