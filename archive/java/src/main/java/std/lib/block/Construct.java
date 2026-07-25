package std.lib.block;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.Vector;

import java.lang.reflect.Array;
import java.math.BigDecimal;
import java.util.ArrayList;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

/** Block constructors and representation helpers from {@code std.block.construct}. */
public final class Construct {
  private Construct() {}

  private static final Map<String, Block.Container.Props> PROPS = new LinkedHashMap<>();
  static {
    add("root", "", "");
    add("list", "(", ")");
    add("vector", "[", "]");
    add("map", "{", "}");
    add("set", "#{", "}");
    add("fn", "#(", ")");
    add("meta", "^", "", 2);
    add("quote", "'", "", 1);
    add("deref", "@", "", 1);
    add("syntax", "`", "", 1);
    add("unquote", "~", "", 1);
    add("unquote-splice", "~@", "", 1);
    add("var", "#'", "", 1);
    add("hash-keyword", "#", "", 2);
    add("hash-meta", "#^", "", 2);
    add("hash-eval", "#=", "", 1);
    add("select", "#?", "", 1);
    add("select-splice", "#?@", "", 1);
  }
  private static void add(String tag, String start, String end) { add(tag, start, end, -1); }
  private static void add(String tag, String start, String end, int count) {
    PROPS.put(tag, new Block.Container.Props(start, end, count));
  }
  public static Block.Container.Props props(String tag) { return PROPS.get(tag); }

  public static final Block.Void SPACE = new Block.Void("linespace", ' ', 1, 0);
  public static final Block.Void NEWLINE = new Block.Void("linebreak", '\n', 0, 1);
  public static final Block.Void RETURN = new Block.Void("linebreak", '\r', 0, 1);
  public static final Block.Void FORMFEED = new Block.Void("linebreak", '\f', 0, 1);

  public static Block.Void voidBlock() { return SPACE; }
  public static Block.Void voidBlock(Character ch) {
    if (ch != null) {
      if (ch == ' ') return SPACE;
      if (ch == '\n') return NEWLINE;
      if (ch == '\r') return RETURN;
      if (ch == '\f') return FORMFEED;
    }
    String tag = Check.voidTag(ch);
    if (tag == null) throw new IllegalArgumentException("Not a valid void character: " + ch);
    int width = tag.equals("eof") || tag.equals("linebreak") ? 0 : tag.equals("linetab") ? Block.TAB_WIDTH : 1;
    return new Block.Void(tag, ch, width, tag.equals("linebreak") ? 1 : 0);
  }
  public static Block.Void space() { return SPACE; }
  public static java.util.List<Block.Void> spaces(int count) {
    return java.util.Collections.nCopies(count, SPACE);
  }
  public static Block.Void tab() { return voidBlock('\t'); }
  public static java.util.List<Block.Void> tabs(int count) {
    return java.util.Collections.nCopies(count, tab());
  }
  public static Block.Void newline() { return NEWLINE; }
  public static java.util.List<Block.Void> newlines(int count) {
    return java.util.Collections.nCopies(count, NEWLINE);
  }
  public static boolean newline(Block block) {
    return block instanceof Block.Void && block.tag().equals("linebreak");
  }
  public static Block.Comment comment(String source) {
    if (!Check.comment(source)) throw new IllegalArgumentException("Not a valid comment: " + source);
    return new Block.Comment(source);
  }

  public static Block.Token token(Object value) {
    String tag = Check.tokenTag(value);
    if (tag == null) throw new IllegalArgumentException("Not a valid token: " + value);
    if (value instanceof String string) return stringToken(string);
    String source = representation(value);
    int[] dimensions = dimensions(source);
    return new Block.Token(tag, source, value, source, dimensions[0], dimensions[1]);
  }

  public static Block.Token stringToken(String value) {
    String source = "\"" + escape(value) + "\"";
    int[] dimensions = dimensions(source);
    return new Block.Token("string", source, value, source, dimensions[0], dimensions[1]);
  }

  public static Block.Token tokenFromString(String source, Object value, String valueString) {
    String tag = Check.tokenTag(value);
    if (tag == null) throw new IllegalArgumentException("Not a valid token: " + source);
    int[] dimensions = dimensions(source);
    return new Block.Token(tag, source, value, valueString == null ? source : valueString,
        dimensions[0], dimensions[1]);
  }

