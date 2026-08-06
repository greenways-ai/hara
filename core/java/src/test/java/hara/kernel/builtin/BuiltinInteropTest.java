package hara.kernel.builtin;

import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;
import java.util.Iterator;

public class BuiltinInteropTest {

  public static class TestClass {
    public static String staticField = "static";
    public String instanceField = "instance";

    public static String staticMethod(String arg) {
      return "static:" + arg;
    }

    public String instanceMethod(String arg) {
      return "instance:" + arg;
    }

    public TestClass() {}

    public TestClass(String arg) {
      this.instanceField = arg;
    }
  }

  @Test
  public void testClassConstructors() {
    assertNotNull(BuiltinInterop.classConstructors(TestClass.class));
  }

  @Test
  public void testClassFields() {
    assertNotNull(BuiltinInterop.classFields(TestClass.class));
  }

  @Test
  public void testClassMethods() {
    assertNotNull(BuiltinInterop.classMethods(TestClass.class));
  }

  @Test
  public void testClassStaticMethods() {
    assertTrue(BuiltinInterop.classStaticMethods(TestClass.class).has("staticMethod"));
  }

  @Test
  public void testClassStaticFields() {
    Iterator<String> it = BuiltinInterop.classStaticFields(TestClass.class);
    boolean found = false;
    while (it.hasNext()) {
      if (it.next().equals("staticField")) found = true;
    }
    assertTrue(found);
  }

  @Test
  public void testInvokeNew() {
    TestClass obj = (TestClass) BuiltinInterop.invokeNew(TestClass.class, Arrays.asList("newVal"));
    assertEquals("newVal", obj.instanceField);
  }

  @Test
  public void testInvokeObj() {
    TestClass obj = new TestClass();
    assertEquals(
        "instance:arg", BuiltinInterop.invokeObj(obj, "instanceMethod", Arrays.asList("arg")));
  }

  @Test
  public void testInvokeGetSet() {
    TestClass obj = new TestClass();
    assertEquals("instance", BuiltinInterop.invokeGet(obj, "instanceField"));

    BuiltinInterop.invokeSet(obj, "instanceField", "newVal");
    assertEquals("newVal", obj.instanceField);
  }

  @Test
  public void testInvokeStatic() {
    assertEquals(
        "static:arg",
        BuiltinInterop.invokeStatic(TestClass.class, "staticMethod", Arrays.asList("arg")));
  }

  @Test
  public void testInvokeGetSetStatic() {
    assertEquals("static", BuiltinInterop.invokeGetStatic(TestClass.class, "staticField"));

    BuiltinInterop.invokeSetStatic(TestClass.class, "staticField", "newStatic");
    assertEquals("newStatic", TestClass.staticField);

    // Reset
    TestClass.staticField = "static";
  }
}
