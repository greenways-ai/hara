package hara.kernel.base;

import org.junit.Test;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertFalse;

public class ReflectTest {

  @Test
  public void testInvokeInstanceMethod() {
    String s = "hello";
    Object result = Reflect.invokeInstanceMethod(s, "toUpperCase", new Object[] {});
    assertEquals("HELLO", result);
  }

  @Test
  public void testInvokeStaticMethod() {
    Object result = Reflect.invokeStaticMethod(String.class, "valueOf", new Object[] {123});
    assertEquals("123", result);
  }

  @Test
  public void testInvokeMatchingMethod() {
    Object result =
        Reflect.invokeStaticMethod(TestClass.class, "testMethod", new Object[] {"test"});
    assertEquals("called with test", result);
  }

  @Test
  public void testCanAccess() throws NoSuchMethodException {
    java.lang.reflect.Method m = String.class.getMethod("length");
    assertTrue(Reflect.canAccess(m, "abc"));
  }

  @Test
  public void testSubsumes() {
    assertTrue(Reflect.subsumes(new Class[] {String.class}, new Class[] {Object.class}));
    assertFalse(Reflect.subsumes(new Class[] {Object.class}, new Class[] {String.class}));
  }

  @Test
  public void testInterfaces() {
    java.util.Collection<Class> ifaces = Reflect.interfaces(java.util.ArrayList.class);
    assertTrue(ifaces.contains(java.util.List.class));
    assertTrue(ifaces.contains(java.util.Collection.class));
  }

  @Test
  public void testInvokeConstructor() {
    Object s = Reflect.invokeConstructor(String.class, new Object[] {"abc"});
    assertEquals("abc", s);
  }

  @Test
  public void testFields() {
    // Static field
    Reflect.setStaticField(TestClass.class, "staticField", "newValue");
    assertEquals("newValue", Reflect.getStaticField(TestClass.class, "staticField"));

    // Instance field
    TestClass tc = new TestClass();
    Reflect.setInstanceField(tc, "instanceField", "newValue");
    assertEquals("newValue", Reflect.getInstanceField(tc, "instanceField"));
  }

  @Test
  public void testInvokeNoArgInstanceMember() {
    String s = "abc";
    assertEquals(3, Reflect.invokeNoArgInstanceMember(s, "length"));
  }

  @Test
  public void testBoxArgs() {
    Object[] args = new Object[] {1, 2.0};
    Class[] params = new Class[] {int.class, double.class};
    Object[] boxed = Reflect.boxArgs(params, args);
    assertEquals(1, boxed[0]);
    assertEquals(2.0, boxed[1]);
  }

  @Test
  public void testIsCongruent() {
    Class[] params = new Class[] {int.class, String.class};
    Object[] args = new Object[] {1, "abc"};
    assertTrue(Reflect.isCongruent(params, args));

    Object[] badArgs = new Object[] {1, 2};
    assertFalse(Reflect.isCongruent(params, badArgs));
  }

  public static class TestClass {
    public static String staticField = "initial";
    public String instanceField = "initial";

    public static String testMethod(String arg) {
      return "called with " + arg;
    }
  }
}
