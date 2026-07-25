package hara.lang.task;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.function.Predicate;
import java.util.regex.Pattern;
import hara.lang.data.types.ILinearType;
import hara.truffle.HaraFunction;

/** Internal execution helpers used by the std.lib.task static facade. */
public final class TaskProcess {
  private TaskProcess() {}

  public static Object[] mainFunction(TaskFunction function, int count) {
    if (count < 1 || count > 4) throw new IllegalArgumentException("count is between 1 and 4");
    int arity = function.minimumArity();
    TaskFunction adapted = new TaskFunction() {
      @Override
      public Object apply(Object[] arguments) throws Exception {
        if (arguments.length < 4) {
          throw new IllegalArgumentException("main function requires input, params, lookup, and env");
        }
        Object[] forwarded = new Object[Math.max(0, arguments.length - (4 - count))];
        if (count == 4) {
          System.arraycopy(arguments, 0, forwarded, 0, arguments.length);
        } else {
          System.arraycopy(arguments, 0, forwarded, 0, count);
          if (arguments.length > 4) {
            System.arraycopy(arguments, 4, forwarded, count, arguments.length - 4);
          }
        }
        return function.apply(forwarded);
      }

      @Override
      public int minimumArity() { return 4; }

      @Override
      public boolean variadic() { return true; }
    };
    return new Object[] {adapted, arity > count};
  }

  public static boolean selectFilter(Object selector, Object id) {
    if (selector instanceof Predicate<?> predicate) {
      @SuppressWarnings("unchecked") Predicate<Object> test = (Predicate<Object>) predicate;
      return test.test(id);
    }
    if (selector instanceof String || selector instanceof Character || selector instanceof Enum<?>) {
      return String.valueOf(id).startsWith(String.valueOf(selector));
    }
    if (selector instanceof Pattern pattern) return pattern.matcher(String.valueOf(id)).find();
    if (selector instanceof Set<?> set) return set.contains(id);
    if (selector instanceof List<?> list) {
      for (Object value : list) if (selectFilter(value, id)) return true;
      return false;
    }
    if (selector instanceof hara.lang.data.Vector<?> vector) {
      for (Object value : vector) if (selectFilter(value, id)) return true;
      return false;
    }
    if (selector instanceof hara.lang.data.List<?> list) {
      for (Object value : list) if (!selectFilter(value, id)) return false;
      return true;
    }
    if (selector instanceof hara.lang.data.Symbol symbol) {
      return String.valueOf(id).startsWith(symbol.display());
    }
    if (selector instanceof hara.lang.data.Keyword keyword) {
      return String.valueOf(id).startsWith(keyword.getName());
    }
    if (selector instanceof hara.lang.data.types.ISetType<?> set) {
      @SuppressWarnings("rawtypes") hara.lang.data.types.ISetType rawSet = (hara.lang.data.types.ISetType) set;
      return rawSet.find(id) != null;
    }
    throw new IllegalArgumentException("Selector not valid: " + selector);
  }

  public static List<?> selectInputs(Task task, Object lookup, Object env, Object selector) throws Exception {
    Object listFunction = nested(task.config(), "item", "list");
    if (!(listFunction instanceof TaskFunction) && !(listFunction instanceof HaraFunction)) {
      throw new IllegalArgumentException("No item.list function defined");
    }
    Object values = invokeFunction(listFunction, new Object[] {lookup, env});
    if ((selector instanceof String && "all".equals(selector))
        || (selector instanceof hara.lang.data.Keyword keyword && "all".equals(keyword.getName()))) {
      return asList(values);
    }
    List<Object> selected = new ArrayList<>();
    for (Object value : asList(values)) if (selectFilter(selector, value)) selected.add(value);
    return selected;
  }

  public static Map<String, Object> taskInputs(Task task, Object... supplied) throws Exception {
    Object input;
    Map<String, Object> params;
    if (supplied.length == 0) {
      input = construct(task, "input", task);
      params = Map.of();
    } else if (supplied.length == 1 && supplied[0] instanceof Map<?, ?>) {
      input = construct(task, "input", task);
      params = asOptions(supplied[0]);
    } else {
      input = supplied[0];
      params = supplied.length > 1 ? asOptions(supplied[1]) : Map.of();
    }
    Object env = supplied.length > 2 ? supplied[2] : construct(task, "env", task);
    Object lookup = supplied.length > 3 ? supplied[3] : construct(task, "lookup", task, env);
    if (supplied.length > 4) {
      // Additional values are intentionally ignored here; invoke consumes them after the
      // constructed input tuple, matching the reference function's variadic tail.
    }
    Map<String, Object> result = new java.util.LinkedHashMap<>();
    result.put("input", input);
    result.put("params", params);
    result.put("lookup", lookup);
    result.put("env", env);
    return result;
  }

  private static Object construct(Task task, String key, Object... arguments) throws Exception {
    Object construct = task.config().get("construct");
    if (construct instanceof Map<?, ?> map) {
      Object function = map.get(key);
      if (function != null) return invokeFunction(function, arguments);
    }
    return Map.of();
  }

  public static Map<String, Object> processNamespaceArgs(String... args) {
    Map<String, Object> result = new java.util.LinkedHashMap<>();
    for (int i = 0; i < args.length; i++) {
      String rawKey = args[i];
      if (rawKey == null || !rawKey.startsWith(":")) continue;
      String key = rawKey.substring(1);
      if (i + 1 >= args.length || (args[i + 1] != null && args[i + 1].startsWith(":"))) {
        result.put(key, Boolean.TRUE);
        continue;
      }
      Object value = parseArgument(args[++i]);
      if ("only".equals(key)) result.put("ns", value);
      else if ("with".equals(key)) result.put("ns", List.of(value));
      else result.put(key, value);
    }
    return result;
  }

