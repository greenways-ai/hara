package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertThrows;

import com.oracle.truffle.api.interop.TruffleObject;
import java.util.Map;
import org.junit.Test;

public class HaraProtocolTest {
  @Test
  public void resolvesHaraTypesJavaClassesAndPrimitiveCategories() throws Exception {
    HaraProtocol protocol = new HaraProtocol("Value", Map.of("value", 1));
    HaraType haraType = new HaraType("Counter", new String[] {"value"});
    HaraStruct haraValue = new HaraStruct(haraType, new Object[] {41});

    protocol.extend(haraType, "value", (receiver, arguments) -> 41);
    protocol.extend(Number.class, "value", (receiver, arguments) -> "java-number");
    protocol.extend(
        HaraDispatchKey.PrimitiveCategory.STRING, "value", (receiver, arguments) -> "hara-string");

    assertEquals(41, protocol.invoke("value", haraValue, new Object[0]));
    assertEquals("java-number", protocol.invoke("value", Integer.valueOf(7), new Object[0]));
    assertEquals("hara-string", protocol.invoke("value", new StringBuilder("hara"), new Object[0]));
  }

  @Test
  public void resolvesNilForeignAndDefaultImplementations() {
    HaraProtocol protocol = new HaraProtocol("Describe", Map.of("describe", 1));
    protocol.extendNil("describe", (receiver, arguments) -> "nil");
    protocol.extendForeign("describe", (receiver, arguments) -> "foreign");
    protocol.extendDefault("describe", (receiver, arguments) -> "default");

    assertEquals("nil", protocol.invoke("describe", null, new Object[0]));
    assertEquals("foreign", protocol.invoke("describe", new ForeignValue(), new Object[0]));
    assertEquals("default", protocol.invoke("describe", new Object(), new Object[0]));
  }

  @Test
  public void exposesFixedAndVariadicMethodMetadata() {
    HaraProtocol protocol = new HaraProtocol("Call", Map.of("call", -1));
    HaraProtocol.HaraProtocolMethod method = protocol.method("call");
    assertTrue(method.variadic());
    assertEquals(1, method.minimumArity());

    protocol.extendDefault("call", (receiver, arguments) -> arguments.length);
    assertEquals(2, protocol.invoke("call", "receiver", new Object[] {1, 2}));

    HaraProtocol fixed = new HaraProtocol("Fixed", Map.of("read", 2));
    fixed.extendDefault("read", (receiver, arguments) -> arguments[0]);
    HaraException error =
        assertThrows(HaraException.class, () -> fixed.invoke("read", "receiver", new Object[0]));
    assertTrue(error.getMessage().contains("expects 1 arguments"));
  }


  @Test
  public void missingImplementationsIdentifyTheDispatchPath() {
    HaraProtocol protocol = new HaraProtocol("Readable", Map.of("read", 1));
    HaraException primitive =
        assertThrows(HaraException.class, () -> protocol.invoke("read", 42L, new Object[0]));
    assertTrue(primitive.getMessage().contains("Readable/read"));
    assertTrue(primitive.getMessage().contains("category=number"));
    assertTrue(primitive.getMessage().contains("primitive:number"));

    HaraException foreign =
        assertThrows(
            HaraException.class,
            () -> protocol.invoke("read", new ForeignValue(), new Object[0]));
    assertTrue(foreign.getMessage().contains("category=foreign"));
    assertTrue(foreign.getMessage().contains("dispatch=foreign"));
  }

  private static final class ForeignValue implements TruffleObject {}
}
