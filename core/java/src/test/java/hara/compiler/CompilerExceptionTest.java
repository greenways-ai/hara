package hara.compiler;

import junit.framework.TestCase;

public class CompilerExceptionTest extends TestCase {

  public void testConstructorWithMessage() {
    String testMessage = "This is a test compiler exception message.";
    CompilerException exception = new CompilerException(testMessage);
    assertEquals(
        "The exception message should match the input message.",
        testMessage,
        exception.getMessage());
  }
}
