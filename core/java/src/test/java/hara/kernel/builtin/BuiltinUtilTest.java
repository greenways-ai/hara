package hara.kernel.builtin;

import hara.lang.data.Keyword;
import hara.lang.data.Map;
import hara.lang.protocol.IObjType;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;

public class BuiltinUtilTest {

  @Test
  public void testPrStr() {
    assertEquals("\"a\"", BuiltinUtil.prStr("a"));
    assertEquals("1", BuiltinUtil.prStr(1));
  }

  @Test
  public void testStr() {
    assertEquals("a1", BuiltinUtil.str(Arrays.asList("a", 1)));
    assertEquals("", BuiltinUtil.str(Arrays.asList((Object) null)));
  }

  @Test
  public void testDoc() {
    // doc prints to stdout, difficult to test output without capturing stdout.
    // Just verify it runs without error on an object with meta.

    IObjType obj = hara.lang.data.Symbol.create("test");
    Map.Standard meta =
        Map.Standard.EMPTY
            .assoc(Keyword.create("doc"), "documentation")
            .assoc(Keyword.create("name"), "atom");

    obj = obj.withMeta(meta);

    BuiltinUtil.doc(obj);
  }
}
