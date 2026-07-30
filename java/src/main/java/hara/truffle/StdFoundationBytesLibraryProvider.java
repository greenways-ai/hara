package hara.truffle;

/** Lazy Java implementation of {@code std.foundation.bytes}. */
public final class StdFoundationBytesLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.foundation.bytes"; }

  @Override
  public int order() { return 20; }

  @Override
  public String fallbackResource() { return "std/foundation/bytes.hal"; }

  @Override
  public void install(HaraContext context) {
    context.collectBuiltins(namespace(), context::installBytesLibrary);
  }
}
