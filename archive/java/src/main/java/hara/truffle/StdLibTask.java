package hara.truffle;

import hara.lang.data.Keyword;
import hara.lang.data.List;
import hara.lang.data.Symbol;
import hara.lang.data.types.ILinearType;
import hara.lang.data.types.IMapType;
import hara.lang.task.Task;
import hara.lang.task.TaskBulk;
import hara.lang.task.TaskFunction;
import hara.lang.task.TaskProcess;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.Map;

/** Static Java implementation exported exclusively as std.lib.task/*. */
public final class StdLibTask {
  private StdLibTask() {}

  @HaraExport(name = "task", doc = "Creates a task.", arglists = {"[config]", "[type name main]"})
  public static Object task(HaraContext context, Object[] values) {
    return createTask(context, values);
  }

  @HaraExport(name = "map->Task", doc = "Creates a task from a configuration map.", arglists = {"[config]"})
  public static Object mapTask(HaraContext context, Object[] values) {
    requireArity("map->Task", values, 1);
    if (!(values[0] instanceof IMapType<?, ?>)) throw new HaraException("map->Task expects a map");
    return createTask(context, values);
  }

  @HaraExport(name = "task?", doc = "Returns true when value is a task.", arglists = {"[value]"})
  public static Object taskPredicate(HaraContext context, Object[] values) {
    requireArity("task?", values, 1);
    return values[0] instanceof Task;
  }

  @HaraExport(name = "task-status", doc = "Returns the task type.", arglists = {"[task]"})
  public static Object taskStatus(HaraContext context, Object[] values) {
    Task task = taskArgument("task-status", values);
    return Keyword.create(task.type());
  }

  @HaraExport(name = "task-info", doc = "Returns identifying task information.", arglists = {"[task]"})
  public static Object taskInfo(HaraContext context, Object[] values) {
    Task task = taskArgument("task-info", values);
    return hara.lang.data.Map.Standard.from(
        null, Keyword.create("fn"), Symbol.create(task.name()));
  }

  @HaraExport(name = "task-defaults", doc = "Returns the default task configuration.", arglists = {"[type]"})
  public static Object taskDefaults(HaraContext context, Object[] values) {
    requireArity("task-defaults", values, 1);
    return hara.lang.data.Map.Standard.from(
        null,
        Keyword.create("main"),
        hara.lang.data.Map.Standard.from(
            null,
            Keyword.create("arglists"),
            hara.lang.data.List.Standard.from(
                null,
                hara.lang.data.Vector.Standard.EMPTY,
                hara.lang.data.Vector.Standard.from(null, Symbol.create("entry")))));
  }

  @HaraExport(name = "single-function-print", arglists = {"[params]"})
  public static Object singleFunctionPrint(HaraContext context, Object[] values) {
    requireArity("single-function-print", values, 1);
    return singleFunctionPrint(values[0]);
  }

  @HaraExport(name = "process-ns-args", arglists = {"[args]", "[arg & more]"})
  public static Object processNamespaceArguments(HaraContext context, Object[] values) {
    ArrayList<String> arguments = new ArrayList<>();
    if (values.length == 1 && values[0] instanceof Iterable<?> iterable) {
      for (Object value : iterable) arguments.add(String.valueOf(value));
    } else {
      for (Object value : values) arguments.add(String.valueOf(value));
    }
    return TaskProcess.processNamespaceArgs(arguments.toArray(String[]::new));
  }

  @HaraExport(name = "invoke-intern-task", arglists = {"[name config]", "[type name config body]"})
  public static Object invokeInternTask(HaraContext context, Object[] values) {
    if (values.length < 2 || !(values[0] instanceof Symbol)) {
      throw new HaraException("invoke-intern-task expects a name and configuration");
    }
    Symbol name = (Symbol) values[0];
    Object config = values[1];
    Object type = config instanceof IMapType<?, ?> map
        ? HaraContext.lookupValue(map, Keyword.create("template"))
        : Keyword.create("default");
    Symbol definedName = withTaskMetadata(name, config);
    return List.Standard.from(
        null,
        Symbol.create("def"),
        definedName,
        List.Standard.from(
            null, Symbol.create("std.lib.task", "task"), type, name.getName(), config));
  }

  @HaraExport(name = "invoke", doc = "Invokes a task.", arglists = {"[task & args]"})
  public static Object invoke(HaraContext context, Object[] values) {
    if (values.length < 1 || !(values[0] instanceof Task task)) {
      throw new HaraException("std.lib.task/invoke expects a task");
    }
    try {
      return TaskProcess.invoke(task, java.util.Arrays.copyOfRange(values, 1, values.length));
    } catch (Exception error) {
      throw new HaraException("Task invocation failed: " + error.getMessage());
    }
  }

