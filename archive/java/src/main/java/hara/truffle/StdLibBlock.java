package hara.truffle;

import std.lib.block.Parser;

/** Static Java implementation exported exclusively as {@code std.lib.block/*}. */
public final class StdLibBlock {
  private StdLibBlock() {}

  @HaraExport(name = "parse", doc = "Parses the first source-preserving block.", arglists = {"[source]"})
  public static Object parse(HaraContext context, Object[] values) {
    return Parser.parseString(source(values, "parse"));
  }

  @HaraExport(name = "parse-root", doc = "Parses all input into a root block.", arglists = {"[source]"})
  public static Object parseRoot(HaraContext context, Object[] values) {
    return Parser.parseRoot(source(values, "parse-root"));
  }

  @HaraExport(name = "parse-first", doc = "Parses the first expression block.", arglists = {"[source]"})
  public static Object parseFirst(HaraContext context, Object[] values) {
    return Parser.parseFirst(source(values, "parse-first"));
  }

  private static String source(Object[] values, String function) {
    if (values.length != 1) {
      throw new HaraException("std.lib.block/" + function + " expects one source string");
    }
    return String.valueOf(HaraBox.unwrap(values[0]));
  }
}
