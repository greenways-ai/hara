package hara.truffle;

/** Lazy Java implementation of {@code std.lib.block}. */
public final class StdBlockLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "std.lib.block"; }

  @Override
  public int order() { return 20; }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdLibBlock.class);
  }
}
