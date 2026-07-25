package hara.lang.task;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import java.util.List;
import java.util.Map;
import org.junit.Test;

public class TaskProcessTest {
  @Test
  public void selectsInputsWithPrefixAndListFunction() throws Exception {
    Task task = new Task(
        "namespace",
        "names",
        args -> args[0],
        null,
        Map.of("item", Map.of("list", (TaskFunction) args -> List.of("code.test", "std.lib.task"))));
    assertEquals(List.of("code.test"), TaskProcess.selectInputs(task, Map.of(), Map.of(), "code"));
  }

  @Test
  public void appliesProcessPreAndPostHooks() throws Exception {
    Task task = new Task(
        "default",
        "double",
        args -> ((Long) args[0]) * 2,
        null,
        Map.of("item", Map.of(
            "pre", (TaskFunction) args -> ((Long) args[0]) + 1,
            "post", (TaskFunction) args -> ((Long) args[0]) + 3)));
    assertEquals(7L, TaskProcess.invoke(task, 1L));
  }

  @Test
  public void bulkExecutionReturnsResultsAndSummary() {
    Task task = new Task("default", "double", args -> ((Long) args[0]) * 2, null, null);
    List<Map<String, Object>> items = TaskBulk.itemsParallel(task, List.of(1L, 2L, 3L));
    assertEquals(3, items.size());
    assertEquals(4L, items.get(1).get("data"));
    assertEquals(3L, TaskBulk.summary(items).get("results"));
    assertTrue(TaskBulk.errors(items).isEmpty());
  }

  @Test
  public void parsesNamespaceArgumentsFromTokenArray() {
    Map<String, Object> parsed = TaskProcess.processNamespaceArgs(
        ":only", "std.lib.task", ":bulk", ":limit", "3", ":dry-run");
    assertEquals("std.lib.task", ((hara.lang.data.Symbol) parsed.get("ns")).display());
    assertEquals(Boolean.TRUE, parsed.get("bulk"));
    assertEquals(3L, parsed.get("limit"));
    assertEquals(Boolean.TRUE, parsed.get("dry-run"));
  }

  @Test
  public void constructsTaskInputsUsingConfiguredStages() throws Exception {
    Task task = new Task(
        "namespace", "inputs", args -> args[0], null,
        Map.of("construct", Map.of(
            "input", (TaskFunction) args -> "input",
            "env", (TaskFunction) args -> Map.of("environment", true),
            "lookup", (TaskFunction) args -> Map.of("lookup", true))));
    Map<String, Object> inputs = TaskProcess.taskInputs(task);
    assertEquals("input", inputs.get("input"));
    assertEquals(Map.of("environment", true), inputs.get("env"));
    assertEquals(Map.of("lookup", true), inputs.get("lookup"));
  }

  @Test
  public void adaptsMainFunctionToTheConfiguredInputCount() throws Exception {
    TaskFunction original = args -> List.of(args);
    Object[] adapted = TaskProcess.mainFunction(original, 2);
    TaskFunction function = (TaskFunction) adapted[0];
    assertEquals(List.of(1L, Map.of("p", true), "extra"),
        function.apply(new Object[] {1L, Map.of("p", true), Map.of(), Map.of(), "extra"}));
    assertTrue((Boolean) adapted[1]);
  }
}
