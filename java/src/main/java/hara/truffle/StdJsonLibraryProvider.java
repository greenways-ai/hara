package hara.truffle;

/** Eager strict JSON v1 implementation exposed as {@code std.foundation.json}. */
public final class StdJsonLibraryProvider implements HaraLibraryProvider {
  @Override public String namespace() { return "std.foundation.json"; }

  @Override public int order() { return 10; }

  @Override public boolean eager() { return true; }

  @Override public void install(HaraContext context) {
    context.installJsonLibrary();
  }
}
