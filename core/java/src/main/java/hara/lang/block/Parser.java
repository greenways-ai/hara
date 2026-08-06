package hara.lang.block;

import hara.kernel.base.Reader;
import hara.lang.data.Vector;

import java.util.HashMap;
import java.util.Map;

public class Parser {

  private static final Map<Character, String> DISPATCH_OPTIONS = new HashMap<>();
  private static final Map<String, Block.Container.Props> CONTAINER_PROPS = new HashMap<>();

  static {
    DISPATCH_OPTIONS.put('^', "meta");
    DISPATCH_OPTIONS.put('#', "hash");
    DISPATCH_OPTIONS.put('\\', "char");
    DISPATCH_OPTIONS.put('(', "list");
    DISPATCH_OPTIONS.put('[', "vector");
    DISPATCH_OPTIONS.put('{', "map");
    DISPATCH_OPTIONS.put('}', "unmatched");
    DISPATCH_OPTIONS.put(']', "unmatched");
    DISPATCH_OPTIONS.put(')', "unmatched");
    DISPATCH_OPTIONS.put('~', "unquote");
    DISPATCH_OPTIONS.put('\'', "quote");
    DISPATCH_OPTIONS.put('`', "syntax");
    DISPATCH_OPTIONS.put(';', "comment");
    DISPATCH_OPTIONS.put('@', "deref");
    DISPATCH_OPTIONS.put('"', "string");
    DISPATCH_OPTIONS.put(':', "keyword");
    DISPATCH_OPTIONS.put(',', "comma");

    CONTAINER_PROPS.put("list", new Block.Container.Props("(", ")"));
    CONTAINER_PROPS.put("vector", new Block.Container.Props("[", "]"));
    CONTAINER_PROPS.put("map", new Block.Container.Props("{", "}"));
    CONTAINER_PROPS.put("set", new Block.Container.Props("#{", "}"));
    CONTAINER_PROPS.put("fn", new Block.Container.Props("#(", ")"));
    CONTAINER_PROPS.put("meta", new Block.Container.Props("^", ""));
    CONTAINER_PROPS.put("quote", new Block.Container.Props("'", ""));
    CONTAINER_PROPS.put("deref", new Block.Container.Props("@", ""));
    CONTAINER_PROPS.put("syntax", new Block.Container.Props("`", ""));
    CONTAINER_PROPS.put("unquote", new Block.Container.Props("~", ""));
    CONTAINER_PROPS.put("unquote-splice", new Block.Container.Props("~@", ""));
  }

  private final Reader reader;
  private Character endDelimiter;

  public Parser(Reader reader) {
    this.reader = reader;
  }

