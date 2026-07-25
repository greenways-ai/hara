package hara.truffle;

/** Lazy Java implementation of {@code std.lib.socket}. */
public final class StdLibSocketLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.lib.socket"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) { context.installSocketLibrary(); }
}
