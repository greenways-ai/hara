package hara.truffle;

/** Optional Java implementation of code.test. */
public final class CodeTestLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() { return "code.test"; }

  @Override
  public int order() { return 10; }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), CodeTest.class);
  }
}
