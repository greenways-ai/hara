package hara.kernel.base;

import hara.lang.data.*;
import org.junit.Test;

import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.file.Path;

import static org.junit.Assert.*;

public class ParserTest {

  @Test
  public void testReadStringNumber() {
    assertEquals(123L, Parser.LispReader.readString("123", null));
    assertEquals(123.45, Parser.LispReader.readString("123.45", null));
    assertEquals(BigInteger.valueOf(123), Parser.LispReader.readString("123N", null));
    assertEquals(new BigDecimal("123.45"), Parser.LispReader.readString("123.45M", null));
    assertEquals(BigDecimal.ONE, Parser.LispReader.readString("1.00M", null));
    assertEquals(0xFFL, Parser.LispReader.readString("0xFF", null));
  }

  @Test
  public void readableNumericPrintingPreservesBigNumberCategories() {
    BigInteger integer = BigInteger.valueOf(123);
    BigDecimal decimal = new BigDecimal("123.45");
    assertEquals("123N", hara.lang.base.G.display(integer));
    assertEquals("123.45M", hara.lang.base.G.display(decimal));
    assertEquals(integer, Parser.LispReader.readString(hara.lang.base.G.display(integer), null));
    assertEquals(decimal, Parser.LispReader.readString(hara.lang.base.G.display(decimal), null));
  }

  @Test
  public void l0ConformanceCorpusIsReadableEdn() throws Exception {
    Object corpus =
        Parser.LispReader.readString(
            Files.readString(Path.of("specs/hara/corpora/l0-conformance.edn")), null);
    assertTrue(corpus instanceof hara.lang.data.types.IMapType);
    hara.lang.data.types.IMapType map = (hara.lang.data.types.IMapType) corpus;
    assertEquals("0.1", map.lookup(Keyword.create("spec/version")));
    assertTrue(map.lookup(Keyword.create("cases")) instanceof hara.lang.data.types.ILinearType);
  }

  @Test
  public void testReadStringSymbol() {
    assertEquals(Symbol.create("a"), Parser.LispReader.readString("a", null));
    assertEquals(Keyword.create("a"), Parser.LispReader.readString(":a", null));
    assertEquals(Keyword.create("ns", "a"), Parser.LispReader.readString(":ns/a", null));
    assertEquals(Symbol.create("ns", "a"), Parser.LispReader.readString("ns/a", null));
  }

  @Test
  public void testReadStringString() {
    assertEquals("hello", Parser.LispReader.readString("\"hello\"", null));
    assertEquals("hello \"world\"", Parser.LispReader.readString("\"hello \\\"world\\\"\"", null));
  }

  @Test
  public void testReadStringChar() {
    assertEquals('a', Parser.LispReader.readString("\\a", null));
    assertEquals('\n', Parser.LispReader.readString("\\newline", null));
    assertEquals(' ', Parser.LispReader.readString("\\space", null));
  }

  @Test
  public void testReadStringList() {
    Object result = Parser.LispReader.readString("(+ 1 2)", null);
    assertTrue(result instanceof List);
    List list = (List) result;
    assertEquals(3, list.count());
    assertEquals(Symbol.create("+"), list.nth(0));
    assertEquals(1L, list.nth(1));
    assertEquals(2L, list.nth(2));
  }

  @Test
  public void testReadStringVector() {
    Object result = Parser.LispReader.readString("[1 2 3]", null);
    // Vectors <= 5 elements might be Tuples if not handled carefully, but
    // Parser.VectorReader
    // checks size > 5
    // Parser.java: if (list.size() > 5) return vector(list); else return
    // tuple(list.toArray());
    // So [1 2 3] is a Tuple.
    assertTrue(result instanceof hara.lang.data.types.ILinearType);
    hara.lang.data.types.ILinearType v = (hara.lang.data.types.ILinearType) result;
    assertEquals(3, v.count());
  }

  @Test
  public void testReadStringVectorLarge() {
    Object result = Parser.LispReader.readString("[1 2 3 4 5 6]", null);
    // > 5 elements -> Vector
    assertTrue(result instanceof Vector);
    Vector v = (Vector) result;
    assertEquals(6, v.count());
  }

