package hara.truffle;

/** Portable fact lifecycle and structured test runner. */
public final class CodeTestLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "code.test";
  }

  @Override
  public String fallbackResource() {
    return "code/test.hal";
  }

  @Override
  public void install(HaraContext context) {}
}