  @HaraExport(
      name = "deftask",
      doc = "Defines a top-level task.",
      arglists = {"[name config & body]"},
      kind = HaraExport.Kind.MACRO,
      intrinsic = true)
  public static Object deftask(HaraContext context, List invocation) {
    return expandDeftask(invocation);
  }

  @HaraExport(name = "select-filter", arglists = {"[selector id]"})
  public static Object selectFilter(HaraContext context, Object[] values) {
    requireArity("select-filter", values, 2);
    try {
      return TaskProcess.selectFilter(values[0], values[1]);
    } catch (RuntimeException error) {
      throw new HaraException(error.getMessage());
    }
  }

  @HaraExport(name = "select-inputs", arglists = {"[task lookup env selector]"})
  public static Object selectInputs(HaraContext context, Object[] values) {
    if (values.length != 4 || !(values[0] instanceof Task task)) {
      throw new HaraException("select-inputs expects task, lookup, environment, and selector");
    }
    try {
      return TaskProcess.selectInputs(task, values[1], values[2], values[3]);
    } catch (Exception error) {
      throw new HaraException("Unable to select task inputs: " + error.getMessage());
    }
  }

  @HaraExport(name = "task-inputs", arglists = {"[task & args]"})
  public static Object taskInputs(HaraContext context, Object[] values) {
    if (values.length < 1 || !(values[0] instanceof Task task)) {
      throw new HaraException("task-inputs expects a task");
    }
    try {
      return TaskProcess.taskInputs(
          task, java.util.Arrays.copyOfRange(values, 1, values.length));
    } catch (Exception error) {
      throw new HaraException("Unable to construct task inputs: " + error.getMessage());
    }
  }

  @HaraExport(name = "main-function", arglists = {"[function count]"})
  public static Object mainFunction(HaraContext context, Object[] values) {
    if (values.length != 2
        || (!(values[0] instanceof TaskFunction) && !(values[0] instanceof HaraFunction))
        || !(values[1] instanceof Number)) {
      throw new HaraException("main-function expects a function and count");
    }
    TaskFunction function =
        values[0] instanceof TaskFunction
            ? (TaskFunction) values[0]
            : toTaskFunction(values[0]);
    return TaskProcess.mainFunction(function, ((Number) values[1]).intValue());
  }

  @HaraExport(name = "wrap-execute", arglists = {"[function task]"})
  public static Object wrapExecute(HaraContext context, Object[] values) {
    if (values.length != 2 || !(values[1] instanceof Task task)) {
      throw new HaraException("wrap-execute expects a function and task");
    }
    return wrapExecuteFunction(context, values[0], task);
  }

  @HaraExport(name = "wrap-input", arglists = {"[function task]"})
  public static Object wrapInput(HaraContext context, Object[] values) {
    if (values.length != 2 || !(values[1] instanceof Task task)) {
      throw new HaraException("wrap-input expects a function and task");
    }
    Object execute = wrapExecuteFunction(context, values[0], task);
    return context.libraryFunction(
        "std.lib.task/wrap-input",
        inputValues -> {
          if (inputValues.length < 4) {
            throw new HaraException("wrapped task expects input, params, lookup, and env");
          }
          Object input = inputValues[0];
          if (input instanceof Keyword keyword && "list".equals(keyword.getName())) {
            Object listFunction = taskConfig(task, "item", "list");
            return context.invokeCallable(
                listFunction, new Object[] {inputValues[2], inputValues[3]});
          }
          if (input instanceof Keyword
              || input instanceof hara.lang.data.Vector<?>
              || input instanceof hara.lang.data.List<?>
              || input instanceof hara.lang.data.Set<?>) {
            try {
              java.util.List<?> selected =
                  TaskProcess.selectInputs(task, inputValues[2], inputValues[3], input);
              ArrayList<Object> results = new ArrayList<>();
              for (Object selectedInput : selected) {
                Object[] forwarded = inputValues.clone();
                forwarded[0] = selectedInput;
                results.add(context.invokeCallable(execute, forwarded));
              }
              return results;
            } catch (Exception error) {
              throw new HaraException("Unable to select task inputs: " + error.getMessage());
            }
          }
          return context.invokeCallable(execute, inputValues);
        });
  }

