package std.lib.block;

import hara.kernel.base.Reader;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.Vector;
import org.junit.Test;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

import static org.junit.Assert.*;

public class BlockComprehensiveTest {
  @Test
  public void checkPredicatesHandlePositiveAndNegativeCases() {
    assertFalse(Check.boundary('a'));
    assertTrue(Check.boundary(null));
    assertFalse(Check.whitespace(null));
    assertFalse(Check.comma('.'));
    assertFalse(Check.linebreak('\t'));
    assertFalse(Check.delimiter(';'));
    assertFalse(Check.linespace('\t'));
    assertTrue(Check.voidspaceOrBoundary('@'));
    assertNull(Check.voidTag('x'));
    assertFalse(Check.isVoid('x'));
    assertNull(Check.tokenTag(new Object()));
    assertFalse(Check.token(new Object()));
    assertNull(Check.collectionTag("not a collection"));
    assertFalse(Check.collection("not a collection"));
    assertFalse(Check.comment(null));
  }

  @Test
  public void tokenTagsCoverEverySupportedJavaRepresentation() {
    assertEquals("nil", Check.tokenTag(null));
    assertEquals("boolean", Check.tokenTag(false));
    assertEquals("byte", Check.tokenTag((byte) 1));
    assertEquals("short", Check.tokenTag((short) 1));
    assertEquals("long", Check.tokenTag(1));
    assertEquals("bigint", Check.tokenTag(BigInteger.TEN));
    assertEquals("float", Check.tokenTag(1.5f));
    assertEquals("double", Check.tokenTag(1.5d));
    assertEquals("bigdec", Check.tokenTag(BigDecimal.TEN));
    assertEquals("string", Check.tokenTag("x"));
    assertEquals("char", Check.tokenTag('x'));
    assertEquals("regexp", Check.tokenTag(Pattern.compile("x")));
  }

  @Test
  public void voidBlocksHaveAccurateTagsDimensionsAndRepresentations() {
    Block.Void eof = Construct.voidBlock(null);
    assertEquals("eof", eof.tag());
    assertEquals("", eof.string());
    assertEquals(0, eof.length());
    assertEquals(0, eof.width());
    assertTrue(eof.verify());

    assertEquals("comma", Construct.voidBlock(',').tag());
    assertEquals("linetab", Construct.tab().tag());
    assertEquals("\\t", Construct.tab().toString());
    assertEquals("\\n", Construct.newline().toString());
    assertEquals("␣", Construct.space().toString());
    assertEquals(3, Construct.spaces(3).size());
    assertEquals(2, Construct.tabs(2).size());
    assertEquals(4, Construct.newlines(4).size());
    assertTrue(Construct.newline(Construct.RETURN));
  }

  @Test
  public void blockTypesVerifyCompareAndReplaceChildren() {
    Block.Comment comment = Construct.comment(";; note");
    assertTrue(comment.verify());
    assertEquals("comment", comment.type());

    Block.Token one = Construct.token(1L);
    Block.Token two = Construct.token(2L);
    assertTrue(one.compareTo(two) < 0);
    assertFalse(new Block.Token("string", "1", 1L, "1", 1, 0).verify());

    Block.Container original = Construct.container("vector", List.of(one));
    Block.Container replaced =
        original.replaceChildren(Vector.Standard.from(null, two));
    assertEquals("[1]", original.string());
    assertEquals("[2]", replaced.string());
    assertTrue(original.verify());
    assertThrows(
        IllegalArgumentException.class,
        () -> new Block.Container("meta", Vector.Standard.from(null, one), Construct.props("meta")));
  }

  @Test
  public void constructionSupportsAllRetainedCollectionKinds() {
    assertEquals("[1 2]", Construct.block(Vector.Standard.from(null, 1L, 2L)).string());
    assertEquals("(1 2)", Construct.block(Arrays.asList(1L, 2L)).string());
    Block.Container set = (Block.Container) Construct.block(Set.of(1L, 2L));
    assertEquals("set", set.tag());
    assertEquals(Set.of(1L, 2L), set.value());

    LinkedHashMap<Object, Object> map = new LinkedHashMap<>();
    map.put(Keyword.create("a"), 1L);
    assertEquals("{:a 1}", Construct.block(map).string());
    assertThrows(IllegalArgumentException.class, () -> Construct.block(new Object()));
    assertThrows(
        IllegalArgumentException.class,
        () -> Construct.container("unknown", List.of()));
  }

