package hara.truffle;

import java.util.ArrayList;
import java.util.List;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.Value;
import org.junit.runner.Description;
import org.junit.runner.RunWith;
import org.junit.runner.Runner;
import org.junit.runner.notification.Failure;
import org.junit.runner.notification.RunNotifier;

/** JUnit 4 adapter for Hara fact resources. */
public final class HaraTestRunner extends Runner {
  private final Class<?> testClass;
  private final List<String> resources;
  private volatile List<TestEntry> entries;

  public HaraTestRunner(Class<?> testClass) {
    this.testClass = testClass;
    HaraTestResources annotation = testClass.getAnnotation(HaraTestResources.class);
    if (annotation == null) throw new IllegalArgumentException("@HaraTestResources is required");
    resources = List.of(annotation.value());
  }

  @Override
  public Description getDescription() {
    Description suite = Description.createSuiteDescription(testClass);
    for (TestEntry entry : discover()) suite.addChild(entry.description);
    return suite;
  }

  @Override
  public void run(RunNotifier notifier) {
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      for (String resource : resources) {
        context.eval(HaraLanguage.ID, "(require \"" + escape(resource) + "\")");
      }
      Value results = context.eval(HaraLanguage.ID, "(code.test/run)");
      List<TestEntry> discovered = discover();
      for (int i = 0; i < discovered.size(); i++) {
        TestEntry entry = discovered.get(i);
        notifier.fireTestStarted(entry.description);
        if (i >= results.getArraySize()) {
          notifier.fireTestFailure(
              new Failure(entry.description, new AssertionError("Hara test produced no result")));
        } else {
          Value result = results.getArrayElement(i);
          String status = result.getHashValue("status").asString();
          if (!"PASS".equals(status)) {
            notifier.fireTestFailure(
                new Failure(
                    entry.description, new AssertionError("Hara fact failed: " + entry.name)));
          }
        }
        notifier.fireTestFinished(entry.description);
      }
    } catch (Throwable error) {
      Description description = Description.createTestDescription(testClass, "Hara suite");
      notifier.fireTestFailure(new Failure(description, error));
    }
  }

  private List<TestEntry> discover() {
    List<TestEntry> current = entries;
    if (current != null) return current;
    List<TestEntry> result = new ArrayList<>();
    try (Context context = Context.newBuilder(HaraLanguage.ID).build()) {
      context.eval(HaraLanguage.ID, "(require 'code.test)");
      for (String resource : resources) {
        context.eval(HaraLanguage.ID, "(require \"" + escape(resource) + "\")");
      }
      Value tests = context.eval(HaraLanguage.ID, "(code.test/tests)");
      for (int i = 0; i < tests.getArraySize(); i++) {
        Value test = tests.getArrayElement(i);
        String namespace = test.getHashValue("namespace").asString();
        String name = test.getHashValue("name").asString();
        result.add(
            new TestEntry(
                namespace + "/" + name,
                Description.createTestDescription(testClass, namespace + "/" + name)));
      }
    } catch (Throwable error) {
      for (String resource : resources)
        result.add(new TestEntry(resource, Description.createTestDescription(testClass, resource)));
    }
    entries = List.copyOf(result);
    return entries;
  }

  private static String escape(String value) {
    return value.replace("\\", "\\\\").replace("\"", "\\\"");
  }

  private static final class TestEntry {
    private final String name;
    private final Description description;

    private TestEntry(String name, Description description) {
      this.name = name;
      this.description = description;
    }
  }
}
