package hara.truffle;

/** Lazy Java implementation of {@code std.lib.zip}. */
public final class StdZipLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.lib.zip"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdLibZip.class);
  }
}