  @HaraExport(name = "bulk-display", arglists = {"[index-length input-length]"})
  public static Object bulkDisplay(HaraContext context, Object[] values) {
    requireArity("bulk-display", values, 2);
    Map<String, Object> display = new LinkedHashMap<>();
    display.put("padding", 1);
    display.put("spacing", 1);
    ArrayList<Object> columns = new ArrayList<>();
    columns.add(Map.of("id", "index", "length", values[0], "align", "right"));
    columns.add(Map.of("id", "input", "length", values[1]));
    columns.add(Map.of("id", "data", "length", 60));
    columns.add(Map.of("id", "time", "length", 10));
    display.put("columns", columns);
    return display;
  }

  @HaraExport(name = "bulk-process-item", arglists = {"[function context params lookup env & args]"})
  public static Object bulkProcessItem(HaraContext context, Object[] values) {
    if (values.length < 2 || !(values[0] instanceof HaraFunction)) {
      throw new HaraException("bulk-process-item expects a function and context");
    }
    Object itemContext = values[1];
    Object input =
        itemContext instanceof IMapType<?, ?> map
            ? HaraContext.lookupValue(map, Keyword.create("input"))
            : null;
    Object params = values.length > 2 ? values[2] : hara.lang.data.Map.Standard.EMPTY;
    Object lookup = values.length > 3 ? values[3] : hara.lang.data.Map.Standard.EMPTY;
    Object env = values.length > 4 ? values[4] : hara.lang.data.Map.Standard.EMPTY;
    Object[] functionArgs = new Object[4 + Math.max(0, values.length - 5)];
    functionArgs[0] = input;
    functionArgs[1] = params;
    functionArgs[2] = lookup;
    functionArgs[3] = env;
    if (values.length > 5) System.arraycopy(values, 5, functionArgs, 4, values.length - 5);
    long start = System.nanoTime();
    try {
      Object returned = HaraBox.unwrap(context.invokeCallable(values[0], functionArgs));
      if (returned instanceof ILinearType<?> pair && pair.count() == 2) {
        Object result = pair.nth(1);
        if (result instanceof IMapType<?, ?> map) {
          @SuppressWarnings("rawtypes")
          IMapType rawMap = (IMapType) map;
          Object withTime =
              rawMap.assoc(
                  Keyword.create("time"), (System.nanoTime() - start) / 1_000_000L);
          return List.Standard.from(null, pair.nth(0), withTime);
        }
        if (result instanceof Map<?, ?> javaMap) {
          Map<Object, Object> withTime = new LinkedHashMap<>(javaMap);
          withTime.put("time", (System.nanoTime() - start) / 1_000_000L);
          return List.Standard.from(null, pair.nth(0), withTime);
        }
        return returned;
      }
      return bulkItem(input, "RETURN", returned, null, start);
    } catch (Throwable error) {
      return bulkItem(input, "ERROR", "errored", error, start);
    }
  }

  @HaraExport(name = "bulk-items", arglists = {"[task inputs]", "[function inputs display params lookup env & args]"})
  public static Object bulkItems(HaraContext context, Object[] values) {
    return bulkItems(context, values, false);
  }

  @HaraExport(name = "bulk-items-single", arglists = {"[task inputs]", "[function inputs display params lookup env & args]"})
  public static Object bulkItemsSingle(HaraContext context, Object[] values) {
    return bulkItems(context, values, false);
  }

  @HaraExport(name = "bulk-items-parallel", arglists = {"[task inputs]", "[function inputs display params lookup env & args]"})
  public static Object bulkItemsParallel(HaraContext context, Object[] values) {
    return bulkItems(context, values, true);
  }

  @HaraExport(name = "bulk-warnings", arglists = {"[params items]"})
  public static Object bulkWarnings(HaraContext context, Object[] values) {
    requireNonEmpty("bulk-warnings", values);
    return filterBulkResults(values[values.length - 1], "WARN");
  }

  @HaraExport(name = "bulk-errors", arglists = {"[params items]"})
  public static Object bulkErrors(HaraContext context, Object[] values) {
    requireNonEmpty("bulk-errors", values);
    return filterBulkResults(values[values.length - 1], "ERROR");
  }