  public static Block.Container container(String tag, Iterable<? extends Block> children) {
    Block.Container.Props props = props(tag);
    if (props == null) throw new IllegalArgumentException("Not a valid container tag: " + tag);
    Vector.Mutable<Block> out = Vector.Mutable.<Block>empty(null);
    for (Block child : children) out.pushLast(child);
    return new Block.Container(tag, out.toPersistent(), props);
  }

  public static Block.Modifier uneval() {
    return new Block.Modifier("hash-uneval", "#_", (acc, input) -> acc);
  }
  public static Block.Modifier cursor() {
    return new Block.Modifier("hash-cursor", "|", (acc, input) -> {
      acc.add(input); return acc;
    });
  }

  public static Block.IBlock block(Object value) {
    if (value instanceof Block.IBlock block) return block;
    if (Check.token(value)) return token(value);
    if (Check.collection(value)) return collectionBlock(value);
    throw new IllegalArgumentException("Invalid block input: " + value);
  }

  public static Block.Container collectionBlock(Object value) {
    String tag = Check.collectionTag(value);
    java.util.List<Object> values = new ArrayList<>();
    if (value instanceof Map<?, ?> map) {
      for (Map.Entry<?, ?> entry : map.entrySet()) {
        values.add(entry.getKey()); values.add(entry.getValue());
      }
    } else if (value instanceof Iterable<?> iterable) {
      iterable.forEach(values::add);
    } else if (value.getClass().isArray()) {
      for (int i = 0; i < Array.getLength(value); i++) values.add(Array.get(value, i));
    }
    return container(tag, children(values));
  }

  public static java.util.List<Block> children(Iterable<?> values) {
    java.util.List<Block> out = new ArrayList<>();
    for (Object value : values) {
      Block next = block(value);
      if (!out.isEmpty() && !(out.get(out.size() - 1) instanceof Block.Void)
          && !(out.get(out.size() - 1) instanceof Block.Modifier)) out.add(SPACE);
      out.add(next);
      if (next instanceof Block.Comment) out.add(NEWLINE);
    }
    return out;
  }

  public static Block.Container addChild(Block.Container container, Object value) {
    Vector.Mutable<Block> out = Vector.Mutable.<Block>empty(null);
    for (Block child : container.children) out.pushLast(child);
    out.pushLast(block(value));
    return container.replaceChildren(out.toPersistent());
  }
  public static Block.Container empty() { return container("list", java.util.List.of()); }
  public static Block.Container root(Iterable<?> values) { return container("root", children(values)); }

  public static Object contents(Block block) {
    if (block instanceof Block.Container container) return container.children;
    return block;
  }
  public static int maxWidth(Block block) { return maxWidth(block, 0); }
  public static int maxWidth(Block block, int offset) {
    if (block.height() == 0) return block.width();
    String[] lines = block.string().split("\\R", -1);
    int max = 0;
    for (int i = 0; i < lines.length; i++) max = Math.max(max, lines[i].length() + (i == 0 ? offset : 0));
    return max - offset;
  }
  public static int lineWidth(Block block) { return lineWidth(block, 0); }
  public static int lineWidth(Block block, int offset) {
    return block.height() == 0 ? block.width() : block.width() - offset;
  }
  public static String rep(Block block) { return block.string(); }
  public static java.util.List<String> lines(Block block) {
    return java.util.Arrays.asList(block.string().split("\\R", -1));
  }

  private static int[] dimensions(String source) {
    String[] lines = source.split("\\R", -1);
    return new int[]{lines[lines.length - 1].length(), lines.length - 1};
  }
  private static String escape(String value) {
    return value.replace("\\", "\\\\").replace("\"", "\\\"")
        .replace("\t", "\\t").replace("\r", "\\r").replace("\b", "\\b").replace("\f", "\\f");
  }
  public static String representation(Object value) {
    if (value == null) return "nil";
    if (value instanceof Boolean || value instanceof Number) {
      if (value instanceof BigDecimal decimal) return decimal.toPlainString() + "M";
      return value.toString();
    }
    if (value instanceof Keyword keyword) return keyword.display();
    if (value instanceof Symbol symbol) return symbol.display();
    if (value instanceof Character character) {
      return switch (character) {
        case '\n' -> "\\newline"; case ' ' -> "\\space"; case '\t' -> "\\tab";
        default -> "\\" + character;
      };
    }
    if (value instanceof Pattern pattern) return "#\"" + pattern.pattern() + "\"";
    if (value instanceof String string) return "\"" + escape(string) + "\"";
    return String.valueOf(value);
  }
}
