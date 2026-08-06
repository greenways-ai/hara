package hara.lang.string;

import org.junit.Test;
import static org.junit.Assert.*;

public class EscaperTest {

  @Test
  public void testJavaUnicodeEscaper() {
    JavaUnicodeEscaper escaper = JavaUnicodeEscaper.above('a');
    assertEquals("a", escaper.translate("a"));
    assertEquals("\\u0062", escaper.translate("b"));

    escaper = JavaUnicodeEscaper.between('a', 'c');
    assertEquals("\\u0061", escaper.translate("a"));
    assertEquals("\\u0062", escaper.translate("b"));
    assertEquals("\\u0063", escaper.translate("c"));
    assertEquals("d", escaper.translate("d"));
  }

  @Test
  public void testNumericEntityEscaper() {
    NumericEntityEscaper escaper = NumericEntityEscaper.above('a');
    assertEquals("a", escaper.translate("a"));
    assertEquals("&#98;", escaper.translate("b"));

    escaper = NumericEntityEscaper.between('a', 'c');
    assertEquals("&#97;", escaper.translate("a"));
    assertEquals("&#98;", escaper.translate("b"));
    assertEquals("&#99;", escaper.translate("c"));
    assertEquals("d", escaper.translate("d"));
  }

  @Test
  public void testUnicodeUnpairedSurrogateRemover() {
    UnicodeUnpairedSurrogateRemover remover = new UnicodeUnpairedSurrogateRemover();
    // High surrogate D800
    assertEquals("", remover.translate("\uD800"));
    assertEquals("a", remover.translate("a"));
  }
}
