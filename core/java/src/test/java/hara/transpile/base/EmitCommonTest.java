package hara.transpile.base;

import hara.lang.data.Keyword;
import org.junit.Before;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

public class EmitCommonTest {

  private Map<String, Object> mopts;
  private Map<String, Object> grammar;
  private EmitCommon.Context ctx;

  @Before
  public void setUp() {
    mopts = new HashMap<>();
    grammar = new HashMap<>();
    ctx = new EmitCommon.Context();
    // Simple emit function that just returns the string representation of the form
    ctx.emitFn = (form, g, m) -> form.toString();
    mopts.put("context", ctx);
  }

  @Test
  public void testNewlineIndent() {
    ctx.indent = 2;
    assertEquals("\n  ", EmitCommon.newlineIndent(ctx));

    ctx.compressed = true;
    assertEquals(" ", EmitCommon.newlineIndent(ctx));
  }

  @Test
  public void testEmitInvoke() {
    assertEquals(
        "foo(a, b)", EmitCommon.emitInvoke("foo", Arrays.asList("a", "b"), grammar, mopts));
    assertEquals("(a)", EmitCommon.emitInvoke(null, Arrays.asList("a"), grammar, mopts));
  }

  @Test
  public void testEmitAssign() {
    assertEquals("a = b", EmitCommon.emitAssign("=", Arrays.asList("a", "b"), grammar, mopts));
  }

  @Test
  public void testEmitInfix() {
    assertEquals(
        "a + b + c", EmitCommon.emitInfix("+", Arrays.asList("a", "b", "c"), grammar, mopts));
  }

  @Test
  public void testEmitPrefix() {
    assertEquals("! a", EmitCommon.emitPrefix("!", Arrays.asList("a"), grammar, mopts));
  }

  @Test
  public void testEmitPostfix() {
    assertEquals("a ++", EmitCommon.emitPostfix("++", Arrays.asList("a"), grammar, mopts));
  }

  @Test
  public void testEmitNew() {
    assertEquals(
        "new Foo(a, b)", EmitCommon.emitNew("new", Arrays.asList("Foo", "a", "b"), grammar, mopts));
  }

  @Test
  public void testEmitIndex() {
    assertEquals(
        "arr[0][1]", EmitCommon.emitIndex("arr", Arrays.asList("arr", "0", "1"), grammar, mopts));
  }

  @Test
  public void testEmitReturn() {
    assertEquals(
        "return val", EmitCommon.emitReturn("return", Arrays.asList("val"), grammar, mopts));
    assertEquals(
        "return", EmitCommon.emitReturn("return", Collections.emptyList(), grammar, mopts));
  }

  @Test
  public void testFormKey() {
    // Test basic form key
    assertEquals(
        Arrays.asList(Keyword.create("keyword"), Keyword.create("token"), true),
        EmitCommon.formKey(Keyword.create("foo"), grammar));
  }
}
