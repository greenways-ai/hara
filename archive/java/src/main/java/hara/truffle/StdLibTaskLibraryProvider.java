package hara.truffle;

/** Optional Java implementation of std.lib.task. */
public final class StdLibTaskLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.task";
  }

  @Override
  public int order() {
    return 20;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdLibTask.class);
  }
}
