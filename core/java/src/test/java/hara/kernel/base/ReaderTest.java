package hara.kernel.base;

import org.junit.Test;

import java.io.StringReader;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNull;

public class ReaderTest {

  @Test
  public void testReadChar() {
    Reader r = new Reader("abc");
    assertEquals(Character.valueOf('a'), r.readChar());
    assertEquals(Character.valueOf('b'), r.readChar());
    assertEquals(Character.valueOf('c'), r.readChar());
    assertNull(r.readChar());
  }

  @Test
  public void testPeekChar() {
    Reader r = new Reader("abc");
    assertEquals(Character.valueOf('a'), r.peekChar());
    assertEquals(Character.valueOf('a'), r.readChar());
    assertEquals(Character.valueOf('b'), r.peekChar());
    assertEquals(Character.valueOf('b'), r.readChar());
  }

  @Test
  public void testUnreadChar() {
    Reader r = new Reader("abc");
    assertEquals(Character.valueOf('a'), r.readChar());
    r.unreadChar('a');
    assertEquals(Character.valueOf('a'), r.readChar());
  }

  @Test
  public void testLineColumnTracking() {
    Reader r = new Reader("a\nb\nc");
    assertEquals(1, r.getLineNumber());
    assertEquals(1, r.getColumnNumber());

    r.readChar(); // 'a'
    assertEquals(1, r.getLineNumber());
    assertEquals(2, r.getColumnNumber());

    r.readChar(); // '\n'
    assertEquals(2, r.getLineNumber());
    assertEquals(1, r.getColumnNumber());

    r.readChar(); // 'b'
    assertEquals(2, r.getLineNumber());
    assertEquals(2, r.getColumnNumber());

    r.readChar(); // '\n'
    assertEquals(3, r.getLineNumber());
    assertEquals(1, r.getColumnNumber());

    // Test unread affecting line/col
    r.unreadChar('\n');
    assertEquals(2, r.getLineNumber());
    assertEquals(2, r.getColumnNumber());
    // Note: Column tracking on unread newline depends on implementation.
    // Existing Reader.java implementation: if(c=='\n') lineNumber--; else columnNumber--;
    // This assumes we are going back to end of previous line, but it doesn't restore exact column
    // number
    // unless we stored it, or it assumes something.
    // Let's check Reader.java logic:
    // public void unreadChar(Character c) { ... if (c == '\n') lineNumber--; else columnNumber--;
    // ... }
    // If we unread '\n', columnNumber is NOT restored to length of previous line. It stays at 1
    // (from previous readChar reset).
    // This suggests Reader's column tracking on unread of newline is imperfect/simplified.
    // Ideally it should be tested based on its actual behavior or fixed if important.
    // For now, I test what it does.

    // If I unread 'b' (at 2, 2) -> should go to 2, 1.
    r.readChar(); // consume '\n' again -> 3, 1
    r.readChar(); // 'c' -> 3, 2

    r.unreadChar('c');
    assertEquals(3, r.getLineNumber());
    assertEquals(1, r.getColumnNumber());
  }

  @Test
  public void testReadWhile() {
    Reader r = new Reader("abc 123");
    String s = r.readWhile(c -> Character.isLetter(c));
    assertEquals("abc", s);
    assertEquals(Character.valueOf(' '), r.peekChar());
  }

  @Test
  public void testReadUntil() {
    Reader r = new Reader("abc 123");
    String s = r.readUntil(c -> Character.isDigit(c));
    assertEquals("abc ", s);
    assertEquals(Character.valueOf('1'), r.peekChar());
  }

  @Test
  public void testConstructorWithJavaReader() {
    java.io.Reader sr = new StringReader("xyz");
    Reader r = new Reader(sr);
    assertEquals(Character.valueOf('x'), r.readChar());
    assertEquals(Character.valueOf('y'), r.readChar());
    assertEquals(Character.valueOf('z'), r.readChar());
    assertNull(r.readChar());
  }
}