  @HaraExport(name = "bulk-results", arglists = {"[params items]"})
  public static Object bulkResults(HaraContext context, Object[] values) {
    requireNonEmpty("bulk-results", values);
    if (values.length >= 3 && !(values[values.length - 1] instanceof Map<?, ?>)) {
      Object value = values[values.length - 1];
      if (value instanceof Iterable<?> iterable) {
        ArrayList<Object> results = new ArrayList<>();
        for (Object item : iterable) {
          if (!(item instanceof ILinearType<?> pair) || pair.count() != 2) continue;
          Object result = pair.nth(1);
          Object status;
          Object data;
          if (result instanceof IMapType<?, ?> map) {
            status = HaraContext.lookupValue(map, Keyword.create("status"));
            data = HaraContext.lookupValue(map, Keyword.create("data"));
          } else if (result instanceof Map<?, ?> map) {
            status = map.get("status");
            data = map.get("data");
          } else continue;
          String name = status instanceof Keyword keyword ? keyword.getName() : String.valueOf(status);
          if ("return".equalsIgnoreCase(name)) {
            Map<String, Object> output = new LinkedHashMap<>();
            output.put("key", pair.nth(0));
            output.put("data", data);
            results.add(output);
          }
        }
        return results;
      }
    }
    return filterBulkResults(values[values.length - 1], "RETURN");
  }

  @HaraExport(name = "prepare-columns", arglists = {"[columns outputs]"})
  public static Object prepareColumns(HaraContext context, Object[] values) {
    requireArity("prepare-columns", values, 2);
    return TaskBulk.prepareColumns(asBulkMaps(values[0]), asObjects(values[1]));
  }

  @HaraExport(name = "bulk-summary", arglists = {"[items]", "[task params items results warnings errors elapsed]"})
  public static Object bulkSummary(HaraContext context, Object[] values) {
    requireNonEmpty("bulk-summary", values);
    if (values.length >= 7 && values[0] instanceof Task) {
      java.util.List<Object> items = asObjects(values[2]);
      java.util.List<Object> results = asObjects(values[3]);
      java.util.List<Object> warnings = asObjects(values[4]);
      java.util.List<Object> errors = asObjects(values[5]);
      long cumulative = 0L;
      for (Object item : items) {
        if (item instanceof ILinearType<?> pair
            && pair.count() == 2
            && pair.nth(1) instanceof IMapType<?, ?> map) {
          Object time = HaraContext.lookupValue(map, Keyword.create("time"));
          if (time instanceof Number number) cumulative += number.longValue();
        }
      }
      Map<String, Object> summary = new LinkedHashMap<>();
      summary.put("items", items.size());
      summary.put("results", results.size());
      summary.put("warnings", warnings.size());
      summary.put("errors", errors.size());
      summary.put("cumulative", cumulative);
      summary.put("elapsed", values[6]);
      return summary;
    }
    return TaskBulk.summary(asBulkMaps(values[values.length - 1]));
  }

  @HaraExport(name = "bulk-package", arglists = {"[bundle return]", "[task bundle return package]"})
  public static Object bulkPackage(HaraContext context, Object[] values) {
    int bundleIndex = values.length > 0 && values[0] instanceof Task ? 1 : 0;
    if (values.length < bundleIndex + 2
        || (!(values[bundleIndex] instanceof Map<?, ?>)
            && !(values[bundleIndex] instanceof IMapType<?, ?>))) {
      throw new HaraException("bulk-package expects a bundle");
    }
    String returnMode =
        values[bundleIndex + 1] instanceof Keyword keyword
            ? keyword.getName()
            : String.valueOf(values[bundleIndex + 1]);
    String packageMode =
        values.length > bundleIndex + 2 && values[bundleIndex + 2] instanceof Keyword keyword
            ? keyword.getName()
            : "map";
    Map<String, Object> converted = new LinkedHashMap<>();
    if (values[bundleIndex] instanceof Map<?, ?> bundle) {
      bundle.forEach((key, value) -> converted.put(keyName(key), value));
    } else {
      for (Object object : (IMapType<?, ?>) values[bundleIndex]) {
        Map.Entry<?, ?> entry = (Map.Entry<?, ?>) object;
        converted.put(keyName(entry.getKey()), entry.getValue());
      }
    }
    return packageBulkValue(converted, returnMode, packageMode);
  }

