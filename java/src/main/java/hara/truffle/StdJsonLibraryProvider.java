package hara.truffle;

/** Portable HAL wrapper over the eager {@code std.native.Json} implementation. */
public final class StdJsonLibraryProvider implements HaraLibraryProvider {
  @Override public String namespace() { return "std.foundation.json"; }

  @Override public int order() { return 10; }

  @Override public boolean eager() { return true; }

  @Override public String fallbackResource() { return "std/foundation/json.hal"; }

  @Override public void install(HaraContext context) {
    context.installJsonLibrary();
  }
}
