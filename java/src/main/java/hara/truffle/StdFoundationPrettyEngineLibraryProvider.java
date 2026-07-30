package hara.truffle;

public final class StdFoundationPrettyEngineLibraryProvider implements HaraLibraryProvider {
  @Override public String namespace() { return "std.foundation.pretty.engine"; }

  @Override public String fallbackResource() { return "std/foundation/pretty/engine.hal"; }

  @Override public void install(HaraContext context) {}
}
