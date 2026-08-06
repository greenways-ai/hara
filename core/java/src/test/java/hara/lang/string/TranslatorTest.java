package hara.lang.string;

import org.junit.Test;
import static org.junit.Assert.*;
import java.io.IOException;
import java.io.Writer;
import java.io.StringWriter;

public class TranslatorTest {

  @Test
  public void testHex() {
    assertEquals("A", CharSequenceTranslator.hex(10));
    assertEquals("F", CharSequenceTranslator.hex(15));
    assertEquals("10", CharSequenceTranslator.hex(16));
  }

  @Test
  public void testAbstractMethods() {
    CharSequenceTranslator translator =
        new CharSequenceTranslator() {
          @Override
          public int translate(CharSequence input, int index, Writer out) throws IOException {
            if (input.charAt(index) == 'a') {
              out.write("b");
              return 1;
            }
            return 0;
          }
        };

    assertEquals("bcd", translator.translate("acd"));
    assertEquals(null, translator.translate(null));

    StringWriter writer = new StringWriter();
    try {
      translator.translate(null, writer);
      assertEquals("", writer.toString());

      translator.translate("acd", writer);
      assertEquals("bcd", writer.toString());
    } catch (IOException e) {
      fail("IOException should not be thrown");
    }
  }

  @Test
  public void testAggregateTranslator() {
    CharSequenceTranslator t1 =
        new CharSequenceTranslator() {
          @Override
          public int translate(CharSequence input, int index, Writer out) throws IOException {
            if (input.charAt(index) == 'a') {
              out.write("1");
              return 1;
            }
            return 0;
          }
        };

    CharSequenceTranslator t2 =
        new CharSequenceTranslator() {
          @Override
          public int translate(CharSequence input, int index, Writer out) throws IOException {
            if (input.charAt(index) == 'b') {
              out.write("2");
              return 1;
            }
            return 0;
          }
        };

    AggregateTranslator agg = new AggregateTranslator(t1, t2);
    assertEquals("12c", agg.translate("abc"));

    // Test with() method
    CharSequenceTranslator t3 = t1.with(t2);
    assertEquals("12c", t3.translate("abc"));
  }
}
