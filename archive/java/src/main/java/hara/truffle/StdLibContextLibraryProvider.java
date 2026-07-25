package hara.truffle;

/** Lazy Java implementation of {@code std.lib.context}. */
public final class StdLibContextLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.context";
  }

  @Override
  public int order() {
    return 20;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdLibContext.class);
  }
}