  @Test
  public void testReadStringMap() {
    Object result = Parser.LispReader.readString("{:a 1 :b 2}", null);
    assertTrue(result instanceof OrderedMap);
    OrderedMap map = (OrderedMap) result;
    assertEquals(2, map.count());
    assertEquals(1L, map.lookup(Keyword.create("a")));
    assertEquals(2L, map.lookup(Keyword.create("b")));
  }

  @Test
  public void testReadQuote() {
    Object result = Parser.LispReader.readString("'a", null);
    assertTrue(result instanceof List);
    List l = (List) result;
    assertEquals(Symbol.create("quote"), l.nth(0));
    assertEquals(Symbol.create("a"), l.nth(1));
  }

  @Test
  public void testReadComment() {
    // Comments return the reader, which loop in read() ignores.
    // readString(";", null) -> might throw EOF if nothing else.
    try {
      Parser.LispReader.readString("; comment", null);
      fail("Should throw EOF");
    } catch (Exception e) {
      // Expected EOF
    }

    assertEquals(1L, Parser.LispReader.readString("; comment\n1", null));
  }

  @Test
  public void testReadMetadata() {
    Object result = Parser.LispReader.readString("^:foo [1]", null);
    assertTrue(result instanceof hara.lang.protocol.IObjType);
    hara.lang.protocol.IObjType obj = (hara.lang.protocol.IObjType) result;
    hara.lang.data.types.IMapType meta = (hara.lang.data.types.IMapType) obj.meta();
    assertEquals(Boolean.TRUE, meta.lookup(Keyword.create("foo")));
  }

  @Test
  public void testReadMapMetadata() {
    Object result = Parser.LispReader.readString("^{:tag \"fast\"} [1]", null);
    assertTrue(result instanceof hara.lang.protocol.IObjType);
    hara.lang.protocol.IObjType obj = (hara.lang.protocol.IObjType) result;
    hara.lang.data.types.IMapType meta = (hara.lang.data.types.IMapType) obj.meta();
    assertEquals("fast", meta.lookup(Keyword.create("tag")));
  }

  @Test
  public void testSyntaxQuote() {
    Object result = Parser.LispReader.readString("`a", null);
    assertTrue(result instanceof List);
    assertEquals(Symbol.create("syntax-quote"), ((List) result).nth(0));
  }

  @Test
  public void testReadUnmatchedDelimiter() {
    try {
      Parser.LispReader.readString(")", null);
      fail("Should throw RuntimeException for unmatched delimiter");
    } catch (Exception e) {
      assertTrue(e.getMessage().contains("Unmatched delimiter"));
    }
  }

  @Test
  public void testReadUnfinishedString() {
    try {
      Parser.LispReader.readString("\"", null);
      fail("Should throw RuntimeException for EOF while reading string");
    } catch (Exception e) {
      // ReaderException wraps the actual exception
      assertTrue(e.getCause().getMessage().contains("EOF while reading string"));
    }
  }

  @Test
  public void testReadInvalidNumber() {
    try {
      // "123a" - Parser splits at macro/whitespace. 'a' is not macro/whitespace.
      // Wait, 123a is read as a token? No, readNumber reads until macro or
      // whitespace.
      // If 1 starts, it calls readNumber.
      // readNumber loops until macro or whitespace. 'a' is neither.
      // So it reads "123a" and tries to matchNumber("123a").
      // matchNumber will return null.
      // Then it throws NumberFormatException.
      Parser.LispReader.readString("123a", null);
      fail("Should throw NumberFormatException");
    } catch (Exception e) {
      assertTrue(e.getCause() instanceof NumberFormatException);
    }
  }

  @Test
  public void testReadNilTrueFalse() {
    assertNull(Parser.LispReader.readString("nil", null));
    assertEquals(Boolean.TRUE, Parser.LispReader.readString("true", null));
    assertEquals(Boolean.FALSE, Parser.LispReader.readString("false", null));
  }

  @Test
  public void testDeref() {
    Object result = Parser.LispReader.readString("@a", null);
    assertTrue(result instanceof List);
    List l = (List) result;
    assertEquals(Symbol.create("deref"), l.nth(0));
    assertEquals(Symbol.create("a"), l.nth(1));
  }

