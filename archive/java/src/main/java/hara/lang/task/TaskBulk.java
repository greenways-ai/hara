package hara.lang.task;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.stream.Collectors;

/** Structured bulk task execution without coupling the runtime to a console printer. */
public final class TaskBulk {
  private TaskBulk() {}

  public static List<Map<String, Object>> items(Task task, List<?> inputs) {
    List<Map<String, Object>> results = new ArrayList<>();
    for (Object input : inputs) results.add(processItemInternal(task, input));
    return results;
  }

  public static Map<String, Object> processItem(Task task, Object input) {
    return processItemInternal(task, input);
  }

  public static List<Map<String, Object>> itemsParallel(Task task, List<?> inputs) {
    List<CompletableFuture<Map<String, Object>>> futures = inputs.stream()
        .map(input -> CompletableFuture.supplyAsync(() -> processItemInternal(task, input)))
        .toList();
    return futures.stream().map(CompletableFuture::join).toList();
  }

  private static Map<String, Object> processItemInternal(Task task, Object input) {
    long start = System.nanoTime();
    Map<String, Object> result = new LinkedHashMap<>();
    result.put("input", input);
    try {
      result.put("status", "RETURN");
      result.put("data", TaskProcess.invoke(task, input));
    } catch (Throwable error) {
      result.put("status", "ERROR");
      result.put("data", "errored");
      result.put("error", error.getMessage());
    }
    result.put("time", (System.nanoTime() - start) / 1_000_000L);
    return result;
  }

  public static List<Map<String, Object>> warnings(List<Map<String, Object>> items) {
    return items.stream().filter(item -> status(item).equals("WARN")).collect(Collectors.toList());
  }

  public static List<Map<String, Object>> errors(List<Map<String, Object>> items) {
    return items.stream().filter(item -> status(item).equals("ERROR") || status(item).equals("CRITICAL")).collect(Collectors.toList());
  }

  public static List<Map<String, Object>> results(List<Map<String, Object>> items) {
    return items.stream().filter(item -> status(item).equals("RETURN")).collect(Collectors.toList());
  }

  private static String status(Map<String, Object> item) {
    Object value = item.get("status");
    return String.valueOf(value).replace(":", "").toUpperCase();
  }

  public static List<Map<String, Object>> prepareColumns(
      List<Map<String, Object>> columns, List<?> outputs) {
    List<Map<String, Object>> prepared = new ArrayList<>();
    for (Map<String, Object> column : columns) {
      Map<String, Object> result = new LinkedHashMap<>(column);
      Object key = column.get("key");
      result.putIfAbsent("id", key);
      if (!result.containsKey("length")) {
        int length = 2;
        for (Object output : outputs) {
          if (output instanceof Map<?, ?> map) {
            length = Math.max(length, String.valueOf(map.get(key)).length() + 2);
          }
        }
        result.put("length", length);
      }
      prepared.add(result);
    }
    return prepared;
  }

  public static Map<String, Object> packageResults(
      Map<String, Object> bundle, String returnMode, String packageMode) {
    if ("all".equals(returnMode)) {
      Map<String, Object> result = new LinkedHashMap<>();
      for (String key : List.of("items", "warnings", "errors", "results", "summary")) {
        if (bundle.containsKey(key)) result.put(key, packageValue(bundle.get(key), key, packageMode));
      }
      return result;
    }
    String key = returnMode == null ? "results" : returnMode;
    Map<String, Object> result = new LinkedHashMap<>();
    result.put(key, packageValue(bundle.get(key), key, packageMode));
    return result;
  }

  private static Object packageValue(Object value, String key, String packageMode) {
    if (!(value instanceof Iterable<?> iterable)
        || "warnings".equals(key) || "errors".equals(key) || "summary".equals(key)) return value;
    List<Object> vector = new ArrayList<>();
    Map<Object, Object> map = new LinkedHashMap<>();
    for (Object item : iterable) {
      Object itemKey = null;
      Object data = item;
      if (item instanceof List<?> pair && pair.size() == 2) {
        itemKey = pair.get(0);
        data = pair.get(1);
        if (data instanceof Map<?, ?> result) data = result.get("data");
      } else if (item instanceof Map<?, ?> result) {
        itemKey = result.get("key");
        if (itemKey == null) itemKey = result.get("input");
        if (result.containsKey("data")) data = result.get("data");
      }
      if ("vector".equals(packageMode)) vector.add(List.of(itemKey, data));
      else map.put(itemKey, data);
    }
    return "vector".equals(packageMode) ? vector : map;
  }

  public static Map<String, Object> summary(List<Map<String, Object>> items) {
    long errors = items.stream().filter(item -> status(item).equals("ERROR") || status(item).equals("CRITICAL")).count();
    long warnings = items.stream().filter(item -> status(item).equals("WARN")).count();
    long results = items.stream().filter(item -> status(item).equals("RETURN")).count();
    Map<String, Object> result = new LinkedHashMap<>();
    result.put("items", items.size());
    result.put("results", results);
    result.put("errors", errors);
    result.put("warnings", warnings);
    result.put("cumulative", items.stream().mapToLong(item -> ((Number) item.get("time")).longValue()).sum());
    return result;
  }
}
