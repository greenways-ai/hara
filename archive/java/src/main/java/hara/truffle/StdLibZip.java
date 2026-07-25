package hara.truffle;

import hara.lang.zip.Zip;
import hara.lang.zip.Zipper;

/** Static Java implementation exported exclusively as {@code std.lib.zip/*}. */
public final class StdLibZip {
  private StdLibZip() {}

  @HaraExport(name = "zipper", arglists = {"[root]"})
  public static Object zipper(HaraContext context, Object[] values) {
    requireArity(values, "zipper");
    return Zip.zipper(HaraBox.unwrap(values[0]));
  }

  @HaraExport(name = "step-left", arglists = {"[zipper]"})
  public static Object stepLeft(HaraContext context, Object[] values) {
    return Zip.stepLeft(zipper(values, "step-left"));
  }

  @HaraExport(name = "step-right", arglists = {"[zipper]"})
  public static Object stepRight(HaraContext context, Object[] values) {
    return Zip.stepRight(zipper(values, "step-right"));
  }

  @HaraExport(name = "step-inside", arglists = {"[zipper]"})
  public static Object stepInside(HaraContext context, Object[] values) {
    return Zip.stepInside(zipper(values, "step-inside"));
  }

  @HaraExport(name = "step-outside", arglists = {"[zipper]"})
  public static Object stepOutside(HaraContext context, Object[] values) {
    return Zip.stepOutside(zipper(values, "step-outside"));
  }

  private static Zipper zipper(Object[] values, String function) {
    requireArity(values, function);
    Object value = HaraBox.unwrap(values[0]);
    if (!(value instanceof Zipper zipper)) {
      throw new HaraException("std.lib.zip/" + function + " expects a zipper");
    }
    return zipper;
  }

  private static void requireArity(Object[] values, String function) {
    if (values.length != 1) {
      throw new HaraException("std.lib.zip/" + function + " expects one argument");
    }
  }
}
