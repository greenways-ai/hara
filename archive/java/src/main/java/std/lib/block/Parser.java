package std.lib.block;

import hara.kernel.base.Reader;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.ArrayList;
import java.util.List;
import java.util.regex.Pattern;

/** Source-preserving reader for the retained {@code std.block.parse} forms. */
public final class Parser {
  private final String source;
  private int index;
  private int line = 1;
  private int column = 1;

  public Parser(String source) { this.source = source == null ? "" : source; }
  public Parser(Reader reader) {
    StringBuilder value = new StringBuilder();
    Character ch;
    while ((ch = reader.readChar()) != null) value.append(ch);
    this.source = value.toString();
  }

  public Block.IBlock parse() {
    Character ch = peek();
    if (ch == null) return Construct.voidBlock(null);
    if (Check.whitespace(ch) || Check.comma(ch)) return Construct.voidBlock(read());
    return switch (ch) {
      case ';' -> parseComment();
      case '"' -> parseStringToken(false);
      case '\\' -> parseCharacter();
      case ':' -> parseKeyword();
      case '(' -> parseCollection("list");
      case '[' -> parseCollection("vector");
      case '{' -> parseCollection("map");
      case ')', ']', '}' -> throw error("Unmatched delimiter '" + ch + "'");
      case '\'' -> parseCons("quote");
      case '@' -> parseCons("deref");
      case '^' -> parseCons("meta");
      case '`' -> parseCons("syntax");
      case '~' -> parseUnquote();
      case '#' -> parseHash();
      default -> parseToken();
    };
  }

  private Block.IBlock parseComment() {
    int start = index;
    while (peek() != null && !Check.linebreak(peek())) read();
    return Construct.comment(source.substring(start, index));
  }

  private Block.IBlock parseStringToken(boolean regexp) {
    int start = index;
    if (regexp) expect('#');
    expect('"');
    StringBuilder value = new StringBuilder();
    boolean escaped = false;
    while (true) {
      Character ch = read();
      if (ch == null) throw error("EOF while reading string");
      if (escaped) {
        char decoded = switch (ch) {
          case 'n' -> '\n'; case 'r' -> '\r'; case 't' -> '\t'; case 'b' -> '\b';
          case 'f' -> '\f'; case '"' -> '"'; case '\\' -> '\\'; default -> ch;
        };
        value.append(decoded);
        escaped = false;
      } else if (ch == '\\') {
        escaped = true;
      } else if (ch == '"') {
        break;
      } else {
        value.append(ch);
      }
    }
    String raw = source.substring(start, index);
    if (regexp) return Construct.tokenFromString(raw, Pattern.compile(value.toString()), raw);
    return Construct.tokenFromString(raw, value.toString(), Construct.representation(value.toString()));
  }

  private Block.IBlock parseCharacter() {
    int start = index;
    expect('\\');
    String name = readToBoundary();
    if (name.isEmpty()) throw error("Invalid character literal");
    Character value = switch (name) {
      case "newline" -> '\n'; case "return" -> '\r'; case "space" -> ' ';
      case "tab" -> '\t'; case "formfeed" -> '\f'; case "backspace" -> '\b';
      default -> {
        if (name.startsWith("u") && name.length() == 5) {
          try { yield (char) Integer.parseInt(name.substring(1), 16); }
          catch (NumberFormatException ex) { throw error("Invalid unicode character: " + name); }
        }
        if (name.length() != 1) throw error("Unsupported character literal: " + name);
        yield name.charAt(0);
      }
    };
    return Construct.tokenFromString(source.substring(start, index), value,
        source.substring(start, index));
  }

  private Block.IBlock parseKeyword() {
    int start = index;
    expect(':');
    boolean auto = peek() != null && peek() == ':';
    if (auto) read();
    String name = readToBoundaryAllowing('\'', ':');
    if (name.isEmpty()) throw error("Invalid keyword");
    try {
      Keyword value = Keyword.create((auto ? ":" : "") + name);
      return Construct.tokenFromString(source.substring(start, index), value,
          source.substring(start, index));
    } catch (IllegalArgumentException ex) {
      throw error("Invalid keyword: " + source.substring(start, index));
    }
  }

  private Block.IBlock parseToken() {
    int start = index;
    String raw = readToBoundaryAllowing('\'', ':');
    if (raw.isEmpty()) throw error("Invalid token");
    Object value;
    try { value = tokenValue(raw); }
    catch (RuntimeException ex) { throw error("Invalid token: " + raw); }
    return Construct.tokenFromString(source.substring(start, index), value, raw);
  }

