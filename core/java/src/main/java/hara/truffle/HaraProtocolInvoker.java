package hara.truffle;

/** Internal ABI for invoking a protocol implementation independently of its host representation. */
@FunctionalInterface
public interface HaraProtocolInvoker {
  Object invoke(Object receiver, Object[] arguments);

  /** The total number of arguments, including the protocol receiver; negative means variadic. */
  default int arity() {
    return -1;
  }
}
