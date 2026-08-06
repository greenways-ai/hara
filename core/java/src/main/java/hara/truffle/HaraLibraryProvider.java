package hara.truffle;

/** A runtime library that can be installed independently of the core language context. */
public interface HaraLibraryProvider {
  String namespace();

  default int order() { return 0; }

  /** Eager providers are installed while the language context is initialized. */
  default boolean eager() { return false; }

  void install(HaraContext context);
}