  @Test
  public void multilineDimensionsAndWidthHelpersMatchSourceLayout() {
    Block.Token token = Construct.stringToken("a\nlong");
    assertEquals(1, token.height());
    assertEquals(5, token.width());
    Block.Container block =
        Construct.container(
            "vector",
            List.of(Construct.token(1L), Construct.newline(), Construct.token(22L)));
    assertEquals("[1\n22]", block.string());
    assertEquals(3, block.width());
    assertEquals(3, Construct.maxWidth(block));
    assertEquals(2, Construct.maxWidth(block, 2));
    assertEquals(3, Construct.lineWidth(block));
    assertEquals(List.of("[1", "22]"), Construct.lines(block));
    assertSame(block.children(), Construct.contents(block));
    assertSame(token, Construct.contents(token));
  }

  @Test
  public void parserPreservesEveryVoidCommentAndNestedSourceCharacter() {
    String source = "(\t1,\r\n;note\n[2 {:a #{3}}])";
    Block.Container parsed = Parser.parseRoot(source);
    assertEquals(source, parsed.string());
    assertEquals("list", ((Block.Container) parsed.children().nth(0)).tag());
    assertEquals("eof", Parser.parseString("").tag());

    Parser readerParser = new Parser(new Reader("[1 2]"));
    assertEquals("[1 2]", readerParser.parse().string());
  }

  @Test
  public void parserReportsPreciseMultilineAndDispatchErrors() {
    RuntimeException mismatch =
        assertThrows(RuntimeException.class, () -> Parser.parseRoot("(\n [1})"));
    assertTrue(mismatch.getMessage().contains("expected ']' but found '}'"));
    assertTrue(mismatch.getMessage().contains("line 2, column 4"));

    assertThrows(RuntimeException.class, () -> Parser.parseString("\\unknown"));
    assertThrows(RuntimeException.class, () -> Parser.parseString(":"));
    assertThrows(RuntimeException.class, () -> Parser.parseString("#\"[\""));
    assertThrows(RuntimeException.class, () -> Parser.parseString("~@"));
    assertThrows(RuntimeException.class, () -> Parser.parseString("^:a"));
  }

  @Test
  public void valuesCoverAllRetainedPrefixAndDispatchForms() {
    assertEquals(
        List.of(Symbol.create("var"), Symbol.create("x")),
        ((Block.IExpression) Parser.parseString("#'x")).value());
    assertEquals(
        List.of(Symbol.create("unquote"), Symbol.create("x")),
        ((Block.IExpression) Parser.parseString("~x")).value());
    assertEquals(
        List.of(Symbol.create("unquote-splicing"), Symbol.create("x")),
        ((Block.IExpression) Parser.parseString("~@x")).value());
    assertEquals(
        List.of(Symbol.create("syntax-quote"), Symbol.create("x")),
        ((Block.IExpression) Parser.parseString("`x")).value());
    assertEquals(
        List.of(Symbol.create("eval"), List.of(Symbol.create("f"))),
        ((Block.IExpression) Parser.parseString("#=(f)")).value());

    List<?> splice = (List<?>) ((Block.IExpression) Parser.parseString("#?@(:clj [x])")).value();
    assertEquals(Symbol.create("?-splicing"), splice.get(0));
    assertEquals(
        List.of(Symbol.create("x")),
        ((Map<?, ?>) splice.get(1)).get(Keyword.create("clj")));

    Value.WithMetadata tagged =
        (Value.WithMetadata) ((Block.IExpression) Parser.parseString("^String [1]")).value();
    assertEquals(Symbol.create("String"), tagged.metadata().get(Keyword.create("tag")));

    Value.WithMetadata mapped =
        (Value.WithMetadata) ((Block.IExpression) Parser.parseString("^{:a 1} [2]")).value();
    assertEquals(1L, mapped.metadata().get(Keyword.create("a")));

    List<?> fn = (List<?>) ((Block.IExpression) Parser.parseString("#(+ % 1)")).value();
    assertEquals(Symbol.create("fn*"), fn.get(0));
  }

  @Test
  public void modifiersApplyInReferenceOrder() {
    Block.Container vector = (Block.Container) Parser.parseString("[#_#_1 2 #|3]");
    assertEquals(List.of(3L), vector.value());

    List<Block> applied =
        Value.applyModifiers(
            List.of(Construct.uneval(), Construct.token(1L), Construct.token(2L)));
    assertEquals(List.of(Construct.token(2L).value()), values(applied));
  }

  private static List<Object> values(List<Block> blocks) {
    return blocks.stream().map(block -> ((Block.IExpression) block).value()).toList();
  }
}
