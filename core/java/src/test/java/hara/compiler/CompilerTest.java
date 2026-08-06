package hara.compiler;

import hara.kernel.Foundation;

import hara.kernel.base.Parser;

import hara.lang.data.List;

import hara.lang.data.Symbol;

import hara.lang.data.Vector;

import junit.framework.TestCase;

import java.io.StringReader;

import java.util.function.Function;

public class CompilerTest extends TestCase {

  private void assertCompile(String expression, Long expected) {

    Foundation f = new Foundation();

    try {

      Object result = f.call("COMPILE", expression);

      assertNotNull(result);

      assertTrue(result instanceof Function);

      @SuppressWarnings("unchecked")
      Function<Long, Long> fn = (Function<Long, Long>) result;

      assertEquals(expected, fn.apply(1L));

    } catch (Exception e) {

      e.printStackTrace();

      fail("Should not have thrown an exception: " + e);
    }
  }

  public void testClassInitialization() {

    // Simply referencing the class should trigger its static initializer

    assertNotNull(Compiler.class);
  }

  public void testConstructor() {

    Compiler compiler = new Compiler();

    assertNotNull(compiler);
  }

  public void testCompileMethodWithSimpleList() throws Exception {

    // Equivalent to (fn [x] (+ x 1))

    String sExpression = "(fn [x] (+ x 1))";

    hara.lang.data.List expression =
        (hara.lang.data.List) Parser.LispReader.readString(sExpression, null);

    Compiler compiler = new Compiler();

    byte[] bytecode = compiler.compile(expression);

    assertNotNull(bytecode);

    assertTrue(bytecode.length > 0);
  }

  public void testCompile() {
    assertCompile("(fn [x] (+ x 1))", 2L);
  }

  public void testCompileWithDifferentLiteral() {
    assertCompile("(fn [x] (+ x 5))", 6L);
  }

  public void testCompileWithSubtraction() {
    assertCompile("(fn [x] (- x 5))", -4L);
  }

  public void testCompileWithMultiplication() {
    assertCompile("(fn [x] (* x 5))", 5L);
  }

  public void testCompileWithDivision() {
    assertCompile("(fn [x] (/ x 5))", 0L);
  }

  public void testCompileWithAlternateOperandOrder() {
    assertCompile("(fn [x] (+ 5 x))", 6L);
  }

  public void testCompileWithInvalidExpression() {
    Foundation f = new Foundation();
    try {
      f.call("COMPILE", "(fn [x] (+ y z))");
      fail("Should have thrown a CompilerException");
    } catch (Exception e) {
      assertTrue(
          "Expected CompilerException, but got " + e.getCause(),
          e.getCause() instanceof CompilerException);
      assertEquals("Unsupported expression format", e.getCause().getMessage());
    }
  }
}
