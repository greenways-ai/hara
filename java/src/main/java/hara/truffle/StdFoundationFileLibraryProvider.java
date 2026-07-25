package hara.truffle;

/** Lazy Java implementation of {@code std.foundation.file}. */
public final class StdFoundationFileLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.foundation.file"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) { context.installFileLibrary(); }
}
