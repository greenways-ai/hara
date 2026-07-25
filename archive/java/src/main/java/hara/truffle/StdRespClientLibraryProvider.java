package hara.truffle;

/** Lazy Java implementation of {@code std.resp.client}. */
public final class StdRespClientLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.resp.client";
  }

  @Override
  public int order() {
    return 20;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdRespClient.class);
  }
}
