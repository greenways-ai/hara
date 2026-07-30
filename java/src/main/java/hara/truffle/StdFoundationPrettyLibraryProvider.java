package hara.truffle;

public final class StdFoundationPrettyLibraryProvider implements HaraLibraryProvider {
  @Override public String namespace() { return "std.foundation.pretty"; }

  @Override public String fallbackResource() { return "std/foundation/pretty.hal"; }

  @Override public void install(HaraContext context) {}
}