  @HaraExport(name = "bulk", arglists = {"[task function inputs params lookup env & args]", "[task inputs & options]"})
  public static Object bulk(HaraContext context, Object[] values) {
    if (values.length >= 6
        && values[0] instanceof Task task
        && (values[1] instanceof HaraFunction || values[1] instanceof TaskFunction)) {
      Object[] bulkArgs = new Object[6 + Math.max(0, values.length - 6)];
      bulkArgs[0] = values[1];
      bulkArgs[1] = values[2];
      bulkArgs[2] = hara.lang.data.Map.Standard.EMPTY;
      bulkArgs[3] = values[3];
      bulkArgs[4] = values[4];
      bulkArgs[5] = values[5];
      if (values.length > 6) System.arraycopy(values, 6, bulkArgs, 6, values.length - 6);
      long start = System.nanoTime();
      Object items =
          bulkItems(context, bulkArgs, optionTrue(values[3], "parallel"));
      Object warnings = filterBulkResults(items, "WARN");
      Object errors = filterBulkResults(items, "ERROR");
      Object results = bulkResults(context, new Object[] {task, values[3], items});
      Object summary =
          bulkSummary(
              context,
              new Object[] {
                task,
                values[3],
                items,
                results,
                warnings,
                errors,
                (System.nanoTime() - start) / 1_000_000L
              });
      Map<String, Object> bundle = new LinkedHashMap<>();
      bundle.put("items", items);
      bundle.put("warnings", warnings);
      bundle.put("errors", errors);
      bundle.put("results", results);
      bundle.put("summary", summary);
      Object returnValue =
          values[3] instanceof IMapType<?, ?> map
              ? HaraContext.lookupValue(map, Keyword.create("return"))
              : null;
      String returnMode =
          returnValue instanceof Keyword keyword ? keyword.getName() : "all";
      return packageBulkValue(bundle, returnMode, "map");
    }
    if (values.length < 2 || !(values[0] instanceof Task task)) {
      throw new HaraException("bulk expects a task and inputs");
    }
    java.util.List<Map<String, Object>> items =
        asBulkMaps(
            bulkItems(context, new Object[] {task, values[1]}, false));
    Map<String, Object> bundle = new LinkedHashMap<>();
    bundle.put("items", items);
    bundle.put("warnings", TaskBulk.warnings(items));
    bundle.put("errors", TaskBulk.errors(items));
    bundle.put("results", TaskBulk.results(items));
    bundle.put("summary", TaskBulk.summary(items));
    return bundle;
  }

  @SuppressWarnings("rawtypes")
  private static boolean optionTrue(Object options, String name) {
    if (options instanceof IMapType<?, ?> map) {
      return Boolean.TRUE.equals(HaraContext.lookupValue(map, Keyword.create(name)));
    }
    return options instanceof Map<?, ?> map && Boolean.TRUE.equals(map.get(name));
  }

  private static Object filterBulkResults(Object value, String status) {
    if (!(value instanceof Iterable<?> iterable)) {
      throw new HaraException("bulk result filter expects items");
    }
    ArrayList<Object> results = new ArrayList<>();
    for (Object item : iterable) {
      Object result = item;
      if (item instanceof ILinearType<?> pair && pair.count() == 2) result = pair.nth(1);
      Object rawStatus =
          result instanceof IMapType<?, ?> map
              ? HaraContext.lookupValue(map, Keyword.create("status"))
              : result instanceof Map<?, ?> map ? map.get("status") : null;
      String actual =
          rawStatus instanceof Keyword keyword
              ? keyword.getName().toUpperCase()
              : String.valueOf(rawStatus).replace(":", "").toUpperCase();
      if (status.equals(actual)
          || ("ERROR".equals(status) && "CRITICAL".equals(actual))) {
        results.add(item);
      }
    }
    return results;
  }

  private static Object packageBulkValue(
      Map<String, Object> bundle, String returnMode, String packageMode) {
    if ("all".equals(returnMode)) {
      Map<String, Object> all = new LinkedHashMap<>();
      for (String key : java.util.List.of("items", "warnings", "errors", "results", "summary")) {
        if (bundle.containsKey(key)) {
          all.put(key, packageBulkValue(bundle, key, packageMode));
        }
      }
      return all;
    }
    Object selected = bundle.get(returnMode);
    if (!(selected instanceof Iterable<?> iterable)
        || "warnings".equals(returnMode)
        || "errors".equals(returnMode)) return selected;
    ArrayList<Object> vector = new ArrayList<>();
    Map<Object, Object> map = new LinkedHashMap<>();
    for (Object value : iterable) {
      Object key = null;
      Object data = value;
      if (value instanceof ILinearType<?> pair && pair.count() == 2) {
        key = pair.nth(0);
        data = pair.nth(1);
        if (data instanceof IMapType<?, ?> haraMap) {
          data = HaraContext.lookupValue(haraMap, Keyword.create("data"));
        } else if (data instanceof Map<?, ?> javaMap) data = javaMap.get("data");
      } else if (value instanceof IMapType<?, ?> haraMap) {
        key = HaraContext.lookupValue(haraMap, Keyword.create("key"));
        if (key == null) key = HaraContext.lookupValue(haraMap, Keyword.create("input"));
        Object nested = HaraContext.lookupValue(haraMap, Keyword.create("data"));
        if (nested != null) data = nested;
      } else if (value instanceof Map<?, ?> javaMap) {
        key = javaMap.get("key");
        if (key == null) key = javaMap.get("input");
        if (javaMap.containsKey("data")) data = javaMap.get("data");
      }
      if ("vector".equals(packageMode)) {
        vector.add(List.Standard.from(null, key, data));
      } else map.put(key, data);
    }
    return "vector".equals(packageMode) ? vector : map;
  }

