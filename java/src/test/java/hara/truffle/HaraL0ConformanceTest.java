package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.fail;

import hara.kernel.base.Parser;
import hara.lang.base.G;
import hara.lang.data.Keyword;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.file.Files;
import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.PolyglotException;
import org.graalvm.polyglot.Value;
import org.junit.Test;

/** Executes the versioned JVM L0 conformance corpus. */
public class HaraL0ConformanceTest {
  private static Keyword key(String name) {
    return Keyword.create(null, name);
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  @Test
  public void executesEveryJvmL0CorpusCase() throws Exception {
    String source = Files.readString(Path.of("specs/hara/corpora/l0-conformance.edn"));
    IMapType manifest = (IMapType) Parser.LispReader.readString(source, null);
    ILinearType<?> cases = (ILinearType<?>) manifest.lookup(key("cases"));
    assertTrue(cases.count() > 0);
    for (Object item : cases) {
      IMapType testCase = (IMapType) item;
      String id = ((Keyword) testCase.lookup(key("id"))).getName();
      String className = ((Keyword) testCase.lookup(key("class"))).getName();
      String form = (String) testCase.lookup(key("source"));
      IMapType expected = (IMapType) testCase.lookup(key("expect"));
      if ("reader".equals(className)) {
        assertReaderCase(id, form, expected);
      } else if (expected.lookup(key("error")) != null) {
        assertErrorCase(id, form, (String) testCase.lookup(key("setup")), expected);
      } else {
        assertValueCase(id, form, (String) testCase.lookup(key("setup")), expected);
      }
    }
  }

  private void assertReaderCase(String id, String form, IMapType expected) {
    Object value = Parser.LispReader.readString(form, null);
    java.util.Map.Entry valueEntry = (java.util.Map.Entry) expected.find(key("value"));
    if (valueEntry == null) {
      assertEquals(id, expected.lookup(key("form")), G.display(value));
    } else {
      Object expectedValue = valueEntry.getValue();
      if (expectedValue instanceof String) {
        assertEquals(id, expectedValue, value == null ? null : value.toString());
      } else {
        assertEquals(
            id,
            expectedValue == null ? null : expectedValue.toString(),
            value == null ? null : value.toString());
      }
    }
  }

  private void assertErrorCase(String id, String form, String setup, IMapType expected) {
    try (Context context = context()) {
      if (setup != null) context.eval(HaraLanguage.ID, setup);
      try {
        context.eval(HaraLanguage.ID, form);
        fail(id + " should fail");
      } catch (PolyglotException error) {
        assertTrue(id + " should report an error", error.isGuestException());
        Object message = expectedMessage(expected);
        if (message != null) {
          assertTrue(
              id + " should contain its specified error",
              error.getMessage().contains(message.toString()));
        }
      }
    }
  }

  private Object expectedMessage(IMapType expected) {
    return expected.lookup(key("message"));
  }

  private void assertValueCase(String id, String form, String setup, IMapType expected) {
    try (Context context = context()) {
      if (setup != null) context.eval(HaraLanguage.ID, setup);
      Value actual = context.eval(HaraLanguage.ID, form);
      assertExpectedValue(id, actual, expected.lookup(key("value")));
    }
  }

  private void assertExpectedValue(String id, Value actual, Object expected) {
    if (expected == null) {
      assertTrue(id + " should return nil", actual.isNull());
    } else if (expected instanceof Boolean) {
      assertEquals(id, expected, actual.asBoolean());
    } else if (expected instanceof String) {
      assertEquals(id, expected, actual.asString());
    } else if (expected instanceof BigInteger) {
      assertEquals(id, expected, actual.as(BigInteger.class));
    } else if (expected instanceof BigDecimal) {
      assertEquals(id, expected, actual.as(BigDecimal.class));
    } else if (expected instanceof Number) {
      assertEquals(id, ((Number) expected).longValue(), actual.asLong());
    } else {
      fail(id + " has unsupported expected value: " + expected);
    }
  }

  private static Context context() {
    return Context.newBuilder(HaraLanguage.ID).build();
  }
}
