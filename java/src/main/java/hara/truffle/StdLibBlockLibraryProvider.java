package hara.truffle;

/** Canonical portable structured source-block library. */
public final class StdLibBlockLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.block";
  }

  @Override
  public String fallbackResource() {
    return "std/lib/block.hal";
  }

  @Override
  public void install(HaraContext context) {}
}
