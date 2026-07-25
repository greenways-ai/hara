package hara.truffle;

/** Lazy Java implementation of {@code std.lib.handle}. */
public final class StdLibHandleLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.lib.handle"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) { context.installHandleLibrary(); }
}
