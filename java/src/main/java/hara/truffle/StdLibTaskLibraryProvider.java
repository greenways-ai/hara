package hara.truffle;

/** Canonical portable task construction and execution library. */
public final class StdLibTaskLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.task";
  }

  @Override
  public String fallbackResource() {
    return "std/lib/task.hal";
  }

  @Override
  public void install(HaraContext context) {}
}
