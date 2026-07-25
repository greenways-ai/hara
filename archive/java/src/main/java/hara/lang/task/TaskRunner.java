package hara.lang.task;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.TimeUnit;

public final class TaskRunner {
  private TaskRunner() {}

  public static TaskResult run(Task task, Object... args) { return task.run(args); }

  public static TaskResult run(Task task, Duration timeout, Object... args) {
    long start = System.nanoTime();
    java.util.concurrent.CompletableFuture<?> pending = null;
    try {
      Object value = task.invoke(args);
      if (value instanceof CompletionStage<?>) {
        pending = ((CompletionStage<?>) value).toCompletableFuture();
        value = pending.get(timeout.toMillis(), TimeUnit.MILLISECONDS);
      }
      return TaskResult.returned(value, elapsed(start));
    } catch (java.util.concurrent.TimeoutException error) {
      if (pending != null) pending.cancel(true);
      return TaskResult.timeout(elapsed(start));
    } catch (java.util.concurrent.CancellationException error) {
      return TaskResult.cancelled(elapsed(start));
    } catch (Throwable error) {
      return TaskResult.failed(error, elapsed(start));
    }
  }

  public static List<TaskResult> runAll(List<Task> tasks) {
    List<TaskResult> results = new ArrayList<>();
    for (Task task : tasks) results.add(run(task));
    return results;
  }

  private static long elapsed(long start) { return (System.nanoTime() - start) / 1_000_000L; }
}
