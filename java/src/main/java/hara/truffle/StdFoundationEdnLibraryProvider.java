package hara.truffle;

/** Portable HAL implementation of {@code std.foundation.edn}. */
public final class StdFoundationEdnLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.foundation.edn";
  }

  @Override
  public int order() {
    return 20;
  }

  @Override
  public boolean eager() {
    return true;
  }

  @Override
  public String fallbackResource() {
    return "std/foundation/edn.hal";
  }

  @Override
  public void install(HaraContext context) {}
}
