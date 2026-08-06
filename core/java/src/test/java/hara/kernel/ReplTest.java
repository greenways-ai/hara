package hara.kernel;

import org.junit.Test;
import static org.junit.Assert.*;

public class ReplTest {

  @Test
  public void testLispParserDelimiters() {
    Repl.LispParser parser = new Repl.LispParser();
    String buffer = "(a b [c] {d})";

    // Test delimiters
    assertTrue(parser.isDelimiterChar(buffer, 0)); // (
    assertTrue(parser.isDelimiterChar(buffer, 2)); // space
    assertTrue(parser.isDelimiterChar(buffer, 4)); // space
    assertTrue(parser.isDelimiterChar(buffer, 5)); // [
    assertTrue(parser.isDelimiterChar(buffer, 7)); // ]
    assertTrue(parser.isDelimiterChar(buffer, 8)); // space
    assertTrue(parser.isDelimiterChar(buffer, 9)); // {
    assertTrue(parser.isDelimiterChar(buffer, 11)); // }
    assertTrue(parser.isDelimiterChar(buffer, 12)); // )

    // Test non-delimiters
    assertFalse(parser.isDelimiterChar(buffer, 1)); // a
    assertFalse(parser.isDelimiterChar(buffer, 3)); // b
    assertFalse(parser.isDelimiterChar(buffer, 6)); // c
    assertFalse(parser.isDelimiterChar(buffer, 10)); // d
  }
}