  private static Object bulkItems(
      HaraContext context, Object[] values, boolean parallel) {
    if (values.length >= 2 && values[0] instanceof HaraFunction) {
      Object function = values[0];
      java.util.List<Object> inputs = asObjects(values[1]);
      Object params =
          values.length > 3 ? values[3] : hara.lang.data.Map.Standard.EMPTY;
      Object lookup =
          values.length > 4 ? values[4] : hara.lang.data.Map.Standard.EMPTY;
      Object env =
          values.length > 5 ? values[5] : hara.lang.data.Map.Standard.EMPTY;
      int extraStart = Math.min(values.length, 6);
      java.util.function.Function<Object, Object> process =
          input -> {
            Object[] callArgs =
                new Object[4 + Math.max(0, values.length - extraStart)];
            callArgs[0] = input;
            callArgs[1] = params;
            callArgs[2] = lookup;
            callArgs[3] = env;
            if (values.length > extraStart) {
              System.arraycopy(
                  values, extraStart, callArgs, 4, values.length - extraStart);
            }
            long start = System.nanoTime();
            try {
              Object returned =
                  HaraBox.unwrap(context.invokeCallable(function, callArgs));
              if (returned instanceof ILinearType<?> pair && pair.count() == 2) {
                Object resultValue = pair.nth(1);
                long elapsed = (System.nanoTime() - start) / 1_000_000L;
                if (resultValue instanceof IMapType<?, ?> resultMap) {
                  @SuppressWarnings("rawtypes")
                  IMapType rawMap = (IMapType) resultMap;
                  return List.Standard.from(
                      null,
                      pair.nth(0),
                      rawMap.assoc(Keyword.create("time"), elapsed));
                }
                if (resultValue instanceof Map<?, ?> resultMap) {
                  Map<Object, Object> withTime = new LinkedHashMap<>(resultMap);
                  withTime.put("time", elapsed);
                  return List.Standard.from(null, pair.nth(0), withTime);
                }
                return returned;
              }
              return bulkItem(input, "RETURN", returned, null, start);
            } catch (Throwable error) {
              return bulkItem(input, "ERROR", "errored", error, start);
            }
          };
      return parallel
          ? inputs.parallelStream().map(process).toList()
          : inputs.stream().map(process).toList();
    }
    if (values.length < 2 || !(values[0] instanceof Task task)) {
      throw new HaraException("bulk-items expects a task and inputs");
    }
    Object inputValue;
    if (values.length >= 3
        && (values[1] instanceof HaraFunction || values[1] instanceof TaskFunction)) {
      task =
          new Task(
              task.type(),
              task.name(),
              toTaskFunction(values[1]),
              task.arglists(),
              task.config());
      inputValue = values[2];
    } else inputValue = values[1];
    java.util.List<Object> inputs = asObjects(inputValue);
    return parallel ? TaskBulk.itemsParallel(task, inputs) : TaskBulk.items(task, inputs);
  }

  private static java.util.List<Object> asObjects(Object value) {
    java.util.List<Object> objects = new ArrayList<>();
    if (value instanceof Iterable<?> iterable) iterable.forEach(objects::add);
    else objects.add(value);
    return objects;
  }

  private static java.util.List<Map<String, Object>> asBulkMaps(Object value) {
    java.util.List<Map<String, Object>> maps = new ArrayList<>();
    for (Object item : asObjects(value)) {
      Map<String, Object> converted = new LinkedHashMap<>();
      if (item instanceof IMapType<?, ?> haraMap) {
        for (Object entryObject : haraMap) {
          Map.Entry<?, ?> entry = (Map.Entry<?, ?>) entryObject;
          converted.put(keyName(entry.getKey()), entry.getValue());
        }
      } else if (item instanceof Map<?, ?> map) {
        map.forEach((key, nested) -> converted.put(keyName(key), nested));
      } else if (item instanceof ILinearType<?> pair && pair.count() == 2) {
        Object result = pair.nth(1);
        converted.put("input", pair.nth(0));
        if (result instanceof IMapType<?, ?> haraMap) {
          for (Object entryObject : haraMap) {
            Map.Entry<?, ?> entry = (Map.Entry<?, ?>) entryObject;
            converted.put(keyName(entry.getKey()), entry.getValue());
          }
        } else if (result instanceof Map<?, ?> map) {
          map.forEach((key, nested) -> converted.put(keyName(key), nested));
        }
      } else continue;
      maps.add(converted);
    }
    return maps;
  }

