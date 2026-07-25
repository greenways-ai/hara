package std.lib.block;

import hara.lang.data.Vector;

import java.util.Objects;
import java.util.function.BiFunction;

/** Immutable source-preserving block model. */
public interface Block {
  int TAB_WIDTH = 4;

  String type();
  String tag();
  String string();
  int length();
  int width();
  int height();
  int prefixed();
  int suffixed();
  boolean verify();

  default boolean expression() { return this instanceof IExpression; }
  default boolean modifier() { return this instanceof IModifier; }

  /** Compatibility name retained from the original Java block API. */
  interface IBlock extends Block {}

  interface IExpression extends IBlock {
    Object value();
    String valueString();
  }

  interface Expression extends IExpression {}

  interface IModifier extends IBlock {}

  interface IContainer extends IExpression {
    Vector<Block> children();
    IContainer replaceChildren(Vector<Block> children);
  }

  interface ContainerBlock extends IContainer {}

  final class Void implements IBlock, Comparable<Block> {
    public final String tag;
    public final Character character;
    public final int width;
    public final int height;

    public Void(String tag, Character character, int width, int height) {
      this.tag = tag; this.character = character; this.width = width; this.height = height;
    }
    public String type() { return "void"; }
    public String tag() { return tag; }
    public String string() { return character == null ? "" : String.valueOf(character); }
    public int length() { return character == null ? 0 : 1; }
    public int width() { return width; }
    public int height() { return height; }
    public int prefixed() { return 0; }
    public int suffixed() { return 0; }
    public boolean verify() { return Objects.equals(tag, Check.voidTag(character)); }
    public int compareTo(Block other) { return compare(this, other); }
    public String toString() {
      if (character == null) return "";
      if (character == ' ') return "␣";
      if (character == '\n') return "\\n";
      if (character == '\t') return "\\t";
      return String.valueOf(character);
    }
  }

  final class Comment implements IBlock, Comparable<Block> {
    public final String source;
    public Comment(String source) { this.source = source; }
    public String type() { return "comment"; }
    public String tag() { return "comment"; }
    public String string() { return source; }
    public int length() { return source.length(); }
    public int width() { return source.length(); }
    public int height() { return 0; }
    public int prefixed() { return 0; }
    public int suffixed() { return 0; }
    public boolean verify() { return Check.comment(source); }
    public int compareTo(Block other) { return compare(this, other); }
    public String toString() { return source; }
  }

  final class Token implements Expression, Comparable<Block> {
    public final String tag;
    public final String source;
    public final Object value;
    public final String valueString;
    public final int width;
    public final int height;
    public Token(String tag, String source, Object value, String valueString, int width, int height) {
      this.tag = tag; this.source = source; this.value = value; this.valueString = valueString;
      this.width = width; this.height = height;
    }
    public String type() { return "token"; }
    public String tag() { return tag; }
    public String string() { return source; }
    public int length() { return source.length(); }
    public int width() { return width; }
    public int height() { return height; }
    public int prefixed() { return 0; }
    public int suffixed() { return 0; }
    public boolean verify() { return Objects.equals(tag, Check.tokenTag(value)); }
    public Object value() { return value; }
    public String valueString() { return valueString; }
    public int compareTo(Block other) { return compare(this, other); }
    public String toString() { return valueString; }
  }

  final class Modifier implements IModifier, Comparable<Block> {
    public final String tag;
    public final String source;
    public final BiFunction<java.util.List<Block>, Block, java.util.List<Block>> command;
    public Modifier(String tag, String source,
        BiFunction<java.util.List<Block>, Block, java.util.List<Block>> command) {
      this.tag = tag; this.source = source; this.command = command;
    }
    public String type() { return "modifier"; }
    public String tag() { return tag; }
    public String string() { return source; }
    public int length() { return source.length(); }
    public int width() { return source.length(); }
    public int height() { return 0; }
    public int prefixed() { return 0; }
    public int suffixed() { return 0; }
    public boolean verify() { return tag != null && command != null; }
    public java.util.List<Block> modify(java.util.List<Block> accumulator, Block input) {
      return command.apply(accumulator, input);
    }
    public int compareTo(Block other) { return compare(this, other); }
    public String toString() { return source; }
  }

  final class Container implements ContainerBlock, Comparable<Block> {
    public final String tag;
    public final Vector<Block> children;
    public final Props props;
    public static final class Props {
      public final String start;
      public final String end;
      public final int expressions;
      public Props(String start, String end) { this(start, end, -1); }
      public Props(String start, String end, int expressions) {
        this.start = start; this.end = end; this.expressions = expressions;
      }
    }
    public Container(String tag, Vector<Block> children, Props props) {
      this.tag = Objects.requireNonNull(tag); this.children = Objects.requireNonNull(children);
      this.props = Objects.requireNonNull(props);
      if (props.expressions >= 0) {
        long count = java.util.stream.StreamSupport.stream(children.spliterator(), false)
            .filter(Block::expression).count();
        if (count != props.expressions) throw new IllegalArgumentException("Invalid children for " + tag);
      }
    }
    public String type() { return "collection"; }
    public String tag() { return tag; }
    public Vector<Block> children() { return children; }
    public Container replaceChildren(Vector<Block> value) { return new Container(tag, value, props); }
    public String string() {
      StringBuilder out = new StringBuilder(tag.equals("root") ? "" : props.start);
      for (Block child : children) out.append(child.string());
      if (!tag.equals("root")) out.append(props.end);
      return out.toString();
    }
    public int length() { return string().length(); }
    public int width() {
      int width = suffixed();
      boolean lastLine = true;
      for (int i = (int) children.count() - 1; i >= 0; i--) {
        Block child = children.nth(i);
        width += child.width();
        if (child.height() > 0) { lastLine = false; break; }
      }
      return width + (lastLine ? prefixed() : 0);
    }
    public int height() {
      int height = 0; for (Block child : children) height += child.height(); return height;
    }
    public int prefixed() { return tag.equals("root") ? 0 : props.start.length(); }
    public int suffixed() { return tag.equals("root") ? 0 : props.end.length(); }
    public boolean verify() { return Construct.props(tag) != null; }
    public Object value() { return Value.containerValue(this); }
    public String valueString() {
      StringBuilder out = new StringBuilder(props.start);
      boolean separated = false;
      for (Block child : children) {
        if (child instanceof IExpression exp) {
          if (separated) out.append(' ');
          out.append(exp.valueString()); separated = true;
        } else if (child instanceof Modifier modifier) {
          if (separated) out.append(' ');
          out.append(modifier.string()); separated = true;
        }
      }
      return out.append(props.end).toString();
    }
    public int compareTo(Block other) { return compare(this, other); }
    public String toString() { return string(); }
  }

  static int compare(Block left, Block right) {
    int tag = left.tag().compareTo(right.tag());
    return tag != 0 ? tag : left.string().compareTo(right.string());
  }
}
