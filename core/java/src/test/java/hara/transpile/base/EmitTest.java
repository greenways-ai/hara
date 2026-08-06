package hara.transpile.base;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import org.junit.Test;
import static org.junit.Assert.*;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;

public class EmitTest {

  @Test
  public void testEmit() {
    // Setup grammar
    Map<Object, Object> reserved = new HashMap<>();
    Map<Object, Object> printProps = new HashMap<>();
    printProps.put(Keyword.create("emit"), "invoke");
    printProps.put(Keyword.create("raw"), "console.log");
    reserved.put(Symbol.create("print"), printProps);

    Map<Object, Object> grammarMap = new HashMap<>();
    grammarMap.put(Keyword.create("reserved"), reserved);

    Grammar grammar = new Grammar();
    grammar.reserved = reserved;

    Map<Object, Object> mopts = new HashMap<>();

    // Test emitting a simple function call: (print hello)
    Object form = Arrays.asList(Symbol.create("print"), Symbol.create("hello"));

    // Emit.emit takes Map grammar, not Grammar object directly in signature, but
    // implementation might expect Map structure
    // Looking at Emit.java: public static String emit(Object form, Map grammar,
    // String namespace, Map mopts)

    String result = Emit.emit(form, grammarMap, "test.ns", mopts);
    assertEquals("console.log(hello)", result);
  }
}
