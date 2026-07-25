package std.lib.block;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import org.junit.Test;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;

import static org.junit.Assert.*;

public class ParserValueTest {
  private static Block.Token token(String source) { return (Block.Token) Parser.parseString(source); }
  private static Block.Container container(String source) {
    return (Block.Container) Parser.parseString(source);
  }

  @Test
  public void parsesTypedTokensAndKeywords() {
    assertNull(token("nil").value());
    assertEquals(true, token("true").value());
    assertEquals(12L, token("12").value());
    assertEquals(-16L, token("-0x10").value());
    assertEquals(new BigInteger("123"), token("123N").value());
    assertEquals(new BigDecimal("12.5"), token("12.5M").value());
    assertEquals(125.0d, (Double) token("1.25e2").value(), 0.0d);
    assertEquals(Symbol.create("hello/world"), token("hello/world").value());
    assertEquals(Keyword.create("hello/world"), token(":hello/world").value());
    assertEquals("keyword", token("::hello").tag());
    assertThrows(RuntimeException.class, () -> Parser.parseString("3/5"));
  }

  @Test
  public void parsesCharactersEscapesAndMultilineStrings() {
    assertEquals('\n', token("\\newline").value());
    assertEquals('A', token("\\u0041").value());
    Block.Token escaped = token("\"a\\n\\\"b\\\\c\"");
    assertEquals("a\n\"b\\c", escaped.value());
    assertEquals("\"a\\n\\\"b\\\\c\"", escaped.string());

    Block.Token multiline = token("\"hello\nworld\"");
    assertEquals("hello\nworld", multiline.value());
    assertEquals(1, multiline.height());
    assertEquals(6, multiline.width());
    assertThrows(RuntimeException.class, () -> Parser.parseString("\"unfinished"));
  }

  @Test
  public void parsesCollectionsPrefixesAndHashDispatch() {
    assertEquals("list", container("(1)").tag());
    assertEquals("vector", container("[1]").tag());
    assertEquals("map", container("{:a 1}").tag());
    assertEquals("set", container("#{1}").tag());
    assertEquals("fn", container("#(+ % 1)").tag());
    assertEquals("quote", container("'x").tag());
    assertEquals("deref", container("@x").tag());
    assertEquals("meta", container("^:dynamic x").tag());
    assertEquals("syntax", container("`x").tag());
    assertEquals("unquote", container("~x").tag());
    assertEquals("unquote-splice", container("~@x").tag());
    assertEquals("var", container("#'x").tag());
    assertEquals("hash-keyword", container("#:hello{:a 1}").tag());
    assertEquals("hash-meta", container("#^:dynamic x").tag());
    assertEquals("hash-eval", container("#=(f)").tag());
    assertEquals("select", container("#?(:clj x)").tag());
    assertEquals("select-splice", container("#?@(:clj [x])").tag());
    assertEquals("hash-uneval", Parser.parseString("#_").tag());
    assertEquals("hash-cursor", Parser.parseString("#|").tag());
    assertEquals("regexp", token("#\"a+\"").tag());
    assertEquals("a+", ((Pattern) token("#\"a+\"").value()).pattern());
    assertThrows(RuntimeException.class, () -> Parser.parseString("#foo"));
  }

  @Test
  public void preservesRootAndReportsReaderPositions() {
    assertEquals("a b", Parser.parseRoot("a b").string());
    assertEquals("a", Parser.parseFirst(" ;comment\na b").string());
    assertNull(Parser.parseFirst(" ;comment"));

    RuntimeException eof = assertThrows(RuntimeException.class, () -> Parser.parseRoot("(1 2"));
    assertTrue(eof.getMessage().contains("expected ')'"));
    assertTrue(eof.getMessage().contains("line 1, column 5"));
    RuntimeException mismatch = assertThrows(RuntimeException.class, () -> Parser.parseRoot("[1}"));
    assertTrue(mismatch.getMessage().contains("expected ']' but found '}'"));
    assertTrue(mismatch.getMessage().contains("line 1, column 3"));
    assertThrows(RuntimeException.class, () -> Parser.parseRoot(")"));
    assertThrows(RuntimeException.class, () -> Parser.parseRoot("'"));
    assertThrows(RuntimeException.class, () -> Parser.parseRoot("#"));
  }

  @Test
  @SuppressWarnings("unchecked")
  public void evaluatesContainersAndModifiers() {
    assertEquals(List.of(1L, 3L), container("[1 #_2 3]").value());
    assertEquals(List.of(Symbol.create("quote"), Symbol.create("hello")), container("'hello").value());
    assertEquals(List.of(Symbol.create("deref"), Symbol.create("hello")), container("@hello").value());
    assertEquals(List.of(1L, 2L), container("(1 2)").value());
    assertEquals(List.of(1L, 2L), container("[1 2]").value());
    assertEquals(Set.of(1L, 2L), container("#{1 2}").value());
    assertEquals(Map.of(1L, 2L, 3L, 4L), container("{1 2 3 4}").value());
    assertThrows(IllegalArgumentException.class, () -> container("{1 2 3}").value());

    List<?> root = (List<?>) Parser.parseRoot("1 2").value();
    assertEquals(Symbol.create("do"), root.get(0));
    assertEquals(List.of(1L, 2L), root.subList(1, 3));

    Value.WithMetadata metadata = (Value.WithMetadata) container("^:dynamic [1]").value();
    assertEquals(List.of(1L), metadata.value());
    assertEquals(true, metadata.metadata().get(Keyword.create("dynamic")));

    Map<Keyword, Object> namespaced = (Map<Keyword, Object>) container("#:hello{:a 1}").value();
    assertEquals(1L, namespaced.get(Keyword.create("hello", "a")));

    List<?> select = (List<?>) container("#?(:clj hello)").value();
    assertEquals(Symbol.create("?"), select.get(0));
    assertEquals(Symbol.create("hello"), ((Map<?, ?>) select.get(1)).get(Keyword.create("clj")));
  }
}
