package hara.truffle;

/** Portable persistent zipper library. */
public final class StdLibZipLibraryProvider implements HaraLibraryProvider {
  @Override
  public String namespace() {
    return "std.lib.zip";
  }

  @Override
  public String fallbackResource() {
    return "std/lib/zip.hal";
  }

  @Override
  public void install(HaraContext context) {}
}
