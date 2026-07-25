package hara.truffle;

/** Optional Java implementation of std.foundation.coroutine. */
public final class StdFoundationCoroutineLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.foundation.coroutine";
  }

  @Override
  public int order() {
    return 30;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdFoundationCoroutine.class);
  }
}
