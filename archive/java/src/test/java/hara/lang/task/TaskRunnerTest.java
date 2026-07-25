package hara.lang.task;

import static org.junit.Assert.assertEquals;

import java.time.Duration;
import java.util.concurrent.CompletableFuture;
import org.junit.Test;

public class TaskRunnerTest {
  @Test
  public void runsSynchronousTasks() {
    Task task = new Task("test", "addition", args -> ((long) args[0]) + ((long) args[1]), null, null);
    TaskResult result = TaskRunner.run(task, 2L, 3L);
    assertEquals(TaskResult.Status.RETURN, result.status());
    assertEquals(5L, result.value());
  }

  @Test
  public void waitsForAsyncTasks() {
    Task task = new Task("test", "async", args -> CompletableFuture.completedFuture("ok"), null, null);
    assertEquals(TaskResult.Status.RETURN, TaskRunner.run(task, Duration.ofSeconds(1)).status());
  }
}