  @Test
  public void testMapDuplicateKey() {
    try {
      Parser.LispReader.readString("{:a 1 :a 2}", null);
      fail("Expected RuntimeException for duplicate key");
    } catch (Parser.LispReader.ReaderException e) {
      assertTrue(e.getCause().getMessage().contains("Duplicate key"));
    } catch (hara.lang.base.Ex.Runtime e) {
      assertTrue(e.getMessage().contains("Duplicate key"));
    }
  }

  @Test
  public void testSetDuplicateItem() {
    try {
      Parser.LispReader.readString("#{1 1}", null);
      fail("Expected RuntimeException for duplicate item");
    } catch (Parser.LispReader.ReaderException e) {
      assertTrue(e.getCause().getMessage().contains("Duplicate item"));
    } catch (hara.lang.base.Ex.Runtime e) {
      assertTrue(e.getMessage().contains("Duplicate item"));
    }
  }

  @Test
  public void testDiscardReader() {
    // #_ ignores the next form
    assertEquals(1L, Parser.LispReader.readString("#_ 2 1", null));
  }

  @Test
  public void taggedHandleSyntaxReadsAsInertData() {
    Object result = Parser.LispReader.readString("#math[:tensor 42]", null);
    assertTrue(result instanceof TaggedLiteral);
    TaggedLiteral tagged = (TaggedLiteral) result;
    assertEquals(Symbol.create("math"), tagged.tag());
    assertEquals("#math[:tensor 42]", tagged.display());
    assertEquals(result, Parser.LispReader.readString(tagged.display(), null));
  }

  @Test
  public void testVarQuoteReader() {
    Object result = Parser.LispReader.readString("#'a", null);
    assertTrue(result instanceof List);
    List l = (List) result;
    assertEquals(Symbol.create("var"), l.nth(0));
    assertEquals(Symbol.create("a"), l.nth(1));
  }

  @Test
  public void testQueueDispatchIsRejected() {
    try {
      Parser.LispReader.readString("#[1 2]", null);
      fail("Expected unknown dispatch macro");
    } catch (Parser.LispReader.ReaderException e) {
      assertTrue(e.getCause().getMessage().contains("No dispatch macro for: ["));
    }
  }

  @Test
  public void testRegexReader() {
    Object result = Parser.LispReader.readString("#\"abc\"", null);
    assertTrue(result instanceof java.util.regex.Pattern);
    java.util.regex.Pattern p = (java.util.regex.Pattern) result;
    assertEquals("abc", p.pattern());
  }

  @Test
  public void testUnquoteReader() {
    // Unquote is usually only valid inside syntax-quote, but the reader just
    // produces a symbol wrapping
    Object result = Parser.LispReader.readString("~a", null);
    assertTrue(result instanceof List);
    List l = (List) result;
    assertEquals(Symbol.create("unquote"), l.nth(0));
    assertEquals(Symbol.create("a"), l.nth(1));

    Object resultSplice = Parser.LispReader.readString("~@a", null);
    assertTrue(resultSplice instanceof List);
    List lSplice = (List) resultSplice;
    assertEquals(Symbol.create("unquote-splicing"), lSplice.nth(0));
    assertEquals(Symbol.create("a"), lSplice.nth(1));
  }

  @Test
  public void testCharacterReaderExtended() {
    assertEquals('\u0000', Parser.LispReader.readString("\\u0000", null));
    assertEquals('\uFFFF', Parser.LispReader.readString("\\uFFFF", null));
    assertEquals('\t', Parser.LispReader.readString("\\tab", null));
    assertEquals('\b', Parser.LispReader.readString("\\backspace", null));
    assertEquals('\f', Parser.LispReader.readString("\\formfeed", null));
    assertEquals('\r', Parser.LispReader.readString("\\return", null));

    // Octal
    assertEquals('\007', Parser.LispReader.readString("\\o007", null));
    assertEquals('\377', Parser.LispReader.readString("\\o377", null));
  }

  @Test
  public void testStringReaderExtended() {
    assertEquals(
        "\t\r\n\b\f\\\"", Parser.LispReader.readString("\"\\t\\r\\n\\b\\f\\\\\\\"\"", null));
    assertEquals("\u0000", Parser.LispReader.readString("\"\\u0000\"", null));
    assertEquals("\7", Parser.LispReader.readString("\"\\7\"", null)); // Octal in string
  }
}