  private static boolean isWhitespace(Character ch) {
    return ch != null && (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' || ch == '\f');
  }

  private static boolean isBoundary(Character ch) {
    return ch == null || isWhitespace(ch) || DISPATCH_OPTIONS.containsKey(ch);
  }

  private String readDispatch(Character ch) {
    if (ch == null) {
      return "eof";
    }
    if (ch.equals(endDelimiter)) {
      return "delimiter";
    }
    if (isWhitespace(ch)) {
      return "void";
    }
    return DISPATCH_OPTIONS.getOrDefault(ch, "token");
  }

  public Block.IBlock parse() {
    Character ch = reader.peekChar();
    String dispatch = readDispatch(ch);

    switch (dispatch) {
      case "comment":
        return parseComment();
      case "void":
      case "eof":
      case "comma":
      case "delimiter":
        return parseVoid();
      case "token":
        return parseToken();
      case "keyword":
        return parseKeyword();
      case "string":
        return parseString();
      case "list":
        return parseCollection("list");
      case "vector":
        return parseCollection("vector");
      case "map":
        return parseCollection("map");
      case "meta":
      case "quote":
      case "deref":
      case "syntax":
        return parseCons(dispatch);
      case "unquote":
        return parseUnquote();
      case "hash":
        return parseHash();
      default:
        throw new Reader.ReaderException(
            "Unsupported dispatch type: " + dispatch,
            reader.getLineNumber(),
            reader.getColumnNumber());
    }
  }

  private Block.IBlock parseVoid() {
    Character c = reader.readChar();
    String tag = c == null ? "eof" : (c == '\n' || c == '\r' ? "linebreak" : "whitespace");
    int width = (c != null && c == '\t') ? 4 : 1;
    int height = (c != null && (c == '\n' || c == '\r')) ? 1 : 0;
    return new Block.Void(tag, c, width, height);
  }

  private Block.IBlock parseComment() {
    String line = reader.readUntil(ch -> ch == '\n' || ch == '\r');
    return new Block.Comment(line);
  }

  private Block.IBlock parseToken() {
    String token = reader.readWhile(ch -> !isBoundary(ch));
    return new Block.Token("token", token, token, token, token.length(), 0);
  }

  private Block.IBlock parseKeyword() {
    reader.readChar(); // consume leading colon
    String keyword = reader.readWhile(ch -> !isBoundary(ch));
    String string = ":" + keyword;
    return new Block.Token("keyword", string, keyword, string, string.length(), 0);
  }

  private Block.IBlock parseString() {
    reader.readChar(); // consume leading quote
    StringBuilder sb = new StringBuilder();
    boolean escape = false;
    while (true) {
      Character c = reader.readChar();
      if (c == null) {
        throw new Reader.ReaderException(
            "EOF while reading string", reader.getLineNumber(), reader.getColumnNumber());
      }
      if (escape) {
        sb.append(c);
        escape = false;
      } else if (c == '\\') {
        escape = true;
      } else if (c == '"') {
        break;
      } else {
        sb.append(c);
      }
    }
    String s = sb.toString();
    return new Block.Token("string", "\"" + s + "\"", s, s, s.length() + 2, 0);
  }

  private Block.IBlock parseCollection(String tag) {
    Block.Container.Props props = CONTAINER_PROPS.get(tag);
    Character previousDelimiter = this.endDelimiter;
    Character expectedDelimiter = props.end.charAt(0);
    this.endDelimiter = expectedDelimiter;
    reader.readChar(); // consume start delimiter
    Vector.Mutable<Block.IBlock> children = Vector.Mutable.empty(null);
    while (true) {
      Character ch = reader.peekChar();
      if (ch == null) {
        this.endDelimiter = previousDelimiter;
        throw new Reader.ReaderException(
            "EOF while reading " + tag + ", expected '" + expectedDelimiter + "'",
            reader.getLineNumber(),
            reader.getColumnNumber());
      }
      if (ch.equals(expectedDelimiter)) {
        break;
      }
      if (ch == ')' || ch == ']' || ch == '}') {
        this.endDelimiter = previousDelimiter;
        throw new Reader.ReaderException(
            "Mismatched delimiter: expected '" + expectedDelimiter + "' but found '" + ch + "'",
            reader.getLineNumber(),
            reader.getColumnNumber());
      }
      children.pushLast(parse());
    }
    reader.readChar(); // consume end delimiter
    this.endDelimiter = previousDelimiter;
    return new Block.Container(tag, children.toPersistent(), props);
  }

  private Block.IBlock parseCons(String tag) {
    Block.Container.Props props = CONTAINER_PROPS.get(tag);
    reader.readChar();
    if (reader.peekChar() == null) {
      throw new Reader.ReaderException(
          "EOF while reading " + tag, reader.getLineNumber(), reader.getColumnNumber());
    }
    Vector.Mutable<Block.IBlock> children = Vector.Mutable.empty(null);
    children.pushLast(parse());
    return new Block.Container(tag, children.toPersistent(), props);
  }

  private Block.IBlock parseUnquote() {
    reader.readChar();
    if (reader.peekChar() == '@') {
      reader.readChar();
      if (reader.peekChar() == null) {
        throw new Reader.ReaderException(
            "EOF while reading unquote-splice", reader.getLineNumber(), reader.getColumnNumber());
      }
      return new Block.Container(
          "unquote-splice",
          Vector.Standard.from(null, parse()),
          CONTAINER_PROPS.get("unquote-splice"));
    }
    if (reader.peekChar() == null) {
      throw new Reader.ReaderException(
          "EOF while reading unquote", reader.getLineNumber(), reader.getColumnNumber());
    }
    return new Block.Container(
        "unquote", Vector.Standard.from(null, parse()), CONTAINER_PROPS.get("unquote"));
  }

  private Block.IBlock parseHash() {
    reader.readChar();
    Character ch = reader.peekChar();
    if (ch == null) {
      throw new Reader.ReaderException(
          "EOF while reading hash dispatch", reader.getLineNumber(), reader.getColumnNumber());
    }
    switch (ch) {
      case '{':
        return parseCollection("set");
      case '(':
        return parseCollection("fn");
      default:
        throw new Reader.ReaderException(
            "Unsupported hash dispatch: " + ch, reader.getLineNumber(), reader.getColumnNumber());
    }
  }

  public static Block.IBlock parseString(String s) {
    return new Parser(new Reader(s)).parse();
  }

  public static Block.Container parseRoot(String s) {
    Reader reader = new Reader(s);
    Parser parser = new Parser(reader);
    Vector.Mutable<Block.IBlock> children = Vector.Mutable.empty(null);
    while (reader.peekChar() != null) {
      children.pushLast(parser.parse());
    }
    return new Block.Container("root", children.toPersistent(), new Block.Container.Props("", ""));
  }
}
