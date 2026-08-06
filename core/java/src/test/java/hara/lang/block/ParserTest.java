package hara.lang.block;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertTrue;

public class ParserTest {

  @Test
  public void testParseComment() {
    Block.IBlock block = Parser.parseString(";this is a comment");
    assertTrue(block instanceof Block.Comment);
    assertEquals(";this is a comment", block.string());
  }

  @Test
  public void testParseToken() {
    Block.IBlock block = Parser.parseString("abc");
    assertTrue(block instanceof Block.Token);
    assertEquals("abc", block.string());
  }

  @Test
  public void testParseString() {
    Block.IBlock block = Parser.parseString("\"hello world\"");
    assertTrue(block instanceof Block.Token);
    assertEquals("\"hello world\"", block.string());
    assertEquals("hello world", ((Block.Token) block).value());
  }

  @Test
  public void testParseList() {
    Block.IBlock block = Parser.parseString("(1 2 3)");
    assertTrue(block instanceof Block.Container);
    Block.Container container = (Block.Container) block;
    assertEquals("list", container.tag());
    assertEquals(5, container.children().count()); // Includes whitespace
  }

  @Test
  public void testParseVector() {
    Block.IBlock block = Parser.parseString("[1 2 3]");
    assertTrue(block instanceof Block.Container);
    Block.Container container = (Block.Container) block;
    assertEquals("vector", container.tag());
    assertEquals(5, container.children().count());
  }

  @Test
  public void testParseMap() {
    Block.IBlock block = Parser.parseString("{:a 1}");
    assertTrue(block instanceof Block.Container);
    Block.Container container = (Block.Container) block;
    assertEquals("map", container.tag());
    assertEquals(3, container.children().count());
  }

  @Test
  public void testParseRoot() {
    Block.Container root = Parser.parseRoot("a b c");
    assertEquals("root", root.tag());
    assertEquals(5, root.children().count());
  }

  @Test
  public void testParseNestedCollectionsRestoresOuterDelimiter() {
    Block.Container root = Parser.parseRoot("([1])");
    assertEquals(1, root.children().count());
  }

  @Test
  public void testParseUnterminatedCollectionReportsExpectedDelimiter() {
    RuntimeException error = assertThrows(RuntimeException.class, () -> Parser.parseRoot("(1 2"));
    assertTrue(error.getMessage().contains("EOF while reading list, expected ')'"));
    assertTrue(error.getMessage().contains("line 1, column 5"));
  }

  @Test
  public void testParseMismatchedCollectionReportsBothDelimiters() {
    RuntimeException error = assertThrows(RuntimeException.class, () -> Parser.parseRoot("(1]"));
    assertTrue(error.getMessage().contains("expected ')' but found ']'"));
    assertTrue(error.getMessage().contains("line 1, column 3"));
  }

  @Test
  public void testParsePrefixAtEofReportsMissingOperand() {
    RuntimeException quote = assertThrows(RuntimeException.class, () -> Parser.parseRoot("'"));
    assertTrue(quote.getMessage().contains("EOF while reading quote"));

    RuntimeException splice = assertThrows(RuntimeException.class, () -> Parser.parseRoot("~@"));
    assertTrue(splice.getMessage().contains("EOF while reading unquote-splice"));
  }

  @Test
  public void testParseHashAtEofReportsReaderError() {
    RuntimeException error = assertThrows(RuntimeException.class, () -> Parser.parseRoot("#"));
    assertTrue(error.getMessage().contains("EOF while reading hash dispatch"));
    assertTrue(error.getMessage().contains("line 1, column 2"));
  }
}
