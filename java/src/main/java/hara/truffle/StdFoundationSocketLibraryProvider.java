package hara.truffle;

/** Lazy Java implementation of {@code std.foundation.socket}. */
public final class StdFoundationSocketLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.foundation.socket"; }

  @Override
  public int order() { return 20; }

  @Override
  public String fallbackResource() { return "std/foundation/socket.hal"; }

  @Override
  public void install(HaraContext context) {
    context.collectBuiltins(namespace(), context::installSocketLibrary);
  }
}