  private Object tokenValue(String raw) {
    if (raw.equals("nil")) return null;
    if (raw.equals("true")) return true;
    if (raw.equals("false")) return false;
    if (raw.matches("[+-]?0[xX][0-9a-fA-F]+")) {
      boolean negative = raw.startsWith("-");
      String digits = raw.replaceFirst("^[+-]?0[xX]", "");
      long value = Long.parseLong(digits, 16); return negative ? -value : value;
    }
    if (raw.matches("[+-]?\\d+N")) return new BigInteger(raw.substring(0, raw.length() - 1));
    if (raw.matches("[+-]?(?:\\d+\\.\\d*|\\d*\\.\\d+|\\d+)[mM]"))
      return new BigDecimal(raw.substring(0, raw.length() - 1));
    if (raw.matches("[+-]?\\d+")) {
      try { return Long.valueOf(raw); } catch (NumberFormatException ex) { return new BigInteger(raw); }
    }
    if (raw.matches("[+-]?(?:\\d+\\.\\d*|\\d*\\.\\d+)(?:[eE][+-]?\\d+)?")
        || raw.matches("[+-]?\\d+[eE][+-]?\\d+")) return Double.valueOf(raw);
    if (raw.contains("/") && raw.matches("[+-]?\\d+/\\d+"))
      throw new IllegalArgumentException("Ratios are not supported by the Java runtime");
    return Symbol.create(raw);
  }

  private Block.Container parseCollection(String tag) {
    Block.Container.Props props = Construct.props(tag);
    consume(props.start);
    char expected = props.end.charAt(0);
    List<Block> children = new ArrayList<>();
    while (true) {
      Character ch = peek();
      if (ch == null) throw error("EOF while reading " + tag + ", expected '" + expected + "'");
      if (ch == expected) { read(); break; }
      if (ch == ')' || ch == ']' || ch == '}')
        throw error("Mismatched delimiter: expected '" + expected + "' but found '" + ch + "'");
      children.add(parse());
    }
    return Construct.container(tag, children);
  }

  private Block.Container parseCons(String tag) {
    Block.Container.Props props = Construct.props(tag);
    consume(props.start);
    List<Block> children = new ArrayList<>();
    int expressions = 0;
    while (expressions < props.expressions) {
      if (peek() == null) throw error("EOF while reading " + tag);
      Block child = parse();
      children.add(child);
      if (child.expression()) expressions++;
    }
    return Construct.container(tag, children);
  }

  private Block.IBlock parseUnquote() {
    return source.startsWith("~@", index) ? parseCons("unquote-splice") : parseCons("unquote");
  }

  private Block.IBlock parseHash() {
    Character dispatch = peek(1);
    if (dispatch == null) throw errorAfter(1, "EOF while reading hash dispatch");
    return switch (dispatch) {
      case '{' -> parseCollection("set");
      case '(' -> parseCollection("fn");
      case '\'' -> parseCons("var");
      case ':' -> parseCons("hash-keyword");
      case '^' -> parseCons("hash-meta");
      case '=' -> parseCons("hash-eval");
      case '_' -> { consume("#_"); yield Construct.uneval(); }
      case '|' -> { consume("#|"); yield Construct.cursor(); }
      case '?' -> parseSelect();
      case '"' -> parseStringToken(true);
      case '[' -> parseCollection("root");
      default -> throw error("Unsupported hash dispatch: " + dispatch);
    };
  }

  private Block.IBlock parseSelect() {
    String tag = source.startsWith("#?@", index) ? "select-splice" : "select";
    return parseCons(tag);
  }

  public static Block.IBlock parseString(String source) { return new Parser(source).parse(); }
  public static Block.IBlock parseFirst(String source) {
    Parser parser = new Parser(source);
    while (parser.peek() != null && (Check.voidspace(parser.peek()) || parser.peek() == ';')) parser.parse();
    return parser.peek() == null ? null : parser.parse();
  }
  public static Block.Container parseRoot(String source) {
    Parser parser = new Parser(source);
    List<Block> children = new ArrayList<>();
    while (parser.peek() != null) children.add(parser.parse());
    return Construct.container("root", children);
  }

  private Character peek() { return peek(0); }
  private Character peek(int offset) {
    int at = index + offset; return at >= source.length() ? null : source.charAt(at);
  }
  private Character read() {
    if (index >= source.length()) return null;
    char ch = source.charAt(index++);
    if (ch == '\n' || ch == '\r' || ch == '\f') { line++; column = 1; } else column++;
    return ch;
  }
  private void expect(char expected) {
    Character actual = read();
    if (actual == null || actual != expected) throw error("Expected '" + expected + "'");
  }
  private void consume(String expected) {
    for (int i = 0; i < expected.length(); i++) expect(expected.charAt(i));
  }
  private String readToBoundary() { return readToBoundaryAllowing(); }
  private String readToBoundaryAllowing(char... allowed) {
    int start = index;
    outer: while (peek() != null) {
      char ch = peek();
      for (char allow : allowed) if (ch == allow) { read(); continue outer; }
      if (Check.voidspaceOrBoundary(ch)) break;
      read();
    }
    return source.substring(start, index);
  }
  private Reader.ReaderException error(String message) {
    return new Reader.ReaderException(message, line, column);
  }
  private Reader.ReaderException errorAfter(int chars, String message) {
    for (int i = 0; i < chars; i++) read();
    return error(message);
  }
}
