package hara.truffle;

/** Lazy Java implementation of {@code std.foundation.promise}. */
public final class StdFoundationPromiseLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.foundation.promise"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) {
    context.collectBuiltins(namespace(), context::installPromiseLibrary);
  }
}
