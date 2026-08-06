package hara.lang.string;

import org.junit.Test;
import static org.junit.Assert.*;

public class EscapeTest {

  @Test
  public void testJava() {
    String input = "He didn't say, \"Stop!\"";
    String expected = "He didn't say, \\\"Stop!\\\"";
    assertEquals(expected, Escape.escapeJava(input));
    assertEquals(input, Escape.unescapeJava(expected));

    assertEquals("\\t", Escape.escapeJava("\t"));
    assertEquals("\t", Escape.unescapeJava("\\t"));
  }

  @Test
  public void testEcmaScript() {
    String input = "He didn't say, \"Stop!\"";
    String expected = "He didn\\'t say, \\\"Stop!\\\"";
    assertEquals(expected, Escape.escapeEcmaScript(input));
    assertEquals(input, Escape.unescapeEcmaScript(expected));

    assertEquals("\\/", Escape.escapeEcmaScript("/"));
  }

  @Test
  public void testJson() {
    String input = "He didn't say, \"Stop!\"";
    String expected = "He didn't say, \\\"Stop!\\\"";
    assertEquals(expected, Escape.escapeJson(input));
    assertEquals(input, Escape.unescapeJson(expected));

    assertEquals("\\/", Escape.escapeJson("/"));
  }

  @Test
  public void testHtml4() {
    String input = "\"bread\" & \"butter\"";
    String expected = "&quot;bread&quot; &amp; &quot;butter&quot;";
    assertEquals(expected, Escape.escapeHtml4(input));
    assertEquals(input, Escape.unescapeHtml4(expected));
  }

  @Test
  public void testHtml3() {
    String input = "\"bread\" & \"butter\"";
    // HTML3 doesn't have &quot;? checking source...
    // Escape.java uses BASIC_ESCAPE for HTML3 which usually includes quot.
    // Let's verify with a simpler one if unsure, or trust standard entities.
    // &lt; &gt; &amp; &quot; are basic.
    String expected = "&quot;bread&quot; &amp; &quot;butter&quot;";
    assertEquals(expected, Escape.escapeHtml3(input));
    assertEquals(input, Escape.unescapeHtml3(expected));
  }

  @Test
  public void testXml() {
    String input = "\"bread\" & 'butter'";
    String expected = "&quot;bread&quot; &amp; &apos;butter&apos;";
    assertEquals(expected, Escape.escapeXml10(input));
    assertEquals(input, Escape.unescapeXml(expected));

    assertEquals(expected, Escape.escapeXml11(input));
  }

  @Test
  public void testXSI() {
    String input = "He didn't say, \"Stop!\"";
    // XSI escapes lots of things including spaces
    String expected = "He\\ didn\\'t\\ say,\\ \\\"Stop!\\\"";
    assertEquals(expected, Escape.escapeXSI(input));
    assertEquals(input, Escape.unescapeXSI(expected));
  }

  @Test
  public void testBuilder() {
    String result =
        Escape.builder(Escape.ESCAPE_HTML4)
            .escape("<b>bold</b>")
            .append(" ")
            .escape("<i>italic</i>")
            .toString();

    assertEquals("&lt;b&gt;bold&lt;/b&gt; &lt;i&gt;italic&lt;/i&gt;", result);
  }
}
