package hara.truffle;

/** Lazy Java implementation of {@code std.foundation.string}. */
public final class StdFoundationStringLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.foundation.string"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) { context.installStringLibrary(); }
}
