package hara.truffle;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import org.junit.runner.Description;
import org.junit.runner.notification.Failure;
import org.junit.runner.notification.RunNotifier;
import org.junit.runners.ParentRunner;
import org.junit.runners.model.InitializationError;

/** JUnit 4 adapter that exposes each Hara test file as an isolated child test. */
public final class HaraJUnitRunner extends ParentRunner<Path> {
  private final List<Path> children;
  private final Path root;

  public HaraJUnitRunner(Class<?> testClass) throws InitializationError {
    super(testClass);
    HaraTestSource source = testClass.getAnnotation(HaraTestSource.class);
    if (source == null || source.value().length == 0) {
      throw new InitializationError("@HaraTestSource requires at least one path");
    }
    root = Path.of(".").toAbsolutePath().normalize();
    ArrayList<Path> discovered = new ArrayList<>();
    try {
      for (String value : source.value()) {
        discovered.addAll(
            HaraNativeTestRunner.discover(root, root.resolve(value).normalize()));
      }
    } catch (Exception error) {
      throw new InitializationError(error);
    }
    if (discovered.isEmpty()) {
      throw new InitializationError("@HaraTestSource did not discover any .hal files");
    }
    children = List.copyOf(discovered);
  }

  @Override
  protected List<Path> getChildren() {
    return children;
  }

  @Override
  protected Description describeChild(Path child) {
    String name = root.relativize(child).toString().replace('\\', '/');
    return Description.createTestDescription(getTestClass().getJavaClass(), name);
  }

  @Override
  protected void runChild(Path child, RunNotifier notifier) {
    Description description = describeChild(child);
    notifier.fireTestStarted(description);
    try {
      HaraNativeTestRunner.Result result = HaraNativeTestRunner.runFile(root, child);
      if (!result.passed()) {
        notifier.fireTestFailure(new Failure(description, new AssertionError(result.failureMessage())));
      }
    } catch (Throwable error) {
      notifier.fireTestFailure(new Failure(description, error));
    } finally {
      notifier.fireTestFinished(description);
    }
  }
}
