package hara.truffle;

/** Eager optimized implementation of the canonical Hara core namespace. */
public final class StdFoundationLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.foundation";
  }

  @Override
  public int order() {
    return 5;
  }

  @Override
  public String fallbackResource() {
    return "std/foundation.hal";
  }

  @Override
  public boolean eager() {
    return true;
  }

  @Override
  public void install(HaraContext context) {
    HaraStaticLibrary.install(context, namespace(), StdFoundationSequence.class);
    HaraStaticLibrary.install(context, namespace(), StdFoundationCollection.class);
  }
}
