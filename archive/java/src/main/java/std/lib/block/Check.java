package std.lib.block;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.Collection;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

/** Character, token, and collection classification from {@code std.block.check}. */
public final class Check {
  private Check() {}

  public static boolean boundary(Character ch) {
    return ch == null || " :;'@^`~()[]{}\\\"".indexOf(ch) >= 0;
  }

  public static boolean whitespace(Character ch) {
    return ch != null && Character.isWhitespace(ch);
  }

  public static boolean comma(Character ch) {
    return ch != null && ch == ',';
  }

  public static boolean linebreak(Character ch) {
    return ch != null && (ch == '\n' || ch == '\r' || ch == '\f');
  }

  public static boolean delimiter(Character ch) {
    return ch != null && "()[]{}".indexOf(ch) >= 0;
  }

  public static boolean voidspace(Character ch) {
    return whitespace(ch) || comma(ch);
  }

  public static boolean linetab(Character ch) {
    return ch != null && ch == '\t';
  }

  public static boolean linespace(Character ch) {
    return whitespace(ch) && !linebreak(ch) && !linetab(ch);
  }

  public static boolean voidspaceOrBoundary(Character ch) {
    return voidspace(ch) || boundary(ch);
  }

  public static String voidTag(Character ch) {
    if (ch == null) return "eof";
    if (linetab(ch)) return "linetab";
    if (linebreak(ch)) return "linebreak";
    if (comma(ch)) return "comma";
    if (delimiter(ch)) return "delimiter";
    if (linespace(ch)) return "linespace";
    return null;
  }

  public static boolean isVoid(Character ch) {
    return voidTag(ch) != null;
  }

  public static String tokenTag(Object value) {
    if (value == null) return "nil";
    if (value instanceof Boolean) return "boolean";
    if (value instanceof Byte) return "byte";
    if (value instanceof Short) return "short";
    if (value instanceof Integer || value instanceof Long) return "long";
    if (value instanceof BigInteger) return "bigint";
    if (value instanceof Float) return "float";
    if (value instanceof Double) return "double";
    if (value instanceof BigDecimal) return "bigdec";
    if (value instanceof Keyword) return "keyword";
    if (value instanceof Symbol) return "symbol";
    if (value instanceof String) return "string";
    if (value instanceof Character) return "char";
    if (value instanceof java.time.Instant || value instanceof java.util.Date) return "inst";
    if (value instanceof Pattern) return "regexp";
    return null;
  }

  public static boolean token(Object value) {
    return tokenTag(value) != null;
  }

  public static String collectionTag(Object value) {
    if (value instanceof Map) return "map";
    if (value instanceof Set) return "set";
    if (value instanceof hara.lang.data.List || value instanceof java.util.List) return "list";
    if (value instanceof hara.lang.data.Vector || value instanceof Collection || value != null && value.getClass().isArray()) {
      return "vector";
    }
    return null;
  }

  public static boolean collection(Object value) {
    return collectionTag(value) != null;
  }

  public static boolean comment(String value) {
    return value != null && value.startsWith(";");
  }
}