  private static Object wrapExecuteFunction(
      HaraContext context, Object function, Task task) {
    return context.libraryFunction(
        "std.lib.task/wrapped-execute",
        values -> {
          if (values.length < 4) {
            throw new HaraException(
                "wrapped task expects input, params, lookup, and env");
          }
          Object input = values[0];
          Object params = values[1];
          Object pre = taskConfig(task, "item", "pre");
          Object post = taskConfig(task, "item", "post");
          Object output = taskConfig(task, "item", "output");
          if (pre != null) input = context.invokeCallable(pre, new Object[] {input});
          Object[] callArgs = values.clone();
          callArgs[0] = input;
          Object result = context.invokeCallable(function, callArgs);
          if (post != null) result = context.invokeCallable(post, new Object[] {result});
          boolean bulk =
              params instanceof IMapType<?, ?> haraParams
                  ? Boolean.TRUE.equals(
                      HaraContext.lookupValue(haraParams, Keyword.create("bulk")))
                  : params instanceof Map<?, ?> javaParams
                      && Boolean.TRUE.equals(javaParams.get("bulk"));
          if (bulk) {
            Map<String, Object> packaged = new LinkedHashMap<>();
            packaged.put("status", "RETURN");
            packaged.put("data", result);
            return List.Standard.from(null, input, packaged);
          }
          return output == null
              ? result
              : context.invokeCallable(output, new Object[] {result});
        });
  }

  private static Object taskConfig(Task task, String first, String second) {
    Object group = task.config().get(first);
    return group instanceof Map<?, ?> map ? map.get(second) : null;
  }

