package hara.kernel.base;

import hara.lang.data.Keyword;
import hara.lang.data.List;
import hara.lang.data.Map;
import hara.lang.data.Symbol;
import org.junit.Before;
import org.junit.Test;

import java.math.BigInteger;

import static org.junit.Assert.*;

public class MacroTest {

  private RT.Instance<Object> rt;

  public static class TestClass {
    public String testField = "hello";

    public String testMethod(String arg) {
      return "method " + arg;
    }

    public TestClass() {}

    public TestClass(String val) {
      this.testField = val;
    }
  }

  @Before
  public void setUp() {
    rt = new RT.Instance<>(null, "test");
    rt.eval(
        rt.readString(
            "(ns test (:flavor :jvm) (:import hara.kernel.base.MacroTest$TestClass [java.lang Exception RuntimeException]))"));
    rt.setObj(Symbol.create("test-instance"), new Var("test-instance", new TestClass()));
  }

  private void assertNumber(long expected, Object actual) {
    if (actual instanceof BigInteger) {
      assertEquals(BigInteger.valueOf(expected), actual);
    } else if (actual instanceof Long) {
      assertEquals(Long.valueOf(expected), actual);
    } else if (actual instanceof Integer) {
      assertEquals(Integer.valueOf((int) expected), actual);
    } else {
      fail("Expected number, got: " + (actual == null ? "null" : actual.getClass().getName()));
    }
  }

  // --- Java Interop ---

  @Test
  public void testNew() {
    // (new hara.kernel.base.MacroTest$TestClass "created")
    String code = "(new TestClass \"created\")";
    Object res = rt.eval(rt.readString(code));
    assertTrue(res instanceof TestClass);
    assertEquals("created", ((TestClass) res).testField);
  }

  @Test
  public void testExplicitJvmFieldWrite() {
    assertEquals(
        "changed",
        rt.eval(
            rt.readString(
                "(do (hara.native.jvm/set! test-instance \"testField\" \"changed\") (. test-instance testField))")));
  }

  @Test
  public void testDotField() {
    String code = "(. test-instance testField)";
    assertEquals("hello", rt.eval(rt.readString(code)));
  }

  @Test
  public void testDotMethod() {
    String code = "(. test-instance (testMethod \"call\"))";
    assertEquals("method call", rt.eval(rt.readString(code)));
  }

  @Test
  public void testDotChained() {
    String code = "(. test-instance testField (toUpperCase))";
    assertEquals("HELLO", rt.eval(rt.readString(code)));
  }

  @Test
  public void testDotVectorIndex() {
    String code = "(. [\"a\" \"b\"] [1])";
    assertEquals("b", rt.eval(rt.readString(code)));
  }

  // --- Control Flow & Definitions ---

  @Test
  public void testDef() {
    String code = "(def my-var 123)";
    rt.eval(rt.readString(code));
    assertNumber(123, rt.eval(Symbol.create("my-var")));
  }

  @Test
  public void testQuote() {
    String code = "(quote (1 2))";
    Object res = rt.eval(rt.readString(code));
    assertTrue(res instanceof List);
    List l = (List) res;
    assertNumber(1, l.nth(0));
    assertNumber(2, l.nth(1));
  }

  @Test
  public void testSyntaxQuoteSimple() {
    String code = "(syntax-quote (1 2))";
    Object res = rt.eval(rt.readString(code));
    assertTrue(res instanceof List);
    assertNumber(2, ((List) res).count());
  }

  @Test
  public void testSyntaxQuoteUnquote() {
    rt.eval(rt.readString("(def a 10)"));
    String code = "(syntax-quote (1 (unquote a)))";
    Object res = rt.eval(rt.readString(code));
    assertTrue(res instanceof List);
    assertNumber(10, ((List) res).nth(1));
  }

  @Test
  public void testSyntaxQuoteUnquoteSplicing() {
    rt.eval(rt.readString("(def l [2 3])"));
    String code = "(syntax-quote (1 (unquote-splicing l) 4))";
    Object res = rt.eval(rt.readString(code));
    assertTrue(res instanceof List);
    assertNumber(4, ((List) res).count());
    assertNumber(2, ((List) res).nth(1));
    assertNumber(3, ((List) res).nth(2));
  }

  @Test
  public void testSyntaxQuoteMap() {
    rt.eval(rt.readString("(def v 1)"));
    String code = "(syntax-quote {:a (unquote v)})";
    Object res = rt.eval(rt.readString(code));
    assertTrue(res instanceof Map);
    Map m = (Map) res;
    assertNumber(1, m.lookup(Keyword.create("a")));
  }

  @Test
  public void testDo() {
    String code = "(do (def x 1) (def x 2) x)";
    assertNumber(2, rt.eval(rt.readString(code)));
  }

  @Test
  public void testFn() {
    String code = "((fn [x] (+ x 1)) 10)";
    assertNumber(11, rt.eval(rt.readString(code)));
  }

  @Test
  public void testLet() {
    String code = "(let [a 1 b 2] (+ a b))";
    assertNumber(3, rt.eval(rt.readString(code)));
  }

  @Test
  public void testIf() {
    assertNumber(1, rt.eval(rt.readString("(if true 1 2)")));
    assertNumber(2, rt.eval(rt.readString("(if false 1 2)")));
    assertNumber(1, rt.eval(rt.readString("(if true 1)")));
    assertNull(rt.eval(rt.readString("(if false 1)")));
  }

  @Test
  public void testCond() {
    String code = "(cond false 1 true 2 3)";
    assertNumber(2, rt.eval(rt.readString(code)));
    assertNull(rt.eval(rt.readString("(cond false 1 false 2)")));
  }

  @Test
  public void testThreadFirst() {
    String code = "(-> 10 (+ 2) (- 1))";
    assertNumber(11, rt.eval(rt.readString(code)));
  }

  @Test
  public void testThreadLast() {
    String code = "(->> 10 (+ 2) (- 1))";
    assertNumber(-11, rt.eval(rt.readString(code)));
  }

  @Test
  public void testTryCatch() {
    String code =
        "(try (throw (new RuntimeException \"error\")) (catch RuntimeException e (. e (getMessage))))";
    Object result = rt.eval(rt.readString(code));
    assertEquals("error", result);
  }

  @Test
  public void testTryFinally() {
    String code = "(let [a (atom \"init\")] (try \"ok\" (finally (reset! a \"done\"))) @a)";
    Object result = rt.eval(rt.readString(code));
    assertEquals("done", result);
  }

  @Test
  public void testThrow() {
    try {
      rt.eval(rt.readString("(throw (new Exception \"fail\"))"));
      fail("Should have thrown exception");
    } catch (Exception e) {
      // Just verify we got an exception
      assertTrue(true);
    }
  }

  @Test
  public void testLoopRecur() {
    String code = "(loop [i 0 acc 0] (if (< i 5) (recur (inc i) (+ acc i)) acc))";
    Object result = rt.eval(rt.readString(code));
    assertNumber(10, result);
  }
}
