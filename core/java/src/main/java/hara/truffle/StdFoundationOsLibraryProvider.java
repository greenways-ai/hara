package hara.truffle;

/** Lazy Java implementation of {@code std.foundation.os}. */
public final class StdFoundationOsLibraryProvider implements HaraLibraryProvider {
  @Override public String namespace() { return "std.foundation.os"; }
  @Override public int order() { return 20; }
  @Override public void install(HaraContext context) {
    context.collectBuiltins(namespace(), context::installOsLibrary);
  }
}
