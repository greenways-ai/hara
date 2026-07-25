package hara.lang.task;

import java.util.Arrays;
import java.util.Collections;
import java.util.Map;
import java.util.Objects;

public final class Task {
  private final String type;
  private final String name;
  private final TaskFunction function;
  private final Object arglists;
  private final Map<String, Object> config;

  public Task(String type, String name, TaskFunction function, Object arglists, Map<String, Object> config) {
    this.type = type == null ? "default" : type;
    this.name = Objects.requireNonNull(name, "name");
    this.function = Objects.requireNonNull(function, "function");
    this.arglists = arglists;
    if (config == null || config.isEmpty()) {
      this.config = Collections.emptyMap();
    } else {
      this.config = Collections.unmodifiableMap(new java.util.LinkedHashMap<>(config));
    }
  }
  public String type() { return type; }
  public String name() { return name; }
  public Object arglists() { return arglists; }
  public Map<String, Object> config() { return config; }
  public TaskFunction function() { return function; }
  public Object invoke(Object... args) throws Exception { return function.apply(args == null ? new Object[0] : args); }
  public TaskResult run(Object... args) {
    long start = System.nanoTime();
    try { return TaskResult.returned(invoke(args), elapsed(start)); }
    catch (Throwable error) { return TaskResult.failed(error, elapsed(start)); }
  }
  public boolean task() { return true; }
  public Map<String, Object> info() {
    Map<String, Object> result = new java.util.LinkedHashMap<>();
    result.put("type", type);
    result.put("name", name);
    result.put("arglists", arglists);
    return result;
  }
  private static long elapsed(long start) { return (System.nanoTime() - start) / 1_000_000L; }
  @Override public String toString() { return "#task[" + type + " " + name + " " + Arrays.toString(new Object[] {arglists}) + "]"; }
}