  private static Object parseArgument(String value) {
    if (value == null) return null;
    if ("nil".equals(value)) return null;
    if ("true".equals(value)) return Boolean.TRUE;
    if ("false".equals(value)) return Boolean.FALSE;
    try {
      if (value.matches("[+-]?\\d+")) return Long.valueOf(value);
      if (value.matches("[+-]?(?:\\d+\\.\\d*|\\d*\\.\\d+)")) return Double.valueOf(value);
    } catch (NumberFormatException ignored) {
      // Fall through to symbol parsing for malformed numeric-looking values.
    }
    {
      // Command-line namespace arguments are symbols in the source implementation.
      if (value.startsWith("'") && value.length() > 1) value = value.substring(1);
      return hara.lang.data.Symbol.create(value);
    }
  }

  public static Object invoke(Task task, Object... args) throws Exception {
    Objects.requireNonNull(task, "task");
    if (task.config().containsKey("item")
        || task.config().containsKey("construct")
        || task.config().containsKey("main")) {
      if (args.length > 0 && args[0] instanceof hara.lang.data.Keyword keyword) {
        List<?> selected = selectInputs(
            task,
            args.length > 2 ? args[2] : Map.of(),
            args.length > 3 ? args[3] : Map.of(),
            keyword);
        List<Object> results = new ArrayList<>();
        for (Object input : selected) {
          Object[] one = args.clone();
          one[0] = input;
          results.add(invokeProcess(task, one));
        }
        return results;
      }
      if (args.length > 0 && isBulkInput(args[0])) {
        List<Object> results = new ArrayList<>();
        for (Object input : asList(args[0])) {
          Object[] one = args.clone();
          one[0] = input;
          results.add(invokeProcess(task, one));
        }
        return results;
      }
      return invokeProcess(task, args);
    }
    if (args.length > 0 && (args[0] instanceof List<?> || args[0] instanceof ILinearType<?>)) {
      List<Object> results = new ArrayList<>();
      for (Object input : asList(args[0])) results.add(invokeOne(task, input, args));
      return results;
    }
    return task.invoke(args);
  }

  @SuppressWarnings("unchecked")
  private static Object invokeProcess(Task task, Object[] args) throws Exception {
    Object input = args.length == 0 ? null : args[0];
    Map<String, Object> params = asOptions(args.length > 1 ? args[1] : null);
    Object lookup = args.length > 2 ? args[2] : Map.of();
    Object env = args.length > 3 ? args[3] : Map.of();
    Object prepared = applyOptional(nested(task.config(), "item", "pre"), input);
    int fixedCount = task.function().variadic()
        ? 4 : Math.max(1, Math.min(4, task.function().minimumArity()));
    int extraCount = Math.max(0, args.length - 4);
    Object[] functionArgs = new Object[fixedCount + extraCount];
    if (fixedCount > 0) functionArgs[0] = prepared;
    if (fixedCount > 1) functionArgs[1] = params;
    if (fixedCount > 2) functionArgs[2] = lookup;
    if (fixedCount > 3) functionArgs[3] = env;
    if (extraCount > 0) System.arraycopy(args, 4, functionArgs, fixedCount, extraCount);
    Object result = task.invoke(functionArgs);
    result = applyOptional(nested(task.config(), "item", "post"), result);
    if (Boolean.TRUE.equals(params.get("bulk"))) {
      return List.of(prepared, Map.of("status", "RETURN", "data", result));
    }
    return applyOptional(nested(task.config(), "item", "output"), result);
  }

  private static Object applyOptional(Object function, Object value) throws Exception {
    return function == null ? value : invokeFunction(function, new Object[] {value});
  }

  private static Object invokeFunction(Object function, Object[] arguments) throws Exception {
    if (function instanceof TaskFunction taskFunction) return taskFunction.apply(arguments);
    if (function instanceof HaraFunction haraFunction) {
      return haraFunction.callTarget().call(haraFunction.callArguments(arguments));
    }
    return function;
  }

  private static Object invokeOne(Task task, Object input, Object[] args) throws Exception {
    Object[] forwarded = args.clone();
    forwarded[0] = input;
    return task.invoke(forwarded);
  }

  private static Object nested(Map<String, Object> config, String first, String second) {
    Object value = config.get(first);
    return value instanceof Map<?, ?> map ? map.get(second) : null;
  }

  @SuppressWarnings({"rawtypes", "unchecked"})
  private static Map<String, Object> asOptions(Object value) {
    if (value instanceof Map<?, ?> map) return (Map<String, Object>) map;
    if (value instanceof hara.lang.data.types.IMapType<?, ?> map) {
      Map<String, Object> result = new java.util.LinkedHashMap<>();
      for (Object object : map) {
        java.util.Map.Entry entry = (java.util.Map.Entry) object;
        Object key = entry.getKey();
        result.put(key instanceof hara.lang.data.Keyword keyword ? keyword.getName() : String.valueOf(key), entry.getValue());
      }
      return result;
    }
    return Map.of();
  }

  private static List<?> asList(Object value) {
    if (value instanceof List<?> list) return list;
    if (value instanceof Iterable<?> iterable) {
      List<Object> result = new ArrayList<>();
      iterable.forEach(result::add);
      return result;
    }
    return List.of(value);
  }

  private static boolean isBulkInput(Object value) {
    return value instanceof List<?>
        || value instanceof ILinearType<?>
        || value instanceof hara.lang.data.Set<?>;
  }

  public interface Arity { int minimumArity(); }
}