  private static Object createTask(HaraContext context, Object[] values) {
    if (values.length == 1 && values[0] instanceof IMapType<?, ?> map) {
      Object type = HaraContext.lookupValue(map, Keyword.create("type"));
      Object name = HaraContext.lookupValue(map, Keyword.create("name"));
      Object main = HaraContext.lookupValue(map, Keyword.create("main"));
      Object function =
          main instanceof IMapType<?, ?> mainMap
              ? HaraContext.lookupValue(mainMap, Keyword.create("fn"))
              : main;
      return newTask(type, name, function, toJavaMap(map));
    }
    if (values.length != 3) {
      throw new HaraException(
          "std.lib.task/task expects type, name, and function/config");
    }
    Object config = values[2];
    Object function =
        config instanceof IMapType<?, ?> map
            ? HaraContext.lookupValue(map, Keyword.create("main")) == null
                ? null
                : HaraContext.lookupValue(
                    (IMapType<?, ?>)
                        HaraContext.lookupValue(map, Keyword.create("main")),
                    Keyword.create("fn"))
            : config;
    return newTask(
        values[0],
        values[1],
        function,
        config instanceof IMapType<?, ?> map ? toJavaMap(map) : Map.of());
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object singleFunctionPrint(Object value) {
    if (!(value instanceof IMapType<?, ?> map)) {
      throw new HaraException("single-function-print expects a map");
    }
    Object bulk = HaraContext.lookupValue(map, Keyword.create("bulk"));
    Object print = HaraContext.lookupValue(map, Keyword.create("print"));
    if (Boolean.TRUE.equals(bulk)) return value;
    if (print instanceof IMapType<?, ?> printMap) {
      if (HaraContext.lookupValue(printMap, Keyword.create("function")) != null) {
        return value;
      }
      return ((IMapType) map)
          .assoc(
              Keyword.create("print"),
              ((IMapType) printMap)
                  .assoc(Keyword.create("function"), Boolean.TRUE));
    }
    return ((IMapType) map)
        .assoc(
            Keyword.create("print"),
            hara.lang.data.Map.Standard.from(
                null, Keyword.create("function"), Boolean.TRUE));
  }

  private static Task newTask(
      Object type, Object name, Object function, Map<String, Object> config) {
    if (name == null || function == null) {
      throw new HaraException("std.lib.task/task requires a name and function");
    }
    String taskType =
        type instanceof Keyword keyword ? keyword.getName() : String.valueOf(type);
    String taskName = String.valueOf(name);
    Object arglists = config.get("arglists");
    Object main = config.get("main");
    if (arglists == null && main instanceof Map<?, ?> mainMap) {
      arglists = mainMap.get("arglists");
    }
    if (arglists == null) {
      arglists =
          List.Standard.from(
              null,
              hara.lang.data.Vector.Standard.EMPTY,
              hara.lang.data.Vector.Standard.from(null, Symbol.create("entry")));
    }
    return new Task(
        taskType, taskName, toTaskFunction(function), arglists, config);
  }

  private static TaskFunction toTaskFunction(Object function) {
    if (function instanceof TaskFunction taskFunction) return taskFunction;
    if (!(function instanceof HaraFunction haraFunction)) {
      throw new HaraException("std.lib.task requires a Hara function");
    }
    return new TaskFunction() {
      @Override
      public Object apply(Object[] arguments) {
        return haraFunction.callTarget().call(haraFunction.callArguments(arguments));
      }

      @Override
      public int minimumArity() {
        return haraFunction.arity() < 0 ? 4 : Math.max(1, haraFunction.arity());
      }

      @Override
      public boolean variadic() {
        return haraFunction.variadic();
      }
    };
  }

  private static Map<String, Object> toJavaMap(IMapType<?, ?> map) {
    Map<String, Object> result = new LinkedHashMap<>();
    for (Object object : map) {
      Map.Entry<?, ?> entry = (Map.Entry<?, ?>) object;
      String key =
          entry.getKey() instanceof Keyword keyword
              ? keyword.getName()
              : String.valueOf(entry.getKey());
      Object value = entry.getValue();
      result.put(
          key, value instanceof IMapType<?, ?> nested ? toJavaMap(nested) : value);
    }
    return result;
  }

  private static Object expandDeftask(List<?> invocation) {
    if (invocation.count() < 3 || !(invocation.nth(1) instanceof Symbol name)) {
      throw new HaraException("deftask expects a name and configuration");
    }
    Object config = invocation.nth(2);
    Object type =
        config instanceof IMapType<?, ?> map
            ? HaraContext.lookupValue(map, Keyword.create("template"))
            : Keyword.create("default");
    Object main =
        config instanceof IMapType<?, ?> map
            ? HaraContext.lookupValue(map, Keyword.create("main"))
            : null;
    if (main instanceof IMapType<?, ?> map) {
      main = HaraContext.lookupValue(map, Keyword.create("fn"));
    }
    if (main == null) throw new HaraException("deftask configuration requires :main");
    return List.Standard.from(
        null,
        Symbol.create("def"),
        withTaskMetadata(name, config),
        List.Standard.from(
            null, Symbol.create("std.lib.task", "task"), type, name.getName(), config));
  }

  private static String keyName(Object key) {
    return key instanceof Keyword keyword
        ? keyword.getName()
        : String.valueOf(key).replaceFirst("^:", "");
  }

  private static Symbol withTaskMetadata(Symbol name, Object config) {
    if (!(config instanceof IMapType<?, ?> map)) return name;
    Object doc = HaraContext.lookupValue(map, Keyword.create("doc"));
    Object arglists = HaraContext.lookupValue(map, Keyword.create("arglists"));
    if (arglists == null) {
      arglists =
          List.Standard.from(
              null,
              hara.lang.data.Vector.Standard.EMPTY,
              hara.lang.data.Vector.Standard.from(null, Symbol.create("entry")));
    }
    ArrayList<Object> metadata = new ArrayList<>();
    if (doc != null) {
      metadata.add(Keyword.create("doc"));
      metadata.add(doc);
    }
    metadata.add(Keyword.create("arglists"));
    metadata.add(arglists);
    return name.withMeta(hara.lang.data.Map.Standard.from(null, metadata.toArray()));
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Object bulkItem(
      Object input, String status, Object data, Throwable error, long start) {
    Map<String, Object> item = new LinkedHashMap<>();
    item.put("input", input);
    item.put("status", status);
    item.put("data", data);
    if (error != null) item.put("error", String.valueOf(error.getMessage()));
    item.put("time", (System.nanoTime() - start) / 1_000_000L);
    return List.Standard.from(null, input, item);
  }

  private static Task taskArgument(String operation, Object[] values) {
    requireArity(operation, values, 1);
    if (!(values[0] instanceof Task task)) {
      throw new HaraException(operation + " expects a task");
    }
    return task;
  }

  private static void requireArity(String operation, Object[] values, int arity) {
    if (values.length != arity) {
      throw new HaraException(operation + " expects " + arity + " arguments");
    }
  }

  private static void requireNonEmpty(String operation, Object[] values) {
    if (values.length == 0) throw new HaraException(operation + " expects arguments");
  }
}
