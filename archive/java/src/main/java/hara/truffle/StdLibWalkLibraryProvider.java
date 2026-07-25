package hara.truffle;

/** Lazy Java implementation of {@code std.lib.walk}. */
public final class StdLibWalkLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.walk";
  }

  @Override
  public int order() {
    return 38;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdLibWalk.class);
  }
}
