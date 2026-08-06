package hara.kernel.flavor;

/** Authority that a runtime may grant independently of selecting a native flavor. */
public enum NativeCapability {
  REFLECTION,
  CLASSPATH,
  COMPILATION
}
